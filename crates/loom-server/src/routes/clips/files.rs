// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! File handlers for clips (list files, get file, get raw file, update files).

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_auth::types::OrgId;
use loom_server_db::clips::ClipsStore;
use uuid::Uuid;

use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{
	detect_content_type, detect_language_from_path, ClipFileResponse, ClipFilesResponse,
	UpdateFilesRequest,
};
use super::types::ClipsErrorResponse;

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
