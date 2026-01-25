// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Cron monitoring HTTP handlers.
//!
//! Implements ping endpoints for simple shell script monitoring and
//! API endpoints for monitor management, including SSE streaming for real-time updates.

pub mod checkins;
pub mod common;
pub mod monitors;
pub mod ping;
pub mod stats;
pub mod stream;

// Re-export all handlers and their utoipa path types
pub use checkins::{
	__path_create_checkin, __path_get_checkin, __path_update_checkin, create_checkin, get_checkin,
	update_checkin,
};
pub use monitors::{
	__path_create_monitor, __path_delete_monitor, __path_get_monitor, __path_list_checkins,
	__path_list_monitors, __path_pause_monitor, __path_resume_monitor, __path_update_monitor,
	create_monitor, delete_monitor, get_monitor, list_checkins, list_monitors, pause_monitor,
	resume_monitor, update_monitor,
};
pub use ping::{
	__path_ping_fail, __path_ping_start, __path_ping_success, __path_ping_with_body, ping_fail,
	ping_start, ping_success, ping_with_body,
};
pub use stats::{
	__path_get_monitor_stats, __path_get_stats_overview, get_monitor_stats, get_stats_overview,
};
pub use stream::{__path_stream_crons, stream_crons};

// Re-export API types for external use
pub use common::{
	CreateCheckInRequest, CreateCheckInResponse, CreateMonitorRequest, CreateMonitorResponse,
	CronsErrorResponse, GetMonitorParams, GetMonitorStatsParams, GetStatsOverviewParams,
	ListCheckInsParams, ListCheckInsResponse, ListMonitorsParams, ListMonitorsResponse,
	MonitorScheduleRequest, MonitorStatsResponse, MonitorSummary, PingParams, PingStartResponse,
	StatsOverviewResponse, StreamCronsParams, UpdateCheckInRequest, UpdateMonitorRequest,
};
