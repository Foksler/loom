// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash capture HTTP handlers.
//!
//! Implements endpoints for crash event ingestion from SDKs.

use axum::{
	extract::State,
	http::{header::HeaderMap, StatusCode},
	Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use loom_crash_core::{
	compute_fingerprint, fingerprint, Breadcrumb, CrashApiKey, CrashEvent, CrashEventId,
	CrashKeyType, Frame, Issue, IssueId, IssueLevel, IssueMetadata, IssuePriority, IssueStatus,
	PersonId, Platform, ProjectId, Stacktrace,
};
use loom_server_auth::middleware::CurrentUser;
use loom_server_auth::types::OrgId as AuthOrgId;
use loom_server_crash::{verify_api_key, CrashRepository};

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::resolve_user_locale;

use super::common::{
	internal_error, parse_project_id, symbolicate_event, verify_org_membership, CrashErrorResponse,
};

/// API key header name for SDK capture requests.
const CRASH_API_KEY_HEADER: &str = "x-crash-api-key";

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request body for crash capture endpoint.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CaptureRequest {
	pub project_id: String,
	pub exception_type: String,
	pub exception_value: String,
	#[serde(default)]
	pub stacktrace: Option<CaptureStacktrace>,
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a CaptureRequest into a CrashEvent.
fn convert_capture_request(
	body: CaptureRequest,
	org_id: loom_crash_core::OrgId,
	project_id: ProjectId,
) -> CrashEvent {
	let platform = body
		.platform
		.as_deref()
		.unwrap_or("javascript")
		.parse()
		.unwrap_or(Platform::JavaScript);

	let stacktrace = Stacktrace {
		frames: body
			.stacktrace
			.map(|st| {
				st.frames
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
					.collect()
			})
			.unwrap_or_default(),
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

	CrashEvent {
		id: CrashEventId::new(),
		org_id,
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
	}
}

/// Find or create an issue for the given crash event.
async fn find_or_create_issue(
	state: &AppState,
	event: &CrashEvent,
	project_id: ProjectId,
	fingerprint: &str,
	locale: &str,
) -> Result<(Issue, bool, bool), (StatusCode, Json<CrashErrorResponse>)> {
	match state
		.crash_repo
		.get_issue_by_fingerprint(project_id, fingerprint)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to find issue by fingerprint");
			internal_error(locale)
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
					internal_error(locale)
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

			Ok((existing_issue, false, is_regression))
		}
		None => {
			// Create new issue
			let short_id = state
				.crash_repo
				.get_next_short_id(project_id)
				.await
				.map_err(|e| {
					tracing::error!(error = %e, "Failed to get next short ID");
					internal_error(locale)
				})?;

			let culprit = fingerprint::find_culprit(event);
			let title = format!(
				"{}: {}",
				event.exception_type,
				fingerprint::truncate(&event.exception_value, 100)
			);

			let issue = Issue {
				id: IssueId::new(),
				org_id: event.org_id,
				project_id,
				short_id,
				fingerprint: fingerprint.to_string(),
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
				internal_error(locale)
			})?;

			// Track person if present
			if let Some(pid) = event.person_id {
				let _ = state.crash_repo.add_issue_person(issue.id, pid).await;
			}

			Ok((issue, true, false))
		}
	}
}

/// Track release information for a crash event.
async fn track_release(
	state: &AppState,
	event: &CrashEvent,
	project_id: ProjectId,
	is_new_issue: bool,
	is_regression: bool,
) {
	if let Some(ref release_version) = event.release {
		// Get or create the release
		if let Err(e) = state
			.crash_repo
			.get_or_create_release(project_id, event.org_id, release_version)
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
}

/// Verify an API key for a project.
/// Returns the verified API key if valid and not revoked.
async fn verify_project_api_key(
	state: &AppState,
	project_id: ProjectId,
	raw_key: &str,
) -> Result<CrashApiKey, (StatusCode, Json<CrashErrorResponse>)> {
	// Get all non-revoked API keys for the project
	let keys = state
		.crash_repo
		.list_api_keys(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list API keys");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: "Internal server error".to_string(),
				}),
			)
		})?;

	// Try to verify against each non-revoked key
	for key in keys {
		if key.is_revoked() {
			continue;
		}

		match verify_api_key(raw_key, &key.key_hash) {
			Ok(true) => {
				// Update last_used timestamp (fire and forget)
				let crash_repo = state.crash_repo.clone();
				let key_id = key.id;
				tokio::spawn(async move {
					let _ = crash_repo.update_api_key_last_used(key_id).await;
				});

				return Ok(key);
			}
			Ok(false) => continue,
			Err(e) => {
				tracing::warn!(error = %e, "API key verification failed");
				continue;
			}
		}
	}

	Err((
		StatusCode::UNAUTHORIZED,
		Json(CrashErrorResponse {
			error: "invalid_api_key".to_string(),
			message: "Invalid or revoked API key".to_string(),
		}),
	))
}

// ============================================================================
// Capture Endpoints
// ============================================================================

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
	let project_id = parse_project_id(&body.project_id)?;

	// Get project
	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			internal_error(&locale)
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
	let mut event = convert_capture_request(body, project.org_id, project_id);

	// Symbolicate the stacktrace if source maps are available
	symbolicate_event(&state, &mut event, project_id).await;

	// Compute fingerprint (based on symbolicated stacktrace for better grouping)
	let fingerprint = compute_fingerprint(&event);

	// Find or create issue
	let (issue, is_new_issue, is_regression) =
		find_or_create_issue(&state, &event, project_id, &fingerprint, &locale).await?;

	// Set issue_id on event and save
	event.issue_id = Some(issue.id);
	state.crash_repo.create_event(&event).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to create event");
		internal_error(&locale)
	})?;

	// Track release if present
	track_release(&state, &event, project_id, is_new_issue, is_regression).await;

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

/// POST /api/crash/capture/sdk - Capture a crash event with API key authentication
///
/// This endpoint accepts API key authentication via the `X-Crash-Api-Key` header.
/// Use this for SDK integrations where user authentication is not available.
#[utoipa::path(
	post,
	path = "/api/crash/capture/sdk",
	request_body = CaptureRequest,
	responses(
		(status = 200, description = "Crash captured", body = CaptureResponse),
		(status = 400, description = "Invalid request", body = CrashErrorResponse),
		(status = 401, description = "Invalid API key", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
		(status = 500, description = "Internal error", body = CrashErrorResponse),
	),
	tag = "crash"
)]
#[instrument(skip(state, headers, body), fields(project_id = %body.project_id))]
pub async fn capture_crash_with_api_key(
	State(state): State<AppState>,
	headers: HeaderMap,
	Json(body): Json<CaptureRequest>,
) -> Result<Json<CaptureResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	// Extract API key from header
	let raw_key = headers
		.get(CRASH_API_KEY_HEADER)
		.and_then(|v| v.to_str().ok())
		.ok_or_else(|| {
			(
				StatusCode::UNAUTHORIZED,
				Json(CrashErrorResponse {
					error: "missing_api_key".to_string(),
					message: format!("Missing {} header", CRASH_API_KEY_HEADER),
				}),
			)
		})?;

	// Parse project ID
	let project_id = parse_project_id(&body.project_id)?;

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
					message: "Internal server error".to_string(),
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

	// Verify API key
	let api_key = verify_project_api_key(&state, project_id, raw_key).await?;

	// Check key type - only capture or admin keys can capture
	if api_key.key_type != CrashKeyType::Capture && api_key.key_type != CrashKeyType::Admin {
		return Err((
			StatusCode::FORBIDDEN,
			Json(CrashErrorResponse {
				error: "forbidden".to_string(),
				message: "API key does not have capture permission".to_string(),
			}),
		));
	}

	// Convert capture request to CrashEvent
	let mut event = convert_capture_request(body, project.org_id, project_id);

	// Symbolicate the stacktrace if source maps are available
	symbolicate_event(&state, &mut event, project_id).await;

	// Compute fingerprint
	let fingerprint = compute_fingerprint(&event);

	// Check if issue already exists or create new one
	let (issue, is_new_issue, is_regression) = match state
		.crash_repo
		.get_issue_by_fingerprint(project_id, &fingerprint)
		.await
	{
		Ok(Some(mut existing_issue)) => {
			// Check for regression
			let is_regression = if existing_issue.status == IssueStatus::Resolved {
				existing_issue.status = IssueStatus::Unresolved;
				existing_issue.times_regressed += 1;
				existing_issue.last_regressed_at = Some(event.timestamp);
				existing_issue.regressed_in_release = event.release.clone();
				true
			} else {
				false
			};

			// Update existing issue
			existing_issue.event_count += 1;
			existing_issue.last_seen = event.timestamp;
			existing_issue.updated_at = Utc::now();

			// Track new user if applicable
			if let Some(pid) = event.person_id {
				if !state
					.crash_repo
					.issue_has_person(existing_issue.id, pid)
					.await
					.unwrap_or(false)
				{
					existing_issue.user_count += 1;
					let _ = state
						.crash_repo
						.add_issue_person(existing_issue.id, pid)
						.await;
				}
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
							message: "Internal server error".to_string(),
						}),
					)
				})?;

			(existing_issue, false, is_regression)
		}
		Ok(None) | Err(_) => {
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
							message: "Internal server error".to_string(),
						}),
					)
				})?;

			let title = format!("{}: {}", event.exception_type, event.exception_value);
			let culprit = event
				.stacktrace
				.frames
				.iter()
				.find(|f| f.in_app)
				.and_then(|f| {
					let func = f.function.as_deref().unwrap_or("<anonymous>");
					Some(format!(
						"{} in {}",
						func,
						f.filename.as_deref().unwrap_or("<unknown>")
					))
				});

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
						message: "Internal server error".to_string(),
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
				message: "Internal server error".to_string(),
			}),
		)
	})?;

	// Track release if present
	track_release(&state, &event, project_id, is_new_issue, is_regression).await;

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
		api_key_id = %api_key.id,
		"Crash event captured via API key"
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
		success_count, error_count, "Batch crash capture completed"
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
	use crate::i18n::t;

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
	let mut event = convert_capture_request(body, project.org_id, project_id);

	// Symbolicate the stacktrace if source maps are available
	symbolicate_event(state, &mut event, project_id).await;

	// Compute fingerprint (based on symbolicated stacktrace for better grouping)
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
	track_release(state, &event, project_id, is_new_issue, is_regression).await;

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
