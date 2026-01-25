// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! SDK check-in endpoints (authenticated).

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use tracing::{info, instrument};

use loom_crons_core::{
	truncate_output, CheckIn, CheckInId, CheckInSource, CheckInStatus, CronStreamEvent,
	MonitorHealth,
};
use loom_server_crons::{calculate_next_expected, CronsRepository};

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::resolve_user_locale;

use super::common::{
	verify_org_membership, CreateCheckInRequest, CreateCheckInResponse, UpdateCheckInRequest,
};

/// POST /api/crons/monitors/{slug}/checkins - Create check-in (SDK)
#[utoipa::path(
	post,
	path = "/api/crons/monitors/{slug}/checkins",
	params(
		("slug" = String, Path, description = "Monitor slug"),
	),
	request_body = CreateCheckInRequest,
	responses(
		(status = 201, description = "Check-in created", body = CreateCheckInResponse),
		(status = 400, description = "Invalid request"),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Monitor not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user, req), fields(slug = %slug, status = %req.status))]
pub async fn create_checkin(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(slug): Path<String>,
	Json(req): Json<CreateCheckInRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &req.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	let monitor = match state
		.crons_repo
		.get_monitor_by_slug(req.org_id, &slug)
		.await
	{
		Ok(Some(m)) => m,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let now = Utc::now();

	// Truncate output if provided
	let output = req.output.map(|o| truncate_output(&o));

	let checkin = CheckIn {
		id: CheckInId::new(),
		monitor_id: monitor.id,
		status: req.status,
		started_at: req
			.started_at
			.or(if req.status == CheckInStatus::InProgress {
				Some(now)
			} else {
				None
			}),
		finished_at: req.finished_at.unwrap_or(now),
		duration_ms: req.duration_ms,
		environment: req.environment,
		release: req.release,
		exit_code: req.exit_code,
		output,
		crash_event_id: req.crash_event_id,
		source: CheckInSource::Sdk,
		created_at: now,
	};

	if let Err(e) = state.crons_repo.create_checkin(&checkin).await {
		tracing::error!(error = %e, "Failed to create checkin");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	// Update monitor state based on check-in status
	let is_failure = matches!(
		req.status,
		CheckInStatus::Error | CheckInStatus::Missed | CheckInStatus::Timeout
	);

	if req.status != CheckInStatus::InProgress {
		let health = if is_failure {
			MonitorHealth::Failing
		} else {
			MonitorHealth::Healthy
		};

		// Calculate next expected check-in time
		let next_expected_at =
			calculate_next_expected(&monitor.schedule, &monitor.timezone, now).ok();

		let _ = state
			.crons_repo
			.update_monitor_health(monitor.id, health)
			.await;
		let _ = state
			.crons_repo
			.update_monitor_last_checkin(monitor.id, req.status, next_expected_at)
			.await;
		let _ = state
			.crons_repo
			.increment_monitor_stats(monitor.id, is_failure)
			.await;
	}

	// Broadcast SSE event
	let sse_event = match req.status {
		CheckInStatus::InProgress => {
			CronStreamEvent::checkin_started(monitor.id, monitor.slug.clone(), checkin.id)
		}
		CheckInStatus::Ok => CronStreamEvent::checkin_ok(
			monitor.id,
			monitor.slug.clone(),
			checkin.id,
			req.duration_ms,
		),
		CheckInStatus::Error | CheckInStatus::Missed | CheckInStatus::Timeout => {
			CronStreamEvent::checkin_error(
				monitor.id,
				monitor.slug.clone(),
				checkin.id,
				req.exit_code,
				monitor.consecutive_failures + 1,
			)
		}
	};
	state
		.crons_broadcaster
		.broadcast(monitor.org_id, sse_event)
		.await;

	info!(
		monitor_id = %monitor.id,
		monitor_slug = %monitor.slug,
		checkin_id = %checkin.id,
		status = %req.status,
		"SDK check-in created"
	);

	(
		StatusCode::CREATED,
		Json(CreateCheckInResponse {
			id: checkin.id,
			status: checkin.status,
		}),
	)
		.into_response()
}

/// PATCH /api/crons/checkins/{id} - Update check-in
#[utoipa::path(
	patch,
	path = "/api/crons/checkins/{id}",
	params(
		("id" = CheckInId, Path, description = "Check-in ID"),
	),
	request_body = UpdateCheckInRequest,
	responses(
		(status = 200, description = "Check-in updated", body = CheckIn),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Check-in not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user, req), fields(checkin_id = %id, status = %req.status))]
pub async fn update_checkin(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<CheckInId>,
	Json(req): Json<UpdateCheckInRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let mut checkin = match state.crons_repo.get_checkin_by_id(id).await {
		Ok(Some(c)) => c,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get checkin");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	// Get the monitor to verify org membership
	let monitor = match state.crons_repo.get_monitor_by_id(checkin.monitor_id).await {
		Ok(Some(m)) => m,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor for checkin");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &monitor.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	let now = Utc::now();

	// Update fields
	checkin.status = req.status;
	checkin.finished_at = req.finished_at.unwrap_or(now);
	if let Some(duration_ms) = req.duration_ms {
		checkin.duration_ms = Some(duration_ms);
	} else if let Some(started_at) = checkin.started_at {
		// Calculate duration from started_at if not provided
		checkin.duration_ms = Some((checkin.finished_at - started_at).num_milliseconds() as u64);
	}
	if let Some(exit_code) = req.exit_code {
		checkin.exit_code = Some(exit_code);
	}
	if let Some(output) = req.output {
		checkin.output = Some(truncate_output(&output));
	}
	if let Some(crash_event_id) = req.crash_event_id {
		checkin.crash_event_id = Some(crash_event_id);
	}

	if let Err(e) = state.crons_repo.update_checkin(&checkin).await {
		tracing::error!(error = %e, "Failed to update checkin");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	// Update monitor state
	let is_failure = matches!(
		req.status,
		CheckInStatus::Error | CheckInStatus::Missed | CheckInStatus::Timeout
	);
	let health = if is_failure {
		MonitorHealth::Failing
	} else {
		MonitorHealth::Healthy
	};

	// Get monitor to calculate next expected time
	let next_expected_at = match state.crons_repo.get_monitor_by_id(checkin.monitor_id).await {
		Ok(Some(monitor)) => {
			calculate_next_expected(&monitor.schedule, &monitor.timezone, now).ok()
		}
		_ => None,
	};

	let _ = state
		.crons_repo
		.update_monitor_health(checkin.monitor_id, health)
		.await;
	let _ = state
		.crons_repo
		.update_monitor_last_checkin(checkin.monitor_id, req.status, next_expected_at)
		.await;
	let _ = state
		.crons_repo
		.increment_monitor_stats(checkin.monitor_id, is_failure)
		.await;

	// Broadcast SSE event
	let sse_event = match req.status {
		CheckInStatus::InProgress => {
			CronStreamEvent::checkin_started(monitor.id, monitor.slug.clone(), checkin.id)
		}
		CheckInStatus::Ok => CronStreamEvent::checkin_ok(
			monitor.id,
			monitor.slug.clone(),
			checkin.id,
			checkin.duration_ms,
		),
		CheckInStatus::Error | CheckInStatus::Missed | CheckInStatus::Timeout => {
			CronStreamEvent::checkin_error(
				monitor.id,
				monitor.slug.clone(),
				checkin.id,
				checkin.exit_code,
				monitor.consecutive_failures + 1,
			)
		}
	};
	state
		.crons_broadcaster
		.broadcast(monitor.org_id, sse_event)
		.await;

	info!(
		checkin_id = %checkin.id,
		monitor_id = %checkin.monitor_id,
		status = %req.status,
		"Check-in updated"
	);

	Json(checkin).into_response()
}

/// GET /api/crons/checkins/{id} - Get check-in by ID
#[utoipa::path(
	get,
	path = "/api/crons/checkins/{id}",
	params(
		("id" = CheckInId, Path, description = "Check-in ID"),
	),
	responses(
		(status = 200, description = "Check-in details", body = CheckIn),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Check-in not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user), fields(checkin_id = %id))]
pub async fn get_checkin(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<CheckInId>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let checkin = match state.crons_repo.get_checkin_by_id(id).await {
		Ok(Some(c)) => c,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get checkin");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	// Get the monitor to verify org membership
	let monitor = match state.crons_repo.get_monitor_by_id(checkin.monitor_id).await {
		Ok(Some(m)) => m,
		Ok(None) => return StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor for checkin");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &monitor.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	Json(checkin).into_response()
}
