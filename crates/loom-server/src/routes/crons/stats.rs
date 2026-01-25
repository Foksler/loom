// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Statistics endpoints for cron monitors.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use tracing::instrument;

use loom_crons_core::{MonitorHealth, MonitorStatus, OrgId, StatsPeriod};
use loom_server_crons::CronsRepository;

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::resolve_user_locale;

use super::common::{
	verify_org_membership, GetMonitorStatsParams, GetStatsOverviewParams, MonitorStatsResponse,
	StatsOverviewResponse,
};

/// GET /api/crons/monitors/{slug}/stats - Get monitor stats
#[utoipa::path(
	get,
	path = "/api/crons/monitors/{slug}/stats",
	params(
		("slug" = String, Path, description = "Monitor slug"),
		("org_id" = OrgId, Query, description = "Organization ID"),
		("period" = Option<StatsPeriod>, Query, description = "Stats period (day, week, month). Default: week"),
	),
	responses(
		(status = 200, description = "Monitor statistics", body = MonitorStatsResponse),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Monitor not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user), fields(slug = %slug))]
pub async fn get_monitor_stats(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(slug): Path<String>,
	Query(params): Query<GetMonitorStatsParams>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &params.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	let monitor = match state
		.crons_repo
		.get_monitor_by_slug(params.org_id, &slug)
		.await
	{
		Ok(Some(m)) => m,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	match state
		.crons_repo
		.get_monitor_stats(monitor.id, params.period)
		.await
	{
		Ok(stats) => Json(MonitorStatsResponse { stats }).into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor stats");
			StatusCode::INTERNAL_SERVER_ERROR.into_response()
		}
	}
}

/// GET /api/crons/stats/overview - Org-wide stats overview
#[utoipa::path(
	get,
	path = "/api/crons/stats/overview",
	params(
		("org_id" = OrgId, Query, description = "Organization ID"),
	),
	responses(
		(status = 200, description = "Organization-wide cron stats overview", body = StatsOverviewResponse),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user))]
pub async fn get_stats_overview(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Query(params): Query<GetStatsOverviewParams>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &params.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	// Get all monitors for the org
	let monitors = match state.crons_repo.list_monitors(params.org_id).await {
		Ok(m) => m,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list monitors");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	// Compute aggregate stats
	let total_monitors = monitors.len() as u64;
	let active_monitors = monitors
		.iter()
		.filter(|m| m.status == MonitorStatus::Active)
		.count() as u64;
	let paused_monitors = monitors
		.iter()
		.filter(|m| m.status == MonitorStatus::Paused)
		.count() as u64;
	let healthy_monitors = monitors
		.iter()
		.filter(|m| m.health == MonitorHealth::Healthy)
		.count() as u64;
	let failing_monitors = monitors
		.iter()
		.filter(|m| m.health == MonitorHealth::Failing)
		.count() as u64;
	let missed_monitors = monitors
		.iter()
		.filter(|m| m.health == MonitorHealth::Missed)
		.count() as u64;

	// Sum up 24h stats from monitors
	// Note: We use total_checkins and total_failures from the monitors as a proxy
	// In a more complete implementation, we'd query the actual 24h window from checkins
	let mut total_checkins_24h = 0u64;
	let mut total_failures_24h = 0u64;

	// For each active monitor, get the day stats and aggregate
	for monitor in &monitors {
		if monitor.status == MonitorStatus::Active {
			if let Ok(stats) = state
				.crons_repo
				.get_monitor_stats(monitor.id, StatsPeriod::Day)
				.await
			{
				total_checkins_24h += stats.total_checkins;
				total_failures_24h +=
					stats.failed_checkins + stats.missed_checkins + stats.timeout_checkins;
			}
		}
	}

	let overall_uptime_percentage = if total_checkins_24h > 0 {
		let successful = total_checkins_24h.saturating_sub(total_failures_24h);
		(successful as f64 / total_checkins_24h as f64) * 100.0
	} else {
		100.0
	};

	Json(StatsOverviewResponse {
		total_monitors,
		active_monitors,
		paused_monitors,
		healthy_monitors,
		failing_monitors,
		missed_monitors,
		total_checkins_24h,
		total_failures_24h,
		overall_uptime_percentage,
	})
	.into_response()
}
