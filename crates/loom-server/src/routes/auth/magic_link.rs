// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Magic link authentication handlers.

use axum::{
	extract::State,
	http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode},
	response::{IntoResponse, Redirect},
	Json,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder};
use loom_server_auth_magiclink::{verify_magic_link_token, MagicLink};
use loom_server_session::{AuthMethod, SessionRequest};

use crate::{api::AppState, client_info::ClientInfo, i18n::t};

/// Log a failed magic link login attempt for security auditing.
fn log_magic_link_login_failed(state: &AppState, error: &str, ip_address: Option<&str>) {
	let mut builder = AuditLogBuilder::new(AuditEventType::LoginFailed)
		.details(serde_json::json!({
			"method": "magic_link",
			"error": error,
		}));

	if let Some(ip) = ip_address {
		builder = builder.ip_address(ip.to_string());
	}

	state.audit_service.log(builder.build());
}

use super::common::{
	AuthErrorResponse, AuthSuccessResponse, MagicLinkRequest, MagicLinkVerifyQuery,
};

#[utoipa::path(
    post,
    path = "/auth/magic-link",
    request_body = MagicLinkRequest,
    responses(
        (status = 200, description = "Magic link sent", body = AuthSuccessResponse),
        (status = 400, description = "Invalid email", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Requests a magic link for passwordless login.
///
/// Generates a magic link token, stores it hashed in the database,
/// and sends an email to the user with the verification link.
///
/// # Security
/// Always returns success to prevent email enumeration attacks.
/// Magic link tokens are hashed with Argon2 before storage.
///
/// # Errors
/// Returns 400 Bad Request only for obviously invalid email format.
#[tracing::instrument(skip(state, payload))]
pub async fn request_magic_link(
	State(state): State<AppState>,
	Json(payload): Json<MagicLinkRequest>,
) -> impl IntoResponse {
	let locale = state.default_locale.as_str();
	let email = payload.email.trim().to_lowercase();

	// Basic email validation
	if !email.contains('@') || email.len() < 5 {
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "invalid_email".to_string(),
				message: t(locale, "server.api.auth.invalid_email").to_string(),
			}),
		)
			.into_response();
	}

	// Always return success to prevent email enumeration
	let success_response = Json(AuthSuccessResponse {
		message: t(locale, "server.api.auth.check_email").to_string(),
	});

	// Invalidate any existing magic links for this email
	if let Err(e) = state
		.session_repo
		.invalidate_magic_links_for_email(&email)
		.await
	{
		tracing::warn!(error = %e, email = %email, "Failed to invalidate existing magic links");
	}

	// Create new magic link
	let (magic_link, plaintext_token) = MagicLink::new(&email);

	// Store the magic link (hashed)
	if let Err(e) = state
		.session_repo
		.create_magic_link(&email, &magic_link.token_hash)
		.await
	{
		tracing::error!(error = %e, email = %email, "Failed to store magic link");
		return success_response.into_response();
	}

	// Send email if email service is configured
	let Some(email_service) = &state.email_service else {
		// SECURITY: Never log the plaintext token - it allows account takeover
		tracing::warn!(
			"Email service not configured, magic link not sent - user will not receive email"
		);
		return success_response.into_response();
	};

	let verification_url = format!(
		"{}/auth/magic-link/verify?token={}",
		state.base_url, plaintext_token
	);

	let request = loom_server_email::EmailRequest::MagicLink { verification_url };

	if let Err(e) = email_service.send(&email, request, None).await {
		tracing::error!(error = %e, email = %email, "Failed to send magic link email");
	} else {
		state.audit_service.log(
			AuditLogBuilder::new(AuditEventType::MagicLinkRequested)
				.details(serde_json::json!({
					"email": &email,
				}))
				.build(),
		);

		tracing::info!(email = %email, "Magic link email sent");
	}

	success_response.into_response()
}

#[utoipa::path(
    get,
    path = "/auth/magic-link/verify",
    params(
        ("token" = String, Query, description = "Magic link token from email")
    ),
    responses(
        (status = 302, description = "Redirect to dashboard on success"),
        (status = 400, description = "Invalid or expired token", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Verifies a magic link token and creates a session.
///
/// Validates the magic link token from the query parameter, creates a session
/// for the user, and redirects to the dashboard with a session cookie.
///
/// # Security
/// Magic link tokens are single-use and hashed with Argon2 for storage.
/// Never log the token from the URL.
///
/// # Errors
/// Returns 400 Bad Request if the token is invalid or expired.
#[tracing::instrument(skip(state, query, headers))]
pub async fn verify_magic_link(
	State(state): State<AppState>,
	headers: HeaderMap,
	axum::extract::Query(query): axum::extract::Query<MagicLinkVerifyQuery>,
) -> impl IntoResponse {
	let client_info = ClientInfo::from_headers(&headers, state.geoip_service.as_ref());
	let locale = state.default_locale.as_str();
	let token = &query.token;

	// Get all pending magic links and verify against each using Argon2
	let pending_links = match state.session_repo.get_pending_magic_links().await {
		Ok(links) => links,
		Err(e) => {
			tracing::error!(error = %e, "Failed to get pending magic links");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AuthErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Find matching magic link using Argon2 verification
	let matching_link = pending_links
		.into_iter()
		.find(|(_, _, stored_hash)| verify_magic_link_token(token, stored_hash));

	let (link_id, email) = match matching_link {
		Some((id, email, _)) => (id, email),
		None => {
			tracing::debug!("Magic link not found or invalid token");
			log_magic_link_login_failed(&state, "invalid_token", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "invalid_token".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Atomically claim the magic link to prevent TOCTOU race conditions.
	// Only one request can successfully claim a link - if another request
	// verified the same link concurrently, this will return false.
	match state.session_repo.claim_magic_link(&link_id).await {
		Ok(true) => {}
		Ok(false) => {
			tracing::debug!(link_id = %link_id, "Magic link already claimed by another request");
			log_magic_link_login_failed(&state, "already_claimed", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "invalid_token".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, link_id = %link_id, "Failed to claim magic link");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AuthErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	}

	// Provision user
	let request = loom_server_provisioning::ProvisioningRequest::magic_link(&email);
	let user = match state.user_provisioning.provision_user(request).await {
		Ok(user) => user,
		Err(loom_server_provisioning::ProvisioningError::SignupsDisabled) => {
			return Redirect::to("/login?error=signups_disabled").into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, email = %email, "Failed to provision user");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AuthErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let session_request =
		SessionRequest::new(user.id, AuthMethod::MagicLink, client_info.into()).with_email(&email);

	let session_response = match state.session_service.create_session(session_request).await {
		Ok(resp) => resp,
		Err(e) => {
			tracing::error!(error = %e, user_id = %user.id, "Failed to create session");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AuthErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	tracing::info!(user_id = %user.id, email = %email, "User authenticated via magic link");

	let mut response_headers = HeaderMap::new();
	if let Ok(value) = HeaderValue::from_str(&session_response.cookie_header) {
		response_headers.insert(SET_COOKIE, value);
	}

	(response_headers, Redirect::to("/")).into_response()
}
