// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Ping endpoints for simple shell script monitoring (unauthenticated).

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use tracing::{info, instrument, warn};

use loom_crons_core::{
	truncate_output, CheckIn, CheckInId, CheckInSource, CheckInStatus, CronStreamEvent,
	MonitorHealth,
};
use loom_server_crons::{calculate_next_expected, CronsRepository};

use crate::api::AppState;

use super::common::{PingParams, PingStartResponse};

/// GET /ping/{key} - Simple success ping
#[utoipa::path(
	get,
	path = "/ping/{key}",
	params(
		("key" = String, Path, description = "Ping key (UUID)"),
		("exit_code" = Option<i32>, Query, description = "Exit code (optional)"),
	),
	responses(
		(status = 200, description = "Ping recorded successfully"),
		(status = 404, description = "Invalid ping key"),
	),
	tag = "crons"
)]
#[instrument(skip(state), fields(ping_key = %key))]
pub async fn ping_success(
	State(state): State<AppState>,
	Path(key): Path<String>,
	Query(params): Query<PingParams>,
) -> impl IntoResponse {
	let monitor = match state.crons_repo.get_monitor_by_ping_key(&key).await {
		Ok(Some(m)) => m,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor by ping key");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let now = Utc::now();

	let status = if params.exit_code.unwrap_or(0) == 0 {
		CheckInStatus::Ok
	} else {
		CheckInStatus::Error
	};

	let is_failure = status == CheckInStatus::Error;

	let checkin = CheckIn {
		id: CheckInId::new(),
		monitor_id: monitor.id,
		status,
		started_at: None,
		finished_at: now,
		duration_ms: None,
		environment: None,
		release: None,
		exit_code: params.exit_code,
		output: None,
		crash_event_id: None,
		source: CheckInSource::Ping,
		created_at: now,
	};

	if let Err(e) = state.crons_repo.create_checkin(&checkin).await {
		tracing::error!(error = %e, "Failed to create checkin");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	let health = if is_failure {
		MonitorHealth::Failing
	} else {
		MonitorHealth::Healthy
	};

	// Calculate next expected check-in time
	let next_expected_at = calculate_next_expected(&monitor.schedule, &monitor.timezone, now).ok();

	let _ = state
		.crons_repo
		.update_monitor_health(monitor.id, health)
		.await;
	let _ = state
		.crons_repo
		.update_monitor_last_checkin(monitor.id, status, next_expected_at)
		.await;
	let _ = state
		.crons_repo
		.increment_monitor_stats(monitor.id, is_failure)
		.await;

	// Broadcast SSE event
	let sse_event = if is_failure {
		CronStreamEvent::checkin_error(
			monitor.id,
			monitor.slug.clone(),
			checkin.id,
			params.exit_code,
			monitor.consecutive_failures + 1,
		)
	} else {
		CronStreamEvent::checkin_ok(monitor.id, monitor.slug.clone(), checkin.id, None)
	};
	state
		.crons_broadcaster
		.broadcast(monitor.org_id, sse_event)
		.await;

	info!(
		monitor_id = %monitor.id,
		monitor_slug = %monitor.slug,
		status = %status,
		"Ping recorded"
	);

	StatusCode::OK.into_response()
}

/// GET /ping/{key}/start - Job starting
#[utoipa::path(
	get,
	path = "/ping/{key}/start",
	params(
		("key" = String, Path, description = "Ping key (UUID)"),
	),
	responses(
		(status = 200, description = "Start ping recorded", body = PingStartResponse),
		(status = 404, description = "Invalid ping key"),
	),
	tag = "crons"
)]
#[instrument(skip(state), fields(ping_key = %key))]
pub async fn ping_start(
	State(state): State<AppState>,
	Path(key): Path<String>,
) -> impl IntoResponse {
	let monitor = match state.crons_repo.get_monitor_by_ping_key(&key).await {
		Ok(Some(m)) => m,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor by ping key");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let now = Utc::now();

	let checkin = CheckIn {
		id: CheckInId::new(),
		monitor_id: monitor.id,
		status: CheckInStatus::InProgress,
		started_at: Some(now),
		finished_at: now,
		duration_ms: None,
		environment: None,
		release: None,
		exit_code: None,
		output: None,
		crash_event_id: None,
		source: CheckInSource::Ping,
		created_at: now,
	};

	if let Err(e) = state.crons_repo.create_checkin(&checkin).await {
		tracing::error!(error = %e, "Failed to create checkin");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	// Broadcast SSE event
	let sse_event = CronStreamEvent::checkin_started(monitor.id, monitor.slug.clone(), checkin.id);
	state
		.crons_broadcaster
		.broadcast(monitor.org_id, sse_event)
		.await;

	info!(
		monitor_id = %monitor.id,
		monitor_slug = %monitor.slug,
		checkin_id = %checkin.id,
		"Start ping recorded"
	);

	Json(PingStartResponse {
		checkin_id: checkin.id,
	})
	.into_response()
}

/// GET /ping/{key}/fail - Job failed
#[utoipa::path(
	get,
	path = "/ping/{key}/fail",
	params(
		("key" = String, Path, description = "Ping key (UUID)"),
		("exit_code" = Option<i32>, Query, description = "Exit code (optional)"),
	),
	responses(
		(status = 200, description = "Fail ping recorded"),
		(status = 404, description = "Invalid ping key"),
	),
	tag = "crons"
)]
#[instrument(skip(state), fields(ping_key = %key))]
pub async fn ping_fail(
	State(state): State<AppState>,
	Path(key): Path<String>,
	Query(params): Query<PingParams>,
) -> impl IntoResponse {
	let monitor = match state.crons_repo.get_monitor_by_ping_key(&key).await {
		Ok(Some(m)) => m,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor by ping key");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let now = Utc::now();

	let checkin = CheckIn {
		id: CheckInId::new(),
		monitor_id: monitor.id,
		status: CheckInStatus::Error,
		started_at: None,
		finished_at: now,
		duration_ms: None,
		environment: None,
		release: None,
		exit_code: params.exit_code,
		output: None,
		crash_event_id: None,
		source: CheckInSource::Ping,
		created_at: now,
	};

	if let Err(e) = state.crons_repo.create_checkin(&checkin).await {
		tracing::error!(error = %e, "Failed to create checkin");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	// Calculate next expected check-in time
	let next_expected_at = calculate_next_expected(&monitor.schedule, &monitor.timezone, now).ok();

	let _ = state
		.crons_repo
		.update_monitor_health(monitor.id, MonitorHealth::Failing)
		.await;
	let _ = state
		.crons_repo
		.update_monitor_last_checkin(monitor.id, CheckInStatus::Error, next_expected_at)
		.await;
	let _ = state
		.crons_repo
		.increment_monitor_stats(monitor.id, true)
		.await;

	// Broadcast SSE event
	let sse_event = CronStreamEvent::checkin_error(
		monitor.id,
		monitor.slug.clone(),
		checkin.id,
		params.exit_code,
		monitor.consecutive_failures + 1,
	);
	state
		.crons_broadcaster
		.broadcast(monitor.org_id, sse_event)
		.await;

	warn!(
		monitor_id = %monitor.id,
		monitor_slug = %monitor.slug,
		exit_code = ?params.exit_code,
		"Fail ping recorded"
	);

	StatusCode::OK.into_response()
}

/// POST /ping/{key} - Ping with body
#[utoipa::path(
	post,
	path = "/ping/{key}",
	params(
		("key" = String, Path, description = "Ping key (UUID)"),
		("exit_code" = Option<i32>, Query, description = "Exit code (0 = success)"),
	),
	request_body = String,
	responses(
		(status = 200, description = "Ping recorded successfully"),
		(status = 404, description = "Invalid ping key"),
	),
	tag = "crons"
)]
#[instrument(skip(state, body), fields(ping_key = %key))]
pub async fn ping_with_body(
	State(state): State<AppState>,
	Path(key): Path<String>,
	Query(params): Query<PingParams>,
	body: String,
) -> impl IntoResponse {
	let monitor = match state.crons_repo.get_monitor_by_ping_key(&key).await {
		Ok(Some(m)) => m,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor by ping key");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let now = Utc::now();

	let status = if params.exit_code.unwrap_or(0) == 0 {
		CheckInStatus::Ok
	} else {
		CheckInStatus::Error
	};

	let is_failure = status == CheckInStatus::Error;

	let output = if body.is_empty() {
		None
	} else {
		Some(truncate_output(&body))
	};

	let checkin = CheckIn {
		id: CheckInId::new(),
		monitor_id: monitor.id,
		status,
		started_at: None,
		finished_at: now,
		duration_ms: None,
		environment: None,
		release: None,
		exit_code: params.exit_code,
		output,
		crash_event_id: None,
		source: CheckInSource::Ping,
		created_at: now,
	};

	if let Err(e) = state.crons_repo.create_checkin(&checkin).await {
		tracing::error!(error = %e, "Failed to create checkin");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	let health = if is_failure {
		MonitorHealth::Failing
	} else {
		MonitorHealth::Healthy
	};

	// Calculate next expected check-in time
	let next_expected_at = calculate_next_expected(&monitor.schedule, &monitor.timezone, now).ok();

	let _ = state
		.crons_repo
		.update_monitor_health(monitor.id, health)
		.await;
	let _ = state
		.crons_repo
		.update_monitor_last_checkin(monitor.id, status, next_expected_at)
		.await;
	let _ = state
		.crons_repo
		.increment_monitor_stats(monitor.id, is_failure)
		.await;

	// Broadcast SSE event
	let sse_event = if is_failure {
		CronStreamEvent::checkin_error(
			monitor.id,
			monitor.slug.clone(),
			checkin.id,
			params.exit_code,
			monitor.consecutive_failures + 1,
		)
	} else {
		CronStreamEvent::checkin_ok(monitor.id, monitor.slug.clone(), checkin.id, None)
	};
	state
		.crons_broadcaster
		.broadcast(monitor.org_id, sse_event)
		.await;

	info!(
		monitor_id = %monitor.id,
		monitor_slug = %monitor.slug,
		status = %status,
		has_output = !body.is_empty(),
		"Ping with body recorded"
	);

	StatusCode::OK.into_response()
}
