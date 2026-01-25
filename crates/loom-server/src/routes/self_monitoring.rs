// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Self-monitoring API routes.
//!
//! Provides endpoints for retrieving self-monitoring configuration,
//! particularly the crash SDK configuration for loom-web.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::self_monitoring::{
	get_cli_crash_config, get_web_analytics_config, get_web_crash_config, LOOM_CLI_PROJECT_ID,
	LOOM_INTERNAL_ORG_ID, LOOM_SERVER_PROJECT_ID, LOOM_WEB_PROJECT_ID,
};

/// Response for the web crash SDK configuration endpoint.
#[derive(Debug, Serialize)]
pub struct WebCrashConfigResponse {
	pub project_id: String,
	pub api_key: String,
	pub release: String,
	pub environment: String,
}

/// Response when self-monitoring is not available.
#[derive(Debug, Serialize)]
pub struct SelfMonitoringUnavailable {
	pub message: String,
}

/// GET /api/self-monitoring/web-config
///
/// Returns the crash SDK configuration for loom-web.
/// This endpoint is public (no auth required) since it's needed during
/// frontend initialization before the user is authenticated.
pub async fn get_web_config() -> impl IntoResponse {
	match get_web_crash_config() {
		Some(config) => (
			StatusCode::OK,
			Json(WebCrashConfigResponse {
				project_id: config.project_id,
				api_key: config.api_key,
				release: config.release,
				environment: config.environment,
			}),
		)
			.into_response(),
		None => (
			StatusCode::SERVICE_UNAVAILABLE,
			Json(SelfMonitoringUnavailable {
				message: "Self-monitoring not initialized".to_string(),
			}),
		)
			.into_response(),
	}
}

/// Response for the CLI crash SDK configuration endpoint.
#[derive(Debug, Serialize)]
pub struct CliCrashConfigResponse {
	pub project_id: String,
	pub api_key: String,
	pub release: String,
	pub environment: String,
}

/// GET /api/self-monitoring/cli-config
///
/// Returns the crash SDK configuration for loom-cli.
/// This endpoint is public (no auth required) since it's needed during
/// CLI initialization before the user is authenticated.
pub async fn get_cli_config() -> impl IntoResponse {
	match get_cli_crash_config() {
		Some(config) => (
			StatusCode::OK,
			Json(CliCrashConfigResponse {
				project_id: config.project_id,
				api_key: config.api_key,
				release: config.release,
				environment: config.environment,
			}),
		)
			.into_response(),
		None => (
			StatusCode::SERVICE_UNAVAILABLE,
			Json(SelfMonitoringUnavailable {
				message: "Self-monitoring not initialized".to_string(),
			}),
		)
			.into_response(),
	}
}

/// Response for the project IDs endpoint.
#[derive(Debug, Serialize)]
pub struct InternalProjectIds {
	pub org_id: String,
	pub server_project_id: String,
	pub web_project_id: String,
	pub cli_project_id: String,
}

/// GET /api/self-monitoring/projects
///
/// Returns the well-known internal project IDs.
/// This is useful for debugging and for the admin UI.
pub async fn get_internal_projects() -> Json<InternalProjectIds> {
	Json(InternalProjectIds {
		org_id: LOOM_INTERNAL_ORG_ID.to_string(),
		server_project_id: LOOM_SERVER_PROJECT_ID.to_string(),
		web_project_id: LOOM_WEB_PROJECT_ID.to_string(),
		cli_project_id: LOOM_CLI_PROJECT_ID.to_string(),
	})
}

/// Response for the web analytics SDK configuration endpoint.
#[derive(Debug, Serialize)]
pub struct WebAnalyticsConfigResponse {
	pub api_key: String,
	pub release: String,
	pub environment: String,
}

/// GET /api/self-monitoring/analytics-config
///
/// Returns the analytics SDK configuration for loom-web.
/// This endpoint is public (no auth required) since it's needed during
/// frontend initialization before the user is authenticated.
pub async fn get_analytics_config() -> impl IntoResponse {
	match get_web_analytics_config() {
		Some(config) => (
			StatusCode::OK,
			Json(WebAnalyticsConfigResponse {
				api_key: config.api_key,
				release: config.release,
				environment: config.environment,
			}),
		)
			.into_response(),
		None => (
			StatusCode::SERVICE_UNAVAILABLE,
			Json(SelfMonitoringUnavailable {
				message: "Self-monitoring not initialized".to_string(),
			}),
		)
			.into_response(),
	}
}
