// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Platform strategy handlers.

use axum::{
	extract::State,
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_flags_core::Strategy;
use loom_server_flags::FlagsRepository;

use crate::{
	api::AppState,
	api_response::internal_error,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	routes::admin::AdminErrorResponse,
};

use super::common::{FlagsErrorResponse, ListStrategiesResponse, StrategyResponse};

/// List all platform-level strategies.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	get,
	path = "/api/admin/flags/strategies",
	responses(
		(status = 200, description = "List of platform strategies", body = ListStrategiesResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id))]
pub async fn list_platform_strategies(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform strategies list attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let strategies = match state.flags_repo.list_strategies(None).await {
		Ok(s) => s,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list platform strategies");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	tracing::info!(
		actor_id = %current_user.user.id,
		count = strategies.len(),
		"Listed platform strategies"
	);

	let strategies_response: Vec<StrategyResponse> =
		strategies.iter().map(strategy_to_response).collect();

	(
		StatusCode::OK,
		Json(ListStrategiesResponse {
			strategies: strategies_response,
		}),
	)
		.into_response()
}

fn strategy_to_response(s: &Strategy) -> StrategyResponse {
	use crate::routes::flags::{condition_to_api, percentage_key_to_api, schedule_to_api};
	StrategyResponse {
		id: s.id.to_string(),
		org_id: s.org_id.map(|id| id.to_string()),
		name: s.name.clone(),
		description: s.description.clone(),
		conditions: s.conditions.iter().map(condition_to_api).collect(),
		percentage: s.percentage,
		percentage_key: percentage_key_to_api(&s.percentage_key),
		schedule: s.schedule.as_ref().map(schedule_to_api),
		created_at: s.created_at,
		updated_at: s.updated_at,
	}
}
