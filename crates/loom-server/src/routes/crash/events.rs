// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash event HTTP handlers.
//!
//! Implements endpoints for querying crash events.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	Json,
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use loom_crash_core::{CrashEventId, IssueId, ProjectId};
use loom_server_crash::CrashRepository;

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::resolve_user_locale;

use super::common::{
	internal_error, not_found, parse_project_id, verify_org_membership, CrashErrorResponse,
};

// ============================================================================
// Request/Response Types
// ============================================================================

/// Response for a crash event.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CrashEventResponse {
	pub id: String,
	pub issue_id: Option<String>,
	pub person_id: Option<String>,
	pub distinct_id: String,
	pub exception_type: String,
	pub exception_value: String,
	pub stacktrace: StacktraceResponse,
	pub release: Option<String>,
	pub dist: Option<String>,
	pub environment: String,
	pub platform: String,
	pub server_name: Option<String>,
	pub tags: std::collections::HashMap<String, String>,
	pub active_flags: std::collections::HashMap<String, String>,
	pub timestamp: String,
	pub received_at: String,
}

/// Response for a stacktrace.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StacktraceResponse {
	pub frames: Vec<FrameResponse>,
}

/// Response for a stack frame.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FrameResponse {
	pub function: Option<String>,
	pub module: Option<String>,
	pub filename: Option<String>,
	pub abs_path: Option<String>,
	pub lineno: Option<u32>,
	pub colno: Option<u32>,
	pub in_app: bool,
	pub context_line: Option<String>,
	pub pre_context: Vec<String>,
	pub post_context: Vec<String>,
}

impl From<loom_crash_core::CrashEvent> for CrashEventResponse {
	fn from(e: loom_crash_core::CrashEvent) -> Self {
		Self {
			id: e.id.to_string(),
			issue_id: e.issue_id.map(|i| i.to_string()),
			person_id: e.person_id.map(|p| p.0.to_string()),
			distinct_id: e.distinct_id,
			exception_type: e.exception_type,
			exception_value: e.exception_value,
			stacktrace: StacktraceResponse {
				frames: e
					.stacktrace
					.frames
					.into_iter()
					.map(|f| FrameResponse {
						function: f.function,
						module: f.module,
						filename: f.filename,
						abs_path: f.abs_path,
						lineno: f.lineno,
						colno: f.colno,
						in_app: f.in_app,
						context_line: f.context_line,
						pre_context: f.pre_context,
						post_context: f.post_context,
					})
					.collect(),
			},
			release: e.release,
			dist: e.dist,
			environment: e.environment,
			platform: e.platform.to_string(),
			server_name: e.server_name,
			tags: e.tags,
			active_flags: e.active_flags,
			timestamp: e.timestamp.to_rfc3339(),
			received_at: e.received_at.to_rfc3339(),
		}
	}
}

/// Query parameters for listing events.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListEventsParams {
	#[serde(default = "default_events_limit")]
	pub limit: u32,
	#[serde(default)]
	pub offset: u32,
}

fn default_events_limit() -> u32 {
	100
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse an issue ID from a string.
fn parse_issue_id(
	issue_id_str: &str,
) -> Result<IssueId, (StatusCode, Json<CrashErrorResponse>)> {
	issue_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_issue_id".to_string(),
				message: "Invalid issue ID".to_string(),
			}),
		)
	})
}

/// Parse an event ID from a string.
fn parse_event_id(
	event_id_str: &str,
) -> Result<CrashEventId, (StatusCode, Json<CrashErrorResponse>)> {
	event_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_event_id".to_string(),
				message: "Invalid event ID".to_string(),
			}),
		)
	})
}

/// Get a project and verify org membership.
async fn get_project_with_auth(
	state: &AppState,
	project_id: ProjectId,
	user_id: &loom_server_auth::types::UserId,
	locale: &str,
) -> Result<loom_crash_core::CrashProject, (StatusCode, Json<CrashErrorResponse>)> {
	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			internal_error(locale)
		})?
		.ok_or_else(|| not_found("project"))?;

	verify_org_membership(state, &project.org_id, user_id, locale).await?;

	Ok(project)
}

// ============================================================================
// Event Endpoints
// ============================================================================

/// GET /api/crash/projects/{project_id}/events - List crash events for a project
///
/// Returns all crash events for a project, paginated.
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/events",
	params(
		("project_id" = String, Path, description = "Project ID"),
		ListEventsParams,
	),
	responses(
		(status = 200, description = "List of crash events", body = Vec<CrashEventResponse>),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn list_events(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
	Query(params): Query<ListEventsParams>,
) -> Result<Json<Vec<CrashEventResponse>>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	// Limit max limit to 1000
	let limit = params.limit.min(1000);

	let events = state
		.crash_repo
		.list_events_for_project(project_id, limit, params.offset)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list events");
			internal_error(&locale)
		})?;

	info!(project_id = %project_id, event_count = %events.len(), "Project events retrieved");

	Ok(Json(
		events.into_iter().map(CrashEventResponse::from).collect(),
	))
}

/// GET /api/crash/projects/{project_id}/events/{event_id} - Get a single crash event
///
/// Returns detailed information about a specific crash event.
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/events/{event_id}",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("event_id" = String, Path, description = "Event ID"),
	),
	responses(
		(status = 200, description = "Crash event detail", body = CrashEventResponse),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Event or project not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn get_event(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, event_id_str)): Path<(String, String)>,
) -> Result<Json<CrashEventResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let event_id = parse_event_id(&event_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	let event = state
		.crash_repo
		.get_event_by_id(event_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get event");
			internal_error(&locale)
		})?
		.ok_or_else(|| not_found("event"))?;

	// Verify event belongs to the specified project
	if event.project_id != project_id {
		return Err(not_found("event"));
	}

	info!(event_id = %event.id, "Crash event retrieved");

	Ok(Json(CrashEventResponse::from(event)))
}

/// GET /api/crash/projects/{project_id}/issues/{issue_id}/events - List events for an issue
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/issues/{issue_id}/events",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("issue_id" = String, Path, description = "Issue ID"),
	),
	responses(
		(status = 200, description = "List of crash events", body = Vec<CrashEventResponse>),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Issue not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn list_issue_events(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, issue_id_str)): Path<(String, String)>,
) -> Result<Json<Vec<CrashEventResponse>>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let issue_id = parse_issue_id(&issue_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	// Verify issue exists and belongs to project
	let issue = state
		.crash_repo
		.get_issue_by_id(issue_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get issue");
			internal_error(&locale)
		})?
		.ok_or_else(|| not_found("issue"))?;

	if issue.project_id != project_id {
		return Err(not_found("issue"));
	}

	let events = state
		.crash_repo
		.list_events_for_issue(issue_id, 100)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list events");
			internal_error(&locale)
		})?;

	info!(issue_id = %issue.id, event_count = %events.len(), "Issue events retrieved");

	Ok(Json(
		events.into_iter().map(CrashEventResponse::from).collect(),
	))
}
