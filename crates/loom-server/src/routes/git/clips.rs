// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git HTTP Smart Protocol handlers for clips.

use axum::{
	body::Bytes,
	extract::{Path, Query, State},
	http::{header, HeaderMap, StatusCode},
	response::{IntoResponse, Response},
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::types::OrgId;
use loom_server_db::clips::{ClipRecord, ClipVisibility, ClipsStore};
use tracing::instrument;

use crate::{api::AppState, auth_middleware::OptionalAuth, error::ServerError, i18n::t};

use super::common::{
	extract_basic_auth_user, git_unauthorized_response, pkt_flush, pkt_line, run_git_command,
	GitService, InfoRefsParams,
};

/// Get the path to a clip's git repository.
fn get_clip_git_path(state: &AppState, clip_id: uuid::Uuid) -> Option<std::path::PathBuf> {
	state
		.clips_git_store
		.as_ref()
		.map(|store| store.get_clip_path(loom_server_clips::ClipId(clip_id)))
}

/// Resolve a clip by owner name and clip name.
async fn resolve_clip(
	owner: &str,
	clip_name: &str,
	state: &AppState,
	locale: &str,
) -> Result<ClipRecord, ServerError> {
	let clip_name = clip_name.strip_suffix(".git").unwrap_or(clip_name);

	let clips_repo = state.clips_repo.as_ref().ok_or_else(|| {
		ServerError::Internal(t(locale, "server.api.error.internal").to_string())
	})?;

	// Look up clip by owner/name
	let clip = clips_repo
		.get_clip_by_owner_name(owner, clip_name)
		.await
		.map_err(|e| ServerError::Internal(e.to_string()))?
		.ok_or_else(|| ServerError::NotFound("Clip not found".to_string()))?;

	Ok(clip)
}

/// Check if user has read access to a clip.
async fn check_clip_read_access(
	clip: &ClipRecord,
	user: Option<&loom_server_auth::middleware::CurrentUser>,
	state: &AppState,
	locale: &str,
) -> Result<(), ServerError> {
	match clip.visibility {
		ClipVisibility::Public => Ok(()),
		ClipVisibility::Internal | ClipVisibility::Private => {
			let user = user.ok_or_else(|| {
				ServerError::Unauthorized(t(locale, "server.api.scm.git.auth_required").to_string())
			})?;

			// Check if user is org member
			if let Some(org_id) = clip.org_id {
				let org_id = OrgId::new(org_id);
				match state
					.org_repo
					.get_membership(&org_id, &user.user.id)
					.await
				{
					Ok(Some(_)) => Ok(()),
					_ => Err(ServerError::Forbidden(
						t(locale, "server.api.error.forbidden").to_string(),
					)),
				}
			} else if clip.created_by == user.user.id.into_inner() {
				Ok(())
			} else {
				Err(ServerError::Forbidden(
					t(locale, "server.api.error.forbidden").to_string(),
				))
			}
		}
	}
}

/// Check if user has write access to a clip.
async fn check_clip_write_access(
	clip: &ClipRecord,
	user: Option<&loom_server_auth::middleware::CurrentUser>,
	state: &AppState,
	locale: &str,
) -> Result<(), ServerError> {
	let user = user.ok_or_else(|| {
		ServerError::Unauthorized(t(locale, "server.api.scm.git.auth_required").to_string())
	})?;

	// Check if user is org member
	if let Some(org_id) = clip.org_id {
		let org_id = OrgId::new(org_id);
		match state
			.org_repo
			.get_membership(&org_id, &user.user.id)
			.await
		{
			Ok(Some(_)) => Ok(()),
			_ => Err(ServerError::Forbidden(
				t(locale, "server.api.error.forbidden").to_string(),
			)),
		}
	} else if clip.created_by == user.user.id.into_inner() {
		Ok(())
	} else {
		Err(ServerError::Forbidden(
			t(locale, "server.api.error.forbidden").to_string(),
		))
	}
}

#[instrument(skip(state, headers), fields(owner = %owner, clip = %clip_name))]
pub async fn clips_info_refs(
	Path((owner, clip_name)): Path<(String, String)>,
	Query(params): Query<InfoRefsParams>,
	OptionalAuth(auth): OptionalAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Response, ServerError> {
	let effective_user = match auth {
		Some(user) => Some(user),
		None => extract_basic_auth_user(&headers, &state).await,
	};

	let locale = effective_user
		.as_ref()
		.and_then(|u| u.user.locale.as_deref())
		.unwrap_or(&state.default_locale);

	let service = GitService::from_str(&params.service)
		.ok_or_else(|| ServerError::BadRequest(format!("Invalid service: {}", params.service)))?;

	let clip = resolve_clip(&owner, &clip_name, &state, locale).await?;

	let repo_path = get_clip_git_path(&state, clip.id)
		.ok_or_else(|| ServerError::Internal("Clips not configured".to_string()))?;

	if !repo_path.exists() {
		return Err(ServerError::NotFound("Clip repository not found".to_string()));
	}

	match service {
		GitService::UploadPack => {
			if let Err(e) =
				check_clip_read_access(&clip, effective_user.as_ref(), &state, locale).await
			{
				return match e {
					ServerError::Unauthorized(msg) => Ok(git_unauthorized_response(&msg)),
					other => Err(other),
				};
			}
		}
		GitService::ReceivePack => {
			if let Err(e) =
				check_clip_write_access(&clip, effective_user.as_ref(), &state, locale).await
			{
				return match e {
					ServerError::Unauthorized(msg) => Ok(git_unauthorized_response(&msg)),
					other => Err(other),
				};
			}
		}
	}

	let git_output = run_git_command(&repo_path, service, &[], true).await?;

	let mut response_body = Vec::new();
	response_body.extend(pkt_line(&format!("# service={}\n", service.as_str())));
	response_body.extend(pkt_flush());
	response_body.extend(git_output);

	Ok((
		StatusCode::OK,
		[
			(header::CONTENT_TYPE, service.content_type()),
			(header::CACHE_CONTROL, "no-cache"),
		],
		response_body,
	)
		.into_response())
}

#[instrument(skip(state, body, headers), fields(owner = %owner, clip = %clip_name))]
pub async fn clips_upload_pack(
	Path((owner, clip_name)): Path<(String, String)>,
	OptionalAuth(auth): OptionalAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
	body: Bytes,
) -> Result<Response, ServerError> {
	let effective_user = match auth {
		Some(user) => Some(user),
		None => extract_basic_auth_user(&headers, &state).await,
	};

	let locale = effective_user
		.as_ref()
		.and_then(|u| u.user.locale.as_deref())
		.unwrap_or(&state.default_locale);

	let content_type = headers
		.get(header::CONTENT_TYPE)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");

	if content_type != "application/x-git-upload-pack-request" {
		return Err(ServerError::BadRequest(format!(
			"Invalid content type: {content_type}"
		)));
	}

	let clip = resolve_clip(&owner, &clip_name, &state, locale).await?;

	let repo_path = get_clip_git_path(&state, clip.id)
		.ok_or_else(|| ServerError::Internal("Clips not configured".to_string()))?;

	if !repo_path.exists() {
		return Err(ServerError::NotFound("Clip repository not found".to_string()));
	}

	if let Err(e) = check_clip_read_access(&clip, effective_user.as_ref(), &state, locale).await {
		// Log access denied
		if let Some(user) = &effective_user {
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::ClipAccessDenied)
					.actor(AuditUserId::new(user.user.id.into_inner()))
					.resource("clip", clip.id.to_string())
					.details(serde_json::json!({
						"action": "upload-pack",
						"reason": "read_access_denied",
					}))
					.build(),
			);
		}
		return match e {
			ServerError::Unauthorized(msg) => Ok(git_unauthorized_response(&msg)),
			other => Err(other),
		};
	}

	let output = run_git_command(&repo_path, GitService::UploadPack, &body, false).await?;

	// Log successful pull/fetch
	if let Some(user) = &effective_user {
		state.audit_service.log(
			AuditLogBuilder::new(AuditEventType::ClipPulled)
				.actor(AuditUserId::new(user.user.id.into_inner()))
				.resource("clip", clip.id.to_string())
				.build(),
		);
	}

	Ok((
		StatusCode::OK,
		[
			(
				header::CONTENT_TYPE,
				GitService::UploadPack.result_content_type(),
			),
			(header::CACHE_CONTROL, "no-cache"),
		],
		output,
	)
		.into_response())
}

#[instrument(skip(state, body, headers), fields(owner = %owner, clip = %clip_name))]
pub async fn clips_receive_pack(
	Path((owner, clip_name)): Path<(String, String)>,
	OptionalAuth(auth): OptionalAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
	body: Bytes,
) -> Result<Response, ServerError> {
	let effective_user = match auth {
		Some(user) => Some(user),
		None => extract_basic_auth_user(&headers, &state).await,
	};

	let locale = effective_user
		.as_ref()
		.and_then(|u| u.user.locale.as_deref())
		.unwrap_or(&state.default_locale);

	let content_type = headers
		.get(header::CONTENT_TYPE)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");

	if content_type != "application/x-git-receive-pack-request" {
		return Err(ServerError::BadRequest(format!(
			"Invalid content type: {content_type}"
		)));
	}

	let clip = resolve_clip(&owner, &clip_name, &state, locale).await?;

	let repo_path = get_clip_git_path(&state, clip.id)
		.ok_or_else(|| ServerError::Internal("Clips not configured".to_string()))?;

	if !repo_path.exists() {
		return Err(ServerError::NotFound("Clip repository not found".to_string()));
	}

	if let Err(e) = check_clip_write_access(&clip, effective_user.as_ref(), &state, locale).await {
		// Log access denied
		if let Some(user) = &effective_user {
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::ClipAccessDenied)
					.actor(AuditUserId::new(user.user.id.into_inner()))
					.resource("clip", clip.id.to_string())
					.details(serde_json::json!({
						"action": "receive-pack",
						"reason": "write_access_denied",
					}))
					.build(),
			);
		}
		return match e {
			ServerError::Unauthorized(msg) => Ok(git_unauthorized_response(&msg)),
			other => Err(other),
		};
	}

	// Ensure user is authenticated for push
	if effective_user.is_none() {
		return Ok(git_unauthorized_response(&t(
			locale,
			"server.api.scm.git.auth_required",
		)));
	}

	let output = run_git_command(&repo_path, GitService::ReceivePack, &body, false).await?;

	// Update clip stats after push
	if let Some(clips_git) = state.clips_git_store.as_ref() {
		let clip_id = loom_server_clips::ClipId(clip.id);
		if let Ok(files) = clips_git.list_files(clip_id, None).await {
			let file_count = files.len() as u32;
			if let Ok(size) = clips_git.get_clip_size(clip_id).await {
				if let Some(clips_repo) = state.clips_repo.as_ref() {
					let _ = clips_repo
						.update_clip_stats(clip.id, file_count, size, None)
						.await;
				}
			}
		}
	}

	// Log successful push
	if let Some(user) = &effective_user {
		state.audit_service.log(
			AuditLogBuilder::new(AuditEventType::ClipPushed)
				.actor(AuditUserId::new(user.user.id.into_inner()))
				.resource("clip", clip.id.to_string())
				.details(serde_json::json!({
					"protocol": "git",
				}))
				.build(),
		);
	}

	Ok((
		StatusCode::OK,
		[
			(
				header::CONTENT_TYPE,
				GitService::ReceivePack.result_content_type(),
			),
			(header::CACHE_CONTROL, "no-cache"),
		],
		output,
	)
		.into_response())
}
