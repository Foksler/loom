// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Star/unstar handlers for clips.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_db::clips::ClipsStore;
use uuid::Uuid;

use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{
	clip_record_to_response, ClipListResponse, ClipResponse, ListClipsQuery, StarClipResponse,
};
use super::types::ClipsErrorResponse;

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
			starred: true, // Returns true even if already starred
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

	let star_count = clips_repo.get_clip_star_count(id).await.unwrap_or(0);

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
