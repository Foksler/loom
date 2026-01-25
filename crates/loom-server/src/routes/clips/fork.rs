// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Fork handler for clips.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_auth::types::OrgId;
use loom_server_db::clips::{ClipsStore, CreateClipParams};
use uuid::Uuid;

use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{clip_record_to_response, ClipResponse, ForkClipRequest};
use super::types::ClipsErrorResponse;

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
