// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Admin routes for Anthropic OAuth pool management.
//!
//! Provides endpoints for managing Claude Max OAuth accounts in the pool:
//! - List accounts with status
//! - Add new accounts via OAuth flow
//! - Remove accounts
//!
//! # Security
//!
//! All endpoints require `SystemRole::Admin`.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::{IntoResponse, Redirect},
	Json,
};
use loom_server_llm_anthropic::{exchange_code, OAuthCredentials, Pkce, CLIENT_ID, SCOPES};
use loom_common_secret::SecretString;
use url::{form_urlencoded, Url};

pub use loom_server_api::admin::{
	AccountDetailsResponse, AccountsSummary, AccountStatus, AdminErrorResponse,
	AnthropicAccountsResponse, AnthropicOAuthCallbackQuery as OAuthCallbackQuery,
	InitiateOAuthRequest, InitiateOAuthResponse, RemoveAccountResponse,
};

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	oauth_state::generate_state,
};

fn url_encode(input: &str) -> String {
	form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

const ANTHROPIC_ADMIN_PROVIDER: &str = "anthropic-admin";

/// List all Anthropic OAuth accounts with status.
///
/// # Authorization
///
/// Requires `system_admin` role.
///
/// # Response
///
/// Returns [`AnthropicAccountsResponse`] with account list and summary.
///
/// # Errors
///
/// - `401 Unauthorized`: Not authenticated
/// - `403 Forbidden`: Not system admin
/// - `501 Not Implemented`: Not in OAuth pool mode
#[utoipa::path(
	get,
	path = "/api/admin/anthropic/accounts",
	responses(
		(status = 200, description = "List of accounts", body = AnthropicAccountsResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 501, description = "Not in OAuth pool mode", body = AdminErrorResponse)
	),
	tag = "admin-anthropic"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id))]
pub async fn list_accounts(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized anthropic accounts list attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let llm_service = match &state.llm_service {
		Some(service) => service,
		None => {
			return (
				StatusCode::NOT_IMPLEMENTED,
				Json(AdminErrorResponse {
					error: "not_implemented".to_string(),
					message: "LLM service not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	if !llm_service.is_anthropic_oauth_pool() {
		return (
			StatusCode::NOT_IMPLEMENTED,
			Json(AdminErrorResponse {
				error: "not_implemented".to_string(),
				message: "Anthropic is not configured in OAuth pool mode".to_string(),
			}),
		)
			.into_response();
	}

	let accounts = match llm_service.anthropic_account_details().await {
		Some(details) => details.into_iter().map(AccountDetailsResponse::from).collect::<Vec<_>>(),
		None => vec![],
	};

	let mut available = 0;
	let mut cooling_down = 0;
	let mut disabled = 0;

	for account in &accounts {
		match account.status {
			AccountStatus::Available => available += 1,
			AccountStatus::CoolingDown => cooling_down += 1,
			AccountStatus::Disabled => disabled += 1,
		}
	}

	let summary = AccountsSummary {
		total: accounts.len(),
		available,
		cooling_down,
		disabled,
	};

	tracing::info!(
		actor_id = %current_user.user.id,
		total = summary.total,
		available = summary.available,
		"Listed Anthropic accounts"
	);

	(StatusCode::OK, Json(AnthropicAccountsResponse { accounts, summary })).into_response()
}

/// Initiate OAuth flow to add a new Anthropic account.
///
/// # Authorization
///
/// Requires `system_admin` role.
///
/// # Request
///
/// Optionally specify `redirect_after` for where to go after OAuth completes.
///
/// # Response
///
/// Returns [`InitiateOAuthResponse`] with the OAuth redirect URL.
///
/// # Errors
///
/// - `401 Unauthorized`: Not authenticated
/// - `403 Forbidden`: Not system admin
/// - `501 Not Implemented`: Not in OAuth pool mode
#[utoipa::path(
	post,
	path = "/api/admin/anthropic/accounts",
	request_body = InitiateOAuthRequest,
	responses(
		(status = 200, description = "OAuth redirect URL", body = InitiateOAuthResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 501, description = "Not in OAuth pool mode", body = AdminErrorResponse)
	),
	tag = "admin-anthropic"
)]
#[tracing::instrument(skip(state, body), fields(actor_id = %current_user.user.id))]
pub async fn initiate_oauth(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Json(body): Json<InitiateOAuthRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized anthropic OAuth initiation attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let llm_service = match &state.llm_service {
		Some(service) => service,
		None => {
			return (
				StatusCode::NOT_IMPLEMENTED,
				Json(AdminErrorResponse {
					error: "not_implemented".to_string(),
					message: "LLM service not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	if !llm_service.is_anthropic_oauth_pool() {
		return (
			StatusCode::NOT_IMPLEMENTED,
			Json(AdminErrorResponse {
				error: "not_implemented".to_string(),
				message: "Anthropic is not configured in OAuth pool mode".to_string(),
			}),
		)
			.into_response();
	}

	let pkce = Pkce::generate();
	let oauth_state = generate_state();

	let redirect_uri = format!("{}/api/admin/anthropic/callback", state.base_url);

	let mut auth_url = Url::parse("https://claude.ai/oauth/authorize").expect("Invalid authorize URL");
	{
		let mut params = auth_url.query_pairs_mut();
		params.append_pair("code", "true");
		params.append_pair("client_id", CLIENT_ID);
		params.append_pair("response_type", "code");
		params.append_pair("redirect_uri", &redirect_uri);
		params.append_pair("scope", SCOPES);
		params.append_pair("code_challenge", &pkce.challenge);
		params.append_pair("code_challenge_method", "S256");
		params.append_pair("state", &oauth_state);
	}

	state.oauth_state_store.store(
		oauth_state.clone(),
		ANTHROPIC_ADMIN_PROVIDER.to_string(),
		Some(pkce.verifier),
		body.redirect_after,
	).await;

	tracing::info!(
		actor_id = %current_user.user.id,
		"Initiated Anthropic OAuth flow"
	);

	(StatusCode::OK, Json(InitiateOAuthResponse { redirect_url: auth_url.to_string() })).into_response()
}

/// Handle OAuth callback from claude.ai.
///
/// This endpoint is called by claude.ai after the user authorizes.
/// It exchanges the code for tokens and adds the account to the pool.
///
/// # Response
///
/// Redirects to `/admin/anthropic-accounts?added={account_id}` on success,
/// or `/admin/anthropic-accounts?error={message}` on failure.
#[utoipa::path(
	get,
	path = "/api/admin/anthropic/callback",
	params(
		("code" = String, Query, description = "Authorization code"),
		("state" = String, Query, description = "OAuth state parameter")
	),
	responses(
		(status = 302, description = "Redirect to admin page"),
		(status = 400, description = "Invalid state or code")
	),
	tag = "admin-anthropic"
)]
#[tracing::instrument(skip(state, query))]
pub async fn oauth_callback(
	State(state): State<AppState>,
	Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
	let entry = match state.oauth_state_store.validate_and_consume(&query.state, ANTHROPIC_ADMIN_PROVIDER).await {
		Some(e) => e,
		None => {
			tracing::warn!("Invalid or expired OAuth state for Anthropic callback");
			return Redirect::to("/admin/anthropic-accounts?error=invalid_state").into_response();
		}
	};

	let verifier = match entry.nonce {
		Some(v) => v,
		None => {
			tracing::error!("Missing PKCE verifier in OAuth state");
			return Redirect::to("/admin/anthropic-accounts?error=missing_verifier").into_response();
		}
	};

	let exchange_result = match exchange_code(&query.code, &verifier).await {
		Ok(result) => result,
		Err(e) => {
			tracing::error!(error = %e, "Failed to exchange OAuth code");
			return Redirect::to(&format!("/admin/anthropic-accounts?error={}", url_encode(&e.to_string()))).into_response();
		}
	};

	let (access, refresh, expires) = match exchange_result {
		loom_server_llm_anthropic::ExchangeResult::Success { access, refresh, expires } => (access, refresh, expires),
		loom_server_llm_anthropic::ExchangeResult::Failed { error } => {
			tracing::error!(error = %error, "OAuth token exchange failed");
			return Redirect::to(&format!("/admin/anthropic-accounts?error={}", url_encode(&error))).into_response();
		}
	};

	let account_id = format!("claude-max-{}", chrono::Utc::now().timestamp());
	let credentials = OAuthCredentials::new(
		SecretString::new(refresh),
		SecretString::new(access),
		expires,
	);

	let llm_service = match &state.llm_service {
		Some(service) => service,
		None => {
			tracing::error!("LLM service not available during OAuth callback");
			return Redirect::to("/admin/anthropic-accounts?error=service_unavailable").into_response();
		}
	};

	if let Err(e) = llm_service.add_anthropic_account(account_id.clone(), credentials).await {
		tracing::error!(error = %e, account_id = %account_id, "Failed to add account to pool");
		return Redirect::to(&format!("/admin/anthropic-accounts?error={}", url_encode(&e.to_string()))).into_response();
	}

	tracing::info!(account_id = %account_id, "Added Anthropic OAuth account to pool");

	let redirect_path = entry.redirect_url.unwrap_or_else(|| "/admin/anthropic-accounts".to_string());
	let redirect_url = if redirect_path.contains('?') {
		format!("{}&added={}", redirect_path, url_encode(&account_id))
	} else {
		format!("{}?added={}", redirect_path, url_encode(&account_id))
	};

	Redirect::to(&redirect_url).into_response()
}

/// Remove an Anthropic OAuth account from the pool.
///
/// # Authorization
///
/// Requires `system_admin` role.
///
/// # Response
///
/// Returns [`RemoveAccountResponse`] with the removed account ID.
///
/// # Errors
///
/// - `401 Unauthorized`: Not authenticated
/// - `403 Forbidden`: Not system admin
/// - `404 Not Found`: Account not found
/// - `501 Not Implemented`: Not in OAuth pool mode
#[utoipa::path(
	delete,
	path = "/api/admin/anthropic/accounts/{id}",
	params(
		("id" = String, Path, description = "Account ID to remove")
	),
	responses(
		(status = 200, description = "Account removed", body = RemoveAccountResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Account not found", body = AdminErrorResponse),
		(status = 501, description = "Not in OAuth pool mode", body = AdminErrorResponse)
	),
	tag = "admin-anthropic"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id, account_id = %id))]
pub async fn remove_account(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized anthropic account removal attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let llm_service = match &state.llm_service {
		Some(service) => service,
		None => {
			return (
				StatusCode::NOT_IMPLEMENTED,
				Json(AdminErrorResponse {
					error: "not_implemented".to_string(),
					message: "LLM service not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	if !llm_service.is_anthropic_oauth_pool() {
		return (
			StatusCode::NOT_IMPLEMENTED,
			Json(AdminErrorResponse {
				error: "not_implemented".to_string(),
				message: "Anthropic is not configured in OAuth pool mode".to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = llm_service.remove_anthropic_account(&id).await {
		let error_msg = e.to_string();
		if error_msg.contains("not found") {
			return (
				StatusCode::NOT_FOUND,
				Json(AdminErrorResponse {
					error: "not_found".to_string(),
					message: format!("Account '{}' not found", id),
				}),
			)
				.into_response();
		}

		tracing::error!(error = %e, account_id = %id, "Failed to remove account");
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
		account_id = %id,
		"Removed Anthropic OAuth account"
	);

	(StatusCode::OK, Json(RemoveAccountResponse { removed: id })).into_response()
}
