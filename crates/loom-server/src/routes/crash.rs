// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash analytics HTTP handlers.
//!
//! Implements endpoints for crash event capture, issue management,
//! and project configuration.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use loom_crash_core::{
	compute_fingerprint, fingerprint, Breadcrumb, CrashEvent, CrashEventId, CrashProject, Frame,
	Issue, IssueId, IssueLevel, IssueMetadata, IssuePriority, IssueStatus, OrgId, PersonId,
	Platform, ProjectId, Stacktrace,
};
use loom_server_auth::types::OrgId as AuthOrgId;
use loom_server_crash::CrashRepository;

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::{resolve_user_locale, t};

/// Error response for crash endpoints.
#[derive(Debug, Serialize)]
#[derive(utoipa::ToSchema)]
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
#[derive(Debug, Deserialize)]
#[derive(utoipa::ToSchema)]
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
#[derive(Debug, Deserialize)]
#[derive(utoipa::ToSchema)]
pub struct CaptureStacktrace {
	pub frames: Vec<CaptureFrame>,
}

/// Frame in capture request.
#[derive(Debug, Deserialize)]
#[derive(utoipa::ToSchema)]
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
#[derive(Debug, Deserialize)]
#[derive(utoipa::ToSchema)]
pub struct CaptureBreadcrumb {
	pub timestamp: Option<String>,
	pub category: Option<String>,
	pub message: Option<String>,
	pub level: Option<String>,
	#[serde(default)]
	pub data: serde_json::Value,
}

/// Response for crash capture endpoint.
#[derive(Debug, Serialize)]
#[derive(utoipa::ToSchema)]
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

	let person_id = body
		.person_id
		.and_then(|s| s.parse().ok())
		.map(PersonId);

	let mut event = CrashEvent {
		id: CrashEventId::new(),
		org_id: project.org_id,
		project_id,
		issue_id: None,
		person_id,
		distinct_id: body.distinct_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
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

			state.crash_repo.update_issue(&existing_issue).await.map_err(|e| {
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
				let _ = state.crash_repo.add_issue_person(existing_issue.id, pid).await;
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
			let short_id = state.crash_repo.get_next_short_id(project_id).await.map_err(|e| {
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
			let title = format!("{}: {}", event.exception_type, fingerprint::truncate(&event.exception_value, 100));

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
// Project Endpoints
// ============================================================================

/// Request to create a crash project.
#[derive(Debug, Deserialize)]
#[derive(utoipa::ToSchema)]
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
#[derive(Debug, Serialize)]
#[derive(utoipa::ToSchema)]
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

	Ok(Json(projects.into_iter().map(ProjectResponse::from).collect()))
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
				message: "Slug must be 3-50 lowercase alphanumeric characters with hyphens/underscores".to_string(),
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

	state.crash_repo.create_project(&project).await.map_err(|e| {
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
#[derive(Debug, Serialize)]
#[derive(utoipa::ToSchema)]
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

	let issues = state.crash_repo.list_issues(project_id, 100).await.map_err(|e| {
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

	state.crash_broadcaster.broadcast_resolved(project_id, &issue).await;

	info!(issue_id = %issue.id, short_id = %issue.short_id, "Issue resolved");

	Ok(Json(IssueResponse::from(issue)))
}
