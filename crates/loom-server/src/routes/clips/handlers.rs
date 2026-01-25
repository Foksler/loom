// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Clips route handlers.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_auth::types::{OrgId, OrgRole};
use loom_server_db::clips::{ClipVisibility, ClipsStore, CreateClipParams, UpdateClipParams};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::types::ClipsErrorResponse;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateClipRequest {
	pub org_id: Uuid,
	pub name: String,
	pub description: Option<String>,
	pub visibility: Option<String>,
	pub files: Vec<CreateClipFile>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateClipFile {
	pub path: String,
	pub content: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateClipRequest {
	pub name: Option<String>,
	pub description: Option<String>,
	pub visibility: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForkClipRequest {
	pub target_org_id: Uuid,
	pub name: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListClipsQuery {
	pub page: Option<u32>,
	pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchClipsQuery {
	/// Search query
	pub q: String,
	pub page: Option<u32>,
	pub per_page: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipSearchHitResponse {
	pub clip: ClipResponse,
	pub score: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipSearchResponse {
	pub hits: Vec<ClipSearchHitResponse>,
	pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipResponse {
	pub id: Uuid,
	pub org_id: Option<Uuid>,
	pub owner: String,
	pub name: String,
	pub description: Option<String>,
	pub visibility: String,
	pub clone_url: String,
	pub file_count: u32,
	pub size_bytes: u64,
	pub language: Option<String>,
	pub star_count: u32,
	pub is_fork: bool,
	pub forked_from: Option<Uuid>,
	pub created_at: String,
	pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StarClipResponse {
	pub starred: bool,
	pub star_count: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipListResponse {
	pub clips: Vec<ClipResponse>,
	pub total: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipFileResponse {
	pub path: String,
	pub content: String,
	pub size: u64,
	pub language: Option<String>,
	pub is_redacted: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipFilesResponse {
	pub files: Vec<ClipFileResponse>,
	pub revision: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipRevisionResponse {
	pub sha: String,
	pub author_name: String,
	pub author_email: String,
	pub timestamp: String,
	pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipRevisionsResponse {
	pub revisions: Vec<ClipRevisionResponse>,
}

// ============================================================================
// Helper functions
// ============================================================================

fn build_clip_url(base_url: &str, owner: &str, name: &str) -> String {
	format!("{}/git/clips/{}/{}.git", base_url.trim_end_matches('/'), owner, name)
}

fn clip_record_to_response(
	clip: loom_server_db::clips::ClipRecord,
	base_url: &str,
) -> ClipResponse {
	ClipResponse {
		id: clip.id,
		org_id: clip.org_id,
		owner: clip.owner.clone(),
		name: clip.name.clone(),
		description: clip.description,
		visibility: clip.visibility.to_string(),
		clone_url: build_clip_url(base_url, &clip.owner, &clip.name),
		file_count: clip.file_count,
		size_bytes: clip.size_bytes,
		language: clip.language,
		star_count: clip.star_count,
		is_fork: clip.is_fork,
		forked_from: clip.forked_from,
		created_at: clip.created_at.to_rfc3339(),
		updated_at: clip.updated_at.to_rfc3339(),
	}
}

fn parse_visibility(s: Option<&str>) -> ClipVisibility {
	match s {
		Some("public") => ClipVisibility::Public,
		Some("internal") => ClipVisibility::Internal,
		_ => ClipVisibility::Private,
	}
}

// ============================================================================
// Handlers
// ============================================================================

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

	// Check org membership
	let org_id = OrgId::new(payload.org_id);
	let membership = match state
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
	let org = match state.org_repo.get_org_by_id(&org_id).await {
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
	};

	// Check if clip name already exists
	if let Ok(true) = clips_repo.clip_name_exists(&org.slug, &payload.name).await {
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
		owner: org.slug.clone(),
		name: payload.name.clone(),
		description: payload.description.clone(),
		visibility,
		created_by: current_user.user.id.into_inner(),
		org_id: Some(payload.org_id),
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
		org_id = %payload.org_id,
		created_by = %current_user.user.id,
		"Clip created"
	);

	// Audit log
	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::ClipCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("clip", clip_id.to_string())
			.details(serde_json::json!({
				"org_id": payload.org_id.to_string(),
				"name": payload.name,
				"visibility": format!("{:?}", visibility),
				"file_count": payload.files.len(),
			}))
			.build(),
	);

	let _ = membership;
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

#[utoipa::path(
    get,
    path = "/api/users/{user_id}/clips",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
        ListClipsQuery
    ),
    responses(
        (status = 200, description = "User's clips", body = ClipListResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_user_clips(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(user_id): Path<Uuid>,
	Query(query): Query<ListClipsQuery>,
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

	let page = query.page.unwrap_or(1);
	let per_page = query.per_page.unwrap_or(20).min(100);
	let offset = (page.saturating_sub(1)) * per_page;

	let clips = match clips_repo.list_user_clips(user_id, per_page, offset).await {
		Ok(c) => c,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list user clips");
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

	let response = ClipListResponse {
		total: clips.len() as i64,
		clips: clips
			.into_iter()
			.map(|c| clip_record_to_response(c, &state.base_url))
			.collect(),
	};

	(StatusCode::OK, Json(response)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/clips",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID"),
        ListClipsQuery
    ),
    responses(
        (status = 200, description = "Organization's clips", body = ClipListResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_org_clips(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<Uuid>,
	Query(query): Query<ListClipsQuery>,
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

	let page = query.page.unwrap_or(1);
	let per_page = query.per_page.unwrap_or(20).min(100);
	let offset = (page.saturating_sub(1)) * per_page;

	let clips = match clips_repo.list_org_clips(org_id, per_page, offset).await {
		Ok(c) => c,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list org clips");
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

	let response = ClipListResponse {
		total: clips.len() as i64,
		clips: clips
			.into_iter()
			.map(|c| clip_record_to_response(c, &state.base_url))
			.collect(),
	};

	(StatusCode::OK, Json(response)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/clips",
    params(ListClipsQuery),
    responses(
        (status = 200, description = "Public clips", body = ClipListResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_public_clips(
	State(state): State<AppState>,
	Query(query): Query<ListClipsQuery>,
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

	let page = query.page.unwrap_or(1);
	let per_page = query.per_page.unwrap_or(20).min(100);
	let offset = (page.saturating_sub(1)) * per_page;

	let clips = match clips_repo.list_public_clips(per_page, offset).await {
		Ok(c) => c,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list public clips");
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

	let response = ClipListResponse {
		total: clips.len() as i64,
		clips: clips
			.into_iter()
			.map(|c| clip_record_to_response(c, &state.base_url))
			.collect(),
	};

	(StatusCode::OK, Json(response)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/clips/search",
    params(SearchClipsQuery),
    responses(
        (status = 200, description = "Search results", body = ClipSearchResponse),
        (status = 400, description = "Invalid query", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn search_clips(
	State(state): State<AppState>,
	Query(query): Query<SearchClipsQuery>,
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

	let search_query = query.q.trim();
	if search_query.is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(ClipsErrorResponse {
				error: "invalid_query".to_string(),
				message: t(locale, "server.api.clips.search_empty").to_string(),
			}),
		)
			.into_response();
	}

	let page = query.page.unwrap_or(1);
	let per_page = query.per_page.unwrap_or(20).min(50);
	let offset = (page.saturating_sub(1)) * per_page;

	let hits = match clips_repo
		.search_public_clips(search_query, per_page, offset)
		.await
	{
		Ok(h) => h,
		Err(e) => {
			tracing::error!(error = %e, "Failed to search clips");
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

	let total = hits.len();
	let response = ClipSearchResponse {
		total,
		hits: hits
			.into_iter()
			.map(|hit| ClipSearchHitResponse {
				clip: clip_record_to_response(hit.clip, &state.base_url),
				score: hit.score,
			})
			.collect(),
	};

	(StatusCode::OK, Json(response)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/clips/{id}/files",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    responses(
        (status = 200, description = "Clip files", body = ClipFilesResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_clip_files(
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
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

	let clips_git = match state.clips_git_store.as_ref() {
		Some(git) => git,
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

	// Verify clip exists
	match clips_repo.get_clip_by_id(id).await {
		Ok(Some(_)) => {}
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
	}

	let clip_id = loom_server_clips::ClipId(id);

	// Get file list
	let file_paths = match clips_git.list_files(clip_id, None).await {
		Ok(files) => files,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list clip files");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ClipsErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.clips.list_files_failed").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Read each file with redaction
	let mut files = Vec::new();
	for path in file_paths {
		match clips_git.read_file_redacted(clip_id, &path, None).await {
			Ok(file) => {
				files.push(ClipFileResponse {
					path: file.path,
					content: file.content,
					size: file.size_bytes,
					language: file.language,
					is_redacted: file.is_redacted,
				});
			}
			Err(e) => {
				tracing::warn!(error = %e, path = %path, "Failed to read file");
			}
		}
	}

	// Get current revision
	let revision = clips_git
		.get_head_commit(clip_id)
		.await
		.ok()
		.flatten()
		.unwrap_or_else(|| "HEAD".to_string());

	let response = ClipFilesResponse { files, revision };

	(StatusCode::OK, Json(response)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/clips/{id}/files/{path}",
    params(
        ("id" = Uuid, Path, description = "Clip ID"),
        ("path" = String, Path, description = "File path")
    ),
    responses(
        (status = 200, description = "File content", body = ClipFileResponse),
        (status = 404, description = "File not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn get_clip_file(
	State(state): State<AppState>,
	Path((id, path)): Path<(Uuid, String)>,
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

	let clips_git = match state.clips_git_store.as_ref() {
		Some(git) => git,
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

	// Verify clip exists
	match clips_repo.get_clip_by_id(id).await {
		Ok(Some(_)) => {}
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
	}

	let clip_id = loom_server_clips::ClipId(id);

	// Read file with redaction
	match clips_git.read_file_redacted(clip_id, &path, None).await {
		Ok(file) => {
			let response = ClipFileResponse {
				path: file.path,
				content: file.content,
				size: file.size_bytes,
				language: file.language,
				is_redacted: file.is_redacted,
			};
			(StatusCode::OK, Json(response)).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to read file");
			(
				StatusCode::NOT_FOUND,
				Json(ClipsErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.clips.file_not_found").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
    get,
    path = "/api/clips/{id}/raw/{path}",
    params(
        ("id" = Uuid, Path, description = "Clip ID"),
        ("path" = String, Path, description = "File path")
    ),
    responses(
        (status = 200, description = "Raw file content"),
        (status = 404, description = "File not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn get_clip_file_raw(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((id, path)): Path<(Uuid, String)>,
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

	// Verify clip exists and user has access
	match clips_repo.get_clip_by_id(id).await {
		Ok(Some(_)) => {}
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
	}

	let clip_id = loom_server_clips::ClipId(id);

	// Read raw file (no redaction)
	match clips_git.read_file_raw(clip_id, &path, None).await {
		Ok(bytes) => {
			// Detect content type from path
			let content_type = detect_content_type(&path);
			(
				StatusCode::OK,
				[(axum::http::header::CONTENT_TYPE, content_type)],
				bytes,
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to read file");
			(
				StatusCode::NOT_FOUND,
				Json(ClipsErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.clips.file_not_found").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFilesRequest {
	pub files: Vec<CreateClipFile>,
	pub message: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/clips/{id}/files",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    request_body = UpdateFilesRequest,
    responses(
        (status = 200, description = "Files updated", body = ClipFilesResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse),
        (status = 403, description = "Not authorized", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state, payload))]
pub async fn update_clip_files(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	Json(payload): Json<UpdateFilesRequest>,
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

	// Get clip and verify ownership
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

	// Check if user has write access
	let has_access = if let Some(org_id) = clip.org_id {
		let org_id = OrgId::new(org_id);
		matches!(
			state.org_repo.get_membership(&org_id, &current_user.user.id).await,
			Ok(Some(_))
		)
	} else {
		clip.created_by == current_user.user.id.into_inner()
	};

	if !has_access {
		return (
			StatusCode::FORBIDDEN,
			Json(ClipsErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.error.forbidden").to_string(),
			}),
		)
			.into_response();
	}

	// Prepare files for commit
	let files: Vec<(String, String)> = payload
		.files
		.iter()
		.map(|f| (f.path.clone(), f.content.clone()))
		.collect();

	let commit_message = payload
		.message
		.unwrap_or_else(|| "Update files".to_string());

	let author_name = &current_user.user.display_name;
	let author_email = current_user
		.user
		.primary_email
		.as_deref()
		.unwrap_or("user@loom.local");

	let clip_id = loom_server_clips::ClipId(id);

	// Commit files
	let commit_hash = match clips_git
		.commit_files(clip_id, &files, author_name, author_email, &commit_message)
		.await
	{
		Ok(hash) => hash,
		Err(e) => {
			tracing::error!(error = %e, "Failed to commit files");
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

	// Update clip stats
	let file_count = files.len() as u32;
	let total_size: u64 = files.iter().map(|(_, content)| content.len() as u64).sum();
	let language = files
		.first()
		.and_then(|(path, _)| detect_language_from_path(path));

	let _ = clips_repo
		.update_clip_stats(id, file_count, total_size, language.as_deref())
		.await;

	// Build response by reading files back with redaction
	let mut file_responses = Vec::new();
	for (path, _) in &files {
		match clips_git.read_file_redacted(clip_id, path, None).await {
			Ok(file) => {
				file_responses.push(ClipFileResponse {
					path: file.path,
					content: file.content,
					size: file.size_bytes,
					language: file.language,
					is_redacted: file.is_redacted,
				});
			}
			Err(e) => {
				tracing::warn!(error = %e, path = %path, "Failed to read back file");
			}
		}
	}

	// Audit log
	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::ClipPushed)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("clip", id.to_string())
			.details(serde_json::json!({
				"commit": commit_hash,
				"file_count": files.len(),
				"message": commit_message,
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(ClipFilesResponse {
			files: file_responses,
			revision: commit_hash.clone(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/clips/{id}/fork",
    params(
        ("id" = Uuid, Path, description = "Clip ID to fork")
    ),
    request_body = ForkClipRequest,
    responses(
        (status = 201, description = "Clip forked", body = ClipResponse),
        (status = 403, description = "Not authorized", body = ClipsErrorResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state, payload))]
pub async fn fork_clip(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	Json(payload): Json<ForkClipRequest>,
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

	// Get source clip
	let source_clip = match clips_repo.get_clip_by_id(id).await {
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

	// Check target org membership
	let target_org_id = OrgId::new(payload.target_org_id);
	let membership = match state
		.org_repo
		.get_membership(&target_org_id, &current_user.user.id)
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

	// Get target org
	let target_org = match state.org_repo.get_org_by_id(&target_org_id).await {
		Ok(Some(o)) => o,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ClipsErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.clips.target_org_not_found").to_string(),
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
	};

	// Determine fork name
	let fork_name = payload.name.unwrap_or_else(|| source_clip.name.clone());

	// Check if name already exists
	if let Ok(true) = clips_repo.clip_name_exists(&target_org.slug, &fork_name).await {
		return (
			StatusCode::CONFLICT,
			Json(ClipsErrorResponse {
				error: "already_exists".to_string(),
				message: t(locale, "server.api.clips.name_exists").to_string(),
			}),
		)
			.into_response();
	}

	// Create forked clip record
	let fork_id = Uuid::now_v7();
	let params = CreateClipParams {
		id: fork_id,
		owner: target_org.slug.clone(),
		name: fork_name.clone(),
		description: source_clip.description.clone(),
		visibility: source_clip.visibility,
		created_by: current_user.user.id.into_inner(),
		org_id: Some(payload.target_org_id),
		is_fork: true,
		forked_from: Some(source_clip.id),
	};

	let forked_clip = match clips_repo.create_clip(params).await {
		Ok(c) => c,
		Err(e) => {
			tracing::error!(error = %e, "Failed to create forked clip");
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

	// Clone git repository
	let source_clip_id = loom_server_clips::ClipId(id);
	let fork_clip_id = loom_server_clips::ClipId(fork_id);

	if let Err(e) = clips_git.clone_repo(source_clip_id, fork_clip_id).await {
		tracing::error!(error = %e, "Failed to clone git repo");
		let _ = clips_repo.delete_clip(fork_id).await;
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ClipsErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.clips.repo_clone_failed").to_string(),
			}),
		)
			.into_response();
	}

	// Copy stats
	let _ = clips_repo
		.update_clip_stats(
			fork_id,
			source_clip.file_count,
			source_clip.size_bytes,
			source_clip.language.as_deref(),
		)
		.await;

	tracing::info!(
		fork_id = %fork_id,
		source_id = %id,
		forked_by = %current_user.user.id,
		"Clip forked"
	);

	// Audit log
	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::ClipForked)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("clip", fork_id.to_string())
			.details(serde_json::json!({
				"source_clip_id": id.to_string(),
				"target_org_id": payload.target_org_id.to_string(),
				"name": fork_name,
			}))
			.build(),
	);

	let _ = membership;
	(
		StatusCode::CREATED,
		Json(clip_record_to_response(forked_clip, &state.base_url)),
	)
		.into_response()
}

// ============================================================================
// Revisions Handler
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/clips/{id}/revisions",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    responses(
        (status = 200, description = "List of revisions", body = ClipRevisionsResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_clip_revisions(
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

	// Verify clip exists
	match clips_repo.get_clip_by_id(id).await {
		Ok(Some(_)) => {}
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
	}

	// Get revisions from git
	let clip_id = loom_server_clips::ClipId(id);
	let revisions = match clips_git.list_commits(clip_id, 50).await {
		Ok(revs) => revs,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list revisions");
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

	let revisions: Vec<ClipRevisionResponse> = revisions
		.into_iter()
		.map(|r| ClipRevisionResponse {
			sha: r.sha,
			author_name: r.author_name,
			author_email: r.author_email,
			timestamp: r.timestamp,
			message: r.message,
		})
		.collect();

	(StatusCode::OK, Json(ClipRevisionsResponse { revisions })).into_response()
}

// ============================================================================
// Star/Unstar Handlers
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/clips/{id}/star",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    responses(
        (status = 200, description = "Star status updated", body = StarClipResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn star_clip(
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

	// Verify clip exists
	match clips_repo.get_clip_by_id(id).await {
		Ok(Some(_)) => {}
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
	}

	// Star the clip
	let starred = match clips_repo.star_clip(id, current_user.user.id.into_inner()).await {
		Ok(s) => s,
		Err(e) => {
			tracing::error!(error = %e, "Failed to star clip");
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

	let star_count = clips_repo.get_clip_star_count(id).await.unwrap_or(0);

	// Audit log (only if newly starred)
	if starred {
		state.audit_service.log(
			AuditLogBuilder::new(AuditEventType::ClipStarred)
				.actor(AuditUserId::new(current_user.user.id.into_inner()))
				.resource("clip", id.to_string())
				.build(),
		);
	}

	(
		StatusCode::OK,
		Json(StarClipResponse {
			starred: starred || true, // Returns true even if already starred
			star_count,
		}),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/clips/{id}/star",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    responses(
        (status = 200, description = "Star removed", body = StarClipResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn unstar_clip(
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

	// Verify clip exists
	match clips_repo.get_clip_by_id(id).await {
		Ok(Some(_)) => {}
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
	}

	// Unstar the clip
	let unstarred = clips_repo
		.unstar_clip(id, current_user.user.id.into_inner())
		.await
		.unwrap_or(false);

	let star_count = clips_repo.get_clip_star_count(id).await.unwrap_or(0);

	// Audit log (only if actually unstarred)
	if unstarred {
		state.audit_service.log(
			AuditLogBuilder::new(AuditEventType::ClipUnstarred)
				.actor(AuditUserId::new(current_user.user.id.into_inner()))
				.resource("clip", id.to_string())
				.build(),
		);
	}

	(
		StatusCode::OK,
		Json(StarClipResponse {
			starred: false,
			star_count,
		}),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/clips/{id}/starred",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    responses(
        (status = 200, description = "Star status", body = StarClipResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn get_clip_star_status(
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

	// Verify clip exists
	match clips_repo.get_clip_by_id(id).await {
		Ok(Some(_)) => {}
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
	}

	// Check star status
	let starred = clips_repo
		.is_clip_starred(id, current_user.user.id.into_inner())
		.await
		.unwrap_or(false);

	let star_count = clips_repo
		.get_clip_star_count(id)
		.await
		.unwrap_or(0);

	(
		StatusCode::OK,
		Json(StarClipResponse {
			starred,
			star_count,
		}),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/clips/starred",
    params(
        ListClipsQuery
    ),
    responses(
        (status = 200, description = "List of starred clips", body = ClipListResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_starred_clips(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Query(query): Query<ListClipsQuery>,
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

	let page = query.page.unwrap_or(1).max(1);
	let per_page = query.per_page.unwrap_or(20).min(100);
	let offset = (page - 1) * per_page;

	let clips = match clips_repo
		.list_user_starred_clips(current_user.user.id.into_inner(), per_page, offset)
		.await
	{
		Ok(c) => c,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list starred clips");
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

	let total = clips.len() as i64;
	let clips: Vec<ClipResponse> = clips
		.into_iter()
		.map(|c| clip_record_to_response(c, &state.base_url))
		.collect();

	(StatusCode::OK, Json(ClipListResponse { clips, total })).into_response()
}

// ============================================================================
// Utility functions
// ============================================================================

fn detect_language_from_path(path: &str) -> Option<String> {
	let ext = std::path::Path::new(path).extension()?.to_str()?;
	match ext.to_lowercase().as_str() {
		"rs" => Some("rust"),
		"js" | "jsx" | "mjs" => Some("javascript"),
		"ts" | "tsx" | "mts" => Some("typescript"),
		"py" => Some("python"),
		"go" => Some("go"),
		"java" => Some("java"),
		"c" | "h" => Some("c"),
		"cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
		"rb" => Some("ruby"),
		"php" => Some("php"),
		"swift" => Some("swift"),
		"kt" => Some("kotlin"),
		"sh" | "bash" => Some("shell"),
		"sql" => Some("sql"),
		"html" | "htm" => Some("html"),
		"css" => Some("css"),
		"json" => Some("json"),
		"yaml" | "yml" => Some("yaml"),
		"toml" => Some("toml"),
		"md" | "markdown" => Some("markdown"),
		"nix" => Some("nix"),
		"svelte" => Some("svelte"),
		_ => None,
	}
	.map(|s| s.to_string())
}

fn detect_content_type(path: &str) -> &'static str {
	let ext = std::path::Path::new(path)
		.extension()
		.and_then(|e| e.to_str())
		.unwrap_or("");
	match ext.to_lowercase().as_str() {
		"html" | "htm" => "text/html; charset=utf-8",
		"css" => "text/css; charset=utf-8",
		"js" | "mjs" => "application/javascript; charset=utf-8",
		"json" => "application/json; charset=utf-8",
		"xml" => "application/xml; charset=utf-8",
		"png" => "image/png",
		"jpg" | "jpeg" => "image/jpeg",
		"gif" => "image/gif",
		"svg" => "image/svg+xml",
		"pdf" => "application/pdf",
		"zip" => "application/zip",
		"tar" => "application/x-tar",
		"gz" => "application/gzip",
		_ => "text/plain; charset=utf-8",
	}
}
