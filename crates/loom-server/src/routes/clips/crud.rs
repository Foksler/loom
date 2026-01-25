// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! CRUD handlers for clips (create, get, update, delete).

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_auth::types::{OrgId, OrgRole};
use loom_server_db::clips::{ClipsStore, CreateClipParams, UpdateClipParams};
use uuid::Uuid;

use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{
	clip_record_to_response, detect_language_from_path, parse_visibility, ClipResponse,
	CreateClipRequest, UpdateClipRequest,
};
use super::types::ClipsErrorResponse;

#[utoipa::path(
    post,
    path = "/api/clips",
    request_body = CreateClipRequest,
    responses(
        (status = 201, description = "Clip created", body = ClipResponse),
        (status = 400, description = "Invalid request", body = ClipsErrorResponse),
        (status = 403, description = "Not authorized", body = ClipsErrorResponse),
        (status = 409, description = "Clip already exists", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state, payload))]
pub async fn create_clip(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Json(payload): Json<CreateClipRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let clips_repo = match state.clips_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let clips_git = match state.clips_git_store.as_ref() {
		Some(git) => git,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Validate name
	if payload.name.is_empty() || payload.name.len() > 100 {
		return (
			StatusCode::BAD_REQUEST,
			Json(ClipsErrorResponse {
				error: "invalid_name".to_string(),
				message: t(locale, "server.api.clips.name_invalid").to_string(),
			}),
		)
			.into_response();
	}

	// Determine owner org: use provided org_id or user's personal org
	let org = if let Some(org_uuid) = payload.org_id {
		// Check org membership
		let org_id = OrgId::new(org_uuid);
		let _membership = match state
			.org_repo
			.get_membership(&org_id, &current_user.user.id)
			.await
		{
			Ok(Some(m)) => m,
			Ok(None) => {
				return (
					StatusCode::FORBIDDEN,
					Json(ClipsErrorResponse {
						error: "forbidden".to_string(),
						message: t(locale, "server.api.error.forbidden").to_string(),
					}),
				)
					.into_response();
			}
			Err(e) => {
				tracing::error!(error = %e, "Failed to check org membership");
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(ClipsErrorResponse {
						error: "internal_error".to_string(),
						message: t(locale, "server.api.error.internal").to_string(),
					}),
				)
					.into_response();
			}
		};

		// Get org for the owner name
		match state.org_repo.get_org_by_id(&org_id).await {
			Ok(Some(o)) => o,
			Ok(None) => {
				return (
					StatusCode::NOT_FOUND,
					Json(ClipsErrorResponse {
						error: "not_found".to_string(),
						message: t(locale, "server.api.clips.org_not_found").to_string(),
					}),
				)
					.into_response();
			}
			Err(e) => {
				tracing::error!(error = %e, "Failed to get organization");
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(ClipsErrorResponse {
						error: "internal_error".to_string(),
						message: t(locale, "server.api.error.internal").to_string(),
					}),
				)
					.into_response();
			}
		}
	} else {
		// Personal clip - get or create user's personal org
		match state.org_repo.ensure_personal_org(&current_user.user.id).await {
			Ok(o) => o,
			Err(e) => {
				tracing::error!(error = %e, "Failed to ensure personal org");
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(ClipsErrorResponse {
						error: "internal_error".to_string(),
						message: t(locale, "server.api.error.internal").to_string(),
					}),
				)
					.into_response();
			}
		}
	};

	let owner_slug = org.slug.clone();
	let org_id_for_record = Some(org.id.into_inner());

	// Check if clip name already exists for this owner
	if let Ok(true) = clips_repo.clip_name_exists(&owner_slug, &payload.name).await {
		return (
			StatusCode::CONFLICT,
			Json(ClipsErrorResponse {
				error: "already_exists".to_string(),
				message: t(locale, "server.api.clips.name_exists").to_string(),
			}),
		)
			.into_response();
	}

	// Create clip record
	let clip_id = Uuid::now_v7();
	let visibility = parse_visibility(payload.visibility.as_deref());

	let params = CreateClipParams {
		id: clip_id,
		owner: owner_slug.clone(),
		name: payload.name.clone(),
		description: payload.description.clone(),
		visibility,
		created_by: current_user.user.id.into_inner(),
		org_id: org_id_for_record,
		is_fork: false,
		forked_from: None,
	};

	let clip = match clips_repo.create_clip(params).await {
		Ok(c) => c,
		Err(e) => {
			tracing::error!(error = %e, "Failed to create clip");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Initialize git repository
	let clip_id_typed = loom_server_clips::ClipId(clip_id);
	if let Err(e) = clips_git.init_repo(clip_id_typed).await {
		tracing::error!(error = %e, "Failed to init git repo");
		let _ = clips_repo.delete_clip(clip_id).await;
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ClipsErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.clips.repo_init_failed").to_string(),
			}),
		)
			.into_response();
	}

	// Commit initial files if provided
	if !payload.files.is_empty() {
		let files: Vec<(String, String)> = payload
			.files
			.iter()
			.map(|f| (f.path.clone(), f.content.clone()))
			.collect();

		let author_email = current_user
			.user
			.primary_email
			.as_deref()
			.unwrap_or("noreply@loom.dev");

		if let Err(e) = clips_git
			.commit_files(
				clip_id_typed,
				&files,
				&current_user.user.display_name,
				author_email,
				"Initial commit",
			)
			.await
		{
			tracing::warn!(error = %e, "Failed to commit initial files");
		} else {
			// Update stats
			let file_count = payload.files.len() as u32;
			let size_bytes: u64 = payload.files.iter().map(|f| f.content.len() as u64).sum();
			let language = payload
				.files
				.first()
				.and_then(|f| detect_language_from_path(&f.path));

			let _ = clips_repo
				.update_clip_stats(clip_id, file_count, size_bytes, language.as_deref())
				.await;
		}
	}

	tracing::info!(
		clip_id = %clip_id,
		name = %payload.name,
		org_id = ?org_id_for_record,
		owner = %owner_slug,
		created_by = %current_user.user.id,
		"Clip created"
	);

	// Audit log
	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::ClipCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("clip", clip_id.to_string())
			.details(serde_json::json!({
				"org_id": org_id_for_record.map(|id| id.to_string()),
				"owner": owner_slug,
				"name": payload.name,
				"visibility": format!("{:?}", visibility),
				"file_count": payload.files.len(),
			}))
			.build(),
	);
	(
		StatusCode::CREATED,
		Json(clip_record_to_response(clip, &state.base_url)),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/clips/{owner}/{name}",
    params(
        ("owner" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Clip name")
    ),
    responses(
        (status = 200, description = "Clip found", body = ClipResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn get_clip(
	State(state): State<AppState>,
	Path((owner, name)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = &state.default_locale;

	let clips_repo = match state.clips_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.clips.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let clip = match clips_repo.get_clip_by_owner_name(&owner, &name).await {
		Ok(Some(c)) => c,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ClipsErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.clips.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get clip");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	(
		StatusCode::OK,
		Json(clip_record_to_response(clip, &state.base_url)),
	)
		.into_response()
}

#[utoipa::path(
    patch,
    path = "/api/clips/{id}",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    request_body = UpdateClipRequest,
    responses(
        (status = 200, description = "Clip updated", body = ClipResponse),
        (status = 403, description = "Not authorized", body = ClipsErrorResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state, payload))]
pub async fn update_clip(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	Json(payload): Json<UpdateClipRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let clips_repo = match state.clips_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get existing clip
	let clip = match clips_repo.get_clip_by_id(id).await {
		Ok(Some(c)) => c,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ClipsErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.clips.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get clip");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Check authorization (must be org member)
	if let Some(org_id) = clip.org_id {
		let org_id = OrgId::new(org_id);
		match state
			.org_repo
			.get_membership(&org_id, &current_user.user.id)
			.await
		{
			Ok(Some(m)) => {
				if m.role != OrgRole::Owner && m.role != OrgRole::Admin {
					return (
						StatusCode::FORBIDDEN,
						Json(ClipsErrorResponse {
							error: "forbidden".to_string(),
							message: t(locale, "server.api.error.forbidden").to_string(),
						}),
					)
						.into_response();
				}
			}
			Ok(None) => {
				return (
					StatusCode::FORBIDDEN,
					Json(ClipsErrorResponse {
						error: "forbidden".to_string(),
						message: t(locale, "server.api.error.forbidden").to_string(),
					}),
				)
					.into_response();
			}
			Err(e) => {
				tracing::error!(error = %e, "Failed to check org membership");
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(ClipsErrorResponse {
						error: "internal_error".to_string(),
						message: t(locale, "server.api.error.internal").to_string(),
					}),
				)
					.into_response();
			}
		}
	} else if clip.created_by != current_user.user.id.into_inner() {
		return (
			StatusCode::FORBIDDEN,
			Json(ClipsErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.error.forbidden").to_string(),
			}),
		)
			.into_response();
	}

	// Update clip
	let params = UpdateClipParams {
		name: payload.name,
		description: payload.description,
		visibility: payload.visibility.as_deref().map(|v| parse_visibility(Some(v))),
	};

	if let Err(e) = clips_repo.update_clip(id, params).await {
		tracing::error!(error = %e, "Failed to update clip");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ClipsErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	// Fetch updated clip
	let updated_clip = match clips_repo.get_clip_by_id(id).await {
		Ok(Some(c)) => c,
		_ => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	tracing::info!(clip_id = %id, updated_by = %current_user.user.id, "Clip updated");

	// Audit log
	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::ClipUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("clip", id.to_string())
			.details(serde_json::json!({
				"name": updated_clip.name,
				"visibility": format!("{:?}", updated_clip.visibility),
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(clip_record_to_response(updated_clip, &state.base_url)),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/clips/{id}",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    responses(
        (status = 204, description = "Clip deleted"),
        (status = 403, description = "Not authorized", body = ClipsErrorResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn delete_clip(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let clips_repo = match state.clips_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let clips_git = match state.clips_git_store.as_ref() {
		Some(git) => git,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get existing clip
	let clip = match clips_repo.get_clip_by_id(id).await {
		Ok(Some(c)) => c,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ClipsErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.clips.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get clip");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Check authorization
	if let Some(org_id) = clip.org_id {
		let org_id = OrgId::new(org_id);
		match state
			.org_repo
			.get_membership(&org_id, &current_user.user.id)
			.await
		{
			Ok(Some(m)) => {
				if m.role != OrgRole::Owner && m.role != OrgRole::Admin {
					return (
						StatusCode::FORBIDDEN,
						Json(ClipsErrorResponse {
							error: "forbidden".to_string(),
							message: t(locale, "server.api.error.forbidden").to_string(),
						}),
					)
						.into_response();
				}
			}
			Ok(None) => {
				return (
					StatusCode::FORBIDDEN,
					Json(ClipsErrorResponse {
						error: "forbidden".to_string(),
						message: t(locale, "server.api.error.forbidden").to_string(),
					}),
				)
					.into_response();
			}
			Err(e) => {
				tracing::error!(error = %e, "Failed to check org membership");
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(ClipsErrorResponse {
						error: "internal_error".to_string(),
						message: t(locale, "server.api.error.internal").to_string(),
					}),
				)
					.into_response();
			}
		}
	} else if clip.created_by != current_user.user.id.into_inner() {
		return (
			StatusCode::FORBIDDEN,
			Json(ClipsErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.error.forbidden").to_string(),
			}),
		)
			.into_response();
	}

	// Delete git repo
	let clip_id_typed = loom_server_clips::ClipId(id);
	if let Err(e) = clips_git.delete_repo(clip_id_typed).await {
		tracing::warn!(error = %e, "Failed to delete git repo");
	}

	// Delete from database
	if let Err(e) = clips_repo.delete_clip(id).await {
		tracing::error!(error = %e, "Failed to delete clip");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ClipsErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(clip_id = %id, deleted_by = %current_user.user.id, "Clip deleted");

	// Audit log
	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::ClipDeleted)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("clip", id.to_string())
			.details(serde_json::json!({
				"name": clip.name,
				"org_id": clip.org_id.map(|id| id.to_string()),
			}))
			.build(),
	);

	StatusCode::NO_CONTENT.into_response()
}
