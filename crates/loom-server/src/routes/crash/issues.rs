// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash issue HTTP handlers.
//!
//! Implements endpoints for issue management including listing, status changes,
//! assignment, and deletion.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use loom_crash_core::{Issue, IssueId, IssueStatus, ProjectId};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
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

/// Request body for resolving an issue.
#[derive(Debug, Deserialize, Default, utoipa::ToSchema)]
pub struct ResolveRequest {
	/// Optional release version this issue is resolved in.
	pub resolved_in_release: Option<String>,
}

/// Request body for assigning an issue.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AssignIssueRequest {
	/// User ID to assign the issue to. If None or null, unassigns the issue.
	pub user_id: Option<String>,
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

/// Get an issue and verify it belongs to the project.
async fn get_issue_for_project(
	state: &AppState,
	project_id: ProjectId,
	issue_id: IssueId,
	locale: &str,
) -> Result<Issue, (StatusCode, Json<CrashErrorResponse>)> {
	let issue = state
		.crash_repo
		.get_issue_by_id(issue_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get issue");
			internal_error(locale)
		})?
		.ok_or_else(|| not_found("issue"))?;

	// Verify issue belongs to project
	if issue.project_id != project_id {
		return Err(not_found("issue"));
	}

	Ok(issue)
}

// ============================================================================
// Issue Endpoints
// ============================================================================

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

	let project_id = parse_project_id(&project_id_str)?;
	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	let issues = state
		.crash_repo
		.list_issues(project_id, 100)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list issues");
			internal_error(&locale)
		})?;

	Ok(Json(issues.into_iter().map(IssueResponse::from).collect()))
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

	let project_id = parse_project_id(&project_id_str)?;
	let issue_id = parse_issue_id(&issue_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;
	let issue = get_issue_for_project(&state, project_id, issue_id, &locale).await?;

	info!(issue_id = %issue.id, short_id = %issue.short_id, "Issue detail retrieved");

	Ok(Json(IssueDetailResponse::from(issue)))
}

/// POST /api/crash/projects/{project_id}/issues/{issue_id}/resolve - Resolve an issue
#[utoipa::path(
	post,
	path = "/api/crash/projects/{project_id}/issues/{issue_id}/resolve",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("issue_id" = String, Path, description = "Issue ID"),
	),
	request_body(content = ResolveRequest, content_type = "application/json", description = "Resolve request with optional release version"),
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
	request: Option<Json<ResolveRequest>>,
) -> Result<Json<IssueResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let issue_id = parse_issue_id(&issue_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;
	let mut issue = get_issue_for_project(&state, project_id, issue_id, &locale).await?;

	issue.status = IssueStatus::Resolved;
	issue.resolved_at = Some(Utc::now());
	issue.resolved_by = Some(loom_crash_core::UserId(current_user.user.id.into_inner()));
	issue.resolved_in_release = request.as_ref().and_then(|r| r.resolved_in_release.clone());

	state.crash_repo.update_issue(&issue).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to update issue");
		internal_error(&locale)
	})?;

	state
		.crash_broadcaster
		.broadcast_resolved(project_id, &issue)
		.await;

	info!(issue_id = %issue.id, short_id = %issue.short_id, "Issue resolved");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CrashIssueResolved)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("crash_issue", issue.id.to_string())
			.details(serde_json::json!({
				"project_id": project_id.to_string(),
				"short_id": issue.short_id.clone(),
				"title": issue.title.clone(),
				"resolved_in_release": issue.resolved_in_release.clone(),
			}))
			.build(),
	);

	Ok(Json(IssueResponse::from(issue)))
}

/// POST /api/crash/projects/{project_id}/issues/{issue_id}/unresolve - Unresolve an issue
///
/// Transitions a resolved or ignored issue back to unresolved status.
#[utoipa::path(
	post,
	path = "/api/crash/projects/{project_id}/issues/{issue_id}/unresolve",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("issue_id" = String, Path, description = "Issue ID"),
	),
	responses(
		(status = 200, description = "Issue unresolve", body = IssueResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Issue not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn unresolve_issue(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, issue_id_str)): Path<(String, String)>,
) -> Result<Json<IssueResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let issue_id = parse_issue_id(&issue_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;
	let mut issue = get_issue_for_project(&state, project_id, issue_id, &locale).await?;

	issue.status = IssueStatus::Unresolved;
	issue.resolved_at = None;
	issue.resolved_by = None;
	issue.resolved_in_release = None;

	state.crash_repo.update_issue(&issue).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to update issue");
		internal_error(&locale)
	})?;

	info!(issue_id = %issue.id, short_id = %issue.short_id, "Issue unresolved");

	Ok(Json(IssueResponse::from(issue)))
}

/// POST /api/crash/projects/{project_id}/issues/{issue_id}/ignore - Ignore an issue
///
/// Marks an issue as ignored, which suppresses it from default views.
#[utoipa::path(
	post,
	path = "/api/crash/projects/{project_id}/issues/{issue_id}/ignore",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("issue_id" = String, Path, description = "Issue ID"),
	),
	responses(
		(status = 200, description = "Issue ignored", body = IssueResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Issue not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn ignore_issue(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, issue_id_str)): Path<(String, String)>,
) -> Result<Json<IssueResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let issue_id = parse_issue_id(&issue_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;
	let mut issue = get_issue_for_project(&state, project_id, issue_id, &locale).await?;

	issue.status = IssueStatus::Ignored;

	state.crash_repo.update_issue(&issue).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to update issue");
		internal_error(&locale)
	})?;

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CrashIssueIgnored)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("crash_issue", issue.id.to_string())
			.details(serde_json::json!({
				"project_id": project_id.to_string(),
				"short_id": issue.short_id.clone(),
				"title": issue.title.clone(),
			}))
			.build(),
	);

	info!(issue_id = %issue.id, short_id = %issue.short_id, "Issue ignored");

	Ok(Json(IssueResponse::from(issue)))
}

/// POST /api/crash/projects/{project_id}/issues/{issue_id}/assign - Assign an issue to a user
///
/// Assigns an issue to a specific user, or unassigns if user_id is null/omitted.
#[utoipa::path(
	post,
	path = "/api/crash/projects/{project_id}/issues/{issue_id}/assign",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("issue_id" = String, Path, description = "Issue ID"),
	),
	request_body = AssignIssueRequest,
	responses(
		(status = 200, description = "Issue assigned", body = IssueDetailResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Issue not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user, body))]
pub async fn assign_issue(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, issue_id_str)): Path<(String, String)>,
	Json(body): Json<AssignIssueRequest>,
) -> Result<Json<IssueDetailResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let issue_id = parse_issue_id(&issue_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;
	let mut issue = get_issue_for_project(&state, project_id, issue_id, &locale).await?;

	// Parse and assign user ID
	let assigned_user_id: Option<loom_crash_core::UserId> = match &body.user_id {
		Some(uid) => {
			let parsed: uuid::Uuid = uid.parse().map_err(|_| {
				(
					StatusCode::BAD_REQUEST,
					Json(CrashErrorResponse {
						error: "invalid_user_id".to_string(),
						message: "Invalid user ID".to_string(),
					}),
				)
			})?;
			Some(loom_crash_core::UserId(parsed))
		}
		None => None,
	};

	issue.assigned_to = assigned_user_id;
	issue.updated_at = Utc::now();

	state.crash_repo.update_issue(&issue).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to update issue");
		internal_error(&locale)
	})?;

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CrashIssueAssigned)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("crash_issue", issue.id.to_string())
			.details(serde_json::json!({
				"project_id": project_id.to_string(),
				"short_id": issue.short_id.clone(),
				"assigned_to": body.user_id.clone(),
			}))
			.build(),
	);

	info!(
		issue_id = %issue.id,
		short_id = %issue.short_id,
		assigned_to = ?body.user_id,
		"Issue assigned"
	);

	Ok(Json(IssueDetailResponse::from(issue)))
}

/// DELETE /api/crash/projects/{project_id}/issues/{issue_id} - Delete an issue
///
/// Permanently deletes an issue and all associated events.
#[utoipa::path(
	delete,
	path = "/api/crash/projects/{project_id}/issues/{issue_id}",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("issue_id" = String, Path, description = "Issue ID"),
	),
	responses(
		(status = 204, description = "Issue deleted"),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Issue not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn delete_issue(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, issue_id_str)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let issue_id = parse_issue_id(&issue_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;
	let issue = get_issue_for_project(&state, project_id, issue_id, &locale).await?;

	let deleted = state.crash_repo.delete_issue(issue_id).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to delete issue");
		internal_error(&locale)
	})?;

	if deleted {
		state.audit_service.log(
			AuditLogBuilder::new(AuditEventType::CrashIssueDeleted)
				.actor(AuditUserId::new(current_user.user.id.into_inner()))
				.resource("crash_issue", issue.id.to_string())
				.details(serde_json::json!({
					"project_id": project_id.to_string(),
					"short_id": issue.short_id.clone(),
					"title": issue.title.clone(),
				}))
				.build(),
		);

		info!(issue_id = %issue_id, short_id = %issue.short_id, "Issue deleted");
		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(not_found("issue"))
	}
}
