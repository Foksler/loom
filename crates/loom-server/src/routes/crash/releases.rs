// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash release HTTP handlers.
//!
//! Implements endpoints for release management.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use loom_crash_core::{ProjectId, Release, ReleaseId};
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

// ============================================================================
// Helper Functions
// ============================================================================

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
// Release Endpoints
// ============================================================================

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

	let project_id = parse_project_id(&project_id_str)?;
	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	let releases = state
		.crash_repo
		.list_releases(project_id, 100)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list releases");
			internal_error(&locale)
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

	let project_id = parse_project_id(&project_id_str)?;

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

	let project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	// Check if release already exists
	if let Some(_existing) = state
		.crash_repo
		.get_release_by_version(project_id, &body.version)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to check for existing release");
			internal_error(&locale)
		})?
	{
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
			internal_error(&locale)
		})?;

	info!(release_id = %release.id, version = %release.version, "Release created");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::CrashReleaseCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("crash_release", release.id.to_string())
			.details(serde_json::json!({
				"project_id": project_id.to_string(),
				"version": release.version.clone(),
			}))
			.build(),
	);

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

	let project_id = parse_project_id(&project_id_str)?;
	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	let release = state
		.crash_repo
		.get_release_by_version(project_id, &version)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get release");
			internal_error(&locale)
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
