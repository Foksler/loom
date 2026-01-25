// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Strategy HTTP handlers.
//!
//! Implements endpoints for strategy CRUD operations.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_flags_core::{Strategy, StrategyId};
use loom_server_api::flags::{
	CreateStrategyRequest, FlagsErrorResponse, FlagsSuccessResponse, ListStrategiesResponse,
	StrategyResponse, UpdateStrategyRequest,
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

use super::common::{
	condition_from_api, condition_to_api, percentage_key_from_api, percentage_key_to_api,
	schedule_from_api, schedule_to_api,
};

// ============================================================================
// Helper Functions
// ============================================================================

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

// ============================================================================
// Strategy Routes
// ============================================================================

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

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::StrategyCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("strategy", strategy.id.to_string())
			.details(serde_json::json!({
				"org_id": flags_org_id.to_string(),
				"name": strategy.name,
				"description": strategy.description,
				"percentage": strategy.percentage,
			}))
			.build(),
	);

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

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::StrategyUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("strategy", strategy.id.to_string())
			.details(serde_json::json!({
				"org_id": strategy.org_id.map(|o| o.to_string()),
				"name": strategy.name,
				"description": strategy.description,
				"percentage": strategy.percentage,
			}))
			.build(),
	);

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
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::StrategyDeleted)
					.actor(AuditUserId::new(current_user.user.id.into_inner()))
					.resource("strategy", strategy_id.to_string())
					.details(serde_json::json!({
						"org_id": flags_org_id.to_string(),
						"name": strategy.name,
					}))
					.build(),
			);

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
