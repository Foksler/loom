// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Flag config HTTP handlers.
//!
//! Implements endpoints for managing flag configurations per environment.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_flags_core::{EnvironmentId, FlagId, FlagStreamEvent, VariantValue};
use loom_server_api::flags::{
	FlagConfigResponse, FlagsErrorResponse, ListFlagConfigsResponse, UpdateFlagConfigRequest,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_flags::FlagsRepository;

use crate::{
	api::AppState,
	api_response::{bad_request, internal_error, not_found},
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	parse_id,
	validation::parse_org_id as shared_parse_org_id,
};

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

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::FlagConfigUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("flag_config", config.id.to_string())
			.details(serde_json::json!({
				"org_id": flags_org_id.to_string(),
				"flag_id": flag.id.to_string(),
				"flag_key": flag.key,
				"environment_id": env_id.to_string(),
				"environment_name": env.name,
				"enabled": config.enabled,
				"strategy_id": config.strategy_id.map(|s| s.to_string()),
			}))
			.build(),
	);

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
	state
		.flags_broadcaster
		.broadcast(flags_org_id, env_id, event)
		.await;

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
