// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Revisions handler for clips.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_db::clips::ClipsStore;
use uuid::Uuid;

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{ClipRevisionResponse, ClipRevisionsResponse};
use super::types::ClipsErrorResponse;

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
