// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for crons routes.

use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use loom_crons_core::{
	CheckIn, CheckInId, CheckInStatus, MonitorHealth, MonitorId, MonitorSchedule, MonitorStatus,
	OrgId, StatsPeriod,
};
use loom_server_auth::types::OrgId as AuthOrgId;

use crate::api::AppState;
use crate::i18n::t;

/// Error response for crons endpoints.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CronsErrorResponse {
	pub error: String,
	pub message: String,
}

/// Verify that the current user is a member of the specified organization.
pub async fn verify_org_membership(
	state: &AppState,
	org_id: &OrgId,
	user_id: &loom_server_auth::types::UserId,
	locale: &str,
) -> Result<(), (StatusCode, Json<CronsErrorResponse>)> {
	// Convert OrgId to AuthOrgId
	let auth_org_id = AuthOrgId::from(org_id.0);

	match state.org_repo.get_membership(&auth_org_id, user_id).await {
		Ok(Some(_)) => Ok(()),
		Ok(None) => Err((
			StatusCode::FORBIDDEN,
			Json(CronsErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.not_a_member").to_string(),
			}),
		)),
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CronsErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			))
		}
	}
}

/// Query parameters for ping endpoints.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PingParams {
	pub exit_code: Option<i32>,
}

/// Response for ping/start endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PingStartResponse {
	pub checkin_id: CheckInId,
}

/// Request to create a new monitor.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateMonitorRequest {
	pub org_id: OrgId,
	pub slug: String,
	pub name: String,
	#[serde(default)]
	pub description: Option<String>,
	pub schedule: MonitorScheduleRequest,
	#[serde(default = "default_timezone")]
	pub timezone: String,
	#[serde(default = "default_margin")]
	pub checkin_margin_minutes: u32,
	#[serde(default)]
	pub max_runtime_minutes: Option<u32>,
	#[serde(default)]
	pub environments: Vec<String>,
}

pub fn default_timezone() -> String {
	"UTC".to_string()
}

pub fn default_margin() -> u32 {
	5
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MonitorScheduleRequest {
	Cron { expression: String },
	Interval { minutes: u32 },
}

impl From<MonitorScheduleRequest> for MonitorSchedule {
	fn from(req: MonitorScheduleRequest) -> Self {
		match req {
			MonitorScheduleRequest::Cron { expression } => MonitorSchedule::Cron { expression },
			MonitorScheduleRequest::Interval { minutes } => MonitorSchedule::Interval { minutes },
		}
	}
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateMonitorResponse {
	pub monitor: loom_crons_core::Monitor,
	pub ping_url: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ListMonitorsParams {
	pub org_id: OrgId,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListMonitorsResponse {
	pub monitors: Vec<MonitorSummary>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MonitorSummary {
	pub id: MonitorId,
	pub slug: String,
	pub name: String,
	pub status: MonitorStatus,
	pub health: MonitorHealth,
	pub last_checkin_at: Option<chrono::DateTime<chrono::Utc>>,
	pub next_expected_at: Option<chrono::DateTime<chrono::Utc>>,
	pub consecutive_failures: u32,
}

impl From<loom_crons_core::Monitor> for MonitorSummary {
	fn from(m: loom_crons_core::Monitor) -> Self {
		Self {
			id: m.id,
			slug: m.slug,
			name: m.name,
			status: m.status,
			health: m.health,
			last_checkin_at: m.last_checkin_at,
			next_expected_at: m.next_expected_at,
			consecutive_failures: m.consecutive_failures,
		}
	}
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct GetMonitorParams {
	pub org_id: OrgId,
}

/// Request body for updating a monitor.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateMonitorRequest {
	pub org_id: OrgId,
	#[serde(default)]
	pub name: Option<String>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub schedule: Option<MonitorScheduleRequest>,
	#[serde(default)]
	pub timezone: Option<String>,
	#[serde(default)]
	pub checkin_margin_minutes: Option<u32>,
	#[serde(default)]
	pub max_runtime_minutes: Option<Option<u32>>,
	#[serde(default)]
	pub environments: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ListCheckInsParams {
	pub org_id: OrgId,
	pub limit: Option<u32>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListCheckInsResponse {
	pub checkins: Vec<CheckIn>,
}

/// Request to create a check-in via SDK.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateCheckInRequest {
	pub org_id: OrgId,
	pub status: CheckInStatus,
	#[serde(default)]
	pub started_at: Option<chrono::DateTime<chrono::Utc>>,
	#[serde(default)]
	pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
	#[serde(default)]
	pub duration_ms: Option<u64>,
	#[serde(default)]
	pub environment: Option<String>,
	#[serde(default)]
	pub release: Option<String>,
	#[serde(default)]
	pub exit_code: Option<i32>,
	#[serde(default)]
	pub output: Option<String>,
	#[serde(default)]
	pub crash_event_id: Option<String>,
}

/// Response for check-in creation.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateCheckInResponse {
	pub id: CheckInId,
	pub status: CheckInStatus,
}

/// Request to update a check-in.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateCheckInRequest {
	pub status: CheckInStatus,
	#[serde(default)]
	pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
	#[serde(default)]
	pub duration_ms: Option<u64>,
	#[serde(default)]
	pub exit_code: Option<i32>,
	#[serde(default)]
	pub output: Option<String>,
	#[serde(default)]
	pub crash_event_id: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct StreamCronsParams {
	pub org_id: OrgId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct GetMonitorStatsParams {
	pub org_id: OrgId,
	#[serde(default = "default_stats_period")]
	pub period: StatsPeriod,
}

pub fn default_stats_period() -> StatsPeriod {
	StatsPeriod::Week
}

/// Response for monitor stats.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MonitorStatsResponse {
	pub stats: loom_crons_core::MonitorStats,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct GetStatsOverviewParams {
	pub org_id: OrgId,
}

/// Overview stats for all monitors in an organization.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StatsOverviewResponse {
	pub total_monitors: u64,
	pub active_monitors: u64,
	pub paused_monitors: u64,
	pub healthy_monitors: u64,
	pub failing_monitors: u64,
	pub missed_monitors: u64,
	pub total_checkins_24h: u64,
	pub total_failures_24h: u64,
	pub overall_uptime_percentage: f64,
}
