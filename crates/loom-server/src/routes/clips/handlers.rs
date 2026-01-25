// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Clips route handlers.
//!
//! These are stub implementations that return NOT_IMPLEMENTED.
//! The actual implementation will be completed in a future iteration.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{api::AppState, auth_middleware::RequireAuth};

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

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipResponse {
	pub id: Uuid,
	pub org_id: Uuid,
	pub org_name: String,
	pub name: String,
	pub description: Option<String>,
	pub visibility: String,
	pub clone_url: String,
	pub created_at: String,
	pub updated_at: String,
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
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipFilesResponse {
	pub files: Vec<ClipFileResponse>,
	pub revision: String,
}

// ============================================================================
// Stub handlers - return NOT_IMPLEMENTED
// ============================================================================

fn not_implemented() -> impl IntoResponse {
	(
		StatusCode::NOT_IMPLEMENTED,
		Json(ClipsErrorResponse {
			error: "not_implemented".to_string(),
			message: "Clips feature is not yet implemented".to_string(),
		}),
	)
}

#[utoipa::path(
    post,
    path = "/api/clips",
    request_body = CreateClipRequest,
    responses(
        (status = 201, description = "Clip created", body = ClipResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state, _payload))]
pub async fn create_clip(
	RequireAuth(_current_user): RequireAuth,
	State(state): State<AppState>,
	Json(_payload): Json<CreateClipRequest>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
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
        (status = 404, description = "Clip not found", body = ClipsErrorResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn get_clip(
	State(state): State<AppState>,
	Path((_owner, _name)): Path<(String, String)>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
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
        (status = 404, description = "Clip not found", body = ClipsErrorResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state, _payload))]
pub async fn update_clip(
	RequireAuth(_current_user): RequireAuth,
	State(state): State<AppState>,
	Path(_id): Path<Uuid>,
	Json(_payload): Json<UpdateClipRequest>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
}

#[utoipa::path(
    delete,
    path = "/api/clips/{id}",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    responses(
        (status = 204, description = "Clip deleted"),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn delete_clip(
	RequireAuth(_current_user): RequireAuth,
	State(state): State<AppState>,
	Path(_id): Path<Uuid>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
}

#[utoipa::path(
    get,
    path = "/api/users/{user_id}/clips",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
        ListClipsQuery
    ),
    responses(
        (status = 200, description = "User's clips", body = ClipListResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_user_clips(
	RequireAuth(_current_user): RequireAuth,
	State(state): State<AppState>,
	Path(_user_id): Path<Uuid>,
	Query(_query): Query<ListClipsQuery>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/clips",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID"),
        ListClipsQuery
    ),
    responses(
        (status = 200, description = "Organization's clips", body = ClipListResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_org_clips(
	RequireAuth(_current_user): RequireAuth,
	State(state): State<AppState>,
	Path(_org_id): Path<Uuid>,
	Query(_query): Query<ListClipsQuery>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
}

#[utoipa::path(
    get,
    path = "/api/clips",
    params(ListClipsQuery),
    responses(
        (status = 200, description = "Public clips", body = ClipListResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_public_clips(
	State(state): State<AppState>,
	Query(_query): Query<ListClipsQuery>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
}

#[utoipa::path(
    get,
    path = "/api/clips/{id}/files",
    params(
        ("id" = Uuid, Path, description = "Clip ID")
    ),
    responses(
        (status = 200, description = "Clip files", body = ClipFilesResponse),
        (status = 404, description = "Clip not found", body = ClipsErrorResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn list_clip_files(
	State(state): State<AppState>,
	Path(_id): Path<Uuid>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
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
        (status = 404, description = "File not found", body = ClipsErrorResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state))]
pub async fn get_clip_file(
	State(state): State<AppState>,
	Path((_id, _path)): Path<(Uuid, String)>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
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
        (status = 404, description = "Clip not found", body = ClipsErrorResponse),
        (status = 501, description = "Not implemented", body = ClipsErrorResponse)
    ),
    tag = "clips"
)]
#[tracing::instrument(skip(state, _payload))]
pub async fn fork_clip(
	RequireAuth(_current_user): RequireAuth,
	State(state): State<AppState>,
	Path(_id): Path<Uuid>,
	Json(_payload): Json<ForkClipRequest>,
) -> impl IntoResponse {
	let _ = state;
	not_implemented()
}
