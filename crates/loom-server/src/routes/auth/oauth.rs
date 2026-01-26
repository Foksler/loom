// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! OAuth provider handlers (GitHub, Google, Okta).

use axum::{
	extract::State,
	http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode},
	response::{IntoResponse, Redirect},
	Json,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder};
use loom_server_session::{AuthMethod, SessionRequest};

use crate::{
	api::AppState,
	client_info::ClientInfo,
	i18n::{t, t_fmt},
	oauth_state::{generate_nonce, generate_state, sanitize_redirect},
};

/// Log a failed OAuth login attempt for security auditing.
fn log_oauth_login_failed(state: &AppState, provider: &str, error: &str, ip_address: Option<&str>) {
	let mut builder = AuditLogBuilder::new(AuditEventType::LoginFailed)
		.details(serde_json::json!({
			"provider": provider,
			"error": error,
		}));

	if let Some(ip) = ip_address {
		builder = builder.ip_address(ip.to_string());
	}

	state.audit_service.log(builder.build());
}

use super::common::{AuthErrorResponse, OAuthCallbackQuery, OAuthLoginQuery, OAuthRedirectResponse};

#[utoipa::path(
    get,
    path = "/auth/login/github",
    params(
        ("redirect" = Option<String>, Query, description = "Redirect URL after successful authentication")
    ),
    responses(
        (status = 200, description = "GitHub OAuth redirect URL", body = OAuthRedirectResponse),
        (status = 501, description = "Not configured", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Initiates GitHub OAuth flow.
///
/// Returns a redirect URL to GitHub's authorization endpoint. The client
/// should redirect the user to this URL to begin the OAuth flow.
///
/// # Errors
/// Returns 501 Not Implemented if GitHub OAuth is not configured.
#[tracing::instrument(skip(state, query), fields(provider = "github"))]
pub async fn login_github(
	State(state): State<AppState>,
	axum::extract::Query(query): axum::extract::Query<OAuthLoginQuery>,
) -> impl IntoResponse {
	let locale = state.default_locale.as_str();
	tracing::debug!("Initiating GitHub OAuth flow");
	let Some(github_client) = &state.github_oauth else {
		return (
			StatusCode::NOT_IMPLEMENTED,
			Json(AuthErrorResponse {
				error: "not_configured".to_string(),
				message: t_fmt(
					locale,
					"server.api.auth.oauth_not_configured",
					&[("provider", "GitHub")],
				),
			}),
		)
			.into_response();
	};

	let oauth_state = generate_state();
	let redirect_url_param = query
		.redirect
		.as_deref()
		.map(|r| sanitize_redirect(Some(r)));
	state
		.oauth_state_store
		.store(
			oauth_state.clone(),
			"github".to_string(),
			None,
			redirect_url_param,
		)
		.await;

	let redirect_url = github_client.authorization_url(&oauth_state);

	Json(OAuthRedirectResponse { redirect_url }).into_response()
}

#[utoipa::path(
    get,
    path = "/auth/login/google",
    params(
        ("redirect" = Option<String>, Query, description = "Redirect URL after successful authentication")
    ),
    responses(
        (status = 200, description = "Google OAuth redirect URL", body = OAuthRedirectResponse),
        (status = 501, description = "Not configured", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Initiates Google OAuth flow.
///
/// Returns a redirect URL to Google's authorization endpoint. The client
/// should redirect the user to this URL to begin the OAuth flow.
///
/// # Errors
/// Returns 501 Not Implemented if Google OAuth is not configured.
#[tracing::instrument(skip(state, query), fields(provider = "google"))]
pub async fn login_google(
	State(state): State<AppState>,
	axum::extract::Query(query): axum::extract::Query<OAuthLoginQuery>,
) -> impl IntoResponse {
	let locale = state.default_locale.as_str();
	tracing::debug!("Initiating Google OAuth flow");
	let Some(google_client) = &state.google_oauth else {
		return (
			StatusCode::NOT_IMPLEMENTED,
			Json(AuthErrorResponse {
				error: "not_configured".to_string(),
				message: t_fmt(
					locale,
					"server.api.auth.oauth_not_configured",
					&[("provider", "Google")],
				),
			}),
		)
			.into_response();
	};

	let oauth_state = generate_state();
	let nonce = generate_nonce();
	let redirect_url_param = query
		.redirect
		.as_deref()
		.map(|r| sanitize_redirect(Some(r)));
	state
		.oauth_state_store
		.store(
			oauth_state.clone(),
			"google".to_string(),
			Some(nonce.clone()),
			redirect_url_param,
		)
		.await;

	let redirect_url = google_client.authorization_url(&oauth_state, &nonce);

	Json(OAuthRedirectResponse { redirect_url }).into_response()
}

#[utoipa::path(
    get,
    path = "/auth/login/okta",
    params(
        ("redirect" = Option<String>, Query, description = "Redirect URL after successful authentication")
    ),
    responses(
        (status = 200, description = "Okta OAuth redirect URL", body = OAuthRedirectResponse),
        (status = 501, description = "Not configured", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Initiates Okta OAuth flow.
///
/// Returns a redirect URL to Okta's authorization endpoint. The client
/// should redirect the user to this URL to begin the OAuth flow.
///
/// # Errors
/// Returns 501 Not Implemented if Okta OAuth is not configured.
#[tracing::instrument(skip(state, query), fields(provider = "okta"))]
pub async fn login_okta(
	State(state): State<AppState>,
	axum::extract::Query(query): axum::extract::Query<OAuthLoginQuery>,
) -> impl IntoResponse {
	let locale = state.default_locale.as_str();
	tracing::debug!("Initiating Okta OAuth flow");
	let Some(okta_client) = &state.okta_oauth else {
		return (
			StatusCode::NOT_IMPLEMENTED,
			Json(AuthErrorResponse {
				error: "not_configured".to_string(),
				message: t_fmt(
					locale,
					"server.api.auth.oauth_not_configured",
					&[("provider", "Okta")],
				),
			}),
		)
			.into_response();
	};

	let oauth_state = generate_state();
	let nonce = generate_nonce();
	let redirect_url_param = query
		.redirect
		.as_deref()
		.map(|r| sanitize_redirect(Some(r)));
	state
		.oauth_state_store
		.store(
			oauth_state.clone(),
			"okta".to_string(),
			Some(nonce.clone()),
			redirect_url_param,
		)
		.await;

	let redirect_url = okta_client.authorization_url(&oauth_state, &nonce);

	Json(OAuthRedirectResponse { redirect_url }).into_response()
}

#[utoipa::path(
    get,
    path = "/auth/github/callback",
    params(
        ("code" = Option<String>, Query, description = "Authorization code from GitHub"),
        ("state" = Option<String>, Query, description = "State parameter for CSRF protection"),
        ("error" = Option<String>, Query, description = "Error code if authorization failed")
    ),
    responses(
        (status = 302, description = "Redirect to dashboard on success"),
        (status = 400, description = "Invalid callback", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Handles the GitHub OAuth callback.
///
/// Exchanges the authorization code for tokens and creates a session.
/// On success, redirects to dashboard with session cookie set.
///
/// # Security
/// Authorization codes are single-use. Never log the code or access token.
///
/// # Errors
/// - 400 Bad Request: Missing code/state, invalid state, token exchange failed
#[tracing::instrument(skip(state, query, headers), fields(provider = "github"))]
pub async fn callback_github(
	State(state): State<AppState>,
	headers: HeaderMap,
	axum::extract::Query(query): axum::extract::Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
	let client_info = ClientInfo::from_headers(&headers, state.geoip_service.as_ref());
	let locale = state.default_locale.as_str();
	if let Some(error) = &query.error {
		tracing::warn!(error = %error, "GitHub OAuth error");
		log_oauth_login_failed(&state, "github", error, client_info.ip_address.as_deref());
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "oauth_error".to_string(),
				message: format!("GitHub authorization failed: {error}"),
			}),
		)
			.into_response();
	}

	let Some(github_client) = &state.github_oauth else {
		return (
			StatusCode::NOT_IMPLEMENTED,
			Json(AuthErrorResponse {
				error: "not_configured".to_string(),
				message: t_fmt(
					locale,
					"server.api.auth.oauth_not_configured",
					&[("provider", "GitHub")],
				),
			}),
		)
			.into_response();
	};

	let (Some(code), Some(oauth_state)) = (&query.code, &query.state) else {
		log_oauth_login_failed(&state, "github", "missing_code_or_state", client_info.ip_address.as_deref());
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "invalid_request".to_string(),
				message: "Missing code or state parameter".to_string(),
			}),
		)
			.into_response();
	};

	let state_entry = match state
		.oauth_state_store
		.validate_and_consume(oauth_state, "github")
		.await
	{
		Some(entry) => entry,
		None => {
			log_oauth_login_failed(&state, "github", "invalid_state", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "invalid_state".to_string(),
					message: "Invalid or expired state parameter".to_string(),
				}),
			)
				.into_response();
		}
	};

	let token_response = match github_client.exchange_code(code).await {
		Ok(resp) => resp,
		Err(e) => {
			tracing::error!(error = %e, "Failed to exchange GitHub code");
			log_oauth_login_failed(&state, "github", "token_exchange_failed", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "token_exchange_failed".to_string(),
					message: "Failed to exchange authorization code".to_string(),
				}),
			)
				.into_response();
		}
	};

	let github_user = match github_client
		.get_user(token_response.access_token.expose())
		.await
	{
		Ok(user) => user,
		Err(e) => {
			tracing::error!(error = %e, "Failed to get GitHub user");
			log_oauth_login_failed(&state, "github", "user_fetch_failed", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "user_fetch_failed".to_string(),
					message: "Failed to fetch user information".to_string(),
				}),
			)
				.into_response();
		}
	};

	let email = if let Some(email) = &github_user.email {
		email.clone()
	} else {
		match github_client
			.get_emails(token_response.access_token.expose())
			.await
		{
			Ok(emails) => {
				match emails
					.into_iter()
					.find(|e| e.primary && e.verified)
					.map(|e| e.email)
				{
					Some(email) => email,
					None => {
						tracing::warn!(login = %github_user.login, "No verified email found for GitHub user");
						log_oauth_login_failed(&state, "github", "no_verified_email", client_info.ip_address.as_deref());
						return (
							StatusCode::BAD_REQUEST,
							Json(AuthErrorResponse {
								error: "no_verified_email".to_string(),
								message: t(locale, "server.api.auth.no_verified_email").to_string(),
							}),
						)
							.into_response();
					}
				}
			}
			Err(e) => {
				tracing::warn!(error = %e, login = %github_user.login, "Failed to fetch GitHub emails");
				log_oauth_login_failed(&state, "github", "email_fetch_failed", client_info.ip_address.as_deref());
				return (
					StatusCode::BAD_REQUEST,
					Json(AuthErrorResponse {
						error: "no_verified_email".to_string(),
						message: t(locale, "server.api.auth.no_verified_email").to_string(),
					}),
				)
					.into_response();
			}
		}
	};

	let display_name = github_user
		.name
		.unwrap_or_else(|| github_user.login.clone());

	let preferred_username = github_user.login.clone();

	complete_oauth_login(
		&state,
		&email,
		&display_name,
		github_user.avatar_url,
		AuthMethod::GitHub,
		client_info,
		state_entry.redirect_url,
		Some(&preferred_username),
	)
	.await
}

#[utoipa::path(
    get,
    path = "/auth/google/callback",
    params(
        ("code" = Option<String>, Query, description = "Authorization code from Google"),
        ("state" = Option<String>, Query, description = "State parameter for CSRF protection"),
        ("error" = Option<String>, Query, description = "Error code if authorization failed")
    ),
    responses(
        (status = 302, description = "Redirect to dashboard on success"),
        (status = 400, description = "Invalid callback", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Handles the Google OAuth callback.
///
/// Exchanges the authorization code for tokens and creates a session.
/// On success, redirects to dashboard with session cookie set.
///
/// # Security
/// Authorization codes are single-use. Never log the code or access token.
///
/// # Errors
/// - 400 Bad Request: Missing code/state, invalid state, token exchange failed
#[tracing::instrument(skip(state, query, headers), fields(provider = "google"))]
pub async fn callback_google(
	State(state): State<AppState>,
	headers: HeaderMap,
	axum::extract::Query(query): axum::extract::Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
	let client_info = ClientInfo::from_headers(&headers, state.geoip_service.as_ref());
	let locale = state.default_locale.as_str();
	if let Some(error) = &query.error {
		tracing::warn!(error = %error, "Google OAuth error");
		log_oauth_login_failed(&state, "google", error, client_info.ip_address.as_deref());
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "oauth_error".to_string(),
				message: format!("Google authorization failed: {error}"),
			}),
		)
			.into_response();
	}

	let Some(google_client) = &state.google_oauth else {
		return (
			StatusCode::NOT_IMPLEMENTED,
			Json(AuthErrorResponse {
				error: "not_configured".to_string(),
				message: t_fmt(
					locale,
					"server.api.auth.oauth_not_configured",
					&[("provider", "Google")],
				),
			}),
		)
			.into_response();
	};

	let (Some(code), Some(oauth_state)) = (&query.code, &query.state) else {
		log_oauth_login_failed(&state, "google", "missing_code_or_state", client_info.ip_address.as_deref());
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "invalid_request".to_string(),
				message: "Missing code or state parameter".to_string(),
			}),
		)
			.into_response();
	};

	let state_entry = match state
		.oauth_state_store
		.validate_and_consume(oauth_state, "google")
		.await
	{
		Some(entry) => entry,
		None => {
			log_oauth_login_failed(&state, "google", "invalid_state", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "invalid_state".to_string(),
					message: "Invalid or expired state parameter".to_string(),
				}),
			)
				.into_response();
		}
	};

	let token_response = match google_client.exchange_code(code).await {
		Ok(resp) => resp,
		Err(e) => {
			tracing::error!(error = %e, "Failed to exchange Google code");
			log_oauth_login_failed(&state, "google", "token_exchange_failed", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "token_exchange_failed".to_string(),
					message: "Failed to exchange authorization code".to_string(),
				}),
			)
				.into_response();
		}
	};

	let google_user = match google_client
		.get_user_info(token_response.access_token.expose())
		.await
	{
		Ok(user) => user,
		Err(e) => {
			tracing::error!(error = %e, "Failed to get Google user");
			log_oauth_login_failed(&state, "google", "user_fetch_failed", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "user_fetch_failed".to_string(),
					message: "Failed to fetch user information".to_string(),
				}),
			)
				.into_response();
		}
	};

	if !google_user.email_verified {
		tracing::warn!(email = %google_user.email, "Google email not verified");
		log_oauth_login_failed(&state, "google", "email_not_verified", client_info.ip_address.as_deref());
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "email_not_verified".to_string(),
				message: t(locale, "server.api.auth.email_not_verified").to_string(),
			}),
		)
			.into_response();
	}

	let display_name = google_user
		.name
		.unwrap_or_else(|| google_user.email.clone());

	complete_oauth_login(
		&state,
		&google_user.email,
		&display_name,
		google_user.picture,
		AuthMethod::Google,
		client_info,
		state_entry.redirect_url,
		None,
	)
	.await
}

#[utoipa::path(
    get,
    path = "/auth/okta/callback",
    params(
        ("code" = Option<String>, Query, description = "Authorization code from Okta"),
        ("state" = Option<String>, Query, description = "State parameter for CSRF protection"),
        ("error" = Option<String>, Query, description = "Error code if authorization failed")
    ),
    responses(
        (status = 302, description = "Redirect to dashboard on success"),
        (status = 400, description = "Invalid callback", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Handles the Okta OAuth callback.
///
/// Exchanges the authorization code for tokens and creates a session.
/// On success, redirects to dashboard with session cookie set.
///
/// # Security
/// Authorization codes are single-use. Never log the code or access token.
///
/// # Errors
/// - 400 Bad Request: Missing code/state, invalid state, token exchange failed
#[tracing::instrument(skip(state, query, headers), fields(provider = "okta"))]
pub async fn callback_okta(
	State(state): State<AppState>,
	headers: HeaderMap,
	axum::extract::Query(query): axum::extract::Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
	let client_info = ClientInfo::from_headers(&headers, state.geoip_service.as_ref());
	let locale = state.default_locale.as_str();
	if let Some(error) = &query.error {
		tracing::warn!(error = %error, "Okta OAuth error");
		log_oauth_login_failed(&state, "okta", error, client_info.ip_address.as_deref());
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "oauth_error".to_string(),
				message: format!("Okta authorization failed: {error}"),
			}),
		)
			.into_response();
	}

	let Some(okta_client) = &state.okta_oauth else {
		return (
			StatusCode::NOT_IMPLEMENTED,
			Json(AuthErrorResponse {
				error: "not_configured".to_string(),
				message: t_fmt(
					locale,
					"server.api.auth.oauth_not_configured",
					&[("provider", "Okta")],
				),
			}),
		)
			.into_response();
	};

	let (Some(code), Some(oauth_state)) = (&query.code, &query.state) else {
		log_oauth_login_failed(&state, "okta", "missing_code_or_state", client_info.ip_address.as_deref());
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "invalid_request".to_string(),
				message: "Missing code or state parameter".to_string(),
			}),
		)
			.into_response();
	};

	let state_entry = match state
		.oauth_state_store
		.validate_and_consume(oauth_state, "okta")
		.await
	{
		Some(entry) => entry,
		None => {
			log_oauth_login_failed(&state, "okta", "invalid_state", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "invalid_state".to_string(),
					message: "Invalid or expired state parameter".to_string(),
				}),
			)
				.into_response();
		}
	};

	let token_response = match okta_client.exchange_code(code).await {
		Ok(resp) => resp,
		Err(e) => {
			tracing::error!(error = %e, "Failed to exchange Okta code");
			log_oauth_login_failed(&state, "okta", "token_exchange_failed", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "token_exchange_failed".to_string(),
					message: "Failed to exchange authorization code".to_string(),
				}),
			)
				.into_response();
		}
	};

	let okta_user = match okta_client
		.get_user_info(token_response.access_token.expose())
		.await
	{
		Ok(user) => user,
		Err(e) => {
			tracing::error!(error = %e, "Failed to get Okta user");
			log_oauth_login_failed(&state, "okta", "user_fetch_failed", client_info.ip_address.as_deref());
			return (
				StatusCode::BAD_REQUEST,
				Json(AuthErrorResponse {
					error: "user_fetch_failed".to_string(),
					message: "Failed to fetch user information".to_string(),
				}),
			)
				.into_response();
		}
	};

	if !okta_user.email_verified.unwrap_or(false) {
		tracing::warn!(email = %okta_user.email, "Okta email not verified");
		log_oauth_login_failed(&state, "okta", "email_not_verified", client_info.ip_address.as_deref());
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "email_not_verified".to_string(),
				message: t(locale, "server.api.auth.email_not_verified").to_string(),
			}),
		)
			.into_response();
	}

	let display_name = okta_user.name.unwrap_or_else(|| okta_user.email.clone());

	complete_oauth_login(
		&state,
		&okta_user.email,
		&display_name,
		None,
		AuthMethod::Okta,
		client_info,
		state_entry.redirect_url,
		okta_user.preferred_username.as_deref(),
	)
	.await
}

/// Complete OAuth login by finding/creating user and creating session.
#[allow(clippy::too_many_arguments)]
pub async fn complete_oauth_login(
	state: &AppState,
	email: &str,
	display_name: &str,
	avatar_url: Option<String>,
	auth_method: AuthMethod,
	client_info: ClientInfo,
	redirect_url: Option<String>,
	preferred_username: Option<&str>,
) -> axum::response::Response {
	let locale = state.default_locale.as_str();

	let request = loom_server_provisioning::ProvisioningRequest::oauth(
		email,
		display_name,
		avatar_url,
		preferred_username.map(|s| s.to_string()),
	);
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
		SessionRequest::new(user.id, auth_method, client_info.into()).with_email(email);

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

	tracing::info!(user_id = %user.id, email = %email, auth_method = %auth_method, "User authenticated via OAuth");

	let mut headers = HeaderMap::new();
	if let Ok(value) = HeaderValue::from_str(&session_response.cookie_header) {
		headers.insert(SET_COOKIE, value);
	}

	let final_redirect = sanitize_redirect(redirect_url.as_deref());
	(headers, Redirect::to(&final_redirect)).into_response()
}
