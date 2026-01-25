// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! List and search handlers for clips.

use axum::{
	extract::{Path, Query, State},
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

use super::common::{
	clip_record_to_response, ClipListResponse, ClipSearchHitResponse, ClipSearchResponse,
	ListClipsQuery, SearchClipsQuery,
};
use super::types::ClipsErrorResponse;

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
