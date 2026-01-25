// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Environment HTTP handlers.
//!
//! Implements endpoints for environment CRUD operations.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_flags_core::{Environment, EnvironmentId};
use loom_server_api::flags::{
	CreateEnvironmentRequest, EnvironmentResponse, FlagsErrorResponse, FlagsSuccessResponse,
	ListEnvironmentsResponse, UpdateEnvironmentRequest,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_flags::FlagsRepository;

use crate::{
	api::AppState,
	api_response::{bad_request, conflict, internal_error, not_found},
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	parse_id,
	validation::parse_org_id as shared_parse_org_id,
};

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

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::EnvironmentCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("environment", env.id.to_string())
			.details(serde_json::json!({
				"org_id": env.org_id.to_string(),
				"name": env.name,
				"color": env.color,
			}))
			.build(),
	);

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

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::EnvironmentUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("environment", env.id.to_string())
			.details(serde_json::json!({
				"org_id": env.org_id.to_string(),
				"name": env.name,
				"color": env.color,
			}))
			.build(),
	);

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
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::EnvironmentDeleted)
					.actor(AuditUserId::new(current_user.user.id.into_inner()))
					.resource("environment", env_id.to_string())
					.details(serde_json::json!({
						"org_id": env.org_id.to_string(),
						"name": env.name,
					}))
					.build(),
			);

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
