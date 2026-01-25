// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Basic auth handlers (providers, me, ws-token, logout).

use axum::{
	extract::State,
	http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode},
	response::IntoResponse,
	Json,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};

use crate::{api::AppState, auth_middleware::RequireAuth, i18n::resolve_user_locale, i18n::t};

use super::common::{
	AuthErrorResponse, AuthProvidersResponse, AuthSuccessResponse, CurrentUserResponse,
	WsTokenResponse,
};

#[utoipa::path(
    get,
    path = "/auth/providers",
    responses(
        (status = 200, description = "List of available auth providers", body = AuthProvidersResponse)
    ),
    tag = "auth"
)]
/// Lists available authentication providers.
///
/// Returns an array of provider names based on server configuration (e.g.,
/// `["github", "google", "magic_link"]`). Clients should use this to display
/// login options.
#[tracing::instrument(skip(state))]
pub async fn get_providers(State(state): State<AppState>) -> impl IntoResponse {
	let mut providers = Vec::new();

	if state.github_oauth.is_some() {
		providers.push("github".to_string());
	}
	if state.google_oauth.is_some() {
		providers.push("google".to_string());
	}
	if state.okta_oauth.is_some() {
		providers.push("okta".to_string());
	}
	if state.smtp_client.is_some() {
		providers.push("magic_link".to_string());
	}

	Json(AuthProvidersResponse { providers })
}

#[utoipa::path(
    get,
    path = "/auth/me",
    responses(
        (status = 200, description = "Current authenticated user", body = CurrentUserResponse),
        (status = 401, description = "Not authenticated", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Returns the current authenticated user's information.
///
/// Requires a valid session cookie or access token. Returns user ID,
/// display name, email, and avatar URL.
///
/// # Errors
/// Returns 401 Unauthorized if the request lacks valid authentication.
#[tracing::instrument(skip(current_user))]
pub async fn get_current_user(RequireAuth(current_user): RequireAuth) -> impl IntoResponse {
	tracing::debug!(user_id = %current_user.user.id, "Retrieved current user");
	Json(CurrentUserResponse {
		id: current_user.user.id.to_string(),
		display_name: current_user.user.display_name.clone(),
		username: current_user.user.username.clone(),
		email: current_user.user.primary_email.clone(),
		avatar_url: current_user.user.avatar_url.clone(),
		locale: current_user.user.locale.clone(),
		global_roles: current_user
			.user
			.global_roles()
			.iter()
			.map(|r| r.to_string())
			.collect(),
		created_at: current_user.user.created_at,
	})
}

#[utoipa::path(
    get,
    path = "/auth/ws-token",
    responses(
        (status = 200, description = "WebSocket authentication token", body = WsTokenResponse),
        (status = 401, description = "Not authenticated", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Returns a short-lived token for WebSocket first-message authentication.
///
/// This endpoint solves the problem of HttpOnly session cookies not being
/// accessible to JavaScript. The returned token can be used in the WebSocket
/// first message: `{"type": "auth", "token": "ws_xxx"}`.
///
/// # Security
/// - Token expires in 30 seconds
/// - Token can only be used once (single-use)
/// - Requires valid session cookie authentication
///
/// # Usage
/// 1. Call this endpoint to get a token
/// 2. Connect to WebSocket at /api/ws/sessions/{session_id}
/// 3. Send first message: {"type": "auth", "token": "<token>"}
#[tracing::instrument(skip(state, current_user))]
pub async fn get_ws_token(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
) -> impl IntoResponse {
	use loom_server_auth::ws_token::{generate_ws_token, WS_TOKEN_EXPIRY_SECONDS};

	let (token, token_hash) = generate_ws_token();

	if let Err(e) = state
		.session_repo
		.create_ws_token(&current_user.user.id, &token_hash)
		.await
	{
		tracing::error!(error = %e, user_id = %current_user.user.id, "Failed to create WS token");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(AuthErrorResponse {
				error: "token_creation_failed".to_string(),
				message: "Failed to create WebSocket token".to_string(),
			}),
		)
			.into_response();
	}

	tracing::debug!(user_id = %current_user.user.id, "WS token created");

	Json(WsTokenResponse {
		token,
		expires_in: WS_TOKEN_EXPIRY_SECONDS,
	})
	.into_response()
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    responses(
        (status = 200, description = "Logout successful", body = AuthSuccessResponse)
    ),
    tag = "auth"
)]
/// Logs out the current user and invalidates their session.
///
/// Deletes the server-side session (if session-based auth) and clears the
/// session cookie. Always returns success even if session deletion fails.
///
/// # Security
/// Session invalidation is best-effort; cookie is always cleared.
#[tracing::instrument(skip(state, current_user))]
pub async fn logout(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::Logout)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.build(),
	);

	tracing::info!(user_id = %current_user.user.id, "User logged out");
	// If session-based auth, delete the session
	if let Some(session_id) = current_user.session_id {
		if let Err(e) = state.session_repo.delete_session(&session_id).await {
			tracing::warn!(error = %e, "Failed to delete session during logout");
		}
	}

	// Build response with cleared session cookie
	let mut headers = HeaderMap::new();
	let cookie_name = &state.auth_config.session_cookie_name;
	let clear_cookie = format!("{cookie_name}=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax");
	if let Ok(value) = HeaderValue::from_str(&clear_cookie) {
		headers.insert(SET_COOKIE, value);
	}

	(
		headers,
		Json(AuthSuccessResponse {
			message: t(locale, "server.api.auth.logged_out").to_string(),
		}),
	)
}
