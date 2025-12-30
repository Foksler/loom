// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! API key management HTTP handlers.
//!
//! Implements API key endpoints per the auth-abac-system.md specification (section 26):
//! - List organization API keys
//! - Create API key
//! - Revoke API key
//! - Get API key usage logs
//!
//! API keys are org-level only. Owners and admins can create/view/revoke.
//!
//! # Security
//!
//! - API key values are only returned once at creation time
//! - Keys are stored as SHA-256 hashes, never in plaintext
//! - Key revocation is permanent and cannot be undone
//! - All key operations are logged to audit trail
//!
//! # Authorization Matrix
//!
//! | Endpoint             | Required Permission  |
//! |---------------------|---------------------|
//! | `list_api_keys`     | `ManageApiKeys`     |
//! | `create_api_key`    | `ManageApiKeys`     |
//! | `revoke_api_key`    | `ManageApiKeys`     |
//! | `get_api_key_usage` | `ManageApiKeys`     |

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::{DateTime, Utc};
use loom_auth::{Action, ApiKeyScope, AuditEventType, AuditLogEntry, OrgId, Visibility};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

use crate::{
	abac_middleware::{build_subject_attrs, org_resource},
	api::AppState,
	auth_middleware::RequireAuth,
	authorize,
	i18n::{resolve_user_locale, t},
};

/// Hash a token using SHA-256 and return the hex-encoded result.
fn hash_token(token: &str) -> String {
	let mut hasher = Sha256::new();
	hasher.update(token.as_bytes());
	hex::encode(hasher.finalize())
}

/// API key scope for request/response types.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyScopeApi {
	/// Read thread data.
	ThreadsRead,
	/// Create and update threads.
	ThreadsWrite,
	/// Delete threads.
	ThreadsDelete,
	/// Use LLM services.
	LlmUse,
	/// Execute tools.
	ToolsUse,
}

impl From<ApiKeyScopeApi> for ApiKeyScope {
	fn from(scope: ApiKeyScopeApi) -> Self {
		match scope {
			ApiKeyScopeApi::ThreadsRead => ApiKeyScope::ThreadsRead,
			ApiKeyScopeApi::ThreadsWrite => ApiKeyScope::ThreadsWrite,
			ApiKeyScopeApi::ThreadsDelete => ApiKeyScope::ThreadsDelete,
			ApiKeyScopeApi::LlmUse => ApiKeyScope::LlmUse,
			ApiKeyScopeApi::ToolsUse => ApiKeyScope::ToolsUse,
		}
	}
}

impl From<ApiKeyScope> for ApiKeyScopeApi {
	fn from(scope: ApiKeyScope) -> Self {
		match scope {
			ApiKeyScope::ThreadsRead => ApiKeyScopeApi::ThreadsRead,
			ApiKeyScope::ThreadsWrite => ApiKeyScopeApi::ThreadsWrite,
			ApiKeyScope::ThreadsDelete => ApiKeyScopeApi::ThreadsDelete,
			ApiKeyScope::LlmUse => ApiKeyScopeApi::LlmUse,
			ApiKeyScope::ToolsUse => ApiKeyScopeApi::ToolsUse,
		}
	}
}

/// An API key in API responses (without the secret key).
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyResponse {
	/// Unique identifier for the API key.
	pub id: String,
	/// Human-readable name for the key.
	pub name: String,
	/// Scopes granted to this key.
	pub scopes: Vec<ApiKeyScopeApi>,
	/// User ID who created the key.
	pub created_by: String,
	/// When the key was created.
	pub created_at: DateTime<Utc>,
	/// When the key was last used.
	pub last_used_at: Option<DateTime<Utc>>,
	/// When the key was revoked (None if active).
	pub revoked_at: Option<DateTime<Utc>>,
}

/// Response for creating a new API key.
///
/// # Security
///
/// The `key` field contains the plaintext API key value. This is the **only time**
/// the key will be shown - store it securely immediately!
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
	/// Unique identifier for the API key.
	pub id: String,
	/// The actual API key value (only shown once!).
	pub key: String,
	/// Human-readable name for the key.
	pub name: String,
	/// Scopes granted to this key.
	pub scopes: Vec<ApiKeyScopeApi>,
	/// When the key was created.
	pub created_at: DateTime<Utc>,
}

/// Response for listing API keys.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListApiKeysResponse {
	pub api_keys: Vec<ApiKeyResponse>,
}

/// Request to create an API key.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
	/// Human-readable name for the key.
	pub name: String,
	/// Scopes to grant to this key.
	pub scopes: Vec<ApiKeyScopeApi>,
}

/// API key usage log entry.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyUsageResponse {
	/// Unique identifier for this usage record.
	pub id: String,
	/// When the request was made.
	pub timestamp: DateTime<Utc>,
	/// Client IP address (if available).
	pub ip_address: Option<String>,
	/// API endpoint accessed.
	pub endpoint: String,
	/// HTTP method used.
	pub method: String,
}

/// Response for API key usage logs.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyUsageListResponse {
	pub usage: Vec<ApiKeyUsageResponse>,
	pub total: i64,
}

/// Success response for API key operations.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeySuccessResponse {
	pub message: String,
}

/// Error response for API key operations.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyErrorResponse {
	pub error: String,
	pub message: String,
}

/// List all API keys for an organization.
///
/// # Authorization
///
/// Requires `ManageApiKeys` permission on the organization. Typically granted to
/// organization owners and admins.
///
/// # Request
///
/// Path parameters:
/// - `org_id`: Organization UUID
///
/// # Response
///
/// Returns [`ListApiKeysResponse`] with all API keys (active and revoked).
/// Note: The actual key values are never returned, only metadata.
///
/// # Errors
///
/// - `400 Bad Request`: Invalid organization ID format
/// - `401 Unauthorized`: Missing or invalid authentication
/// - `403 Forbidden`: Caller lacks `ManageApiKeys` permission
/// - `404 Not Found`: Organization does not exist
/// - `500 Internal Server Error`: Database error
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/api-keys",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of API keys", body = ListApiKeysResponse),
        (status = 401, description = "Not authenticated", body = ApiKeyErrorResponse),
        (status = 403, description = "Not authorized (must be owner or admin)", body = ApiKeyErrorResponse),
        (status = 404, description = "Organization not found", body = ApiKeyErrorResponse)
    ),
    tag = "api-keys"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id
	)
)]
pub async fn list_api_keys(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = match org_id.parse::<uuid::Uuid>() {
		Ok(id) => OrgId::new(id),
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ApiKeyErrorResponse {
					error: "invalid_org_id".to_string(),
					message: t(locale, "server.api.api_key.invalid_org_id").to_string(),
				}),
			)
				.into_response();
		}
	};

	if state.org_repo.get_org_by_id(&org_id).await.unwrap_or(None).is_none() {
		return (
			StatusCode::NOT_FOUND,
			Json(ApiKeyErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.org.not_found").to_string(),
			}),
		)
			.into_response();
	}

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = org_resource(org_id, Visibility::Private);

	if let Err(e) = authorize!(&subject, Action::ManageApiKeys, &resource) {
		tracing::warn!(
			actor_id = %current_user.user.id,
			org_id = %org_id,
			"Unauthorized API key list attempt"
		);
		return e.into_response();
	}

	match state.api_key_repo.list_api_keys_for_org(&org_id).await {
		Ok(keys) => {
			let api_keys: Vec<ApiKeyResponse> = keys
				.into_iter()
				.map(|k| ApiKeyResponse {
					id: k.id.to_string(),
					name: k.name,
					scopes: k.scopes.into_iter().map(Into::into).collect(),
					created_by: k.created_by.to_string(),
					created_at: k.created_at,
					last_used_at: k.last_used_at,
					revoked_at: k.revoked_at,
				})
				.collect();

			tracing::info!(
				actor_id = %current_user.user.id,
				org_id = %org_id,
				key_count = api_keys.len(),
				"Listed API keys"
			);

			(StatusCode::OK, Json(ListApiKeysResponse { api_keys })).into_response()
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				org_id = %org_id,
				"Failed to list API keys"
			);
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ApiKeyErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.api_key.list_failed").to_string(),
				}),
			)
				.into_response()
		}
	}
}

/// Create a new API key for an organization.
///
/// # Authorization
///
/// Requires `ManageApiKeys` permission on the organization.
///
/// # Request
///
/// Path parameters:
/// - `org_id`: Organization UUID
///
/// Body ([`CreateApiKeyRequest`]):
/// - `name`: Human-readable name for the key
/// - `scopes`: Array of permission scopes to grant
///
/// # Response
///
/// Returns [`CreateApiKeyResponse`] including the plaintext key value.
///
/// # Security
///
/// **IMPORTANT**: The API key value is only returned in this response!
/// Store it securely immediately - it cannot be retrieved again.
///
/// # Errors
///
/// - `400 Bad Request`: Empty name or no scopes specified
/// - `401 Unauthorized`: Missing or invalid authentication
/// - `403 Forbidden`: Caller lacks `ManageApiKeys` permission
/// - `404 Not Found`: Organization does not exist
/// - `500 Internal Server Error`: Database error
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/api-keys",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid request (bad name or scopes)", body = ApiKeyErrorResponse),
        (status = 401, description = "Not authenticated", body = ApiKeyErrorResponse),
        (status = 403, description = "Not authorized (must be owner or admin)", body = ApiKeyErrorResponse),
        (status = 404, description = "Organization not found", body = ApiKeyErrorResponse)
    ),
    tag = "api-keys"
)]
#[tracing::instrument(
	skip(state, payload),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id
	)
)]
pub async fn create_api_key(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = match org_id.parse::<uuid::Uuid>() {
		Ok(id) => OrgId::new(id),
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ApiKeyErrorResponse {
					error: "invalid_org_id".to_string(),
					message: t(locale, "server.api.api_key.invalid_org_id").to_string(),
				}),
			)
				.into_response();
		}
	};

	if state.org_repo.get_org_by_id(&org_id).await.unwrap_or(None).is_none() {
		return (
			StatusCode::NOT_FOUND,
			Json(ApiKeyErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.org.not_found").to_string(),
			}),
		)
			.into_response();
	}

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = org_resource(org_id, Visibility::Private);

	if let Err(e) = authorize!(&subject, Action::ManageApiKeys, &resource) {
		tracing::warn!(
			actor_id = %current_user.user.id,
			org_id = %org_id,
			"Unauthorized API key creation attempt"
		);
		return e.into_response();
	}

	if payload.name.trim().is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(ApiKeyErrorResponse {
				error: "invalid_name".to_string(),
				message: t(locale, "server.api.api_key.name_empty").to_string(),
			}),
		)
			.into_response();
	}

	if payload.scopes.is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(ApiKeyErrorResponse {
				error: "invalid_scopes".to_string(),
				message: t(locale, "server.api.api_key.no_scopes").to_string(),
			}),
		)
			.into_response();
	}

	let scopes: Vec<ApiKeyScope> = payload.scopes.iter().cloned().map(Into::into).collect();
	let (plaintext_key, _argon_hash) = loom_auth::generate_api_key();
	let token_hash = hash_token(&plaintext_key);

	match state
		.api_key_repo
		.create_api_key(&org_id, &payload.name, &token_hash, &scopes, &current_user.user.id)
		.await
	{
		Ok(id) => {
			let now = Utc::now();

			let audit_entry = AuditLogEntry::builder(AuditEventType::ApiKeyCreated)
				.actor(current_user.user.id)
				.resource("api_key", &id)
				.action("Created API key")
				.details(json!({
					"org_id": org_id.to_string(),
					"name": payload.name,
					"scopes": scopes.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
				}))
				.build();

			if let Err(e) = state.audit_repo.log_event(&audit_entry).await {
				tracing::warn!(error = %e, "Failed to log API key creation audit event");
			}

			// NOTE: plaintext_key is intentionally NOT logged - it's a secret
			tracing::info!(
				actor_id = %current_user.user.id,
				org_id = %org_id,
				api_key_id = %id,
				key_name = %payload.name,
				"API key created"
			);

			(
				StatusCode::CREATED,
				Json(CreateApiKeyResponse {
					id,
					key: plaintext_key,
					name: payload.name,
					scopes: payload.scopes,
					created_at: now,
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				org_id = %org_id,
				"Failed to create API key"
			);
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ApiKeyErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.api_key.create_failed").to_string(),
				}),
			)
				.into_response()
		}
	}
}

/// Revoke an API key.
///
/// # Authorization
///
/// Requires `ManageApiKeys` permission on the organization.
///
/// # Request
///
/// Path parameters:
/// - `org_id`: Organization UUID
/// - `id`: API key ID to revoke
///
/// # Response
///
/// Returns [`ApiKeySuccessResponse`] confirming revocation.
///
/// # Errors
///
/// - `400 Bad Request`: Invalid organization ID format
/// - `401 Unauthorized`: Missing or invalid authentication
/// - `403 Forbidden`: Caller lacks `ManageApiKeys` permission
/// - `404 Not Found`: Organization or API key not found
/// - `409 Conflict`: API key is already revoked
/// - `500 Internal Server Error`: Database error
///
/// # Security
///
/// - Revocation is permanent and cannot be undone
/// - Revoked keys immediately stop working
/// - Revocation is logged to audit trail
#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/api-keys/{id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key revoked", body = ApiKeySuccessResponse),
        (status = 401, description = "Not authenticated", body = ApiKeyErrorResponse),
        (status = 403, description = "Not authorized (must be owner or admin)", body = ApiKeyErrorResponse),
        (status = 404, description = "Organization or API key not found", body = ApiKeyErrorResponse),
        (status = 409, description = "API key already revoked", body = ApiKeyErrorResponse)
    ),
    tag = "api-keys"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id,
		api_key_id = %id
	)
)]
pub async fn revoke_api_key(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((org_id, id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = match org_id.parse::<uuid::Uuid>() {
		Ok(id) => OrgId::new(id),
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ApiKeyErrorResponse {
					error: "invalid_org_id".to_string(),
					message: t(locale, "server.api.api_key.invalid_org_id").to_string(),
				}),
			)
				.into_response();
		}
	};

	if state.org_repo.get_org_by_id(&org_id).await.unwrap_or(None).is_none() {
		return (
			StatusCode::NOT_FOUND,
			Json(ApiKeyErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.org.not_found").to_string(),
			}),
		)
			.into_response();
	}

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = org_resource(org_id, Visibility::Private);

	if let Err(e) = authorize!(&subject, Action::ManageApiKeys, &resource) {
		tracing::warn!(
			actor_id = %current_user.user.id,
			org_id = %org_id,
			api_key_id = %id,
			"Unauthorized API key revocation attempt"
		);
		return e.into_response();
	}

	let api_key = match state.api_key_repo.get_api_key_by_id(&id).await {
		Ok(Some(key)) => key,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ApiKeyErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.api_key.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				api_key_id = %id,
				"Failed to get API key"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ApiKeyErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if api_key.org_id != org_id {
		return (
			StatusCode::NOT_FOUND,
			Json(ApiKeyErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.api_key.not_found").to_string(),
			}),
		)
			.into_response();
	}

	if api_key.revoked_at.is_some() {
		return (
			StatusCode::CONFLICT,
			Json(ApiKeyErrorResponse {
				error: "already_revoked".to_string(),
				message: t(locale, "server.api.api_key.already_revoked").to_string(),
			}),
		)
			.into_response();
	}

	match state.api_key_repo.revoke_api_key(&id, &current_user.user.id).await {
		Ok(true) => {
			let audit_entry = AuditLogEntry::builder(AuditEventType::ApiKeyRevoked)
				.actor(current_user.user.id)
				.resource("api_key", &id)
				.action("Revoked API key")
				.details(json!({
					"org_id": org_id.to_string(),
					"name": api_key.name,
				}))
				.build();

			if let Err(e) = state.audit_repo.log_event(&audit_entry).await {
				tracing::warn!(error = %e, "Failed to log API key revocation audit event");
			}

			tracing::info!(
				actor_id = %current_user.user.id,
				org_id = %org_id,
				api_key_id = %id,
				key_name = %api_key.name,
				"API key revoked"
			);

			(
				StatusCode::OK,
				Json(ApiKeySuccessResponse {
					message: t(locale, "server.api.api_key.revoked").to_string(),
				}),
			)
				.into_response()
		}
		Ok(false) => (
			StatusCode::CONFLICT,
			Json(ApiKeyErrorResponse {
				error: "already_revoked".to_string(),
				message: t(locale, "server.api.api_key.already_revoked").to_string(),
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				api_key_id = %id,
				"Failed to revoke API key"
			);
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ApiKeyErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.api_key.revoke_failed").to_string(),
				}),
			)
				.into_response()
		}
	}
}

/// Get usage logs for an API key.
///
/// # Authorization
///
/// Requires `ManageApiKeys` permission on the organization.
///
/// # Request
///
/// Path parameters:
/// - `org_id`: Organization UUID
/// - `id`: API key ID
///
/// # Response
///
/// Returns [`ApiKeyUsageListResponse`] with recent usage records.
///
/// # Errors
///
/// - `400 Bad Request`: Invalid organization ID format
/// - `401 Unauthorized`: Missing or invalid authentication
/// - `403 Forbidden`: Caller lacks `ManageApiKeys` permission
/// - `404 Not Found`: Organization or API key not found
/// - `500 Internal Server Error`: Database error
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/api-keys/{id}/usage",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key usage log", body = ApiKeyUsageListResponse),
        (status = 401, description = "Not authenticated", body = ApiKeyErrorResponse),
        (status = 403, description = "Not authorized (must be owner or admin)", body = ApiKeyErrorResponse),
        (status = 404, description = "Organization or API key not found", body = ApiKeyErrorResponse)
    ),
    tag = "api-keys"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id,
		api_key_id = %id
	)
)]
pub async fn get_api_key_usage(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((org_id, id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = match org_id.parse::<uuid::Uuid>() {
		Ok(id) => OrgId::new(id),
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ApiKeyErrorResponse {
					error: "invalid_org_id".to_string(),
					message: t(locale, "server.api.api_key.invalid_org_id").to_string(),
				}),
			)
				.into_response();
		}
	};

	if state.org_repo.get_org_by_id(&org_id).await.unwrap_or(None).is_none() {
		return (
			StatusCode::NOT_FOUND,
			Json(ApiKeyErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.org.not_found").to_string(),
			}),
		)
			.into_response();
	}

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = org_resource(org_id, Visibility::Private);

	if let Err(e) = authorize!(&subject, Action::ManageApiKeys, &resource) {
		tracing::warn!(
			actor_id = %current_user.user.id,
			org_id = %org_id,
			api_key_id = %id,
			"Unauthorized API key usage access attempt"
		);
		return e.into_response();
	}

	let api_key = match state.api_key_repo.get_api_key_by_id(&id).await {
		Ok(Some(key)) => key,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(ApiKeyErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.api_key.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				api_key_id = %id,
				"Failed to get API key"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ApiKeyErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if api_key.org_id != org_id {
		return (
			StatusCode::NOT_FOUND,
			Json(ApiKeyErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.api_key.not_found").to_string(),
			}),
		)
			.into_response();
	}

	match state.api_key_repo.get_usage_logs(&id, 100, 0).await {
		Ok((logs, total)) => {
			let usage: Vec<ApiKeyUsageResponse> = logs
				.into_iter()
				.map(|l| ApiKeyUsageResponse {
					id: l.id.to_string(),
					timestamp: l.timestamp,
					ip_address: l.ip_address.map(|ip| ip.to_string()),
					endpoint: l.endpoint,
					method: l.method,
				})
				.collect();

			tracing::info!(
				actor_id = %current_user.user.id,
				org_id = %org_id,
				api_key_id = %id,
				usage_count = usage.len(),
				"Retrieved API key usage logs"
			);

			(StatusCode::OK, Json(ApiKeyUsageListResponse { usage, total })).into_response()
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				api_key_id = %id,
				"Failed to get API key usage logs"
			);
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ApiKeyErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.api_key.usage_failed").to_string(),
				}),
			)
				.into_response()
		}
	}
}
