// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Share link HTTP handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use chrono::{DateTime, Utc};
use loom_common_thread::ThreadId;
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::ShareLink;

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	error::ServerError,
	i18n::{resolve_user_locale, t},
};

use super::common::{
	CreateShareLinkRequest, CreateShareLinkResponse, ShareLinkErrorResponse,
	ShareLinkSuccessResponse, SharedThreadResponse,
};

/// POST /api/threads/{id}/share - Create a share link for a thread.
///
/// Creates a read-only shareable link for external users. Each thread can have
/// only one active share link; creating a new one replaces the previous.
#[utoipa::path(
    post,
    path = "/api/threads/{id}/share",
    params(
        ("id" = String, Path, description = "Thread ID")
    ),
    request_body = CreateShareLinkRequest,
    responses(
        (status = 201, description = "Share link created", body = CreateShareLinkResponse),
        (status = 403, description = "Not authorized to share this thread", body = ShareLinkErrorResponse),
        (status = 404, description = "Thread not found", body = ShareLinkErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "share"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		thread_id = %id
	)
)]
pub async fn create_share_link(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(id): Path<String>,
	Json(body): Json<CreateShareLinkRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let thread_id = match ThreadId::parse(&id) {
		Ok(tid) => tid,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ShareLinkErrorResponse {
					code: "invalid_thread_id".to_string(),
					message: t(locale, "server.api.share.invalid_thread_id").to_string(),
				}),
			)
				.into_response();
		}
	};

	let thread = match state.repo.get(&thread_id).await {
		Ok(Some(thread)) => thread,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ShareLinkErrorResponse {
					code: "not_found".to_string(),
					message: t(locale, "server.api.share.thread_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, thread_id = %id, "Failed to get thread");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ShareLinkErrorResponse {
					code: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let _ = thread;

	let owner_user_id = match state.repo.get_thread_owner_user_id(&id).await {
		Ok(Some(owner_id)) => owner_id,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ShareLinkErrorResponse {
					code: "not_found".to_string(),
					message: t(locale, "server.api.share.thread_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, thread_id = %id, "Failed to get thread owner");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ShareLinkErrorResponse {
					code: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if owner_user_id != current_user.user.id.to_string() {
		tracing::warn!(
			actor_id = %current_user.user.id,
			thread_id = %id,
			owner_id = %owner_user_id,
			"Non-owner attempted to create share link"
		);
		return (
			StatusCode::FORBIDDEN,
			Json(ShareLinkErrorResponse {
				code: "not_owner".to_string(),
				message: t(locale, "server.api.share.not_owner").to_string(),
			}),
		)
			.into_response();
	}

	let expires_at: Option<DateTime<Utc>> = match body.expires_at {
		Some(ref s) => match DateTime::parse_from_rfc3339(s) {
			Ok(dt) => Some(dt.with_timezone(&Utc)),
			Err(_) => {
				return (
					StatusCode::BAD_REQUEST,
					Json(ShareLinkErrorResponse {
						code: "invalid_expires_at".to_string(),
						message: t(locale, "server.api.share.invalid_expires_at").to_string(),
					}),
				)
					.into_response();
			}
		},
		None => None,
	};

	if let Err(e) = state.share_repo.revoke_share_link(&id).await {
		tracing::warn!(error = %e, thread_id = %id, "Failed to revoke existing share links");
	}

	let (share_link, plaintext_token) = ShareLink::new(id.clone(), current_user.user.id, expires_at);

	if let Err(e) = state.share_repo.create_share_link(&share_link).await {
		tracing::error!(error = %e, thread_id = %id, "Failed to create share link");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ShareLinkErrorResponse {
				code: "internal_error".to_string(),
				message: t(locale, "server.api.share.create_failed").to_string(),
			}),
		)
			.into_response();
	}

	let url = format!(
		"{}/api/threads/{}/share/{}",
		state.base_url, id, plaintext_token
	);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::ThreadShared)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("thread", id.clone())
			.details(serde_json::json!({
				"share_link_id": share_link.id.to_string(),
				"expires_at": share_link.expires_at.map(|dt| dt.to_rfc3339()),
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		thread_id = %id,
		share_link_id = %share_link.id,
		"Share link created"
	);

	(
		StatusCode::CREATED,
		Json(CreateShareLinkResponse {
			url,
			expires_at: share_link.expires_at.map(|dt| dt.to_rfc3339()),
			created_at: share_link.created_at.to_rfc3339(),
		}),
	)
		.into_response()
}

/// DELETE /api/threads/{id}/share - Revoke the share link for a thread.
///
/// Immediately invalidates the existing share link. Anyone with the link
/// will no longer be able to access the thread.
#[utoipa::path(
    delete,
    path = "/api/threads/{id}/share",
    params(
        ("id" = String, Path, description = "Thread ID")
    ),
    responses(
        (status = 200, description = "Share link revoked", body = ShareLinkSuccessResponse),
        (status = 403, description = "Not authorized to revoke share link", body = ShareLinkErrorResponse),
        (status = 404, description = "Thread or share link not found", body = ShareLinkErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "share"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		thread_id = %id
	)
)]
pub async fn revoke_share_link(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let thread_id = match ThreadId::parse(&id) {
		Ok(tid) => tid,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ShareLinkErrorResponse {
					code: "invalid_thread_id".to_string(),
					message: t(locale, "server.api.share.invalid_thread_id").to_string(),
				}),
			)
				.into_response();
		}
	};

	let thread = match state.repo.get(&thread_id).await {
		Ok(Some(thread)) => thread,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ShareLinkErrorResponse {
					code: "not_found".to_string(),
					message: t(locale, "server.api.share.thread_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, thread_id = %id, "Failed to get thread");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ShareLinkErrorResponse {
					code: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let _ = thread;

	let owner_user_id = match state.repo.get_thread_owner_user_id(&id).await {
		Ok(Some(owner_id)) => owner_id,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ShareLinkErrorResponse {
					code: "not_found".to_string(),
					message: t(locale, "server.api.share.thread_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, thread_id = %id, "Failed to get thread owner");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ShareLinkErrorResponse {
					code: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if owner_user_id != current_user.user.id.to_string() {
		tracing::warn!(
			actor_id = %current_user.user.id,
			thread_id = %id,
			owner_id = %owner_user_id,
			"Non-owner attempted to revoke share link"
		);
		return (
			StatusCode::FORBIDDEN,
			Json(ShareLinkErrorResponse {
				code: "not_owner".to_string(),
				message: t(locale, "server.api.share.not_owner").to_string(),
			}),
		)
			.into_response();
	}

	let existing_link = match state.share_repo.get_share_link_by_thread(&id).await {
		Ok(Some(link)) => link,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ShareLinkErrorResponse {
					code: "not_found".to_string(),
					message: t(locale, "server.api.share.link_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, thread_id = %id, "Failed to get share link");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ShareLinkErrorResponse {
					code: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	match state.share_repo.revoke_share_link(&id).await {
		Ok(count) if count > 0 => {
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::ThreadUnshared)
					.actor(AuditUserId::new(current_user.user.id.into_inner()))
					.resource("thread", id.clone())
					.details(serde_json::json!({
						"share_link_id": existing_link.id.to_string(),
					}))
					.build(),
			);

			tracing::info!(
				actor_id = %current_user.user.id,
				thread_id = %id,
				share_link_id = %existing_link.id,
				"Share link revoked"
			);

			(
				StatusCode::OK,
				Json(ShareLinkSuccessResponse {
					message: t(locale, "server.api.share.revoked").to_string(),
				}),
			)
				.into_response()
		}
		Ok(_) => (
			StatusCode::NOT_FOUND,
			Json(ShareLinkErrorResponse {
				code: "not_found".to_string(),
				message: t(locale, "server.api.share.link_not_found").to_string(),
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, thread_id = %id, "Failed to revoke share link");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ShareLinkErrorResponse {
					code: "internal_error".to_string(),
					message: t(locale, "server.api.share.revoke_failed").to_string(),
				}),
			)
				.into_response()
		}
	}
}

/// GET /api/threads/{id}/share/{token} - Access a shared thread (public).
///
/// This endpoint does NOT require authentication. The token in the URL
/// serves as the authentication mechanism for read-only access.
#[utoipa::path(
    get,
    path = "/api/threads/{id}/share/{token}",
    params(
        ("id" = String, Path, description = "Thread ID"),
        ("token" = String, Path, description = "Share token (48 hex characters)")
    ),
    responses(
        (status = 200, description = "Shared thread content", body = SharedThreadResponse),
        (status = 404, description = "Thread not found or link invalid/expired", body = ShareLinkErrorResponse),
        (status = 410, description = "Share link has been revoked", body = ShareLinkErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "share"
)]
#[tracing::instrument(skip(state, token), fields(thread_id = %id))]
pub async fn get_shared_thread(
	State(state): State<AppState>,
	Path((id, token)): Path<(String, String)>,
) -> Result<Response, ServerError> {
	let locale = state.default_locale.as_str();

	let thread_id = match ThreadId::parse(&id) {
		Ok(tid) => tid,
		Err(_) => {
			return Ok(
				(
					StatusCode::BAD_REQUEST,
					Json(ShareLinkErrorResponse {
						code: "invalid_thread_id".to_string(),
						message: t(locale, "server.api.share.invalid_thread_id").to_string(),
					}),
				)
					.into_response(),
			);
		}
	};

	let token_hash = loom_server_auth::hash_share_token(&token);

	let share_link = match state.share_repo.get_share_link_by_hash(&token_hash).await {
		Ok(Some(link)) => link,
		Ok(None) => {
			return Ok(
				(
					StatusCode::NOT_FOUND,
					Json(ShareLinkErrorResponse {
						code: "invalid_token".to_string(),
						message: t(locale, "server.api.share.invalid_or_expired").to_string(),
					}),
				)
					.into_response(),
			);
		}
		Err(e) => {
			tracing::error!(error = %e, "failed to look up share link");
			return Err(ServerError::Internal(
				t(locale, "server.api.share.validation_failed").to_string(),
			));
		}
	};

	if share_link.thread_id != id {
		return Ok(
			(
				StatusCode::NOT_FOUND,
				Json(ShareLinkErrorResponse {
					code: "invalid_token".to_string(),
					message: t(locale, "server.api.share.invalid_or_expired").to_string(),
				}),
			)
				.into_response(),
		);
	}

	if share_link.revoked_at.is_some() {
		return Ok(
			(
				StatusCode::GONE,
				Json(ShareLinkErrorResponse {
					code: "revoked".to_string(),
					message: t(locale, "server.api.share.link_revoked").to_string(),
				}),
			)
				.into_response(),
		);
	}

	if let Some(expires_at) = share_link.expires_at {
		if Utc::now() >= expires_at {
			return Ok(
				(
					StatusCode::GONE,
					Json(ShareLinkErrorResponse {
						code: "expired".to_string(),
						message: t(locale, "server.api.share.link_expired").to_string(),
					}),
				)
					.into_response(),
			);
		}
	}

	if !share_link.verify(&token) {
		return Ok(
			(
				StatusCode::NOT_FOUND,
				Json(ShareLinkErrorResponse {
					code: "invalid_token".to_string(),
					message: t(locale, "server.api.share.invalid_or_expired").to_string(),
				}),
			)
				.into_response(),
		);
	}

	let thread = match state.repo.get(&thread_id).await {
		Ok(Some(thread)) => thread,
		Ok(None) => {
			return Ok(
				(
					StatusCode::NOT_FOUND,
					Json(ShareLinkErrorResponse {
						code: "not_found".to_string(),
						message: t(locale, "server.api.share.thread_not_found").to_string(),
					}),
				)
					.into_response(),
			);
		}
		Err(e) => {
			tracing::error!(error = %e, "failed to get shared thread");
			return Err(ServerError::Internal(
				t(locale, "server.api.share.fetch_failed").to_string(),
			));
		}
	};

	let content = serde_json::to_value(&thread.conversation).unwrap_or(serde_json::Value::Null);

	tracing::info!(share_link_id = %share_link.id, "shared thread accessed");

	Ok(
		(
			StatusCode::OK,
			Json(SharedThreadResponse {
				id: thread.id.to_string(),
				title: thread.metadata.title,
				created_at: thread.created_at,
				updated_at: thread.updated_at,
				content,
			}),
		)
			.into_response(),
	)
}
