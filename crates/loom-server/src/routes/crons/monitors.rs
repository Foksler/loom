// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Monitor CRUD endpoints (authenticated).

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use tracing::{info, instrument};

use loom_crons_core::{Monitor, MonitorHealth, MonitorId, MonitorSchedule, MonitorStatus, OrgId};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_crons::{calculate_next_expected, CronsRepository};

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::resolve_user_locale;

use super::common::{
	verify_org_membership, CreateMonitorRequest, CreateMonitorResponse, GetMonitorParams,
	ListCheckInsParams, ListCheckInsResponse, ListMonitorsParams, ListMonitorsResponse,
	MonitorSummary, UpdateMonitorRequest,
};

/// GET /api/crons/monitors - List monitors
#[utoipa::path(
	get,
	path = "/api/crons/monitors",
	params(
		("org_id" = OrgId, Query, description = "Organization ID"),
	),
	responses(
		(status = 200, description = "List of monitors", body = ListMonitorsResponse),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user))]
pub async fn list_monitors(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Query(params): Query<ListMonitorsParams>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &params.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	match state.crons_repo.list_monitors(params.org_id).await {
		Ok(monitors) => {
			let summaries: Vec<MonitorSummary> = monitors.into_iter().map(Into::into).collect();
			Json(ListMonitorsResponse {
				monitors: summaries,
			})
			.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list monitors");
			StatusCode::INTERNAL_SERVER_ERROR.into_response()
		}
	}
}

/// POST /api/crons/monitors - Create monitor
#[utoipa::path(
	post,
	path = "/api/crons/monitors",
	request_body = CreateMonitorRequest,
	responses(
		(status = 201, description = "Monitor created", body = CreateMonitorResponse),
		(status = 400, description = "Invalid request"),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 409, description = "Duplicate slug"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user, req), fields(org_id = %req.org_id, slug = %req.slug))]
pub async fn create_monitor(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Json(req): Json<CreateMonitorRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &req.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	if !Monitor::validate_slug(&req.slug) {
		return (
			StatusCode::BAD_REQUEST,
			Json(serde_json::json!({"error": "Invalid slug"})),
		)
			.into_response();
	}

	if let Ok(Some(_)) = state
		.crons_repo
		.get_monitor_by_slug(req.org_id, &req.slug)
		.await
	{
		return (
			StatusCode::CONFLICT,
			Json(serde_json::json!({"error": "Duplicate slug"})),
		)
			.into_response();
	}

	let now = Utc::now();
	let ping_key = Monitor::generate_ping_key();
	let schedule: MonitorSchedule = req.schedule.into();

	// Calculate initial next_expected_at based on schedule
	let next_expected_at = calculate_next_expected(&schedule, &req.timezone, now).ok();

	let monitor = Monitor {
		id: MonitorId::new(),
		org_id: req.org_id,
		slug: req.slug,
		name: req.name,
		description: req.description,
		status: MonitorStatus::Active,
		health: MonitorHealth::Unknown,
		schedule,
		timezone: req.timezone,
		checkin_margin_minutes: req.checkin_margin_minutes,
		max_runtime_minutes: req.max_runtime_minutes,
		ping_key: ping_key.clone(),
		environments: req.environments,
		last_checkin_at: None,
		last_checkin_status: None,
		next_expected_at,
		consecutive_failures: 0,
		total_checkins: 0,
		total_failures: 0,
		created_at: now,
		updated_at: now,
	};

	if let Err(e) = state.crons_repo.create_monitor(&monitor).await {
		tracing::error!(error = %e, "Failed to create monitor");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	info!(monitor_id = %monitor.id, slug = %monitor.slug, "Monitor created");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CronMonitorCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("cron_monitor", monitor.id.to_string())
			.details(serde_json::json!({
				"org_id": monitor.org_id.to_string(),
				"slug": monitor.slug.clone(),
				"name": monitor.name.clone(),
				"schedule": format!("{:?}", monitor.schedule),
			}))
			.build(),
	);

	let ping_url = format!("{}/ping/{}", state.base_url, ping_key);

	(
		StatusCode::CREATED,
		Json(CreateMonitorResponse { monitor, ping_url }),
	)
		.into_response()
}

/// GET /api/crons/monitors/{slug} - Get monitor
#[utoipa::path(
	get,
	path = "/api/crons/monitors/{slug}",
	params(
		("slug" = String, Path, description = "Monitor slug"),
		("org_id" = OrgId, Query, description = "Organization ID"),
	),
	responses(
		(status = 200, description = "Monitor details", body = Monitor),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Monitor not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user), fields(slug = %slug))]
pub async fn get_monitor(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(slug): Path<String>,
	Query(params): Query<GetMonitorParams>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &params.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	match state
		.crons_repo
		.get_monitor_by_slug(params.org_id, &slug)
		.await
	{
		Ok(Some(monitor)) => Json(monitor).into_response(),
		Ok(None) => StatusCode::NOT_FOUND.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get monitor");
			StatusCode::INTERNAL_SERVER_ERROR.into_response()
		}
	}
}

/// DELETE /api/crons/monitors/{slug} - Delete monitor
#[utoipa::path(
	delete,
	path = "/api/crons/monitors/{slug}",
	params(
		("slug" = String, Path, description = "Monitor slug"),
		("org_id" = OrgId, Query, description = "Organization ID"),
	),
	responses(
		(status = 204, description = "Monitor deleted"),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Monitor not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user), fields(slug = %slug))]
pub async fn delete_monitor(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(slug): Path<String>,
	Query(params): Query<GetMonitorParams>,
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

	if let Err(e) = state.crons_repo.delete_monitor(monitor.id).await {
		tracing::error!(error = %e, "Failed to delete monitor");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	info!(monitor_id = %monitor.id, "Monitor deleted");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CronMonitorDeleted)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("cron_monitor", monitor.id.to_string())
			.details(serde_json::json!({
				"org_id": monitor.org_id.to_string(),
				"slug": monitor.slug.clone(),
				"name": monitor.name.clone(),
			}))
			.build(),
	);

	StatusCode::NO_CONTENT.into_response()
}

/// PATCH /api/crons/monitors/{slug} - Update monitor
#[utoipa::path(
	patch,
	path = "/api/crons/monitors/{slug}",
	params(
		("slug" = String, Path, description = "Monitor slug"),
	),
	request_body = UpdateMonitorRequest,
	responses(
		(status = 200, description = "Monitor updated", body = Monitor),
		(status = 400, description = "Invalid request"),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Monitor not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user, req), fields(slug = %slug))]
pub async fn update_monitor(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(slug): Path<String>,
	Json(req): Json<UpdateMonitorRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &req.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	let mut monitor = match state
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

	// Track what changed for audit log
	let mut changes = serde_json::Map::new();

	// Apply updates to monitor
	if let Some(name) = req.name {
		changes.insert(
			"name".to_string(),
			serde_json::json!({"old": &monitor.name, "new": &name}),
		);
		monitor.name = name;
	}
	if let Some(description) = req.description {
		changes.insert(
			"description".to_string(),
			serde_json::json!({"old": &monitor.description, "new": &description}),
		);
		monitor.description = Some(description);
	}
	if let Some(schedule_req) = req.schedule {
		let schedule: MonitorSchedule = schedule_req.into();
		changes.insert(
			"schedule".to_string(),
			serde_json::json!({"old": serde_json::to_value(&monitor.schedule).unwrap_or_default(), "new": serde_json::to_value(&schedule).unwrap_or_default()}),
		);
		monitor.schedule = schedule;
		// Recalculate next_expected_at when schedule changes
		monitor.next_expected_at =
			calculate_next_expected(&monitor.schedule, &monitor.timezone, Utc::now()).ok();
	}
	if let Some(timezone) = req.timezone {
		changes.insert(
			"timezone".to_string(),
			serde_json::json!({"old": &monitor.timezone, "new": &timezone}),
		);
		monitor.timezone = timezone;
		// Recalculate next_expected_at when timezone changes
		monitor.next_expected_at =
			calculate_next_expected(&monitor.schedule, &monitor.timezone, Utc::now()).ok();
	}
	if let Some(margin) = req.checkin_margin_minutes {
		changes.insert(
			"checkin_margin_minutes".to_string(),
			serde_json::json!({"old": monitor.checkin_margin_minutes, "new": margin}),
		);
		monitor.checkin_margin_minutes = margin;
	}
	if let Some(max_runtime) = req.max_runtime_minutes {
		changes.insert(
			"max_runtime_minutes".to_string(),
			serde_json::json!({"old": monitor.max_runtime_minutes, "new": max_runtime}),
		);
		monitor.max_runtime_minutes = max_runtime;
	}
	if let Some(environments) = req.environments {
		changes.insert(
			"environments".to_string(),
			serde_json::json!({"old": &monitor.environments, "new": &environments}),
		);
		monitor.environments = environments;
	}

	if let Err(e) = state.crons_repo.update_monitor(&monitor).await {
		tracing::error!(error = %e, "Failed to update monitor");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	info!(monitor_id = %monitor.id, "Monitor updated");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CronMonitorUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("cron_monitor", monitor.id.to_string())
			.details(serde_json::json!({
				"org_id": monitor.org_id.to_string(),
				"slug": monitor.slug.clone(),
				"changes": changes,
			}))
			.build(),
	);

	Json(monitor).into_response()
}

/// POST /api/crons/monitors/{slug}/pause - Pause monitoring
#[utoipa::path(
	post,
	path = "/api/crons/monitors/{slug}/pause",
	params(
		("slug" = String, Path, description = "Monitor slug"),
		("org_id" = OrgId, Query, description = "Organization ID"),
	),
	responses(
		(status = 200, description = "Monitor paused", body = Monitor),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Monitor not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user), fields(slug = %slug))]
pub async fn pause_monitor(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(slug): Path<String>,
	Query(params): Query<GetMonitorParams>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &params.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	let mut monitor = match state
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

	// Set status to paused
	let old_status = monitor.status;
	monitor.status = MonitorStatus::Paused;

	if let Err(e) = state.crons_repo.update_monitor(&monitor).await {
		tracing::error!(error = %e, "Failed to pause monitor");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	info!(monitor_id = %monitor.id, "Monitor paused");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CronMonitorPaused)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("cron_monitor", monitor.id.to_string())
			.details(serde_json::json!({
				"org_id": monitor.org_id.to_string(),
				"slug": monitor.slug.clone(),
				"previous_status": old_status.to_string(),
			}))
			.build(),
	);

	Json(monitor).into_response()
}

/// POST /api/crons/monitors/{slug}/resume - Resume monitoring
#[utoipa::path(
	post,
	path = "/api/crons/monitors/{slug}/resume",
	params(
		("slug" = String, Path, description = "Monitor slug"),
		("org_id" = OrgId, Query, description = "Organization ID"),
	),
	responses(
		(status = 200, description = "Monitor resumed", body = Monitor),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Monitor not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user), fields(slug = %slug))]
pub async fn resume_monitor(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(slug): Path<String>,
	Query(params): Query<GetMonitorParams>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	if let Err(resp) =
		verify_org_membership(&state, &params.org_id, &current_user.user.id, &locale).await
	{
		return resp.into_response();
	}

	let mut monitor = match state
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

	// Set status to active and recalculate next expected time
	let old_status = monitor.status;
	monitor.status = MonitorStatus::Active;
	monitor.next_expected_at =
		calculate_next_expected(&monitor.schedule, &monitor.timezone, Utc::now()).ok();

	if let Err(e) = state.crons_repo.update_monitor(&monitor).await {
		tracing::error!(error = %e, "Failed to resume monitor");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	info!(monitor_id = %monitor.id, "Monitor resumed");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CronMonitorResumed)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("cron_monitor", monitor.id.to_string())
			.details(serde_json::json!({
				"org_id": monitor.org_id.to_string(),
				"slug": monitor.slug.clone(),
				"previous_status": old_status.to_string(),
			}))
			.build(),
	);

	Json(monitor).into_response()
}

/// GET /api/crons/monitors/{slug}/checkins - List check-ins
#[utoipa::path(
	get,
	path = "/api/crons/monitors/{slug}/checkins",
	params(
		("slug" = String, Path, description = "Monitor slug"),
		("org_id" = OrgId, Query, description = "Organization ID"),
		("limit" = Option<u32>, Query, description = "Max results (default 50)"),
	),
	responses(
		(status = 200, description = "List of check-ins", body = ListCheckInsResponse),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Monitor not found"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user), fields(slug = %slug))]
pub async fn list_checkins(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(slug): Path<String>,
	Query(params): Query<ListCheckInsParams>,
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

	let limit = params.limit.unwrap_or(50);
	match state.crons_repo.list_checkins(monitor.id, limit).await {
		Ok(checkins) => Json(ListCheckInsResponse { checkins }).into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to list checkins");
			StatusCode::INTERNAL_SERVER_ERROR.into_response()
		}
	}
}
