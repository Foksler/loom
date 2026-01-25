// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash API key HTTP handlers.
//!
//! Implements endpoints for API key management.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use loom_crash_core::{CrashApiKey, CrashApiKeyId, CrashKeyType, ProjectId, UserId};
use loom_server_crash::{generate_api_key, hash_api_key, CrashRepository, KEY_PREFIX_ADMIN, KEY_PREFIX_CAPTURE};

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::resolve_user_locale;

use super::common::{
	internal_error, not_found, parse_project_id, verify_org_membership, CrashErrorResponse,
};

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request body for creating an API key.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateApiKeyRequest {
	pub name: String,
	/// "capture" for client-safe keys, "admin" for management keys
	pub key_type: String,
	pub rate_limit_per_minute: Option<u32>,
	#[serde(default)]
	pub allowed_origins: Vec<String>,
}

/// Response for creating an API key.
/// Note: The raw key is only returned once at creation time.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateApiKeyResponse {
	pub id: String,
	pub key: String,
	pub name: String,
	pub key_type: String,
	pub created_at: String,
}

/// Response for listing API keys.
/// Note: key_hash is not exposed, only metadata.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiKeyResponse {
	pub id: String,
	pub name: String,
	pub key_type: String,
	pub rate_limit_per_minute: Option<u32>,
	pub allowed_origins: Vec<String>,
	pub created_at: String,
	pub last_used_at: Option<String>,
	pub revoked_at: Option<String>,
}

impl From<CrashApiKey> for ApiKeyResponse {
	fn from(key: CrashApiKey) -> Self {
		Self {
			id: key.id.to_string(),
			name: key.name,
			key_type: key.key_type.to_string(),
			rate_limit_per_minute: key.rate_limit_per_minute,
			allowed_origins: key.allowed_origins,
			created_at: key.created_at.to_rfc3339(),
			last_used_at: key.last_used_at.map(|dt| dt.to_rfc3339()),
			revoked_at: key.revoked_at.map(|dt| dt.to_rfc3339()),
		}
	}
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse an API key ID from a string.
fn parse_api_key_id(
	key_id_str: &str,
) -> Result<CrashApiKeyId, (StatusCode, Json<CrashErrorResponse>)> {
	key_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_key_id".to_string(),
				message: "Invalid API key ID".to_string(),
			}),
		)
	})
}

/// Get a project and verify org membership.
async fn get_project_with_auth(
	state: &AppState,
	project_id: ProjectId,
	user_id: &loom_server_auth::types::UserId,
	locale: &str,
) -> Result<loom_crash_core::CrashProject, (StatusCode, Json<CrashErrorResponse>)> {
	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			internal_error(locale)
		})?
		.ok_or_else(|| not_found("project"))?;

	verify_org_membership(state, &project.org_id, user_id, locale).await?;

	Ok(project)
}

// ============================================================================
// API Key Endpoints
// ============================================================================

/// POST /api/crash/projects/{project_id}/api-keys - Create a new API key
#[utoipa::path(
	post,
	path = "/api/crash/projects/{project_id}/api-keys",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	request_body = CreateApiKeyRequest,
	responses(
		(status = 201, description = "API key created", body = CreateApiKeyResponse),
		(status = 400, description = "Invalid request", body = CrashErrorResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	tag = "crash"
)]
#[instrument(skip(state, current_user, body), fields(project_id = %project_id))]
pub async fn create_api_key(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id): Path<String>,
	Json(body): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Parse project ID
	let project_id = parse_project_id(&project_id)?;

	// Get project
	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	// Validate key type
	let key_type: CrashKeyType = body.key_type.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_key_type".to_string(),
				message: "Key type must be 'capture' or 'admin'".to_string(),
			}),
		)
	})?;

	// Generate the raw key
	let prefix = match key_type {
		CrashKeyType::Capture => KEY_PREFIX_CAPTURE,
		CrashKeyType::Admin => KEY_PREFIX_ADMIN,
	};
	let raw_key = generate_api_key(prefix);

	// Hash the key for storage
	let key_hash = hash_api_key(&raw_key).map_err(|e| {
		tracing::error!(error = %e, "Failed to hash API key");
		internal_error(&locale)
	})?;

	let now = Utc::now();
	let api_key = CrashApiKey {
		id: CrashApiKeyId::new(),
		project_id,
		name: body.name.clone(),
		key_type,
		key_hash,
		rate_limit_per_minute: body.rate_limit_per_minute,
		allowed_origins: body.allowed_origins,
		created_by: UserId(current_user.user.id.into_inner()),
		created_at: now,
		last_used_at: None,
		revoked_at: None,
	};

	state
		.crash_repo
		.create_api_key(&api_key)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to create API key");
			internal_error(&locale)
		})?;

	info!(
		api_key_id = %api_key.id,
		project_id = %project_id,
		key_type = %key_type,
		"API key created"
	);

	Ok((
		StatusCode::CREATED,
		Json(CreateApiKeyResponse {
			id: api_key.id.to_string(),
			key: raw_key,
			name: api_key.name,
			key_type: api_key.key_type.to_string(),
			created_at: api_key.created_at.to_rfc3339(),
		}),
	))
}

/// GET /api/crash/projects/{project_id}/api-keys - List API keys for a project
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/api-keys",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	responses(
		(status = 200, description = "API keys retrieved", body = Vec<ApiKeyResponse>),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	tag = "crash"
)]
#[instrument(skip(state, current_user), fields(project_id = %project_id))]
pub async fn list_api_keys(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id): Path<String>,
) -> Result<Json<Vec<ApiKeyResponse>>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Parse project ID
	let project_id = parse_project_id(&project_id)?;

	// Get project
	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	// List API keys
	let keys = state
		.crash_repo
		.list_api_keys(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list API keys");
			internal_error(&locale)
		})?;

	Ok(Json(keys.into_iter().map(ApiKeyResponse::from).collect()))
}

/// DELETE /api/crash/projects/{project_id}/api-keys/{key_id} - Revoke an API key
#[utoipa::path(
	delete,
	path = "/api/crash/projects/{project_id}/api-keys/{key_id}",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("key_id" = String, Path, description = "API key ID"),
	),
	responses(
		(status = 204, description = "API key revoked"),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "API key not found", body = CrashErrorResponse),
	),
	tag = "crash"
)]
#[instrument(skip(state, current_user), fields(project_id = %project_id, key_id = %key_id))]
pub async fn revoke_api_key(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id, key_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Parse IDs
	let project_id = parse_project_id(&project_id)?;
	let key_id = parse_api_key_id(&key_id)?;

	// Get project
	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	// Verify API key exists and belongs to project
	let api_key = state
		.crash_repo
		.get_api_key_by_id(key_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get API key");
			internal_error(&locale)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "api_key_not_found".to_string(),
					message: "API key not found".to_string(),
				}),
			)
		})?;

	if api_key.project_id != project_id {
		return Err((
			StatusCode::NOT_FOUND,
			Json(CrashErrorResponse {
				error: "api_key_not_found".to_string(),
				message: "API key not found".to_string(),
			}),
		));
	}

	// Revoke the key
	let revoked = state.crash_repo.revoke_api_key(key_id).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to revoke API key");
		internal_error(&locale)
	})?;

	if revoked {
		info!(api_key_id = %key_id, "API key revoked");
		Ok(StatusCode::NO_CONTENT)
	} else {
		// Key was already revoked
		Err((
			StatusCode::NOT_FOUND,
			Json(CrashErrorResponse {
				error: "api_key_not_found".to_string(),
				message: "API key not found or already revoked".to_string(),
			}),
		))
	}
}
