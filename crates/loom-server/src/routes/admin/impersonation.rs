// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Admin impersonation HTTP handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, AuditSeverity, UserId as AuditUserId};
use serde_json::json;

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{
	parse_user_id, AdminErrorResponse, AdminSuccessResponse, ImpersonateRequest,
	ImpersonateResponse, ImpersonationState, ImpersonationUserInfo,
};

/// Get current impersonation state.
///
/// Returns the current impersonation state for the authenticated admin user.
/// If the admin is currently impersonating another user, returns details about
/// both the original admin and the impersonated user.
///
/// Requires `system_admin` role.
#[utoipa::path(
    get,
    path = "/api/admin/impersonate/state",
    responses(
        (status = 200, description = "Impersonation state", body = ImpersonationState),
        (status = 401, description = "Not authenticated", body = AdminErrorResponse),
        (status = 403, description = "Not authorized (system_admin required)", body = AdminErrorResponse)
    ),
    tag = "admin"
)]
#[tracing::instrument(
    skip(state),
    fields(actor_id = %current_user.user.id)
)]
pub async fn get_impersonation_state(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(
			actor_id = %current_user.user.id,
			"Unauthorized impersonation state access attempt"
		);
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	match state
		.session_repo
		.get_active_impersonation_session(&current_user.user.id)
		.await
	{
		Ok(Some((_session_id, target_user_id))) => {
			match state.user_repo.get_user_by_id(&target_user_id).await {
				Ok(Some(target_user)) => Json(ImpersonationState {
					is_impersonating: true,
					original_user: Some(ImpersonationUserInfo {
						id: current_user.user.id.to_string(),
						display_name: current_user.user.display_name.clone(),
					}),
					impersonated_user: Some(ImpersonationUserInfo {
						id: target_user.id.to_string(),
						display_name: target_user.display_name,
					}),
				})
				.into_response(),
				Ok(None) => {
					tracing::warn!(
						actor_id = %current_user.user.id,
						target_id = %target_user_id,
						"Impersonation session refers to missing user"
					);
					Json(ImpersonationState {
						is_impersonating: false,
						original_user: None,
						impersonated_user: None,
					})
					.into_response()
				}
				Err(e) => {
					tracing::error!(error = %e, "Failed to fetch impersonated user");
					(
						StatusCode::INTERNAL_SERVER_ERROR,
						Json(AdminErrorResponse {
							error: "internal_error".to_string(),
							message: t(locale, "server.api.error.internal").to_string(),
						}),
					)
						.into_response()
				}
			}
		}
		Ok(None) => Json(ImpersonationState {
			is_impersonating: false,
			original_user: None,
			impersonated_user: None,
		})
		.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to check impersonation state");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AdminErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

/// Start impersonating a user.
///
/// # Authorization
///
/// Requires `system_admin` role. Returns 403 Forbidden otherwise.
///
/// # Request
///
/// Path parameters:
/// - `id`: Target user's UUID to impersonate
///
/// Body ([`ImpersonateRequest`]):
/// - `reason`: Required justification for impersonation (for audit)
///
/// # Response
///
/// Returns [`ImpersonateResponse`] with new session ID.
///
/// # Errors
///
/// - `400 Bad Request`: Invalid user ID or attempting to impersonate self
/// - `401 Unauthorized`: Missing or invalid authentication
/// - `403 Forbidden`: Caller lacks `system_admin` role
/// - `404 Not Found`: Target user does not exist
/// - `409 Conflict`: Already impersonating another user
/// - `500 Internal Server Error`: Database error
///
/// # Security
///
/// - Creates an auditable impersonation session
/// - Original admin identity is preserved in audit logs
/// - Admin cannot impersonate themselves
/// - Must stop current impersonation before starting new one
#[utoipa::path(
    post,
    path = "/api/admin/users/{id}/impersonate",
    params(
        ("id" = String, Path, description = "User ID to impersonate")
    ),
    request_body = ImpersonateRequest,
    responses(
        (status = 200, description = "Impersonation started", body = ImpersonateResponse),
        (status = 400, description = "Invalid request", body = AdminErrorResponse),
        (status = 401, description = "Not authenticated", body = AdminErrorResponse),
        (status = 403, description = "Not authorized (system_admin required)", body = AdminErrorResponse),
        (status = 404, description = "User not found", body = AdminErrorResponse),
        (status = 409, description = "Already impersonating", body = AdminErrorResponse)
    ),
    tag = "admin"
)]
#[tracing::instrument(
	skip(state, payload),
	fields(
		actor_id = %current_user.user.id,
		target_id = %user_id
	)
)]
pub async fn start_impersonation(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(user_id): Path<String>,
	Json(payload): Json<ImpersonateRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(
			actor_id = %current_user.user.id,
			target_id = %user_id,
			"Unauthorized impersonation attempt"
		);
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let target_user_id = match parse_user_id(&user_id, locale) {
		Ok(id) => id,
		Err(e) => return (StatusCode::BAD_REQUEST, Json(e)).into_response(),
	};

	if current_user.user.id == target_user_id {
		return (
			StatusCode::BAD_REQUEST,
			Json(AdminErrorResponse {
				error: "bad_request".to_string(),
				message: "Cannot impersonate yourself".to_string(),
			}),
		)
			.into_response();
	}

	if let Ok(Some(_)) = state
		.session_repo
		.get_active_impersonation_session(&current_user.user.id)
		.await
	{
		return (
			StatusCode::CONFLICT,
			Json(AdminErrorResponse {
				error: "conflict".to_string(),
				message: "Already impersonating another user. Stop current impersonation first."
					.to_string(),
			}),
		)
			.into_response();
	}

	let target_user = match state.user_repo.get_user_by_id(&target_user_id).await {
		Ok(Some(user)) => user,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(AdminErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.user.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				target_id = %target_user_id,
				"Failed to get user"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AdminErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let session_id = match state
		.session_repo
		.create_impersonation_session(&current_user.user.id, &target_user_id, &payload.reason)
		.await
	{
		Ok(id) => id,
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				target_id = %target_user_id,
				"Failed to create impersonation session"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AdminErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	tracing::info!(
		actor_id = %current_user.user.id,
		target_id = %target_user_id,
		session_id = %session_id,
		"Admin started impersonation"
	);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::ImpersonationStarted)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.severity(AuditSeverity::Warning)
			.resource("user", target_user.id.to_string())
			.details(json!({
				"reason": payload.reason,
				"session_id": session_id,
				"target_user_id": target_user_id.to_string(),
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(ImpersonateResponse {
			session_id,
			message: format!("Now impersonating user {}", target_user.display_name),
		}),
	)
		.into_response()
}

/// Stop impersonating a user.
///
/// # Authorization
///
/// Requires authentication. Any authenticated user with an active impersonation
/// session can stop it.
///
/// # Response
///
/// Returns [`AdminSuccessResponse`] confirming impersonation ended.
///
/// # Errors
///
/// - `401 Unauthorized`: Missing or invalid authentication
/// - `404 Not Found`: No active impersonation session
/// - `500 Internal Server Error`: Database error
///
/// # Security
///
/// - Ends the impersonation session and restores admin identity
/// - Session termination is logged to audit trail
#[utoipa::path(
    post,
    path = "/api/admin/impersonate/stop",
    responses(
        (status = 200, description = "Impersonation stopped", body = AdminSuccessResponse),
        (status = 401, description = "Not authenticated", body = AdminErrorResponse),
        (status = 404, description = "Not currently impersonating", body = AdminErrorResponse)
    ),
    tag = "admin"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id))]
pub async fn stop_impersonation(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let (session_id, target_user_id) = match state
		.session_repo
		.get_active_impersonation_session(&current_user.user.id)
		.await
	{
		Ok(Some((id, target_id))) => (id, target_id),
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(AdminErrorResponse {
					error: "not_found".to_string(),
					message: "Not currently impersonating any user".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				"Failed to get impersonation session"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AdminErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if let Err(e) = state
		.session_repo
		.end_impersonation_session(&session_id)
		.await
	{
		tracing::error!(
			error = %e,
			actor_id = %current_user.user.id,
			session_id = %session_id,
			"Failed to end impersonation session"
		);
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(AdminErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(
		actor_id = %current_user.user.id,
		target_id = %target_user_id,
		session_id = %session_id,
		"Admin stopped impersonation"
	);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::ImpersonationEnded)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.severity(AuditSeverity::Warning)
			.resource("user", target_user_id.to_string())
			.details(json!({
				"session_id": session_id,
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(AdminSuccessResponse {
			message: "Impersonation ended".to_string(),
		}),
	)
		.into_response()
}
