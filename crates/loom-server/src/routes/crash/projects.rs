// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash project HTTP handlers.
//!
//! Implements CRUD endpoints for crash projects.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use loom_crash_core::{CrashProject, OrgId, Platform, ProjectId};
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

/// Response wrapper for list projects endpoint
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProjectListResponse {
	pub projects: Vec<ProjectResponse>,
}

/// Query parameters for listing projects.
#[derive(Debug, Deserialize)]
pub struct ListProjectsParams {
	pub org_id: String,
}

/// Request body for updating a project.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateProjectRequest {
	/// New project name (optional)
	pub name: Option<String>,
	/// Auto-resolve age in days (optional, set to null to disable)
	pub auto_resolve_age_days: Option<u32>,
}

// ============================================================================
// Project Endpoints
// ============================================================================

/// GET /api/crash/projects - List crash projects
#[utoipa::path(
	get,
	path = "/api/crash/projects",
	params(
		("org_id" = String, Query, description = "Organization ID"),
	),
	responses(
		(status = 200, description = "List of projects", body = ProjectListResponse),
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
) -> Result<Json<ProjectListResponse>, (StatusCode, Json<CrashErrorResponse>)> {
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
		internal_error(&locale)
	})?;

	Ok(Json(ProjectListResponse {
		projects: projects.into_iter().map(ProjectResponse::from).collect(),
	}))
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
			internal_error(&locale)
		})?;

	info!(project_id = %project.id, slug = %project.slug, "Crash project created");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CrashProjectCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("crash_project", project.id.to_string())
			.details(serde_json::json!({
				"org_id": project.org_id.to_string(),
				"name": project.name.clone(),
				"slug": project.slug.clone(),
				"platform": project.platform.to_string(),
			}))
			.build(),
	);

	Ok((StatusCode::CREATED, Json(ProjectResponse::from(project))))
}

/// GET /api/crash/projects/{project_id} - Get project detail
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	responses(
		(status = 200, description = "Project detail", body = ProjectResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn get_project(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
) -> Result<Json<ProjectResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;

	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			internal_error(&locale)
		})?
		.ok_or_else(|| not_found("project"))?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	Ok(Json(ProjectResponse::from(project)))
}

/// PATCH /api/crash/projects/{project_id} - Update a project
#[utoipa::path(
	patch,
	path = "/api/crash/projects/{project_id}",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	request_body = UpdateProjectRequest,
	responses(
		(status = 200, description = "Project updated", body = ProjectResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user, body))]
pub async fn update_project(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
	Json(body): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;

	let mut project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			internal_error(&locale)
		})?
		.ok_or_else(|| not_found("project"))?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	// Apply updates
	if let Some(name) = &body.name {
		if name.is_empty() {
			return Err((
				StatusCode::BAD_REQUEST,
				Json(CrashErrorResponse {
					error: "invalid_name".to_string(),
					message: "Project name cannot be empty".to_string(),
				}),
			));
		}
		project.name = name.clone();
	}

	if body.auto_resolve_age_days.is_some() {
		project.auto_resolve_age_days = body.auto_resolve_age_days;
	}

	project.updated_at = Utc::now();

	state
		.crash_repo
		.update_project(&project)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to update project");
			internal_error(&locale)
		})?;

	info!(project_id = %project.id, "Project updated");

	Ok(Json(ProjectResponse::from(project)))
}

/// DELETE /api/crash/projects/{project_id} - Delete a project
///
/// Permanently deletes a project and all associated issues, events, and artifacts.
#[utoipa::path(
	delete,
	path = "/api/crash/projects/{project_id}",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	responses(
		(status = 204, description = "Project deleted"),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn delete_project(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;

	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			internal_error(&locale)
		})?
		.ok_or_else(|| not_found("project"))?;

	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	let deleted = state
		.crash_repo
		.delete_project(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to delete project");
			internal_error(&locale)
		})?;

	if deleted {
		state.audit_service.log(
			AuditLogBuilder::new(AuditEventType::CrashProjectDeleted)
				.actor(AuditUserId::new(current_user.user.id.into_inner()))
				.resource("crash_project", project.id.to_string())
				.details(serde_json::json!({
					"org_id": project.org_id.to_string(),
					"name": project.name.clone(),
					"slug": project.slug.clone(),
				}))
				.build(),
		);

		info!(project_id = %project_id, slug = %project.slug, "Project deleted");
		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(not_found("project"))
	}
}
