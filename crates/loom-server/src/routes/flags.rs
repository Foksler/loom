// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Feature flags HTTP handlers.
//!
//! Implements environment, SDK key, and flag management endpoints.

use std::convert::Infallible;

use axum::{
	extract::{Path, Query, State},
	http::{HeaderMap, StatusCode},
	response::{
		sse::{Event, Sse},
		IntoResponse,
	},
	Json,
};
use futures::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use chrono::Utc;
use loom_flags_core::{
	AttributeOperator, Condition, Environment, EnvironmentId, Flag, FlagConfig, FlagConfigId, FlagId,
	FlagPrerequisite, FlagState, FlagStreamEvent, GeoField, GeoOperator, KillSwitch, KillSwitchId,
	KillSwitchState, PercentageKey, Schedule, ScheduleStep, SdkKey, SdkKeyId, SdkKeyType, Strategy,
	StrategyId, Variant, VariantValue,
};
pub use loom_server_api::flags::{
	ActivateKillSwitchRequest, AttributeOperatorApi, ConditionApi, CreateEnvironmentRequest,
	CreateFlagRequest, CreateKillSwitchRequest, CreateSdkKeyRequest, CreateSdkKeyResponse,
	CreateStrategyRequest, EnvironmentResponse, EvaluateAllFlagsRequest, EvaluateAllFlagsResponse,
	EvaluateFlagRequest, EvaluationContextApi, EvaluationReasonApi, EvaluationResultApi,
	FlagConfigResponse, FlagPrerequisiteApi, FlagResponse, FlagsErrorResponse, FlagsSuccessResponse,
	GeoContextApi, GeoFieldApi, GeoOperatorApi, KillSwitchResponse, ListEnvironmentsResponse,
	ListFlagConfigsResponse, ListFlagsQuery, ListFlagsResponse, ListKillSwitchesResponse,
	ListSdkKeysResponse, ListStrategiesResponse, PercentageKeyApi, ScheduleApi, ScheduleStepApi,
	SdkKeyResponse, SdkKeyTypeApi, StrategyResponse, UpdateEnvironmentRequest, UpdateFlagConfigRequest,
	UpdateFlagRequest, UpdateKillSwitchRequest, UpdateStrategyRequest, VariantApi, VariantValueApi,
};
use loom_server_flags::{evaluate_flag, hash_sdk_key, EvaluationContext, EvaluationReason, FlagsRepository, GeoContext};

use crate::{
	api::AppState,
	api_response::{bad_request, conflict, internal_error, not_found},
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	impl_api_error_response, parse_id,
	validation::parse_org_id as shared_parse_org_id,
};

impl_api_error_response!(FlagsErrorResponse);

// ============================================================================
// Environment Routes
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/environments",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of environments", body = ListEnvironmentsResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// List environments for an organization.
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_environments(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	let environments = match state.flags_repo.list_environments(flags_org_id).await {
		Ok(envs) => envs,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list environments");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let env_responses: Vec<EnvironmentResponse> = environments
		.into_iter()
		.map(|env| EnvironmentResponse {
			id: env.id.to_string(),
			org_id: env.org_id.to_string(),
			name: env.name,
			color: env.color,
			created_at: env.created_at,
		})
		.collect();

	(
		StatusCode::OK,
		Json(ListEnvironmentsResponse {
			environments: env_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/environments",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateEnvironmentRequest,
    responses(
        (status = 201, description = "Environment created", body = EnvironmentResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 409, description = "Environment name already exists", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Create a new environment.
#[tracing::instrument(skip(state, payload), fields(%org_id, name = %payload.name))]
pub async fn create_environment(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateEnvironmentRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	// Validate environment name
	if !Environment::validate_name(&payload.name) {
		return bad_request::<FlagsErrorResponse>(
			"invalid_name",
			t(locale, "server.api.flags.invalid_environment_name"),
		)
		.into_response();
	}

	// Validate color if provided
	if let Some(ref color) = payload.color {
		if !Environment::validate_color(color) {
			return bad_request::<FlagsErrorResponse>(
				"invalid_color",
				t(locale, "server.api.flags.invalid_environment_color"),
			)
			.into_response();
		}
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	// Check for duplicate name
	if let Ok(Some(_)) = state
		.flags_repo
		.get_environment_by_name(flags_org_id, &payload.name)
		.await
	{
		return conflict::<FlagsErrorResponse>(
			"duplicate_name",
			t(locale, "server.api.flags.duplicate_environment_name"),
		)
		.into_response();
	}

	let env = Environment {
		id: EnvironmentId::new(),
		org_id: flags_org_id,
		name: payload.name,
		color: payload.color,
		created_at: Utc::now(),
	};

	if let Err(e) = state.flags_repo.create_environment(&env).await {
		tracing::error!(error = %e, ?org_id, "Failed to create environment");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::info!(env_id = %env.id, name = %env.name, "Environment created");

	(
		StatusCode::CREATED,
		Json(EnvironmentResponse {
			id: env.id.to_string(),
			org_id: env.org_id.to_string(),
			name: env.name,
			color: env.color,
			created_at: env.created_at,
		}),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/environments/{env_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("env_id" = String, Path, description = "Environment ID")
    ),
    responses(
        (status = 200, description = "Environment details", body = EnvironmentResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Environment not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Get environment details.
#[tracing::instrument(skip(state), fields(%org_id, %env_id))]
pub async fn get_environment(
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

	// Verify environment belongs to the org
	if env.org_id.0 != org_id.into_inner() {
		return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
			.into_response();
	}

	(
		StatusCode::OK,
		Json(EnvironmentResponse {
			id: env.id.to_string(),
			org_id: env.org_id.to_string(),
			name: env.name,
			color: env.color,
			created_at: env.created_at,
		}),
	)
		.into_response()
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/flags/environments/{env_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("env_id" = String, Path, description = "Environment ID")
    ),
    request_body = UpdateEnvironmentRequest,
    responses(
        (status = 200, description = "Environment updated", body = EnvironmentResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Environment not found", body = FlagsErrorResponse),
        (status = 409, description = "Environment name already exists", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Update an environment.
#[tracing::instrument(skip(state, payload), fields(%org_id, %env_id))]
pub async fn update_environment(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, env_id)): Path<(String, String)>,
	Json(payload): Json<UpdateEnvironmentRequest>,
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

	let mut env = match state.flags_repo.get_environment_by_id(env_id).await {
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

	// Verify environment belongs to the org
	if env.org_id.0 != org_id.into_inner() {
		return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
			.into_response();
	}

	// Update name if provided
	if let Some(ref new_name) = payload.name {
		if !Environment::validate_name(new_name) {
			return bad_request::<FlagsErrorResponse>(
				"invalid_name",
				t(locale, "server.api.flags.invalid_environment_name"),
			)
			.into_response();
		}

		// Check for duplicate name (only if name is changing)
		if new_name != &env.name {
			if let Ok(Some(_)) = state
				.flags_repo
				.get_environment_by_name(env.org_id, new_name)
				.await
			{
				return conflict::<FlagsErrorResponse>(
					"duplicate_name",
					t(locale, "server.api.flags.duplicate_environment_name"),
				)
				.into_response();
			}
		}

		env.name = new_name.clone();
	}

	// Update color if provided
	if let Some(ref new_color) = payload.color {
		if !Environment::validate_color(new_color) {
			return bad_request::<FlagsErrorResponse>(
				"invalid_color",
				t(locale, "server.api.flags.invalid_environment_color"),
			)
			.into_response();
		}
		env.color = Some(new_color.clone());
	}

	if let Err(e) = state.flags_repo.update_environment(&env).await {
		tracing::error!(error = %e, %env_id, "Failed to update environment");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::info!(%env_id, "Environment updated");

	(
		StatusCode::OK,
		Json(EnvironmentResponse {
			id: env.id.to_string(),
			org_id: env.org_id.to_string(),
			name: env.name,
			color: env.color,
			created_at: env.created_at,
		}),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/flags/environments/{env_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("env_id" = String, Path, description = "Environment ID")
    ),
    responses(
        (status = 200, description = "Environment deleted", body = FlagsSuccessResponse),
        (status = 400, description = "Environment has active SDK keys", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Environment not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Delete an environment.
#[tracing::instrument(skip(state), fields(%org_id, %env_id))]
pub async fn delete_environment(
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

	// Verify environment belongs to the org
	if env.org_id.0 != org_id.into_inner() {
		return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
			.into_response();
	}

	// Check for active SDK keys
	let sdk_keys = match state.flags_repo.list_sdk_keys(env_id).await {
		Ok(keys) => keys,
		Err(e) => {
			tracing::error!(error = %e, %env_id, "Failed to list SDK keys");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let active_keys = sdk_keys.iter().filter(|k| !k.is_revoked()).count();
	if active_keys > 0 {
		return bad_request::<FlagsErrorResponse>(
			"has_active_keys",
			t(locale, "server.api.flags.environment_has_active_keys"),
		)
		.into_response();
	}

	match state.flags_repo.delete_environment(env_id).await {
		Ok(true) => {
			tracing::info!(%env_id, "Environment deleted");
			(
				StatusCode::OK,
				Json(FlagsSuccessResponse {
					message: t(locale, "server.api.flags.environment_deleted").to_string(),
				}),
			)
				.into_response()
		}
		Ok(false) => {
			not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %env_id, "Failed to delete environment");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

// ============================================================================
// SDK Key Routes
// ============================================================================

fn sdk_key_type_to_api(kt: SdkKeyType) -> SdkKeyTypeApi {
	match kt {
		SdkKeyType::ClientSide => SdkKeyTypeApi::ClientSide,
		SdkKeyType::ServerSide => SdkKeyTypeApi::ServerSide,
	}
}

fn sdk_key_type_from_api(kt: SdkKeyTypeApi) -> SdkKeyType {
	match kt {
		SdkKeyTypeApi::ClientSide => SdkKeyType::ClientSide,
		SdkKeyTypeApi::ServerSide => SdkKeyType::ServerSide,
	}
}

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

	tracing::info!(%key_id, "SDK key revoked");

	(
		StatusCode::OK,
		Json(FlagsSuccessResponse {
			message: t(locale, "server.api.flags.sdk_key_revoked_success").to_string(),
		}),
	)
		.into_response()
}

// ============================================================================
// Flag Routes
// ============================================================================

fn variant_to_api(v: &Variant) -> VariantApi {
	VariantApi {
		name: v.name.clone(),
		value: variant_value_to_api(&v.value),
		weight: v.weight,
	}
}

fn variant_value_to_api(v: &VariantValue) -> VariantValueApi {
	match v {
		VariantValue::Boolean(b) => VariantValueApi::Boolean(*b),
		VariantValue::String(s) => VariantValueApi::String(s.clone()),
		VariantValue::Json(j) => VariantValueApi::Json(j.clone()),
	}
}

fn variant_from_api(v: &VariantApi) -> Variant {
	Variant {
		name: v.name.clone(),
		value: variant_value_from_api(&v.value),
		weight: v.weight,
	}
}

fn variant_value_from_api(v: &VariantValueApi) -> VariantValue {
	match v {
		VariantValueApi::Boolean(b) => VariantValue::Boolean(*b),
		VariantValueApi::String(s) => VariantValue::String(s.clone()),
		VariantValueApi::Json(j) => VariantValue::Json(j.clone()),
	}
}

fn prerequisite_to_api(p: &FlagPrerequisite) -> FlagPrerequisiteApi {
	FlagPrerequisiteApi {
		flag_key: p.flag_key.clone(),
		required_variant: p.required_variant.clone(),
	}
}

fn prerequisite_from_api(p: &FlagPrerequisiteApi) -> FlagPrerequisite {
	FlagPrerequisite {
		flag_key: p.flag_key.clone(),
		required_variant: p.required_variant.clone(),
	}
}

fn flag_to_response(flag: &Flag) -> FlagResponse {
	FlagResponse {
		id: flag.id.to_string(),
		org_id: flag.org_id.map(|id| id.to_string()),
		key: flag.key.clone(),
		name: flag.name.clone(),
		description: flag.description.clone(),
		tags: flag.tags.clone(),
		maintainer_user_id: flag.maintainer_user_id.map(|id| id.to_string()),
		variants: flag.variants.iter().map(variant_to_api).collect(),
		default_variant: flag.default_variant.clone(),
		prerequisites: flag.prerequisites.iter().map(prerequisite_to_api).collect(),
		is_archived: flag.is_archived(),
		created_at: flag.created_at,
		updated_at: flag.updated_at,
		archived_at: flag.archived_at,
	}
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("include_archived" = Option<bool>, Query, description = "Include archived flags")
    ),
    responses(
        (status = 200, description = "List of flags", body = ListFlagsResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// List flags for an organization.
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_flags(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Query(query): Query<ListFlagsQuery>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	let flags = match state
		.flags_repo
		.list_flags(Some(flags_org_id), query.include_archived)
		.await
	{
		Ok(flags) => flags,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list flags");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let flag_responses: Vec<FlagResponse> = flags.iter().map(flag_to_response).collect();

	(
		StatusCode::OK,
		Json(ListFlagsResponse {
			flags: flag_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateFlagRequest,
    responses(
        (status = 201, description = "Flag created", body = FlagResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 409, description = "Flag key already exists", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Create a new flag.
#[tracing::instrument(skip(state, payload), fields(%org_id, key = %payload.key))]
pub async fn create_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateFlagRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	// Validate flag key
	if !Flag::validate_key(&payload.key) {
		return bad_request::<FlagsErrorResponse>(
			"invalid_key",
			t(locale, "server.api.flags.invalid_flag_key"),
		)
		.into_response();
	}

	// Validate variants
	if payload.variants.is_empty() {
		return bad_request::<FlagsErrorResponse>("no_variants", "At least one variant is required")
			.into_response();
	}

	// Check for duplicate variant names
	let mut variant_names: Vec<&str> = payload.variants.iter().map(|v| v.name.as_str()).collect();
	variant_names.sort();
	for i in 1..variant_names.len() {
		if variant_names[i] == variant_names[i - 1] {
			return bad_request::<FlagsErrorResponse>(
				"duplicate_variant",
				format!("Duplicate variant name: {}", variant_names[i]),
			)
			.into_response();
		}
	}

	// Validate default variant exists
	if !payload
		.variants
		.iter()
		.any(|v| v.name == payload.default_variant)
	{
		return bad_request::<FlagsErrorResponse>(
			"default_variant_missing",
			t(locale, "server.api.flags.default_variant_missing"),
		)
		.into_response();
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	// Check for duplicate key
	if let Ok(Some(_)) = state
		.flags_repo
		.get_flag_by_key(Some(flags_org_id), &payload.key)
		.await
	{
		return conflict::<FlagsErrorResponse>(
			"duplicate_key",
			t(locale, "server.api.flags.duplicate_flag_key"),
		)
		.into_response();
	}

	// Parse maintainer user ID if provided
	let maintainer_user_id = match &payload.maintainer_user_id {
		Some(id) => match id.parse::<uuid::Uuid>() {
			Ok(uuid) => Some(loom_flags_core::UserId(uuid)),
			Err(_) => {
				return bad_request::<FlagsErrorResponse>(
					"invalid_maintainer_id",
					"Invalid maintainer user ID format",
				)
				.into_response();
			}
		},
		None => None,
	};

	let now = Utc::now();
	let flag = Flag {
		id: FlagId::new(),
		org_id: Some(flags_org_id),
		key: payload.key,
		name: payload.name,
		description: payload.description,
		tags: payload.tags,
		maintainer_user_id,
		variants: payload.variants.iter().map(variant_from_api).collect(),
		default_variant: payload.default_variant,
		prerequisites: payload
			.prerequisites
			.iter()
			.map(prerequisite_from_api)
			.collect(),
		created_at: now,
		updated_at: now,
		archived_at: None,
	};

	if let Err(e) = state.flags_repo.create_flag(&flag).await {
		tracing::error!(error = %e, flag_key = %flag.key, "Failed to create flag");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	// Auto-create FlagConfig for each environment
	let environments = match state.flags_repo.list_environments(flags_org_id).await {
		Ok(envs) => envs,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list environments for flag config creation");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	for env in environments {
		let config = FlagConfig {
			id: FlagConfigId::new(),
			flag_id: flag.id,
			environment_id: env.id,
			enabled: false,
			strategy_id: None,
			created_at: now,
			updated_at: now,
		};

		if let Err(e) = state.flags_repo.create_flag_config(&config).await {
			tracing::error!(error = %e, flag_id = %flag.id, env_id = %env.id, "Failed to create flag config");
		}
	}

	tracing::info!(flag_id = %flag.id, flag_key = %flag.key, "Flag created");

	(StatusCode::CREATED, Json(flag_to_response(&flag))).into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/{flag_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID")
    ),
    responses(
        (status = 200, description = "Flag details", body = FlagResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Get flag details.
#[tracing::instrument(skip(state), fields(%org_id, %flag_id))]
pub async fn get_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
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

	let flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify flag belongs to the org
	match flag.org_id {
		Some(flag_org_id) if flag_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
	}

	(StatusCode::OK, Json(flag_to_response(&flag))).into_response()
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/flags/{flag_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID")
    ),
    request_body = UpdateFlagRequest,
    responses(
        (status = 200, description = "Flag updated", body = FlagResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Update a flag.
#[tracing::instrument(skip(state, payload), fields(%org_id, %flag_id))]
pub async fn update_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id)): Path<(String, String)>,
	Json(payload): Json<UpdateFlagRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
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

	let mut flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify flag belongs to the org
	match flag.org_id {
		Some(flag_org_id) if flag_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
	}

	// Check if flag is archived
	if flag.is_archived() {
		return bad_request::<FlagsErrorResponse>(
			"flag_archived",
			"Cannot update an archived flag. Restore it first.",
		)
		.into_response();
	}

	// Update name if provided
	if let Some(ref name) = payload.name {
		flag.name = name.clone();
	}

	// Update description if provided
	if let Some(ref description) = payload.description {
		flag.description = Some(description.clone());
	}

	// Update tags if provided
	if let Some(ref tags) = payload.tags {
		flag.tags = tags.clone();
	}

	// Update maintainer if provided
	if let Some(ref maintainer_id) = payload.maintainer_user_id {
		match maintainer_id.parse::<uuid::Uuid>() {
			Ok(uuid) => flag.maintainer_user_id = Some(loom_flags_core::UserId(uuid)),
			Err(_) => {
				return bad_request::<FlagsErrorResponse>(
					"invalid_maintainer_id",
					"Invalid maintainer user ID format",
				)
				.into_response();
			}
		}
	}

	// Update variants if provided
	if let Some(ref variants) = payload.variants {
		if variants.is_empty() {
			return bad_request::<FlagsErrorResponse>("no_variants", "At least one variant is required")
				.into_response();
		}

		// Check for duplicate variant names
		let mut variant_names: Vec<&str> = variants.iter().map(|v| v.name.as_str()).collect();
		variant_names.sort();
		for i in 1..variant_names.len() {
			if variant_names[i] == variant_names[i - 1] {
				return bad_request::<FlagsErrorResponse>(
					"duplicate_variant",
					format!("Duplicate variant name: {}", variant_names[i]),
				)
				.into_response();
			}
		}

		flag.variants = variants.iter().map(variant_from_api).collect();
	}

	// Update default_variant if provided
	if let Some(ref default_variant) = payload.default_variant {
		flag.default_variant = default_variant.clone();
	}

	// Validate default variant exists in variants
	if !flag.variants.iter().any(|v| v.name == flag.default_variant) {
		return bad_request::<FlagsErrorResponse>(
			"default_variant_missing",
			t(locale, "server.api.flags.default_variant_missing"),
		)
		.into_response();
	}

	// Update prerequisites if provided
	if let Some(ref prerequisites) = payload.prerequisites {
		flag.prerequisites = prerequisites.iter().map(prerequisite_from_api).collect();
	}

	flag.updated_at = Utc::now();

	if let Err(e) = state.flags_repo.update_flag(&flag).await {
		tracing::error!(error = %e, %flag_id, "Failed to update flag");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::info!(%flag_id, "Flag updated");

	(StatusCode::OK, Json(flag_to_response(&flag))).into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/{flag_id}/archive",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID")
    ),
    responses(
        (status = 200, description = "Flag archived", body = FlagsSuccessResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Archive a flag.
#[tracing::instrument(skip(state), fields(%org_id, %flag_id))]
pub async fn archive_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
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

	// Verify flag exists and belongs to org
	let flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	match flag.org_id {
		Some(flag_org_id) if flag_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
	}

	if flag.is_archived() {
		return bad_request::<FlagsErrorResponse>("already_archived", "Flag is already archived")
			.into_response();
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	match state.flags_repo.archive_flag(flag_id).await {
		Ok(true) => {
			tracing::info!(%flag_id, "Flag archived");

			// Broadcast flag archived event to all environments
			let event = FlagStreamEvent::flag_archived(flag.key.clone());
			state.flags_broadcaster.broadcast_to_org(flags_org_id, event).await;

			(
				StatusCode::OK,
				Json(FlagsSuccessResponse {
					message: t(locale, "server.api.flags.flag_archived").to_string(),
				}),
			)
				.into_response()
		}
		Ok(false) => {
			not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found")).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to archive flag");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/{flag_id}/restore",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID")
    ),
    responses(
        (status = 200, description = "Flag restored", body = FlagsSuccessResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Restore an archived flag.
#[tracing::instrument(skip(state), fields(%org_id, %flag_id))]
pub async fn restore_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
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

	// Verify flag exists and belongs to org
	let flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	match flag.org_id {
		Some(flag_org_id) if flag_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
	}

	if !flag.is_archived() {
		return bad_request::<FlagsErrorResponse>("not_archived", "Flag is not archived")
			.into_response();
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	match state.flags_repo.restore_flag(flag_id).await {
		Ok(true) => {
			tracing::info!(%flag_id, "Flag restored");

			// Broadcast flag restored event to all environments
			// We broadcast to all environments since the flag is now available again
			let environments = state.flags_repo.list_environments(flags_org_id).await.unwrap_or_default();
			for env in environments {
				let config = state
					.flags_repo
					.get_flag_config(flag.id, env.id)
					.await
					.ok()
					.flatten();
				let enabled = config.map(|c| c.enabled).unwrap_or(false);
				let event = FlagStreamEvent::flag_restored(flag.key.clone(), env.name, enabled);
				state.flags_broadcaster.broadcast(flags_org_id, env.id, event).await;
			}

			(
				StatusCode::OK,
				Json(FlagsSuccessResponse {
					message: t(locale, "server.api.flags.flag_restored").to_string(),
				}),
			)
				.into_response()
		}
		Ok(false) => {
			not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found")).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to restore flag");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

// ============================================================================
// Flag Config Routes
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/{flag_id}/configs",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID")
    ),
    responses(
        (status = 200, description = "List of flag configs", body = ListFlagConfigsResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// List flag configs for all environments.
#[tracing::instrument(skip(state), fields(%org_id, %flag_id))]
pub async fn list_flag_configs(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
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

	// Verify flag exists and belongs to org
	let flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	match flag.org_id {
		Some(flag_org_id) if flag_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
	}

	// Get configs
	let configs = match state.flags_repo.list_flag_configs(flag_id).await {
		Ok(configs) => configs,
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to list flag configs");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get environments for names
	let environments = match state.flags_repo.list_environments(flags_org_id).await {
		Ok(envs) => envs,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list environments");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let env_names: std::collections::HashMap<_, _> = environments
		.iter()
		.map(|e| (e.id, e.name.clone()))
		.collect();

	let config_responses: Vec<FlagConfigResponse> = configs
		.iter()
		.map(|c| FlagConfigResponse {
			id: c.id.to_string(),
			flag_id: c.flag_id.to_string(),
			environment_id: c.environment_id.to_string(),
			environment_name: env_names
				.get(&c.environment_id)
				.cloned()
				.unwrap_or_default(),
			enabled: c.enabled,
			strategy_id: c.strategy_id.map(|s| s.to_string()),
			created_at: c.created_at,
			updated_at: c.updated_at,
		})
		.collect();

	(
		StatusCode::OK,
		Json(ListFlagConfigsResponse {
			configs: config_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/{flag_id}/configs/{env_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID"),
        ("env_id" = String, Path, description = "Environment ID")
    ),
    responses(
        (status = 200, description = "Flag config details", body = FlagConfigResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag config not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Get flag config for a specific environment.
#[tracing::instrument(skip(state), fields(%org_id, %flag_id, %env_id))]
pub async fn get_flag_config(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id, env_id)): Path<(String, String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
			)
			.into_response();
		}
	};

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	// Verify flag exists and belongs to org
	let flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	match flag.org_id {
		Some(flag_org_id) if flag_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
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

	if env.org_id != flags_org_id {
		return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
			.into_response();
	}

	// Get config
	let config = match state.flags_repo.get_flag_config(flag_id, env_id).await {
		Ok(Some(config)) => config,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>("Flag config not found for this environment")
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, %env_id, "Failed to get flag config");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	(
		StatusCode::OK,
		Json(FlagConfigResponse {
			id: config.id.to_string(),
			flag_id: config.flag_id.to_string(),
			environment_id: config.environment_id.to_string(),
			environment_name: env.name,
			enabled: config.enabled,
			strategy_id: config.strategy_id.map(|s| s.to_string()),
			created_at: config.created_at,
			updated_at: config.updated_at,
		}),
	)
		.into_response()
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/flags/{flag_id}/configs/{env_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID"),
        ("env_id" = String, Path, description = "Environment ID")
    ),
    request_body = UpdateFlagConfigRequest,
    responses(
        (status = 200, description = "Flag config updated", body = FlagConfigResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag config not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Update flag config for a specific environment.
#[tracing::instrument(skip(state, payload), fields(%org_id, %flag_id, %env_id))]
pub async fn update_flag_config(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id, env_id)): Path<(String, String, String)>,
	Json(payload): Json<UpdateFlagConfigRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
			)
			.into_response();
		}
	};

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	// Verify flag exists and belongs to org
	let flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	match flag.org_id {
		Some(flag_org_id) if flag_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
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

	if env.org_id != flags_org_id {
		return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.environment_not_found"))
			.into_response();
	}

	// Get config
	let mut config = match state.flags_repo.get_flag_config(flag_id, env_id).await {
		Ok(Some(config)) => config,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>("Flag config not found for this environment")
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, %env_id, "Failed to get flag config");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Update enabled if provided
	if let Some(enabled) = payload.enabled {
		config.enabled = enabled;
	}

	// Update strategy_id if provided
	if let Some(strategy_id_opt) = payload.strategy_id {
		match strategy_id_opt {
			Some(strategy_id_str) => {
				let strategy_id: loom_flags_core::StrategyId = match strategy_id_str.parse() {
					Ok(id) => id,
					Err(_) => {
						return bad_request::<FlagsErrorResponse>(
							"invalid_strategy_id",
							t(locale, "server.api.flags.strategy_not_found"),
						)
						.into_response();
					}
				};

				// Verify strategy exists
				match state.flags_repo.get_strategy_by_id(strategy_id).await {
					Ok(Some(_)) => config.strategy_id = Some(strategy_id),
					Ok(None) => {
						return not_found::<FlagsErrorResponse>(t(
							locale,
							"server.api.flags.strategy_not_found",
						))
						.into_response();
					}
					Err(e) => {
						tracing::error!(error = %e, %strategy_id, "Failed to get strategy");
						return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
							.into_response();
					}
				}
			}
			None => config.strategy_id = None,
		}
	}

	config.updated_at = Utc::now();

	if let Err(e) = state.flags_repo.update_flag_config(&config).await {
		tracing::error!(error = %e, config_id = %config.id, "Failed to update flag config");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::info!(config_id = %config.id, "Flag config updated");

	// Broadcast flag update event
	let default_value = flag
		.variants
		.iter()
		.find(|v| v.name == flag.default_variant)
		.map(|v| v.value.clone())
		.unwrap_or(VariantValue::Boolean(false));

	let event = FlagStreamEvent::flag_updated(
		flag.key.clone(),
		env.name.clone(),
		config.enabled,
		flag.default_variant.clone(),
		default_value,
	);
	state.flags_broadcaster.broadcast(flags_org_id, env_id, event).await;

	(
		StatusCode::OK,
		Json(FlagConfigResponse {
			id: config.id.to_string(),
			flag_id: config.flag_id.to_string(),
			environment_id: config.environment_id.to_string(),
			environment_name: env.name,
			enabled: config.enabled,
			strategy_id: config.strategy_id.map(|s| s.to_string()),
			created_at: config.created_at,
			updated_at: config.updated_at,
		}),
	)
		.into_response()
}

// ============================================================================
// Strategy Routes
// ============================================================================

fn condition_to_api(c: &Condition) -> ConditionApi {
	match c {
		Condition::Attribute {
			attribute,
			operator,
			value,
		} => ConditionApi::Attribute {
			attribute: attribute.clone(),
			operator: attribute_operator_to_api(*operator),
			value: value.clone(),
		},
		Condition::Geographic {
			field,
			operator,
			values,
		} => ConditionApi::Geographic {
			field: geo_field_to_api(*field),
			operator: geo_operator_to_api(*operator),
			values: values.clone(),
		},
		Condition::Environment { environments } => ConditionApi::Environment {
			environments: environments.clone(),
		},
	}
}

fn condition_from_api(c: &ConditionApi) -> Condition {
	match c {
		ConditionApi::Attribute {
			attribute,
			operator,
			value,
		} => Condition::Attribute {
			attribute: attribute.clone(),
			operator: attribute_operator_from_api(*operator),
			value: value.clone(),
		},
		ConditionApi::Geographic {
			field,
			operator,
			values,
		} => Condition::Geographic {
			field: geo_field_from_api(*field),
			operator: geo_operator_from_api(*operator),
			values: values.clone(),
		},
		ConditionApi::Environment { environments } => Condition::Environment {
			environments: environments.clone(),
		},
	}
}

fn attribute_operator_to_api(op: AttributeOperator) -> AttributeOperatorApi {
	match op {
		AttributeOperator::Equals => AttributeOperatorApi::Equals,
		AttributeOperator::NotEquals => AttributeOperatorApi::NotEquals,
		AttributeOperator::Contains => AttributeOperatorApi::Contains,
		AttributeOperator::StartsWith => AttributeOperatorApi::StartsWith,
		AttributeOperator::EndsWith => AttributeOperatorApi::EndsWith,
		AttributeOperator::GreaterThan => AttributeOperatorApi::GreaterThan,
		AttributeOperator::LessThan => AttributeOperatorApi::LessThan,
		AttributeOperator::GreaterThanOrEquals => AttributeOperatorApi::GreaterThanOrEquals,
		AttributeOperator::LessThanOrEquals => AttributeOperatorApi::LessThanOrEquals,
		AttributeOperator::In => AttributeOperatorApi::In,
		AttributeOperator::NotIn => AttributeOperatorApi::NotIn,
	}
}

fn attribute_operator_from_api(op: AttributeOperatorApi) -> AttributeOperator {
	match op {
		AttributeOperatorApi::Equals => AttributeOperator::Equals,
		AttributeOperatorApi::NotEquals => AttributeOperator::NotEquals,
		AttributeOperatorApi::Contains => AttributeOperator::Contains,
		AttributeOperatorApi::StartsWith => AttributeOperator::StartsWith,
		AttributeOperatorApi::EndsWith => AttributeOperator::EndsWith,
		AttributeOperatorApi::GreaterThan => AttributeOperator::GreaterThan,
		AttributeOperatorApi::LessThan => AttributeOperator::LessThan,
		AttributeOperatorApi::GreaterThanOrEquals => AttributeOperator::GreaterThanOrEquals,
		AttributeOperatorApi::LessThanOrEquals => AttributeOperator::LessThanOrEquals,
		AttributeOperatorApi::In => AttributeOperator::In,
		AttributeOperatorApi::NotIn => AttributeOperator::NotIn,
	}
}

fn geo_field_to_api(f: GeoField) -> GeoFieldApi {
	match f {
		GeoField::Country => GeoFieldApi::Country,
		GeoField::Region => GeoFieldApi::Region,
		GeoField::City => GeoFieldApi::City,
	}
}

fn geo_field_from_api(f: GeoFieldApi) -> GeoField {
	match f {
		GeoFieldApi::Country => GeoField::Country,
		GeoFieldApi::Region => GeoField::Region,
		GeoFieldApi::City => GeoField::City,
	}
}

fn geo_operator_to_api(op: GeoOperator) -> GeoOperatorApi {
	match op {
		GeoOperator::In => GeoOperatorApi::In,
		GeoOperator::NotIn => GeoOperatorApi::NotIn,
	}
}

fn geo_operator_from_api(op: GeoOperatorApi) -> GeoOperator {
	match op {
		GeoOperatorApi::In => GeoOperator::In,
		GeoOperatorApi::NotIn => GeoOperator::NotIn,
	}
}

fn percentage_key_to_api(pk: &PercentageKey) -> PercentageKeyApi {
	match pk {
		PercentageKey::UserId => PercentageKeyApi::UserId,
		PercentageKey::OrgId => PercentageKeyApi::OrgId,
		PercentageKey::SessionId => PercentageKeyApi::SessionId,
		PercentageKey::Custom(s) => PercentageKeyApi::Custom(s.clone()),
	}
}

fn percentage_key_from_api(pk: &PercentageKeyApi) -> PercentageKey {
	match pk {
		PercentageKeyApi::UserId => PercentageKey::UserId,
		PercentageKeyApi::OrgId => PercentageKey::OrgId,
		PercentageKeyApi::SessionId => PercentageKey::SessionId,
		PercentageKeyApi::Custom(s) => PercentageKey::Custom(s.clone()),
	}
}

fn schedule_to_api(s: &Schedule) -> ScheduleApi {
	ScheduleApi {
		steps: s.steps.iter().map(schedule_step_to_api).collect(),
	}
}

fn schedule_from_api(s: &ScheduleApi) -> Schedule {
	Schedule {
		steps: s.steps.iter().map(schedule_step_from_api).collect(),
	}
}

fn schedule_step_to_api(s: &ScheduleStep) -> ScheduleStepApi {
	ScheduleStepApi {
		percentage: s.percentage,
		start_at: s.start_at,
	}
}

fn schedule_step_from_api(s: &ScheduleStepApi) -> ScheduleStep {
	ScheduleStep {
		percentage: s.percentage,
		start_at: s.start_at,
	}
}

fn strategy_to_response(strategy: &Strategy) -> StrategyResponse {
	StrategyResponse {
		id: strategy.id.to_string(),
		org_id: strategy.org_id.map(|id| id.to_string()),
		name: strategy.name.clone(),
		description: strategy.description.clone(),
		conditions: strategy.conditions.iter().map(condition_to_api).collect(),
		percentage: strategy.percentage,
		percentage_key: percentage_key_to_api(&strategy.percentage_key),
		schedule: strategy.schedule.as_ref().map(schedule_to_api),
		created_at: strategy.created_at,
		updated_at: strategy.updated_at,
	}
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/strategies",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of strategies", body = ListStrategiesResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// List strategies for an organization.
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_strategies(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	let strategies = match state.flags_repo.list_strategies(Some(flags_org_id)).await {
		Ok(s) => s,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list strategies");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let strategy_responses: Vec<StrategyResponse> =
		strategies.iter().map(strategy_to_response).collect();

	(
		StatusCode::OK,
		Json(ListStrategiesResponse {
			strategies: strategy_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/strategies",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateStrategyRequest,
    responses(
        (status = 201, description = "Strategy created", body = StrategyResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Create a new strategy.
#[tracing::instrument(skip(state, payload), fields(%org_id, name = %payload.name))]
pub async fn create_strategy(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateStrategyRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	// Validate name
	if payload.name.is_empty() || payload.name.len() > 100 {
		return bad_request::<FlagsErrorResponse>(
			"invalid_name",
			"Strategy name must be between 1 and 100 characters",
		)
		.into_response();
	}

	// Validate percentage if provided
	if let Some(pct) = payload.percentage {
		if pct > 100 {
			return bad_request::<FlagsErrorResponse>(
				"invalid_percentage",
				"Percentage must be between 0 and 100",
			)
			.into_response();
		}
	}

	// Validate schedule steps if provided
	if let Some(ref schedule) = payload.schedule {
		for step in &schedule.steps {
			if step.percentage > 100 {
				return bad_request::<FlagsErrorResponse>(
					"invalid_schedule_percentage",
					"Schedule step percentage must be between 0 and 100",
				)
				.into_response();
			}
		}
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	let now = Utc::now();

	let strategy = Strategy {
		id: StrategyId::new(),
		org_id: Some(flags_org_id),
		name: payload.name,
		description: payload.description,
		conditions: payload.conditions.iter().map(condition_from_api).collect(),
		percentage: payload.percentage,
		percentage_key: percentage_key_from_api(&payload.percentage_key),
		schedule: payload.schedule.as_ref().map(schedule_from_api),
		created_at: now,
		updated_at: now,
	};

	if let Err(e) = state.flags_repo.create_strategy(&strategy).await {
		tracing::error!(error = %e, strategy_id = %strategy.id, "Failed to create strategy");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::info!(strategy_id = %strategy.id, strategy_name = %strategy.name, "Strategy created");

	(StatusCode::CREATED, Json(strategy_to_response(&strategy))).into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/strategies/{strategy_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("strategy_id" = String, Path, description = "Strategy ID")
    ),
    responses(
        (status = 200, description = "Strategy details", body = StrategyResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Strategy not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Get strategy details.
#[tracing::instrument(skip(state), fields(%org_id, %strategy_id))]
pub async fn get_strategy(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, strategy_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let strategy_id: StrategyId = match strategy_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.strategy_not_found"),
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

	let strategy = match state.flags_repo.get_strategy_by_id(strategy_id).await {
		Ok(Some(s)) => s,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.strategy_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %strategy_id, "Failed to get strategy");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify strategy belongs to the org
	match strategy.org_id {
		Some(strategy_org_id) if strategy_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.strategy_not_found"))
				.into_response();
		}
	}

	(StatusCode::OK, Json(strategy_to_response(&strategy))).into_response()
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/flags/strategies/{strategy_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("strategy_id" = String, Path, description = "Strategy ID")
    ),
    request_body = UpdateStrategyRequest,
    responses(
        (status = 200, description = "Strategy updated", body = StrategyResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Strategy not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Update a strategy.
#[tracing::instrument(skip(state, payload), fields(%org_id, %strategy_id))]
pub async fn update_strategy(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, strategy_id)): Path<(String, String)>,
	Json(payload): Json<UpdateStrategyRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let strategy_id: StrategyId = match strategy_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.strategy_not_found"),
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

	let mut strategy = match state.flags_repo.get_strategy_by_id(strategy_id).await {
		Ok(Some(s)) => s,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.strategy_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %strategy_id, "Failed to get strategy");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify strategy belongs to the org
	match strategy.org_id {
		Some(strategy_org_id) if strategy_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.strategy_not_found"))
				.into_response();
		}
	}

	// Update name if provided
	if let Some(ref name) = payload.name {
		if name.is_empty() || name.len() > 100 {
			return bad_request::<FlagsErrorResponse>(
				"invalid_name",
				"Strategy name must be between 1 and 100 characters",
			)
			.into_response();
		}
		strategy.name = name.clone();
	}

	// Update description if provided
	if let Some(ref description) = payload.description {
		strategy.description = Some(description.clone());
	}

	// Update conditions if provided
	if let Some(ref conditions) = payload.conditions {
		strategy.conditions = conditions.iter().map(condition_from_api).collect();
	}

	// Update percentage if provided
	if let Some(percentage) = payload.percentage {
		if let Some(pct) = percentage {
			if pct > 100 {
				return bad_request::<FlagsErrorResponse>(
					"invalid_percentage",
					"Percentage must be between 0 and 100",
				)
				.into_response();
			}
		}
		strategy.percentage = percentage;
	}

	// Update percentage_key if provided
	if let Some(ref percentage_key) = payload.percentage_key {
		strategy.percentage_key = percentage_key_from_api(percentage_key);
	}

	// Update schedule if provided
	if let Some(ref schedule_opt) = payload.schedule {
		match schedule_opt {
			Some(schedule) => {
				for step in &schedule.steps {
					if step.percentage > 100 {
						return bad_request::<FlagsErrorResponse>(
							"invalid_schedule_percentage",
							"Schedule step percentage must be between 0 and 100",
						)
						.into_response();
					}
				}
				strategy.schedule = Some(schedule_from_api(schedule));
			}
			None => strategy.schedule = None,
		}
	}

	strategy.updated_at = Utc::now();

	if let Err(e) = state.flags_repo.update_strategy(&strategy).await {
		tracing::error!(error = %e, %strategy_id, "Failed to update strategy");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::info!(%strategy_id, "Strategy updated");

	(StatusCode::OK, Json(strategy_to_response(&strategy))).into_response()
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/flags/strategies/{strategy_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("strategy_id" = String, Path, description = "Strategy ID")
    ),
    responses(
        (status = 200, description = "Strategy deleted", body = FlagsSuccessResponse),
        (status = 400, description = "Strategy is in use", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Strategy not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Delete a strategy.
#[tracing::instrument(skip(state), fields(%org_id, %strategy_id))]
pub async fn delete_strategy(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, strategy_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let strategy_id: StrategyId = match strategy_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.strategy_not_found"),
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

	let strategy = match state.flags_repo.get_strategy_by_id(strategy_id).await {
		Ok(Some(s)) => s,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.strategy_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %strategy_id, "Failed to get strategy");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify strategy belongs to the org
	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	match strategy.org_id {
		Some(strategy_org_id) if strategy_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.strategy_not_found"))
				.into_response();
		}
	}

	// Check if strategy is in use by any flag configs
	let flags = match state.flags_repo.list_flags(Some(flags_org_id), true).await {
		Ok(f) => f,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list flags");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	for flag in flags {
		let configs = match state.flags_repo.list_flag_configs(flag.id).await {
			Ok(c) => c,
			Err(e) => {
				tracing::error!(error = %e, flag_id = %flag.id, "Failed to list flag configs");
				return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
					.into_response();
			}
		};

		for config in configs {
			if config.strategy_id == Some(strategy_id) {
				return bad_request::<FlagsErrorResponse>(
					"strategy_in_use",
					t(locale, "server.api.flags.strategy_in_use"),
				)
				.into_response();
			}
		}
	}

	match state.flags_repo.delete_strategy(strategy_id).await {
		Ok(true) => {
			tracing::info!(%strategy_id, "Strategy deleted");
			(
				StatusCode::OK,
				Json(FlagsSuccessResponse {
					message: t(locale, "server.api.flags.strategy_deleted").to_string(),
				}),
			)
				.into_response()
		}
		Ok(false) => not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.strategy_not_found"))
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, %strategy_id, "Failed to delete strategy");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

// ============================================================================
// Kill Switch Routes
// ============================================================================

fn kill_switch_to_response(kill_switch: KillSwitch) -> KillSwitchResponse {
	KillSwitchResponse {
		id: kill_switch.id.to_string(),
		org_id: kill_switch.org_id.map(|id| id.0.to_string()),
		key: kill_switch.key,
		name: kill_switch.name,
		description: kill_switch.description,
		linked_flag_keys: kill_switch.linked_flag_keys,
		is_active: kill_switch.is_active,
		activated_at: kill_switch.activated_at,
		activated_by: kill_switch.activated_by.map(|id| id.0.to_string()),
		activation_reason: kill_switch.activation_reason,
		created_at: kill_switch.created_at,
		updated_at: kill_switch.updated_at,
	}
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/kill-switches",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of kill switches", body = ListKillSwitchesResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// List kill switches for an organization.
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_kill_switches(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	match state
		.flags_repo
		.list_kill_switches(Some(flags_org_id))
		.await
	{
		Ok(kill_switches) => {
			let response = ListKillSwitchesResponse {
				kill_switches: kill_switches.into_iter().map(kill_switch_to_response).collect(),
			};
			(StatusCode::OK, Json(response)).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list kill switches");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/kill-switches",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateKillSwitchRequest,
    responses(
        (status = 201, description = "Kill switch created", body = KillSwitchResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse),
        (status = 409, description = "Kill switch key already exists", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Create a new kill switch.
#[tracing::instrument(skip(state, payload), fields(%org_id, key = %payload.key))]
pub async fn create_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateKillSwitchRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	if !KillSwitch::validate_key(&payload.key) {
		return bad_request::<FlagsErrorResponse>(
			"invalid_key",
			t(locale, "server.api.flags.invalid_kill_switch_key"),
		)
		.into_response();
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	if let Ok(Some(_)) = state
		.flags_repo
		.get_kill_switch_by_key(Some(flags_org_id), &payload.key)
		.await
	{
		return conflict::<FlagsErrorResponse>(
			"duplicate_key",
			t(locale, "server.api.flags.duplicate_kill_switch_key"),
		)
		.into_response();
	}

	let now = Utc::now();
	let kill_switch = KillSwitch {
		id: KillSwitchId::new(),
		org_id: Some(flags_org_id),
		key: payload.key,
		name: payload.name,
		description: payload.description,
		linked_flag_keys: payload.linked_flag_keys,
		is_active: false,
		activated_at: None,
		activated_by: None,
		activation_reason: None,
		created_at: now,
		updated_at: now,
	};

	if let Err(e) = state.flags_repo.create_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, kill_switch_id = %kill_switch.id, "Failed to create kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::info!(kill_switch_id = %kill_switch.id, key = %kill_switch.key, "Kill switch created");

	(StatusCode::CREATED, Json(kill_switch_to_response(kill_switch))).into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    responses(
        (status = 200, description = "Kill switch details", body = KillSwitchResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Get a kill switch by ID.
#[tracing::instrument(skip(state), fields(%org_id, %kill_switch_id))]
pub async fn get_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(kill_switch)) => {
			// Verify kill switch belongs to the org
			match kill_switch.org_id {
				Some(ks_org_id) if ks_org_id == flags_org_id => {}
				_ => {
					return not_found::<FlagsErrorResponse>(t(
						locale,
						"server.api.flags.kill_switch_not_found",
					))
					.into_response();
				}
			}

			(StatusCode::OK, Json(kill_switch_to_response(kill_switch))).into_response()
		}
		Ok(None) => {
			not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    request_body = UpdateKillSwitchRequest,
    responses(
        (status = 200, description = "Kill switch updated", body = KillSwitchResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Update a kill switch.
#[tracing::instrument(skip(state, payload), fields(%org_id, %kill_switch_id))]
pub async fn update_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
	Json(payload): Json<UpdateKillSwitchRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	let mut kill_switch = match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify kill switch belongs to the org
	match kill_switch.org_id {
		Some(ks_org_id) if ks_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
	}

	if let Some(name) = payload.name {
		kill_switch.name = name;
	}
	if let Some(description) = payload.description {
		kill_switch.description = Some(description);
	}
	if let Some(linked_flag_keys) = payload.linked_flag_keys {
		kill_switch.linked_flag_keys = linked_flag_keys;
	}
	kill_switch.updated_at = Utc::now();

	if let Err(e) = state.flags_repo.update_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, %kill_switch_id, "Failed to update kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::info!(%kill_switch_id, "Kill switch updated");

	(StatusCode::OK, Json(kill_switch_to_response(kill_switch))).into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}/activate",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    request_body = ActivateKillSwitchRequest,
    responses(
        (status = 200, description = "Kill switch activated", body = KillSwitchResponse),
        (status = 400, description = "Invalid request or missing reason", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Activate a kill switch (emergency shutoff).
#[tracing::instrument(skip(state, payload), fields(%org_id, %kill_switch_id))]
pub async fn activate_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
	Json(payload): Json<ActivateKillSwitchRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

	// Reason is mandatory for activation (audit trail)
	if payload.reason.trim().is_empty() {
		return bad_request::<FlagsErrorResponse>(
			"reason_required",
			t(locale, "server.api.flags.activation_reason_required"),
		)
		.into_response();
	}

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	let mut kill_switch = match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify kill switch belongs to the org
	match kill_switch.org_id {
		Some(ks_org_id) if ks_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
	}

	let user_id = loom_flags_core::UserId(current_user.user.id.into_inner());
	kill_switch.activate(user_id, payload.reason);

	if let Err(e) = state.flags_repo.update_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, %kill_switch_id, "Failed to activate kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::warn!(
		%kill_switch_id,
		key = %kill_switch.key,
		user_id = %current_user.user.id,
		reason = ?kill_switch.activation_reason,
		linked_flags = ?kill_switch.linked_flag_keys,
		"Kill switch activated"
	);

	// Broadcast kill switch activation to all environments in the org
	let event = FlagStreamEvent::kill_switch_activated(
		kill_switch.key.clone(),
		kill_switch.linked_flag_keys.clone(),
		kill_switch.activation_reason.clone().unwrap_or_default(),
	);
	state.flags_broadcaster.broadcast_to_org(flags_org_id, event).await;

	(
		StatusCode::OK,
		Json(FlagsSuccessResponse {
			message: t(locale, "server.api.flags.kill_switch_activated").to_string(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}/deactivate",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    responses(
        (status = 200, description = "Kill switch deactivated", body = KillSwitchResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Deactivate a kill switch.
#[tracing::instrument(skip(state), fields(%org_id, %kill_switch_id))]
pub async fn deactivate_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	let mut kill_switch = match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify kill switch belongs to the org
	match kill_switch.org_id {
		Some(ks_org_id) if ks_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
	}

	kill_switch.deactivate();

	if let Err(e) = state.flags_repo.update_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, %kill_switch_id, "Failed to deactivate kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	tracing::info!(
		%kill_switch_id,
		key = %kill_switch.key,
		user_id = %current_user.user.id,
		"Kill switch deactivated"
	);

	// Broadcast kill switch deactivation to all environments in the org
	let event = FlagStreamEvent::kill_switch_deactivated(
		kill_switch.key.clone(),
		kill_switch.linked_flag_keys.clone(),
	);
	state.flags_broadcaster.broadcast_to_org(flags_org_id, event).await;

	(
		StatusCode::OK,
		Json(FlagsSuccessResponse {
			message: t(locale, "server.api.flags.kill_switch_deactivated").to_string(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    responses(
        (status = 200, description = "Kill switch deleted", body = FlagsSuccessResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Delete a kill switch.
#[tracing::instrument(skip(state), fields(%org_id, %kill_switch_id))]
pub async fn delete_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	// Verify kill switch exists and belongs to the org
	let kill_switch = match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	match kill_switch.org_id {
		Some(ks_org_id) if ks_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
	}

	match state.flags_repo.delete_kill_switch(kill_switch_id).await {
		Ok(true) => {
			tracing::info!(%kill_switch_id, "Kill switch deleted");
			(
				StatusCode::OK,
				Json(FlagsSuccessResponse {
					message: t(locale, "server.api.flags.kill_switch_deleted").to_string(),
				}),
			)
				.into_response()
		}
		Ok(false) => {
			not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to delete kill switch");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

// ============================================================================
// Evaluation Routes
// ============================================================================

/// Helper to convert API context to core context.
fn to_core_context(api_ctx: &EvaluationContextApi) -> EvaluationContext {
	let mut ctx = EvaluationContext::new(&api_ctx.environment);

	if let Some(ref user_id) = api_ctx.user_id {
		ctx = ctx.with_user_id(user_id);
	}
	if let Some(ref org_id) = api_ctx.org_id {
		ctx = ctx.with_org_id(org_id);
	}
	if let Some(ref session_id) = api_ctx.session_id {
		ctx = ctx.with_session_id(session_id);
	}

	for (key, value) in &api_ctx.attributes {
		ctx = ctx.with_attribute(key, value.clone());
	}

	if let Some(ref geo) = api_ctx.geo {
		let mut geo_ctx = GeoContext::new();
		if let Some(ref country) = geo.country {
			geo_ctx = geo_ctx.with_country(country);
		}
		if let Some(ref region) = geo.region {
			geo_ctx = geo_ctx.with_region(region);
		}
		if let Some(ref city) = geo.city {
			geo_ctx = geo_ctx.with_city(city);
		}
		ctx = ctx.with_geo(geo_ctx);
	}

	ctx
}

/// Helper to convert core evaluation reason to API reason.
fn to_api_reason(reason: &EvaluationReason) -> EvaluationReasonApi {
	match reason {
		EvaluationReason::Default => EvaluationReasonApi::Default,
		EvaluationReason::Strategy { strategy_id } => EvaluationReasonApi::Strategy {
			strategy_id: strategy_id.to_string(),
		},
		EvaluationReason::KillSwitch { kill_switch_id } => EvaluationReasonApi::KillSwitch {
			kill_switch_id: kill_switch_id.to_string(),
		},
		EvaluationReason::Prerequisite { missing_flag } => EvaluationReasonApi::Prerequisite {
			missing_flag: missing_flag.clone(),
		},
		EvaluationReason::Disabled => EvaluationReasonApi::Disabled,
		EvaluationReason::Error { message } => EvaluationReasonApi::Error {
			message: message.clone(),
		},
	}
}

/// Helper to convert core variant value to API variant value.
fn to_api_variant_value(value: &VariantValue) -> VariantValueApi {
	match value {
		VariantValue::Boolean(b) => VariantValueApi::Boolean(*b),
		VariantValue::String(s) => VariantValueApi::String(s.clone()),
		VariantValue::Json(v) => VariantValueApi::Json(v.clone()),
	}
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/evaluate",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = EvaluateAllFlagsRequest,
    responses(
        (status = 200, description = "All flags evaluated", body = EvaluateAllFlagsResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Evaluate all flags for a given context.
///
/// This endpoint evaluates all non-archived flags for the organization
/// and returns the evaluation results for each flag. The evaluation
/// takes into account:
/// - Environment configuration (enabled/disabled)
/// - Kill switches (platform and org level)
/// - Prerequisites
/// - Strategy conditions, percentage targeting, and schedules
#[tracing::instrument(skip(state, payload), fields(%org_id, environment = %payload.context.environment))]
pub async fn evaluate_all_flags(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<EvaluateAllFlagsRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	let context = to_core_context(&payload.context);

	// Get the environment for this context
	let environment = match state
		.flags_repo
		.get_environment_by_name(flags_org_id, &payload.context.environment)
		.await
	{
		Ok(Some(env)) => env,
		Ok(None) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_environment",
				t(locale, "server.api.flags.environment_not_found"),
			)
			.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, environment = %payload.context.environment, "Failed to get environment");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get all flags for this org (non-archived)
	let flags = match state.flags_repo.list_flags(Some(flags_org_id), false).await {
		Ok(flags) => flags,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list flags");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get platform flags as well (they override org flags)
	let platform_flags = match state.flags_repo.list_flags(None, false).await {
		Ok(flags) => flags,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list platform flags");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get active kill switches (platform first, then org)
	let platform_kill_switches = match state.flags_repo.list_active_kill_switches(None).await {
		Ok(ks) => ks,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list platform kill switches");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let org_kill_switches = match state
		.flags_repo
		.list_active_kill_switches(Some(flags_org_id))
		.await
	{
		Ok(ks) => ks,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list org kill switches");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Combine kill switches (platform first for precedence)
	let all_kill_switches: Vec<_> = platform_kill_switches
		.into_iter()
		.chain(org_kill_switches.into_iter())
		.collect();

	// Build a map of flag keys to flags, with platform flags taking precedence
	let mut flag_map: std::collections::HashMap<String, &loom_flags_core::Flag> =
		std::collections::HashMap::new();

	// Add org flags first
	for flag in &flags {
		flag_map.insert(flag.key.clone(), flag);
	}

	// Platform flags override org flags
	for flag in &platform_flags {
		flag_map.insert(flag.key.clone(), flag);
	}

	// Evaluate each flag
	let mut results = Vec::with_capacity(flag_map.len());

	for (_, flag) in &flag_map {
		// Get config for this environment
		let config = match state
			.flags_repo
			.get_flag_config(flag.id, environment.id)
			.await
		{
			Ok(config) => config,
			Err(e) => {
				tracing::error!(error = %e, flag_key = %flag.key, "Failed to get flag config");
				// Return an error result for this flag
				results.push(EvaluationResultApi {
					flag_key: flag.key.clone(),
					variant: flag.default_variant.clone(),
					value: to_api_variant_value(
						&flag
							.get_default_variant()
							.map(|v| v.value.clone())
							.unwrap_or(VariantValue::Boolean(false)),
					),
					reason: EvaluationReasonApi::Error {
						message: "Failed to get flag config".to_string(),
					},
				});
				continue;
			}
		};

		// Get strategy if configured
		let strategy = match &config {
			Some(c) => match c.strategy_id {
				Some(strategy_id) => match state.flags_repo.get_strategy_by_id(strategy_id).await {
					Ok(s) => s,
					Err(e) => {
						tracing::error!(error = %e, %strategy_id, "Failed to get strategy");
						None
					}
				},
				None => None,
			},
			None => None,
		};

		// Evaluate prerequisites - collect owned strings to avoid lifetime issues
		let mut prereq_results: Vec<(String, String)> = Vec::new();
		for prereq in &flag.prerequisites {
			if let Some(prereq_flag) = flag_map.get(&prereq.flag_key) {
				let prereq_config = state
					.flags_repo
					.get_flag_config(prereq_flag.id, environment.id)
					.await
					.ok()
					.flatten();
				let prereq_result = evaluate_flag(
					prereq_flag,
					prereq_config.as_ref(),
					None,
					&all_kill_switches,
					&[],
					&context,
				);
				prereq_results.push((prereq.flag_key.clone(), prereq_result.variant.clone()));
			}
		}

		// Convert prereq_results to the expected format
		let prereq_refs: Vec<(&str, &str)> = prereq_results
			.iter()
			.map(|(k, v)| (k.as_str(), v.as_str()))
			.collect();

		let result = evaluate_flag(
			flag,
			config.as_ref(),
			strategy.as_ref(),
			&all_kill_switches,
			&prereq_refs,
			&context,
		);

		results.push(EvaluationResultApi {
			flag_key: result.flag_key,
			variant: result.variant,
			value: to_api_variant_value(&result.value),
			reason: to_api_reason(&result.reason),
		});
	}

	tracing::debug!(
		?org_id,
		environment = %payload.context.environment,
		flag_count = results.len(),
		"Evaluated all flags"
	);

	(
		StatusCode::OK,
		Json(EvaluateAllFlagsResponse {
			results,
			evaluated_at: chrono::Utc::now(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/{flag_key}/evaluate",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_key" = String, Path, description = "Flag key to evaluate")
    ),
    request_body = EvaluateFlagRequest,
    responses(
        (status = 200, description = "Flag evaluated", body = EvaluationResultApi),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag or organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Evaluate a single flag for a given context.
///
/// This endpoint evaluates a specific flag and returns the evaluation result.
/// Platform flags take precedence over org flags with the same key.
#[tracing::instrument(skip(state, payload), fields(%org_id, %flag_key, environment = %payload.context.environment))]
pub async fn evaluate_flag_endpoint(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_key)): Path<(String, String)>,
	Json(payload): Json<EvaluateFlagRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

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

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	let context = to_core_context(&payload.context);

	// Get the environment for this context
	let environment = match state
		.flags_repo
		.get_environment_by_name(flags_org_id, &payload.context.environment)
		.await
	{
		Ok(Some(env)) => env,
		Ok(None) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_environment",
				t(locale, "server.api.flags.environment_not_found"),
			)
			.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, environment = %payload.context.environment, "Failed to get environment");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Check for platform flag first (takes precedence)
	let flag = match state.flags_repo.get_flag_by_key(None, &flag_key).await {
		Ok(Some(f)) => f,
		Ok(None) => {
			// Try org flag
			match state
				.flags_repo
				.get_flag_by_key(Some(flags_org_id), &flag_key)
				.await
			{
				Ok(Some(f)) => f,
				Ok(None) => {
					return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
						.into_response();
				}
				Err(e) => {
					tracing::error!(error = %e, %flag_key, "Failed to get flag");
					return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
						.into_response();
				}
			}
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_key, "Failed to get platform flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get active kill switches (platform first, then org)
	let platform_kill_switches = match state.flags_repo.list_active_kill_switches(None).await {
		Ok(ks) => ks,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list platform kill switches");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let org_kill_switches = match state
		.flags_repo
		.list_active_kill_switches(Some(flags_org_id))
		.await
	{
		Ok(ks) => ks,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list org kill switches");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let all_kill_switches: Vec<_> = platform_kill_switches
		.into_iter()
		.chain(org_kill_switches.into_iter())
		.collect();

	// Get config for this environment
	let config = match state
		.flags_repo
		.get_flag_config(flag.id, environment.id)
		.await
	{
		Ok(config) => config,
		Err(e) => {
			tracing::error!(error = %e, %flag_key, "Failed to get flag config");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get strategy if configured
	let strategy = match &config {
		Some(c) => match c.strategy_id {
			Some(strategy_id) => match state.flags_repo.get_strategy_by_id(strategy_id).await {
				Ok(s) => s,
				Err(e) => {
					tracing::error!(error = %e, %strategy_id, "Failed to get strategy");
					None
				}
			},
			None => None,
		},
		None => None,
	};

	// Evaluate prerequisites
	let mut prereq_results: Vec<(String, String)> = Vec::new();
	for prereq in &flag.prerequisites {
		// Try platform flag first, then org flag
		let prereq_flag = match state.flags_repo.get_flag_by_key(None, &prereq.flag_key).await {
			Ok(Some(f)) => Some(f),
			Ok(None) => state
				.flags_repo
				.get_flag_by_key(Some(flags_org_id), &prereq.flag_key)
				.await
				.ok()
				.flatten(),
			Err(_) => None,
		};

		if let Some(prereq_flag) = prereq_flag {
			let prereq_config = state
				.flags_repo
				.get_flag_config(prereq_flag.id, environment.id)
				.await
				.ok()
				.flatten();
			let prereq_result = evaluate_flag(
				&prereq_flag,
				prereq_config.as_ref(),
				None,
				&all_kill_switches,
				&[],
				&context,
			);
			prereq_results.push((prereq.flag_key.clone(), prereq_result.variant));
		}
	}

	let prereq_refs: Vec<(&str, &str)> = prereq_results
		.iter()
		.map(|(k, v)| (k.as_str(), v.as_str()))
		.collect();

	let result = evaluate_flag(
		&flag,
		config.as_ref(),
		strategy.as_ref(),
		&all_kill_switches,
		&prereq_refs,
		&context,
	);

	tracing::debug!(
		%flag_key,
		variant = %result.variant,
		?result.reason,
		"Flag evaluated"
	);

	(
		StatusCode::OK,
		Json(EvaluationResultApi {
			flag_key: result.flag_key,
			variant: result.variant,
			value: to_api_variant_value(&result.value),
			reason: to_api_reason(&result.reason),
		}),
	)
		.into_response()
}

// ============================================================================
// SSE Streaming Routes
// ============================================================================

/// Query parameters for the flag stream.
#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct StreamFlagsParams {
	/// Environment name to subscribe to (e.g., "prod", "staging").
	pub environment: String,
}

/// Authenticate an SDK key from the Authorization header.
///
/// Returns (org_id, environment_id, environment_name) on success.
async fn authenticate_sdk_key(
	state: &AppState,
	headers: &HeaderMap,
) -> Result<(loom_flags_core::OrgId, EnvironmentId, String), (StatusCode, String)> {
	// Extract bearer token
	let auth_header = headers
		.get("authorization")
		.and_then(|v| v.to_str().ok())
		.ok_or((
			StatusCode::UNAUTHORIZED,
			"Missing Authorization header".to_string(),
		))?;

	if !auth_header.to_lowercase().starts_with("bearer ") {
		return Err((
			StatusCode::UNAUTHORIZED,
			"Invalid Authorization header format".to_string(),
		));
	}

	let sdk_key_raw = &auth_header[7..]; // Skip "Bearer "

	// Parse SDK key to extract type and environment
	let (_key_type, env_name, _) = SdkKey::parse_key(sdk_key_raw).ok_or((
		StatusCode::UNAUTHORIZED,
		"Invalid SDK key format".to_string(),
	))?;

	// Find and verify the SDK key
	// This iterates through all SDK keys for environments with matching name
	// and verifies against the Argon2 hash. It's O(n) but acceptable for
	// connection establishment (one-time per SSE stream).
	let (sdk_key_record, environment) = state
		.flags_repo
		.find_sdk_key_by_verification(sdk_key_raw, &env_name)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to verify SDK key");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				"Database error".to_string(),
			)
		})?
		.ok_or((StatusCode::UNAUTHORIZED, "Invalid SDK key".to_string()))?;

	// Update last used timestamp (fire and forget)
	let flags_repo = state.flags_repo.clone();
	let key_id = sdk_key_record.id;
	tokio::spawn(async move {
		let _ = flags_repo.update_sdk_key_last_used(key_id).await;
	});

	Ok((environment.org_id, environment.id, env_name))
}

#[utoipa::path(
    get,
    path = "/api/flags/stream",
    params(StreamFlagsParams),
    responses(
        (status = 200, description = "SSE stream of flag updates", content_type = "text/event-stream"),
        (status = 401, description = "Invalid or missing SDK key"),
        (status = 404, description = "Environment not found")
    ),
    tag = "flags",
    security(
        ("sdk_key" = [])
    )
)]
/// Stream real-time flag updates via Server-Sent Events.
///
/// This endpoint requires SDK key authentication via the Authorization header.
/// On connection, an `init` event is sent with the full state of all flags.
/// Subsequent events are sent when flags are updated, archived, or when kill
/// switches are activated/deactivated.
///
/// Event types:
/// - `init`: Full state of all flags on connect
/// - `flag.updated`: Flag or config changed
/// - `flag.archived`: Flag archived
/// - `flag.restored`: Flag restored from archive
/// - `killswitch.activated`: Kill switch activated
/// - `killswitch.deactivated`: Kill switch deactivated
/// - `heartbeat`: Keep-alive (every 30s)
#[tracing::instrument(skip(state, headers))]
pub async fn stream_flags(
	State(state): State<AppState>,
	headers: HeaderMap,
	Query(params): Query<StreamFlagsParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
	// Authenticate SDK key
	let (org_id, env_id, env_name) = authenticate_sdk_key(&state, &headers).await?;

	// Verify the environment matches the query param
	if env_name != params.environment {
		return Err((
			StatusCode::BAD_REQUEST,
			format!(
				"SDK key is for environment '{}', but '{}' was requested",
				env_name, params.environment
			),
		));
	}

	tracing::info!(
		org_id = %org_id,
		environment = %params.environment,
		"Client connected to flag stream"
	);

	// Build initial state
	let flags = state
		.flags_repo
		.list_flags(Some(org_id), false)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list flags for init");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				"Failed to load flags".to_string(),
			)
		})?;

	let kill_switches = state
		.flags_repo
		.list_active_kill_switches(Some(org_id))
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list kill switches for init");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				"Failed to load kill switches".to_string(),
			)
		})?;

	// Build FlagState for each flag
	let mut flag_states = Vec::with_capacity(flags.len());
	for flag in &flags {
		let config = state
			.flags_repo
			.get_flag_config(flag.id, env_id)
			.await
			.ok()
			.flatten();
		flag_states.push(FlagState::from_flag_and_config(flag, config.as_ref()));
	}

	// Build KillSwitchState for each active kill switch
	let kill_switch_states: Vec<KillSwitchState> = kill_switches.iter().map(|ks| ks.into()).collect();

	// Create init event
	let init_event = FlagStreamEvent::init(flag_states, kill_switch_states);

	// Subscribe to broadcast channel
	let receiver = state.flags_broadcaster.subscribe(org_id, env_id).await;
	let broadcast_stream = BroadcastStream::new(receiver);

	// Create a stream that first yields the init event, then yields broadcast events
	let init_stream = futures::stream::once(async move {
		let json = serde_json::to_string(&init_event).unwrap_or_else(|_| "{}".to_string());
		Ok::<_, Infallible>(Event::default().event("init").data(json))
	});

	let updates_stream = broadcast_stream.filter_map(|result| match result {
		Ok(event) => {
			let event_type = event.event_type();
			match serde_json::to_string(&event) {
				Ok(json) => Some(Ok::<_, Infallible>(Event::default().event(event_type).data(json))),
				Err(e) => {
					tracing::warn!(error = %e, "Failed to serialize SSE event");
					None
				}
			}
		}
		Err(e) => {
			tracing::debug!(error = %e, "Broadcast stream error (client may have disconnected)");
			None
		}
	});

	let combined_stream = init_stream.chain(updates_stream);

	Ok(Sse::new(combined_stream).keep_alive(
		axum::response::sse::KeepAlive::new()
			.interval(std::time::Duration::from_secs(30))
			.text("heartbeat"),
	))
}

/// Get broadcaster statistics.
#[utoipa::path(
    get,
    path = "/api/flags/stream/stats",
    responses(
        (status = 200, description = "Broadcaster statistics"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized")
    ),
    tag = "flags"
)]
#[tracing::instrument(skip(state))]
pub async fn stream_stats(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
) -> impl IntoResponse {
	// Only allow system admins to view stats
	if !current_user.user.is_system_admin() {
		return (
			StatusCode::FORBIDDEN,
			Json(serde_json::json!({ "error": "Forbidden" })),
		)
			.into_response();
	}

	let stats = state.flags_broadcaster.stats().await;
	(StatusCode::OK, Json(stats)).into_response()
}
