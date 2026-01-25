// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! SDK key HTTP handlers.
//!
//! Implements endpoints for SDK key management.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_flags_core::{EnvironmentId, SdkKey, SdkKeyId};
use loom_server_api::flags::{
	CreateSdkKeyRequest, CreateSdkKeyResponse, FlagsErrorResponse, FlagsSuccessResponse,
	ListSdkKeysResponse, SdkKeyResponse,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_flags::{hash_sdk_key, FlagsRepository};

use crate::{
	api::AppState,
	api_response::{bad_request, internal_error, not_found},
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	parse_id,
	validation::parse_org_id as shared_parse_org_id,
};

use super::common::{sdk_key_type_from_api, sdk_key_type_to_api};

// ============================================================================
// SDK Key Routes
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/environments/{env_id}/sdk-keys",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("env_id" = String, Path, description = "Environment ID")
    ),
    responses(
        (status = 200, description = "List of SDK keys", body = ListSdkKeysResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Environment not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// List SDK keys for an environment.
#[tracing::instrument(skip(state), fields(%org_id, %env_id))]
pub async fn list_sdk_keys(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, env_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let env_id: EnvironmentId = match env_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.environment_not_found"),
			)
			.into_response();
		}
	};

	// Check org membership
	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	// Verify environment exists and belongs to org
	let env = match state.flags_repo.get_environment_by_id(env_id).await {
		Ok(Some(env)) => env,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %env_id, "Failed to get environment");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	if env.org_id.0 != org_id.into_inner() {
		return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
			.into_response();
	}

	let sdk_keys = match state.flags_repo.list_sdk_keys(env_id).await {
		Ok(keys) => keys,
		Err(e) => {
			tracing::error!(error = %e, %env_id, "Failed to list SDK keys");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let key_responses: Vec<SdkKeyResponse> = sdk_keys
		.into_iter()
		.map(|key| SdkKeyResponse {
			id: key.id.to_string(),
			environment_id: key.environment_id.to_string(),
			environment_name: env.name.clone(),
			key_type: sdk_key_type_to_api(key.key_type),
			name: key.name,
			created_by: key.created_by.to_string(),
			created_at: key.created_at,
			last_used_at: key.last_used_at,
			revoked_at: key.revoked_at,
		})
		.collect();

	(
		StatusCode::OK,
		Json(ListSdkKeysResponse {
			sdk_keys: key_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/environments/{env_id}/sdk-keys",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("env_id" = String, Path, description = "Environment ID")
    ),
    request_body = CreateSdkKeyRequest,
    responses(
        (status = 201, description = "SDK key created", body = CreateSdkKeyResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Environment not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Create a new SDK key.
#[tracing::instrument(skip(state, payload), fields(%org_id, %env_id, name = %payload.name))]
pub async fn create_sdk_key(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, env_id)): Path<(String, String)>,
	Json(payload): Json<CreateSdkKeyRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let env_id: EnvironmentId = match env_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.environment_not_found"),
			)
			.into_response();
		}
	};

	// Check org membership
	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	// Verify environment exists and belongs to org
	let env = match state.flags_repo.get_environment_by_id(env_id).await {
		Ok(Some(env)) => env,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %env_id, "Failed to get environment");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	if env.org_id.0 != org_id.into_inner() {
		return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
			.into_response();
	}

	// Validate name
	if payload.name.is_empty() || payload.name.len() > 100 {
		return bad_request::<FlagsErrorResponse>(
			"invalid_name",
			"SDK key name must be between 1 and 100 characters",
		)
		.into_response();
	}

	let key_type = sdk_key_type_from_api(payload.key_type);

	// Generate the raw key
	let raw_key = SdkKey::generate_key(key_type, &env.name);

	// Hash the key for storage
	let key_hash = match hash_sdk_key(&raw_key) {
		Ok(hash) => hash,
		Err(e) => {
			tracing::error!(error = %e, "Failed to hash SDK key");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let user_id = loom_flags_core::UserId(current_user.user.id.into_inner());

	let sdk_key = SdkKey {
		id: SdkKeyId::new(),
		environment_id: env_id,
		key_type,
		name: payload.name,
		key_hash,
		created_by: user_id,
		created_at: Utc::now(),
		last_used_at: None,
		revoked_at: None,
	};

	if let Err(e) = state.flags_repo.create_sdk_key(&sdk_key).await {
		tracing::error!(error = %e, "Failed to create SDK key");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::SdkKeyCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("sdk_key", sdk_key.id.to_string())
			.details(serde_json::json!({
				"environment_id": env_id.to_string(),
				"environment_name": env.name,
				"key_type": format!("{:?}", sdk_key.key_type),
				"name": sdk_key.name,
			}))
			.build(),
	);

	tracing::info!(sdk_key_id = %sdk_key.id, "SDK key created");

	(
		StatusCode::CREATED,
		Json(CreateSdkKeyResponse {
			id: sdk_key.id.to_string(),
			key: raw_key,
			environment_id: sdk_key.environment_id.to_string(),
			key_type: sdk_key_type_to_api(sdk_key.key_type),
			name: sdk_key.name,
			created_at: sdk_key.created_at,
		}),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/flags/sdk-keys/{key_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("key_id" = String, Path, description = "SDK Key ID")
    ),
    responses(
        (status = 200, description = "SDK key revoked", body = FlagsSuccessResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "SDK key not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Revoke an SDK key.
#[tracing::instrument(skip(state), fields(%org_id, %key_id))]
pub async fn revoke_sdk_key(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, key_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let key_id: SdkKeyId = match key_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.sdk_key_not_found"),
			)
			.into_response();
		}
	};

	// Check org membership
	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	// Get the SDK key
	let sdk_key = match state.flags_repo.get_sdk_key_by_id(key_id).await {
		Ok(Some(key)) => key,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.sdk_key_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %key_id, "Failed to get SDK key");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify the SDK key belongs to an environment in this org
	let env = match state
		.flags_repo
		.get_environment_by_id(sdk_key.environment_id)
		.await
	{
		Ok(Some(env)) => env,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.sdk_key_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, env_id = %sdk_key.environment_id, "Failed to get environment");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	if env.org_id.0 != org_id.into_inner() {
		return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.sdk_key_not_found"))
			.into_response();
	}

	// Check if already revoked
	if sdk_key.is_revoked() {
		return bad_request::<FlagsErrorResponse>(
			"already_revoked",
			t(locale, "server.api.flags.sdk_key_revoked"),
		)
		.into_response();
	}

	// Revoke the key
	match state.flags_repo.revoke_sdk_key(key_id).await {
		Ok(true) => {}
		Ok(false) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.sdk_key_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %key_id, "Failed to revoke SDK key");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::SdkKeyRevoked)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("sdk_key", key_id.to_string())
			.details(serde_json::json!({
				"environment_id": sdk_key.environment_id.to_string(),
				"environment_name": env.name,
				"name": sdk_key.name,
			}))
			.build(),
	);

	tracing::info!(%key_id, "SDK key revoked");

	(
		StatusCode::OK,
		Json(FlagsSuccessResponse {
			message: t(locale, "server.api.flags.sdk_key_revoked_success").to_string(),
		}),
	)
		.into_response()
}
