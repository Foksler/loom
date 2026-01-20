// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash analytics HTTP handlers.
//!
//! Implements endpoints for crash event capture, issue management,
//! and project configuration.

use std::convert::Infallible;

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::sse::{Event, Sse},
	Json,
};
use chrono::Utc;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tracing::{info, instrument};

use loom_crash_core::{
	compute_fingerprint, fingerprint, Breadcrumb, CrashEvent, CrashEventId, CrashProject, Frame,
	Issue, IssueId, IssueLevel, IssueMetadata, IssuePriority, IssueStatus, OrgId, PersonId, Platform,
	ProjectId, Release, ReleaseId, Stacktrace,
};
use loom_server_auth::middleware::CurrentUser;
use loom_server_auth::types::OrgId as AuthOrgId;
use loom_server_crash::{CrashRepository, CrashStreamEvent};

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::{resolve_user_locale, t};

/// Error response for crash endpoints.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CrashErrorResponse {
	pub error: String,
	pub message: String,
}

/// Verify that the current user is a member of the specified organization.
async fn verify_org_membership(
	state: &AppState,
	org_id: &OrgId,
	user_id: &loom_server_auth::types::UserId,
	locale: &str,
) -> Result<(), (StatusCode, Json<CrashErrorResponse>)> {
	let auth_org_id = AuthOrgId::from(org_id.0);

	match state.org_repo.get_membership(&auth_org_id, user_id).await {
		Ok(Some(_)) => Ok(()),
		Ok(None) => Err((
			StatusCode::FORBIDDEN,
			Json(CrashErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.not_a_member").to_string(),
			}),
		)),
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			))
		}
	}
}

// ============================================================================
// Capture Endpoint (SDK ingestion)
// ============================================================================

/// Request body for crash capture endpoint.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CaptureRequest {
	pub project_id: String,
	pub exception_type: String,
	pub exception_value: String,
	pub stacktrace: CaptureStacktrace,
	#[serde(default)]
	pub environment: Option<String>,
	pub platform: Option<String>,
	pub release: Option<String>,
	pub dist: Option<String>,
	pub distinct_id: Option<String>,
	pub person_id: Option<String>,
	pub server_name: Option<String>,
	#[serde(default)]
	pub tags: std::collections::HashMap<String, String>,
	#[serde(default)]
	pub extra: serde_json::Value,
	#[serde(default)]
	pub active_flags: std::collections::HashMap<String, String>,
	#[serde(default)]
	pub breadcrumbs: Vec<CaptureBreadcrumb>,
	pub timestamp: Option<String>,
}

/// Stacktrace in capture request.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CaptureStacktrace {
	pub frames: Vec<CaptureFrame>,
}

/// Frame in capture request.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CaptureFrame {
	pub function: Option<String>,
	pub module: Option<String>,
	pub filename: Option<String>,
	pub abs_path: Option<String>,
	pub lineno: Option<u32>,
	pub colno: Option<u32>,
	#[serde(default)]
	pub in_app: bool,
}

/// Breadcrumb in capture request.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CaptureBreadcrumb {
	pub timestamp: Option<String>,
	pub category: Option<String>,
	pub message: Option<String>,
	pub level: Option<String>,
	#[serde(default)]
	pub data: serde_json::Value,
}

/// Response for crash capture endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CaptureResponse {
	pub event_id: String,
	pub issue_id: String,
	pub short_id: String,
	pub is_new_issue: bool,
	pub is_regression: bool,
}

/// POST /api/crash/capture - Capture a crash event
#[utoipa::path(
	post,
	path = "/api/crash/capture",
	request_body = CaptureRequest,
	responses(
		(status = 200, description = "Crash captured", body = CaptureResponse),
		(status = 400, description = "Invalid request", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
		(status = 500, description = "Internal error", body = CrashErrorResponse),
	),
	tag = "crash"
)]
#[instrument(skip(state, current_user, body), fields(project_id = %body.project_id))]
pub async fn capture_crash(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Json(body): Json<CaptureRequest>,
) -> Result<Json<CaptureResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Parse project ID
	let project_id: ProjectId = body.project_id.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})?;

	// Get project
	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
		})?;

	// Verify org membership
	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	// Convert capture request to CrashEvent
	let platform = body
		.platform
		.as_deref()
		.unwrap_or("javascript")
		.parse()
		.unwrap_or(Platform::JavaScript);

	let stacktrace = Stacktrace {
		frames: body
			.stacktrace
			.frames
			.into_iter()
			.map(|f| Frame {
				function: f.function,
				module: f.module,
				filename: f.filename,
				abs_path: f.abs_path,
				lineno: f.lineno,
				colno: f.colno,
				in_app: f.in_app,
				..Default::default()
			})
			.collect(),
	};

	let breadcrumbs: Vec<Breadcrumb> = body
		.breadcrumbs
		.into_iter()
		.map(|b| Breadcrumb {
			timestamp: b
				.timestamp
				.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
				.map(|dt| dt.with_timezone(&Utc))
				.unwrap_or_else(Utc::now),
			category: b.category.unwrap_or_default(),
			message: b.message,
			level: b
				.level
				.and_then(|l| l.parse().ok())
				.unwrap_or(loom_crash_core::BreadcrumbLevel::Info),
			data: b.data,
		})
		.collect();

	let timestamp = body
		.timestamp
		.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
		.map(|dt| dt.with_timezone(&Utc))
		.unwrap_or_else(Utc::now);

	let person_id = body.person_id.and_then(|s| s.parse().ok()).map(PersonId);

	let mut event = CrashEvent {
		id: CrashEventId::new(),
		org_id: project.org_id,
		project_id,
		issue_id: None,
		person_id,
		distinct_id: body
			.distinct_id
			.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
		exception_type: body.exception_type,
		exception_value: body.exception_value,
		stacktrace,
		raw_stacktrace: None,
		release: body.release,
		dist: body.dist,
		environment: body.environment.unwrap_or_else(|| "production".to_string()),
		platform,
		runtime: None,
		server_name: body.server_name,
		tags: body.tags,
		extra: body.extra,
		user_context: None,
		device_context: None,
		browser_context: None,
		os_context: None,
		active_flags: body.active_flags,
		request: None,
		breadcrumbs,
		timestamp,
		received_at: Utc::now(),
	};

	// Compute fingerprint
	let fingerprint = compute_fingerprint(&event);

	// Find or create issue
	let (issue, is_new_issue, is_regression) = match state
		.crash_repo
		.get_issue_by_fingerprint(project_id, &fingerprint)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to find issue by fingerprint");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})? {
		Some(mut existing_issue) => {
			let is_regression = existing_issue.status == IssueStatus::Resolved;

			// Update issue
			existing_issue.event_count += 1;
			existing_issue.last_seen = event.timestamp;

			if is_regression {
				existing_issue.status = IssueStatus::Regressed;
				existing_issue.times_regressed += 1;
				existing_issue.last_regressed_at = Some(Utc::now());
				existing_issue.regressed_in_release = event.release.clone();
			}

			state
				.crash_repo
				.update_issue(&existing_issue)
				.await
				.map_err(|e| {
					tracing::error!(error = %e, "Failed to update issue");
					(
						StatusCode::INTERNAL_SERVER_ERROR,
						Json(CrashErrorResponse {
							error: "internal_error".to_string(),
							message: t(&locale, "server.api.error.internal").to_string(),
						}),
					)
				})?;

			// Track person if present
			if let Some(pid) = event.person_id {
				let _ = state
					.crash_repo
					.add_issue_person(existing_issue.id, pid)
					.await;
			}

			// Broadcast regression if needed
			if is_regression {
				state
					.crash_broadcaster
					.broadcast_regression(project_id, &existing_issue)
					.await;
			}

			(existing_issue, false, is_regression)
		}
		None => {
			// Create new issue
			let short_id = state
				.crash_repo
				.get_next_short_id(project_id)
				.await
				.map_err(|e| {
					tracing::error!(error = %e, "Failed to get next short ID");
					(
						StatusCode::INTERNAL_SERVER_ERROR,
						Json(CrashErrorResponse {
							error: "internal_error".to_string(),
							message: t(&locale, "server.api.error.internal").to_string(),
						}),
					)
				})?;

			let culprit = fingerprint::find_culprit(&event);
			let title = format!(
				"{}: {}",
				event.exception_type,
				fingerprint::truncate(&event.exception_value, 100)
			);

			let issue = Issue {
				id: IssueId::new(),
				org_id: project.org_id,
				project_id,
				short_id,
				fingerprint,
				title,
				culprit,
				metadata: IssueMetadata {
					exception_type: event.exception_type.clone(),
					exception_value: event.exception_value.clone(),
					filename: event
						.stacktrace
						.frames
						.iter()
						.find(|f| f.in_app)
						.and_then(|f| f.filename.clone()),
					function: event
						.stacktrace
						.frames
						.iter()
						.find(|f| f.in_app)
						.and_then(|f| f.function.clone()),
				},
				status: IssueStatus::Unresolved,
				level: IssueLevel::Error,
				priority: IssuePriority::Medium,
				event_count: 1,
				user_count: if event.person_id.is_some() { 1 } else { 0 },
				first_seen: event.timestamp,
				last_seen: event.timestamp,
				resolved_at: None,
				resolved_by: None,
				resolved_in_release: None,
				times_regressed: 0,
				last_regressed_at: None,
				regressed_in_release: None,
				assigned_to: None,
				created_at: Utc::now(),
				updated_at: Utc::now(),
			};

			state.crash_repo.create_issue(&issue).await.map_err(|e| {
				tracing::error!(error = %e, "Failed to create issue");
				(
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(CrashErrorResponse {
						error: "internal_error".to_string(),
						message: t(&locale, "server.api.error.internal").to_string(),
					}),
				)
			})?;

			// Track person if present
			if let Some(pid) = event.person_id {
				let _ = state.crash_repo.add_issue_person(issue.id, pid).await;
			}

			(issue, true, false)
		}
	};

	// Set issue_id on event and save
	event.issue_id = Some(issue.id);
	state.crash_repo.create_event(&event).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to create event");
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(CrashErrorResponse {
				error: "internal_error".to_string(),
				message: t(&locale, "server.api.error.internal").to_string(),
			}),
		)
	})?;

	// Track release if present
	if let Some(ref release_version) = event.release {
		// Get or create the release
		if let Err(e) = state
			.crash_repo
			.get_or_create_release(project_id, project.org_id, release_version)
			.await
		{
			tracing::warn!(error = %e, release = %release_version, "Failed to get/create release");
		}

		// Update release crash count
		if let Err(e) = state
			.crash_repo
			.increment_release_crash_count(project_id, release_version, is_new_issue, is_regression)
			.await
		{
			tracing::warn!(error = %e, release = %release_version, "Failed to increment release crash count");
		}
	}

	// Broadcast new crash event
	state
		.crash_broadcaster
		.broadcast_new_crash(project_id, event.id, &issue, is_new_issue)
		.await;

	info!(
		event_id = %event.id,
		issue_id = %issue.id,
		short_id = %issue.short_id,
		is_new_issue,
		is_regression,
		"Crash event captured"
	);

	Ok(Json(CaptureResponse {
		event_id: event.id.to_string(),
		issue_id: issue.id.to_string(),
		short_id: issue.short_id,
		is_new_issue,
		is_regression,
	}))
}

// ============================================================================
// Batch Capture Endpoint
// ============================================================================

/// Request body for batch crash capture endpoint.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BatchCaptureRequest {
	/// List of crash events to capture (max 100 per request)
	pub events: Vec<CaptureRequest>,
}

/// Result for a single event in a batch capture.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BatchCaptureEventResult {
	/// Index of the event in the request array
	pub index: usize,
	/// Whether the event was successfully captured
	pub success: bool,
	/// Event ID if successful
	pub event_id: Option<String>,
	/// Issue ID if successful
	pub issue_id: Option<String>,
	/// Short ID if successful
	pub short_id: Option<String>,
	/// Whether this created a new issue
	pub is_new_issue: Option<bool>,
	/// Whether this is a regression
	pub is_regression: Option<bool>,
	/// Error message if failed
	pub error: Option<String>,
}

/// Response for batch crash capture endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BatchCaptureResponse {
	/// Total number of events in the request
	pub total: usize,
	/// Number of successfully captured events
	pub success_count: usize,
	/// Number of failed events
	pub error_count: usize,
	/// Results for each event in the batch
	pub results: Vec<BatchCaptureEventResult>,
}

/// POST /api/crash/batch - Capture multiple crash events in a single request
#[utoipa::path(
	post,
	path = "/api/crash/batch",
	request_body = BatchCaptureRequest,
	responses(
		(status = 200, description = "Batch capture results", body = BatchCaptureResponse),
		(status = 400, description = "Invalid request", body = CrashErrorResponse),
		(status = 500, description = "Internal error", body = CrashErrorResponse),
	),
	tag = "crash"
)]
#[instrument(skip(state, current_user, body), fields(event_count = body.events.len()))]
pub async fn batch_capture_crash(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Json(body): Json<BatchCaptureRequest>,
) -> Result<Json<BatchCaptureResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Validate batch size
	const MAX_BATCH_SIZE: usize = 100;
	if body.events.len() > MAX_BATCH_SIZE {
		return Err((
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "batch_too_large".to_string(),
				message: format!("Batch size exceeds maximum of {} events", MAX_BATCH_SIZE),
			}),
		));
	}

	if body.events.is_empty() {
		return Ok(Json(BatchCaptureResponse {
			total: 0,
			success_count: 0,
			error_count: 0,
			results: vec![],
		}));
	}

	let mut results = Vec::with_capacity(body.events.len());
	let mut success_count = 0;
	let mut error_count = 0;

	// Process each event
	for (index, event_request) in body.events.into_iter().enumerate() {
		let result =
			process_single_capture(&state, &current_user, &locale, event_request, index).await;

		match result {
			Ok(capture_result) => {
				success_count += 1;
				results.push(BatchCaptureEventResult {
					index,
					success: true,
					event_id: Some(capture_result.event_id),
					issue_id: Some(capture_result.issue_id),
					short_id: Some(capture_result.short_id),
					is_new_issue: Some(capture_result.is_new_issue),
					is_regression: Some(capture_result.is_regression),
					error: None,
				});
			}
			Err(error_msg) => {
				error_count += 1;
				results.push(BatchCaptureEventResult {
					index,
					success: false,
					event_id: None,
					issue_id: None,
					short_id: None,
					is_new_issue: None,
					is_regression: None,
					error: Some(error_msg),
				});
			}
		}
	}

	info!(
		total = results.len(),
		success_count,
		error_count,
		"Batch crash capture completed"
	);

	Ok(Json(BatchCaptureResponse {
		total: results.len(),
		success_count,
		error_count,
		results,
	}))
}

/// Internal helper to process a single capture request within a batch.
/// Returns Ok(CaptureResponse) on success, Err(String) with error message on failure.
async fn process_single_capture(
	state: &AppState,
	current_user: &CurrentUser,
	locale: &str,
	body: CaptureRequest,
	_index: usize,
) -> Result<CaptureResponse, String> {
	// Parse project ID
	let project_id: ProjectId = body
		.project_id
		.parse()
		.map_err(|_| "Invalid project ID".to_string())?;

	// Get project
	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| format!("Failed to get project: {}", e))?
		.ok_or_else(|| "Project not found".to_string())?;

	// Verify org membership
	let auth_org_id = AuthOrgId::from(project.org_id.0);
	match state
		.org_repo
		.get_membership(&auth_org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => return Err(t(locale, "server.api.org.not_a_member").to_string()),
		Err(e) => return Err(format!("Failed to check org membership: {}", e)),
	}

	// Convert capture request to CrashEvent
	let platform = body
		.platform
		.as_deref()
		.unwrap_or("javascript")
		.parse()
		.unwrap_or(Platform::JavaScript);

	let stacktrace = Stacktrace {
		frames: body
			.stacktrace
			.frames
			.into_iter()
			.map(|f| Frame {
				function: f.function,
				module: f.module,
				filename: f.filename,
				abs_path: f.abs_path,
				lineno: f.lineno,
				colno: f.colno,
				in_app: f.in_app,
				..Default::default()
			})
			.collect(),
	};

	let breadcrumbs: Vec<Breadcrumb> = body
		.breadcrumbs
		.into_iter()
		.map(|b| Breadcrumb {
			timestamp: b
				.timestamp
				.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
				.map(|dt| dt.with_timezone(&Utc))
				.unwrap_or_else(Utc::now),
			category: b.category.unwrap_or_default(),
			message: b.message,
			level: b
				.level
				.and_then(|l| l.parse().ok())
				.unwrap_or(loom_crash_core::BreadcrumbLevel::Info),
			data: b.data,
		})
		.collect();

	let timestamp = body
		.timestamp
		.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
		.map(|dt| dt.with_timezone(&Utc))
		.unwrap_or_else(Utc::now);

	let person_id = body.person_id.and_then(|s| s.parse().ok()).map(PersonId);

	let mut event = CrashEvent {
		id: CrashEventId::new(),
		org_id: project.org_id,
		project_id,
		issue_id: None,
		person_id,
		distinct_id: body
			.distinct_id
			.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
		exception_type: body.exception_type,
		exception_value: body.exception_value,
		stacktrace,
		raw_stacktrace: None,
		release: body.release,
		dist: body.dist,
		environment: body.environment.unwrap_or_else(|| "production".to_string()),
		platform,
		runtime: None,
		server_name: body.server_name,
		tags: body.tags,
		extra: body.extra,
		user_context: None,
		device_context: None,
		browser_context: None,
		os_context: None,
		active_flags: body.active_flags,
		request: None,
		breadcrumbs,
		timestamp,
		received_at: Utc::now(),
	};

	// Compute fingerprint
	let fingerprint = compute_fingerprint(&event);

	// Find or create issue
	let (issue, is_new_issue, is_regression) = match state
		.crash_repo
		.get_issue_by_fingerprint(project_id, &fingerprint)
		.await
		.map_err(|e| format!("Failed to find issue: {}", e))?
	{
		Some(mut existing_issue) => {
			let is_regression = existing_issue.status == IssueStatus::Resolved;

			// Update issue
			existing_issue.event_count += 1;
			existing_issue.last_seen = event.timestamp;

			if is_regression {
				existing_issue.status = IssueStatus::Regressed;
				existing_issue.times_regressed += 1;
				existing_issue.last_regressed_at = Some(Utc::now());
				existing_issue.regressed_in_release = event.release.clone();
			}

			state
				.crash_repo
				.update_issue(&existing_issue)
				.await
				.map_err(|e| format!("Failed to update issue: {}", e))?;

			// Track person if present
			if let Some(pid) = event.person_id {
				let _ = state
					.crash_repo
					.add_issue_person(existing_issue.id, pid)
					.await;
			}

			// Broadcast regression if needed
			if is_regression {
				state
					.crash_broadcaster
					.broadcast_regression(project_id, &existing_issue)
					.await;
			}

			(existing_issue, false, is_regression)
		}
		None => {
			// Create new issue
			let short_id = state
				.crash_repo
				.get_next_short_id(project_id)
				.await
				.map_err(|e| format!("Failed to get short ID: {}", e))?;

			let culprit = fingerprint::find_culprit(&event);
			let title = format!(
				"{}: {}",
				event.exception_type,
				fingerprint::truncate(&event.exception_value, 100)
			);

			let issue = Issue {
				id: IssueId::new(),
				org_id: project.org_id,
				project_id,
				short_id,
				fingerprint,
				title,
				culprit,
				metadata: IssueMetadata {
					exception_type: event.exception_type.clone(),
					exception_value: event.exception_value.clone(),
					filename: event
						.stacktrace
						.frames
						.iter()
						.find(|f| f.in_app)
						.and_then(|f| f.filename.clone()),
					function: event
						.stacktrace
						.frames
						.iter()
						.find(|f| f.in_app)
						.and_then(|f| f.function.clone()),
				},
				status: IssueStatus::Unresolved,
				level: IssueLevel::Error,
				priority: IssuePriority::Medium,
				event_count: 1,
				user_count: if event.person_id.is_some() { 1 } else { 0 },
				first_seen: event.timestamp,
				last_seen: event.timestamp,
				resolved_at: None,
				resolved_by: None,
				resolved_in_release: None,
				times_regressed: 0,
				last_regressed_at: None,
				regressed_in_release: None,
				assigned_to: None,
				created_at: Utc::now(),
				updated_at: Utc::now(),
			};

			state
				.crash_repo
				.create_issue(&issue)
				.await
				.map_err(|e| format!("Failed to create issue: {}", e))?;

			// Track person if present
			if let Some(pid) = event.person_id {
				let _ = state.crash_repo.add_issue_person(issue.id, pid).await;
			}

			(issue, true, false)
		}
	};

	// Set issue_id on event and save
	event.issue_id = Some(issue.id);
	state
		.crash_repo
		.create_event(&event)
		.await
		.map_err(|e| format!("Failed to create event: {}", e))?;

	// Track release if present
	if let Some(ref release_version) = event.release {
		// Get or create the release
		if let Err(e) = state
			.crash_repo
			.get_or_create_release(project_id, project.org_id, release_version)
			.await
		{
			tracing::warn!(error = %e, release = %release_version, "Failed to get/create release");
		}

		// Update release crash count
		if let Err(e) = state
			.crash_repo
			.increment_release_crash_count(project_id, release_version, is_new_issue, is_regression)
			.await
		{
			tracing::warn!(error = %e, release = %release_version, "Failed to increment release crash count");
		}
	}

	// Broadcast new crash event
	state
		.crash_broadcaster
		.broadcast_new_crash(project_id, event.id, &issue, is_new_issue)
		.await;

	Ok(CaptureResponse {
		event_id: event.id.to_string(),
		issue_id: issue.id.to_string(),
		short_id: issue.short_id,
		is_new_issue,
		is_regression,
	})
}

// ============================================================================
// Project Endpoints
// ============================================================================

/// Request to create a crash project.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateProjectRequest {
	pub org_id: String,
	pub name: String,
	pub slug: String,
	#[serde(default = "default_platform")]
	pub platform: String,
}

fn default_platform() -> String {
	"javascript".to_string()
}

/// Response for project operations.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProjectResponse {
	pub id: String,
	pub org_id: String,
	pub name: String,
	pub slug: String,
	pub platform: String,
	pub created_at: String,
	pub updated_at: String,
}

impl From<CrashProject> for ProjectResponse {
	fn from(p: CrashProject) -> Self {
		Self {
			id: p.id.to_string(),
			org_id: p.org_id.to_string(),
			name: p.name,
			slug: p.slug,
			platform: p.platform.to_string(),
			created_at: p.created_at.to_rfc3339(),
			updated_at: p.updated_at.to_rfc3339(),
		}
	}
}

/// GET /api/crash/projects - List crash projects
#[utoipa::path(
	get,
	path = "/api/crash/projects",
	params(
		("org_id" = String, Query, description = "Organization ID"),
	),
	responses(
		(status = 200, description = "List of projects", body = Vec<ProjectResponse>),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn list_projects(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Query(params): Query<ListProjectsParams>,
) -> Result<Json<Vec<ProjectResponse>>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id: OrgId = params.org_id.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_org_id".to_string(),
				message: "Invalid organization ID".to_string(),
			}),
		)
	})?;

	verify_org_membership(&state, &org_id, &current_user.user.id, &locale).await?;

	let projects = state.crash_repo.list_projects(org_id).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to list projects");
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(CrashErrorResponse {
				error: "internal_error".to_string(),
				message: t(&locale, "server.api.error.internal").to_string(),
			}),
		)
	})?;

	Ok(Json(
		projects.into_iter().map(ProjectResponse::from).collect(),
	))
}

#[derive(Debug, Deserialize)]
pub struct ListProjectsParams {
	pub org_id: String,
}

/// POST /api/crash/projects - Create a crash project
#[utoipa::path(
	post,
	path = "/api/crash/projects",
	request_body = CreateProjectRequest,
	responses(
		(status = 201, description = "Project created", body = ProjectResponse),
		(status = 400, description = "Invalid request", body = CrashErrorResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user, body))]
pub async fn create_project(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Json(body): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectResponse>), (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id: OrgId = body.org_id.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_org_id".to_string(),
				message: "Invalid organization ID".to_string(),
			}),
		)
	})?;

	verify_org_membership(&state, &org_id, &current_user.user.id, &locale).await?;

	// Validate slug
	if !CrashProject::validate_slug(&body.slug) {
		return Err((
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_slug".to_string(),
				message: "Slug must be 3-50 lowercase alphanumeric characters with hyphens/underscores"
					.to_string(),
			}),
		));
	}

	let platform: Platform = body.platform.parse().unwrap_or(Platform::JavaScript);

	let now = Utc::now();
	let project = CrashProject {
		id: ProjectId::new(),
		org_id,
		name: body.name,
		slug: body.slug,
		platform,
		auto_resolve_age_days: None,
		fingerprint_rules: vec![],
		created_at: now,
		updated_at: now,
	};

	state
		.crash_repo
		.create_project(&project)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to create project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?;

	info!(project_id = %project.id, slug = %project.slug, "Crash project created");

	Ok((StatusCode::CREATED, Json(ProjectResponse::from(project))))
}

// ============================================================================
// Issue Endpoints
// ============================================================================

/// Response for issue operations.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct IssueResponse {
	pub id: String,
	pub project_id: String,
	pub short_id: String,
	pub title: String,
	pub culprit: Option<String>,
	pub status: String,
	pub level: String,
	pub priority: String,
	pub event_count: u64,
	pub user_count: u64,
	pub first_seen: String,
	pub last_seen: String,
	pub times_regressed: u32,
}

impl From<Issue> for IssueResponse {
	fn from(i: Issue) -> Self {
		Self {
			id: i.id.to_string(),
			project_id: i.project_id.to_string(),
			short_id: i.short_id,
			title: i.title,
			culprit: i.culprit,
			status: i.status.to_string(),
			level: i.level.to_string(),
			priority: i.priority.to_string(),
			event_count: i.event_count,
			user_count: i.user_count,
			first_seen: i.first_seen.to_rfc3339(),
			last_seen: i.last_seen.to_rfc3339(),
			times_regressed: i.times_regressed,
		}
	}
}

/// GET /api/crash/projects/{project_id}/issues - List issues
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/issues",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	responses(
		(status = 200, description = "List of issues", body = Vec<IssueResponse>),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn list_issues(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
) -> Result<Json<Vec<IssueResponse>>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id: ProjectId = project_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})?;

	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
		})?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	let issues = state
		.crash_repo
		.list_issues(project_id, 100)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list issues");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?;

	Ok(Json(issues.into_iter().map(IssueResponse::from).collect()))
}

/// POST /api/crash/projects/{project_id}/issues/{issue_id}/resolve - Resolve an issue
#[utoipa::path(
	post,
	path = "/api/crash/projects/{project_id}/issues/{issue_id}/resolve",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("issue_id" = String, Path, description = "Issue ID"),
	),
	responses(
		(status = 200, description = "Issue resolved", body = IssueResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Issue not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn resolve_issue(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, issue_id_str)): Path<(String, String)>,
) -> Result<Json<IssueResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id: ProjectId = project_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})?;

	let issue_id: IssueId = issue_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_issue_id".to_string(),
				message: "Invalid issue ID".to_string(),
			}),
		)
	})?;

	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
		})?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	let mut issue = state
		.crash_repo
		.get_issue_by_id(issue_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get issue");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "issue_not_found".to_string(),
					message: "Issue not found".to_string(),
				}),
			)
		})?;

	// Verify issue belongs to project
	if issue.project_id != project_id {
		return Err((
			StatusCode::NOT_FOUND,
			Json(CrashErrorResponse {
				error: "issue_not_found".to_string(),
				message: "Issue not found".to_string(),
			}),
		));
	}

	issue.status = IssueStatus::Resolved;
	issue.resolved_at = Some(Utc::now());
	issue.resolved_by = Some(loom_crash_core::UserId(current_user.user.id.into_inner()));

	state.crash_repo.update_issue(&issue).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to update issue");
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(CrashErrorResponse {
				error: "internal_error".to_string(),
				message: t(&locale, "server.api.error.internal").to_string(),
			}),
		)
	})?;

	state
		.crash_broadcaster
		.broadcast_resolved(project_id, &issue)
		.await;

	info!(issue_id = %issue.id, short_id = %issue.short_id, "Issue resolved");

	Ok(Json(IssueResponse::from(issue)))
}

// ============================================================================
// Issue Detail Endpoint
// ============================================================================

/// Detailed response for a single issue including metadata.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct IssueDetailResponse {
	pub id: String,
	pub org_id: String,
	pub project_id: String,
	pub short_id: String,
	pub fingerprint: String,
	pub title: String,
	pub culprit: Option<String>,
	pub metadata: IssueMetadataResponse,
	pub status: String,
	pub level: String,
	pub priority: String,
	pub event_count: u64,
	pub user_count: u64,
	pub first_seen: String,
	pub last_seen: String,
	pub resolved_at: Option<String>,
	pub resolved_by: Option<String>,
	pub resolved_in_release: Option<String>,
	pub times_regressed: u32,
	pub last_regressed_at: Option<String>,
	pub regressed_in_release: Option<String>,
	pub assigned_to: Option<String>,
	pub created_at: String,
	pub updated_at: String,
}

/// Issue metadata response.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct IssueMetadataResponse {
	pub exception_type: String,
	pub exception_value: String,
	pub filename: Option<String>,
	pub function: Option<String>,
}

impl From<Issue> for IssueDetailResponse {
	fn from(i: Issue) -> Self {
		Self {
			id: i.id.to_string(),
			org_id: i.org_id.to_string(),
			project_id: i.project_id.to_string(),
			short_id: i.short_id,
			fingerprint: i.fingerprint,
			title: i.title,
			culprit: i.culprit,
			metadata: IssueMetadataResponse {
				exception_type: i.metadata.exception_type,
				exception_value: i.metadata.exception_value,
				filename: i.metadata.filename,
				function: i.metadata.function,
			},
			status: i.status.to_string(),
			level: i.level.to_string(),
			priority: i.priority.to_string(),
			event_count: i.event_count,
			user_count: i.user_count,
			first_seen: i.first_seen.to_rfc3339(),
			last_seen: i.last_seen.to_rfc3339(),
			resolved_at: i.resolved_at.map(|dt| dt.to_rfc3339()),
			resolved_by: i.resolved_by.map(|u| u.0.to_string()),
			resolved_in_release: i.resolved_in_release,
			times_regressed: i.times_regressed,
			last_regressed_at: i.last_regressed_at.map(|dt| dt.to_rfc3339()),
			regressed_in_release: i.regressed_in_release,
			assigned_to: i.assigned_to.map(|u| u.0.to_string()),
			created_at: i.created_at.to_rfc3339(),
			updated_at: i.updated_at.to_rfc3339(),
		}
	}
}

/// GET /api/crash/projects/{project_id}/issues/{issue_id} - Get issue detail
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/issues/{issue_id}",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("issue_id" = String, Path, description = "Issue ID"),
	),
	responses(
		(status = 200, description = "Issue detail", body = IssueDetailResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Issue not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn get_issue(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, issue_id_str)): Path<(String, String)>,
) -> Result<Json<IssueDetailResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id: ProjectId = project_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})?;

	let issue_id: IssueId = issue_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_issue_id".to_string(),
				message: "Invalid issue ID".to_string(),
			}),
		)
	})?;

	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
		})?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	let issue = state
		.crash_repo
		.get_issue_by_id(issue_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get issue");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "issue_not_found".to_string(),
					message: "Issue not found".to_string(),
				}),
			)
		})?;

	// Verify issue belongs to project
	if issue.project_id != project_id {
		return Err((
			StatusCode::NOT_FOUND,
			Json(CrashErrorResponse {
				error: "issue_not_found".to_string(),
				message: "Issue not found".to_string(),
			}),
		));
	}

	info!(issue_id = %issue.id, short_id = %issue.short_id, "Issue detail retrieved");

	Ok(Json(IssueDetailResponse::from(issue)))
}

// ============================================================================
// Issue Events Endpoint
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

	let project_id: ProjectId = project_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})?;

	let issue_id: IssueId = issue_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_issue_id".to_string(),
				message: "Invalid issue ID".to_string(),
			}),
		)
	})?;

	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
		})?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	// Verify issue exists and belongs to project
	let issue = state
		.crash_repo
		.get_issue_by_id(issue_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get issue");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "issue_not_found".to_string(),
					message: "Issue not found".to_string(),
				}),
			)
		})?;

	if issue.project_id != project_id {
		return Err((
			StatusCode::NOT_FOUND,
			Json(CrashErrorResponse {
				error: "issue_not_found".to_string(),
				message: "Issue not found".to_string(),
			}),
		));
	}

	let events = state
		.crash_repo
		.list_events_for_issue(issue_id, 100)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list events");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?;

	info!(issue_id = %issue.id, event_count = %events.len(), "Issue events retrieved");

	Ok(Json(
		events.into_iter().map(CrashEventResponse::from).collect(),
	))
}

// ============================================================================
// SSE Stream Endpoint
// ============================================================================

/// Query parameters for the crash stream endpoint.
#[derive(Debug, Deserialize)]
pub struct StreamCrashParams {
	pub project_id: String,
}

/// GET /api/crash/projects/{project_id}/stream - SSE stream for crash events
///
/// Streams real-time updates for crash events including:
/// - `init`: Initial state with issue count on connect
/// - `crash.new`: New crash event received
/// - `issue.regressed`: Resolved issue regressed
/// - `issue.resolved`: Issue was resolved
/// - `issue.assigned`: Issue was assigned
/// - `heartbeat`: Keep-alive (every 30s)
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/stream",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	responses(
		(status = 200, description = "SSE stream connection established"),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Project not found"),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn stream_crash(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
) -> Result<
	Sse<impl Stream<Item = Result<Event, Infallible>>>,
	(StatusCode, Json<CrashErrorResponse>),
> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id: ProjectId = project_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})?;

	// Get project to verify it exists and get org_id
	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
		})?;

	// Verify org membership
	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	info!(
		project_id = %project_id,
		user_id = %current_user.user.id,
		"Client connected to crash stream"
	);

	// Get issue count for init event
	let issues = state
		.crash_repo
		.list_issues(project_id, 1)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get issue count for init");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?;

	// Get the actual count - we need a count method, but for now we'll use a rough estimate
	let issue_count = state
		.crash_repo
		.get_issue_count(project_id)
		.await
		.unwrap_or(issues.len() as u64);

	// Create init event
	let init_event = CrashStreamEvent::init(project_id, issue_count);

	// Subscribe to broadcast channel
	let receiver = state.crash_broadcaster.subscribe(project_id).await;
	let broadcast_stream = BroadcastStream::new(receiver);

	// Create a stream that first yields the init event, then yields broadcast events
	let init_stream = futures::stream::once(async move {
		let json = serde_json::to_string(&init_event).unwrap_or_else(|_| "{}".to_string());
		Ok::<_, Infallible>(Event::default().event("init").data(json))
	});

	let updates_stream = broadcast_stream.filter_map(|result| match result {
		Ok(event) => {
			let event_type = event.event_type();
			match serde_json::to_string(&event) {
				Ok(json) => Some(Ok::<_, Infallible>(
					Event::default().event(event_type).data(json),
				)),
				Err(e) => {
					tracing::warn!(error = %e, "Failed to serialize crash SSE event");
					None
				}
			}
		}
		Err(e) => {
			tracing::debug!(error = %e, "Broadcast stream error (client may have disconnected)");
			None
		}
	});

	let combined_stream = init_stream.chain(updates_stream);

	Ok(
		Sse::new(combined_stream).keep_alive(
			axum::response::sse::KeepAlive::new()
				.interval(std::time::Duration::from_secs(30))
				.text("heartbeat"),
		),
	)
}

// ============================================================================
// Release Endpoints
// ============================================================================

/// Response for release operations.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ReleaseResponse {
	pub id: String,
	pub project_id: String,
	pub version: String,
	pub short_version: Option<String>,
	pub url: Option<String>,
	pub crash_count: u64,
	pub new_issue_count: u64,
	pub regression_count: u64,
	pub user_count: u64,
	pub date_released: Option<String>,
	pub first_event: Option<String>,
	pub last_event: Option<String>,
	pub created_at: String,
}

impl From<Release> for ReleaseResponse {
	fn from(r: Release) -> Self {
		Self {
			id: r.id.to_string(),
			project_id: r.project_id.to_string(),
			version: r.version,
			short_version: r.short_version,
			url: r.url,
			crash_count: r.crash_count,
			new_issue_count: r.new_issue_count,
			regression_count: r.regression_count,
			user_count: r.user_count,
			date_released: r.date_released.map(|dt| dt.to_rfc3339()),
			first_event: r.first_event.map(|dt| dt.to_rfc3339()),
			last_event: r.last_event.map(|dt| dt.to_rfc3339()),
			created_at: r.created_at.to_rfc3339(),
		}
	}
}

/// Request to create a release.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateReleaseRequest {
	pub version: String,
	pub short_version: Option<String>,
	pub url: Option<String>,
	pub date_released: Option<String>,
}

/// GET /api/crash/projects/{project_id}/releases - List releases for a project
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/releases",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	responses(
		(status = 200, description = "List of releases", body = Vec<ReleaseResponse>),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn list_releases(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
) -> Result<Json<Vec<ReleaseResponse>>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id: ProjectId = project_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})?;

	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
		})?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	let releases = state
		.crash_repo
		.list_releases(project_id, 100)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list releases");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?;

	info!(project_id = %project_id, release_count = %releases.len(), "Releases listed");

	Ok(Json(
		releases.into_iter().map(ReleaseResponse::from).collect(),
	))
}

/// POST /api/crash/projects/{project_id}/releases - Create a release
#[utoipa::path(
	post,
	path = "/api/crash/projects/{project_id}/releases",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	request_body = CreateReleaseRequest,
	responses(
		(status = 201, description = "Release created", body = ReleaseResponse),
		(status = 400, description = "Invalid request", body = CrashErrorResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
		(status = 409, description = "Release already exists", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user, body))]
pub async fn create_release(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
	Json(body): Json<CreateReleaseRequest>,
) -> Result<(StatusCode, Json<ReleaseResponse>), (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id: ProjectId = project_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})?;

	// Validate version
	if body.version.is_empty() || body.version.len() > 200 {
		return Err((
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_version".to_string(),
				message: "Version must be 1-200 characters".to_string(),
			}),
		));
	}

	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
		})?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	// Check if release already exists
	if let Some(_existing) = state
		.crash_repo
		.get_release_by_version(project_id, &body.version)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to check for existing release");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})? {
		return Err((
			StatusCode::CONFLICT,
			Json(CrashErrorResponse {
				error: "release_exists".to_string(),
				message: format!("Release {} already exists", body.version),
			}),
		));
	}

	let date_released = body
		.date_released
		.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
		.map(|dt| dt.with_timezone(&Utc));

	let release = Release {
		id: ReleaseId::new(),
		org_id: project.org_id,
		project_id,
		version: body.version.clone(),
		short_version: body.short_version,
		url: body.url,
		crash_count: 0,
		new_issue_count: 0,
		regression_count: 0,
		user_count: 0,
		date_released,
		first_event: None,
		last_event: None,
		created_at: Utc::now(),
	};

	state
		.crash_repo
		.create_release(&release)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to create release");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?;

	info!(release_id = %release.id, version = %release.version, "Release created");

	Ok((StatusCode::CREATED, Json(ReleaseResponse::from(release))))
}

/// GET /api/crash/projects/{project_id}/releases/{version} - Get release detail
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/releases/{version}",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("version" = String, Path, description = "Release version"),
	),
	responses(
		(status = 200, description = "Release detail", body = ReleaseResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Release not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn get_release(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, version)): Path<(String, String)>,
) -> Result<Json<ReleaseResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id: ProjectId = project_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})?;

	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
		})?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	let release = state
		.crash_repo
		.get_release_by_version(project_id, &version)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get release");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				Json(CrashErrorResponse {
					error: "release_not_found".to_string(),
					message: format!("Release {} not found", version),
				}),
			)
		})?;

	info!(release_id = %release.id, version = %release.version, "Release retrieved");

	Ok(Json(ReleaseResponse::from(release)))
}
