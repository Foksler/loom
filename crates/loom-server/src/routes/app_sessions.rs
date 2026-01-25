// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! App session analytics HTTP handlers.
//!
//! Implements session tracking endpoints for release health metrics:
//! - Start a session
//! - End a session
//! - Update session (error counts)
//! - List sessions
//! - Get session detail
//! - Release health endpoints

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::{DateTime, Duration, Utc};
use loom_crash_core::ProjectId;
use loom_server_auth::types::OrgId as AuthOrgId;
use loom_server_crash::CrashRepository;
use loom_server_sessions::SessionsRepository;
use loom_sessions_core::{
	sampling::should_sample, Platform, ReleaseHealth, Session, SessionId, SessionStatus,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{api::AppState, auth_middleware::RequireAuth, impl_api_error_response};

// ============================================================================
// Error Response
// ============================================================================

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionsErrorResponse {
	pub error: String,
	pub message: String,
}

impl_api_error_response!(SessionsErrorResponse);

/// Verify that the current user is a member of the specified organization.
async fn verify_org_membership(
	state: &AppState,
	org_id: &uuid::Uuid,
	user_id: &loom_server_auth::types::UserId,
) -> Result<(), (StatusCode, Json<SessionsErrorResponse>)> {
	let auth_org_id = AuthOrgId::from(*org_id);

	match state.org_repo.get_membership(&auth_org_id, user_id).await {
		Ok(Some(_)) => Ok(()),
		Ok(None) => Err((
			StatusCode::FORBIDDEN,
			Json(SessionsErrorResponse {
				error: "forbidden".to_string(),
				message: "Not a member of this organization".to_string(),
			}),
		)),
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Internal error".to_string(),
				}),
			))
		}
	}
}

// ============================================================================
// Session Start Endpoint
// ============================================================================

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SessionStartRequest {
	pub project_id: String,
	pub distinct_id: String,
	#[serde(default)]
	pub person_id: Option<String>,
	pub platform: String,
	#[serde(default = "default_environment")]
	pub environment: String,
	#[serde(default)]
	pub release: Option<String>,
	#[serde(default)]
	pub user_agent: Option<String>,
	#[serde(default = "default_sample_rate")]
	pub sample_rate: f64,
}

fn default_environment() -> String {
	"production".to_string()
}

fn default_sample_rate() -> f64 {
	1.0
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SessionStartResponse {
	pub session_id: String,
	pub sampled: bool,
}

#[utoipa::path(
    post,
    path = "/api/sessions/start",
    request_body = SessionStartRequest,
    responses(
        (status = 201, description = "Session started", body = SessionStartResponse),
        (status = 400, description = "Invalid request", body = SessionsErrorResponse),
        (status = 401, description = "Not authenticated", body = SessionsErrorResponse),
        (status = 403, description = "Not authorized", body = SessionsErrorResponse)
    ),
    tag = "app-sessions"
)]
#[instrument(skip(state, auth), fields(project_id = %body.project_id))]
pub async fn start_session(
	State(state): State<AppState>,
	RequireAuth(auth): RequireAuth,
	Json(body): Json<SessionStartRequest>,
) -> impl IntoResponse {
	// Parse project_id and get the project to verify org membership
	let project_id: ProjectId = match body.project_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_project_id".to_string(),
					message: "Invalid project ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get project to verify org membership
	let project = match state.crash_repo.get_project_by_id(project_id).await {
		Ok(Some(p)) => p,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(SessionsErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get project");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to get project".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Verify org membership
	if let Err(response) = verify_org_membership(&state, &project.org_id.0, &auth.user.id).await {
		return response.into_response();
	}

	// Parse platform
	let platform: Platform = match body.platform.parse() {
		Ok(p) => p,
		Err(_) => Platform::Other,
	};

	// Generate session ID first for deterministic sampling
	let session_id = SessionId::new();

	// Deterministic sampling based on session ID hash
	let sampled = should_sample(&session_id.to_string(), body.sample_rate);
	let now = Utc::now();

	let session = Session {
		id: session_id.clone(),
		org_id: project.org_id.to_string(),
		project_id: body.project_id.clone(),
		person_id: body.person_id,
		distinct_id: body.distinct_id,
		status: SessionStatus::Active,
		release: body.release,
		environment: body.environment,
		error_count: 0,
		crash_count: 0,
		crashed: false,
		started_at: now,
		ended_at: None,
		duration_ms: None,
		platform,
		user_agent: body.user_agent,
		sampled,
		sample_rate: body.sample_rate,
		created_at: now,
		updated_at: now,
	};

	// Only store if sampled
	if sampled {
		if let Err(e) = state.sessions_repo.create_session(&session).await {
			tracing::error!(error = %e, "Failed to create session");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to create session".to_string(),
				}),
			)
				.into_response();
		}
	}

	(
		StatusCode::CREATED,
		Json(SessionStartResponse {
			session_id: session_id.to_string(),
			sampled,
		}),
	)
		.into_response()
}

// ============================================================================
// Session End Endpoint
// ============================================================================

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SessionEndRequest {
	pub project_id: String,
	pub session_id: String,
	pub status: String,
	#[serde(default)]
	pub error_count: u32,
	#[serde(default)]
	pub crash_count: u32,
	pub duration_ms: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SessionEndResponse {
	pub success: bool,
}

#[utoipa::path(
    post,
    path = "/api/sessions/end",
    request_body = SessionEndRequest,
    responses(
        (status = 200, description = "Session ended", body = SessionEndResponse),
        (status = 400, description = "Invalid request", body = SessionsErrorResponse),
        (status = 401, description = "Not authenticated", body = SessionsErrorResponse),
        (status = 403, description = "Not a member of this organization", body = SessionsErrorResponse),
        (status = 404, description = "Session not found", body = SessionsErrorResponse)
    ),
    tag = "app-sessions"
)]
#[instrument(skip(state, auth), fields(session_id = %body.session_id))]
pub async fn end_session(
	State(state): State<AppState>,
	RequireAuth(auth): RequireAuth,
	Json(body): Json<SessionEndRequest>,
) -> impl IntoResponse {
	// Parse project_id and get the project to verify org membership
	let project_id: ProjectId = match body.project_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_project_id".to_string(),
					message: "Invalid project ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get the project to check org membership
	let project = match state.crash_repo.get_project_by_id(project_id).await {
		Ok(Some(p)) => p,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(SessionsErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get project");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Internal error".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Verify org membership
	if let Err(response) = verify_org_membership(&state, &project.org_id.0, &auth.user.id).await {
		return response.into_response();
	}

	let session_id: SessionId = match body.session_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_session_id".to_string(),
					message: "Invalid session ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	let status: SessionStatus = match body.status.parse() {
		Ok(s) => s,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_status".to_string(),
					message: "Invalid session status".to_string(),
				}),
			)
				.into_response();
		}
	};

	let ended_at = Utc::now();

	if let Err(e) = state
		.sessions_repo
		.end_session(
			&session_id,
			status,
			body.error_count,
			body.crash_count,
			ended_at,
			body.duration_ms,
		)
		.await
	{
		tracing::error!(error = %e, "Failed to end session");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(SessionsErrorResponse {
				error: "internal_error".to_string(),
				message: "Failed to end session".to_string(),
			}),
		)
			.into_response();
	}

	Json(SessionEndResponse { success: true }).into_response()
}

// ============================================================================
// List Sessions Endpoint
// ============================================================================

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ListSessionsQuery {
	pub project_id: String,
	#[serde(default = "default_limit")]
	pub limit: u32,
	#[serde(default)]
	pub offset: u32,
}

fn default_limit() -> u32 {
	50
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SessionResponse {
	pub id: String,
	pub distinct_id: String,
	pub status: String,
	pub release: Option<String>,
	pub environment: String,
	pub error_count: u32,
	pub crash_count: u32,
	pub crashed: bool,
	pub platform: String,
	pub started_at: DateTime<Utc>,
	pub ended_at: Option<DateTime<Utc>>,
	pub duration_ms: Option<u64>,
}

impl From<Session> for SessionResponse {
	fn from(s: Session) -> Self {
		Self {
			id: s.id.to_string(),
			distinct_id: s.distinct_id,
			status: s.status.to_string(),
			release: s.release,
			environment: s.environment,
			error_count: s.error_count,
			crash_count: s.crash_count,
			crashed: s.crashed,
			platform: s.platform.to_string(),
			started_at: s.started_at,
			ended_at: s.ended_at,
			duration_ms: s.duration_ms,
		}
	}
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListSessionsResponse {
	pub sessions: Vec<SessionResponse>,
}

#[utoipa::path(
    get,
    path = "/api/app-sessions",
    params(
        ("project_id" = String, Query, description = "Project ID"),
        ("limit" = u32, Query, description = "Maximum number of sessions to return"),
        ("offset" = u32, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "List of sessions", body = ListSessionsResponse),
        (status = 401, description = "Not authenticated", body = SessionsErrorResponse),
        (status = 403, description = "Not authorized", body = SessionsErrorResponse)
    ),
    tag = "app-sessions"
)]
#[instrument(skip(state, auth), fields(project_id = %query.project_id))]
pub async fn list_sessions(
	State(state): State<AppState>,
	RequireAuth(auth): RequireAuth,
	Query(query): Query<ListSessionsQuery>,
) -> impl IntoResponse {
	// Parse project_id
	let project_id: ProjectId = match query.project_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_project_id".to_string(),
					message: "Invalid project ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get project to verify org membership
	let project = match state.crash_repo.get_project_by_id(project_id).await {
		Ok(Some(p)) => p,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(SessionsErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get project");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to get project".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Verify org membership
	if let Err(response) = verify_org_membership(&state, &project.org_id.0, &auth.user.id).await {
		return response.into_response();
	}

	match state
		.sessions_repo
		.list_sessions(&query.project_id, query.limit, query.offset)
		.await
	{
		Ok(sessions) => {
			let responses: Vec<SessionResponse> = sessions.into_iter().map(Into::into).collect();
			Json(ListSessionsResponse {
				sessions: responses,
			})
			.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list sessions");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to list sessions".to_string(),
				}),
			)
				.into_response()
		}
	}
}

// ============================================================================
// Release Health Endpoints
// ============================================================================

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ReleaseHealthQuery {
	pub project_id: String,
	#[serde(default = "default_environment")]
	pub environment: String,
	#[serde(default = "default_days")]
	pub days: u32,
}

fn default_days() -> u32 {
	7
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ReleaseHealthResponse {
	pub release: String,
	pub environment: String,
	pub total_sessions: u64,
	pub crashed_sessions: u64,
	pub crash_free_session_rate: f64,
	pub crash_free_user_rate: f64,
	pub adoption_rate: f64,
	pub adoption_stage: String,
	pub first_seen: DateTime<Utc>,
	pub last_seen: DateTime<Utc>,
}

impl From<ReleaseHealth> for ReleaseHealthResponse {
	fn from(h: ReleaseHealth) -> Self {
		Self {
			release: h.release,
			environment: h.environment,
			total_sessions: h.total_sessions,
			crashed_sessions: h.crashed_sessions,
			crash_free_session_rate: h.crash_free_session_rate,
			crash_free_user_rate: h.crash_free_user_rate,
			adoption_rate: h.adoption_rate,
			adoption_stage: h.adoption_stage.to_string(),
			first_seen: h.first_seen,
			last_seen: h.last_seen,
		}
	}
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListReleasesHealthResponse {
	pub releases: Vec<ReleaseHealthResponse>,
}

#[utoipa::path(
    get,
    path = "/api/app-sessions/releases",
    params(
        ("project_id" = String, Query, description = "Project ID"),
        ("environment" = String, Query, description = "Environment"),
        ("days" = u32, Query, description = "Number of days to look back")
    ),
    responses(
        (status = 200, description = "List of release health metrics", body = ListReleasesHealthResponse),
        (status = 401, description = "Not authenticated", body = SessionsErrorResponse),
        (status = 403, description = "Not authorized", body = SessionsErrorResponse)
    ),
    tag = "app-sessions"
)]
#[instrument(skip(state, auth), fields(project_id = %query.project_id))]
pub async fn list_release_health(
	State(state): State<AppState>,
	RequireAuth(auth): RequireAuth,
	Query(query): Query<ReleaseHealthQuery>,
) -> impl IntoResponse {
	// Parse project_id
	let project_id: ProjectId = match query.project_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_project_id".to_string(),
					message: "Invalid project ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get project to verify org membership
	let project = match state.crash_repo.get_project_by_id(project_id).await {
		Ok(Some(p)) => p,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(SessionsErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get project");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to get project".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Verify org membership
	if let Err(response) = verify_org_membership(&state, &project.org_id.0, &auth.user.id).await {
		return response.into_response();
	}

	let end = Utc::now();
	let start = end - Duration::days(query.days as i64);

	// Get all releases for this project/environment
	let releases = match state
		.sessions_repo
		.get_releases(&query.project_id, &query.environment)
		.await
	{
		Ok(r) => r,
		Err(e) => {
			tracing::error!(error = %e, "Failed to get releases");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to get releases".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get total sessions for adoption rate calculation
	let total_sessions = match state
		.sessions_repo
		.get_total_sessions_in_range(&query.project_id, &query.environment, start, end)
		.await
	{
		Ok(t) => t,
		Err(e) => {
			tracing::error!(error = %e, "Failed to get total sessions");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to get total sessions".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Calculate health for each release
	let mut health_responses = Vec::new();
	for release in releases {
		let aggregates = match state
			.sessions_repo
			.get_aggregates(
				&query.project_id,
				Some(&release),
				&query.environment,
				start,
				end,
			)
			.await
		{
			Ok(a) => a,
			Err(e) => {
				tracing::error!(error = %e, release = %release, "Failed to get aggregates");
				continue;
			}
		};

		let health = ReleaseHealth::calculate(
			&query.project_id,
			&release,
			&query.environment,
			&aggregates,
			total_sessions,
		);
		health_responses.push(health.into());
	}

	Json(ListReleasesHealthResponse {
		releases: health_responses,
	})
	.into_response()
}

// ============================================================================
// Get Release Health Detail
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/app-sessions/releases/{version}",
    params(
        ("version" = String, Path, description = "Release version"),
        ("project_id" = String, Query, description = "Project ID"),
        ("environment" = String, Query, description = "Environment"),
        ("days" = u32, Query, description = "Number of days to look back")
    ),
    responses(
        (status = 200, description = "Release health detail", body = ReleaseHealthResponse),
        (status = 401, description = "Not authenticated", body = SessionsErrorResponse),
        (status = 403, description = "Not authorized", body = SessionsErrorResponse),
        (status = 404, description = "Release not found", body = SessionsErrorResponse)
    ),
    tag = "app-sessions"
)]
#[instrument(skip(state, auth), fields(project_id = %query.project_id, version = %version))]
pub async fn get_release_health(
	State(state): State<AppState>,
	RequireAuth(auth): RequireAuth,
	Path(version): Path<String>,
	Query(query): Query<ReleaseHealthQuery>,
) -> impl IntoResponse {
	// Parse project_id
	let project_id: ProjectId = match query.project_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_project_id".to_string(),
					message: "Invalid project ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get project to verify org membership
	let project = match state.crash_repo.get_project_by_id(project_id).await {
		Ok(Some(p)) => p,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(SessionsErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get project");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to get project".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Verify org membership
	if let Err(response) = verify_org_membership(&state, &project.org_id.0, &auth.user.id).await {
		return response.into_response();
	}

	let end = Utc::now();
	let start = end - Duration::days(query.days as i64);

	// Get aggregates for this release
	let aggregates = match state
		.sessions_repo
		.get_aggregates(
			&query.project_id,
			Some(&version),
			&query.environment,
			start,
			end,
		)
		.await
	{
		Ok(a) => a,
		Err(e) => {
			tracing::error!(error = %e, "Failed to get aggregates");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to get aggregates".to_string(),
				}),
			)
				.into_response();
		}
	};

	if aggregates.is_empty() {
		return (
			StatusCode::NOT_FOUND,
			Json(SessionsErrorResponse {
				error: "release_not_found".to_string(),
				message: "No data found for this release".to_string(),
			}),
		)
			.into_response();
	}

	// Get total sessions for adoption rate
	let total_sessions = match state
		.sessions_repo
		.get_total_sessions_in_range(&query.project_id, &query.environment, start, end)
		.await
	{
		Ok(t) => t,
		Err(e) => {
			tracing::error!(error = %e, "Failed to get total sessions");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to get total sessions".to_string(),
				}),
			)
				.into_response();
		}
	};

	let health = ReleaseHealth::calculate(
		&query.project_id,
		&version,
		&query.environment,
		&aggregates,
		total_sessions,
	);

	Json::<ReleaseHealthResponse>(health.into()).into_response()
}

// ============================================================================
// SDK Endpoints (API Key Authentication)
// ============================================================================

use axum::http::header::HeaderMap;
use loom_crash_core::{CrashApiKey, CrashKeyType};
use loom_server_crash::verify_api_key;

/// API key header name for SDK capture requests.
const CRASH_API_KEY_HEADER: &str = "x-crash-api-key";

/// Verify an API key for a project.
/// Returns the verified API key if valid and not revoked.
async fn verify_project_api_key(
	state: &AppState,
	project_id: ProjectId,
	raw_key: &str,
) -> Result<CrashApiKey, (StatusCode, Json<SessionsErrorResponse>)> {
	// Get all non-revoked API keys for the project
	let keys = state
		.crash_repo
		.list_api_keys(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list API keys");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
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
		Json(SessionsErrorResponse {
			error: "invalid_api_key".to_string(),
			message: "Invalid or revoked API key".to_string(),
		}),
	))
}

/// POST /api/sessions/start/sdk - Start a session with API key authentication
///
/// This endpoint accepts API key authentication via the `X-Crash-Api-Key` header.
/// Use this for SDK integrations where user authentication is not available.
#[utoipa::path(
	post,
	path = "/api/sessions/start/sdk",
	request_body = SessionStartRequest,
	responses(
		(status = 201, description = "Session started", body = SessionStartResponse),
		(status = 400, description = "Invalid request", body = SessionsErrorResponse),
		(status = 401, description = "Invalid API key", body = SessionsErrorResponse),
		(status = 404, description = "Project not found", body = SessionsErrorResponse),
		(status = 500, description = "Internal error", body = SessionsErrorResponse),
	),
	tag = "app-sessions"
)]
#[instrument(skip(state, headers, body), fields(project_id = %body.project_id))]
pub async fn start_session_with_api_key(
	State(state): State<AppState>,
	headers: HeaderMap,
	Json(body): Json<SessionStartRequest>,
) -> impl IntoResponse {
	// Extract API key from header
	let raw_key = match headers
		.get(CRASH_API_KEY_HEADER)
		.and_then(|v| v.to_str().ok())
	{
		Some(k) => k,
		None => {
			return (
				StatusCode::UNAUTHORIZED,
				Json(SessionsErrorResponse {
					error: "missing_api_key".to_string(),
					message: format!("Missing {} header", CRASH_API_KEY_HEADER),
				}),
			)
				.into_response();
		}
	};

	// Parse project_id
	let project_id: ProjectId = match body.project_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_project_id".to_string(),
					message: "Invalid project ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get project
	let project = match state.crash_repo.get_project_by_id(project_id).await {
		Ok(Some(p)) => p,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(SessionsErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get project");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Internal server error".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Verify API key
	let api_key = match verify_project_api_key(&state, project_id, raw_key).await {
		Ok(k) => k,
		Err(e) => return e.into_response(),
	};

	// Check key type - only capture or admin keys can start sessions
	if api_key.key_type != CrashKeyType::Capture && api_key.key_type != CrashKeyType::Admin {
		return (
			StatusCode::FORBIDDEN,
			Json(SessionsErrorResponse {
				error: "forbidden".to_string(),
				message: "API key does not have session tracking permission".to_string(),
			}),
		)
			.into_response();
	}

	// Parse platform
	let platform: Platform = match body.platform.parse() {
		Ok(p) => p,
		Err(_) => Platform::Other,
	};

	// Generate session ID first for deterministic sampling
	let session_id = SessionId::new();

	// Deterministic sampling based on session ID hash
	let sampled = should_sample(&session_id.to_string(), body.sample_rate);
	let now = Utc::now();

	let session = Session {
		id: session_id.clone(),
		org_id: project.org_id.to_string(),
		project_id: body.project_id.clone(),
		person_id: body.person_id,
		distinct_id: body.distinct_id,
		status: SessionStatus::Active,
		release: body.release,
		environment: body.environment,
		error_count: 0,
		crash_count: 0,
		crashed: false,
		started_at: now,
		ended_at: None,
		duration_ms: None,
		platform,
		user_agent: body.user_agent,
		sampled,
		sample_rate: body.sample_rate,
		created_at: now,
		updated_at: now,
	};

	// Only store if sampled
	if sampled {
		if let Err(e) = state.sessions_repo.create_session(&session).await {
			tracing::error!(error = %e, "Failed to create session");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to create session".to_string(),
				}),
			)
				.into_response();
		}
	}

	tracing::info!(
		session_id = %session_id,
		sampled,
		api_key_id = %api_key.id,
		"Session started via API key"
	);

	(
		StatusCode::CREATED,
		Json(SessionStartResponse {
			session_id: session_id.to_string(),
			sampled,
		}),
	)
		.into_response()
}

/// POST /api/sessions/end/sdk - End a session with API key authentication
///
/// This endpoint accepts API key authentication via the `X-Crash-Api-Key` header.
/// Use this for SDK integrations where user authentication is not available.
#[utoipa::path(
	post,
	path = "/api/sessions/end/sdk",
	request_body = SessionEndRequest,
	responses(
		(status = 200, description = "Session ended", body = SessionEndResponse),
		(status = 400, description = "Invalid request", body = SessionsErrorResponse),
		(status = 401, description = "Invalid API key", body = SessionsErrorResponse),
		(status = 404, description = "Project not found", body = SessionsErrorResponse),
		(status = 500, description = "Internal error", body = SessionsErrorResponse),
	),
	tag = "app-sessions"
)]
#[instrument(skip(state, headers, body), fields(session_id = %body.session_id))]
pub async fn end_session_with_api_key(
	State(state): State<AppState>,
	headers: HeaderMap,
	Json(body): Json<SessionEndRequest>,
) -> impl IntoResponse {
	// Extract API key from header
	let raw_key = match headers
		.get(CRASH_API_KEY_HEADER)
		.and_then(|v| v.to_str().ok())
	{
		Some(k) => k,
		None => {
			return (
				StatusCode::UNAUTHORIZED,
				Json(SessionsErrorResponse {
					error: "missing_api_key".to_string(),
					message: format!("Missing {} header", CRASH_API_KEY_HEADER),
				}),
			)
				.into_response();
		}
	};

	// Parse project_id
	let project_id: ProjectId = match body.project_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_project_id".to_string(),
					message: "Invalid project ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get project (just verifying it exists)
	match state.crash_repo.get_project_by_id(project_id).await {
		Ok(Some(_)) => {}
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(SessionsErrorResponse {
					error: "project_not_found".to_string(),
					message: "Project not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get project");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SessionsErrorResponse {
					error: "internal_error".to_string(),
					message: "Internal server error".to_string(),
				}),
			)
				.into_response();
		}
	};

	// Verify API key
	let api_key = match verify_project_api_key(&state, project_id, raw_key).await {
		Ok(k) => k,
		Err(e) => return e.into_response(),
	};

	// Check key type - only capture or admin keys can end sessions
	if api_key.key_type != CrashKeyType::Capture && api_key.key_type != CrashKeyType::Admin {
		return (
			StatusCode::FORBIDDEN,
			Json(SessionsErrorResponse {
				error: "forbidden".to_string(),
				message: "API key does not have session tracking permission".to_string(),
			}),
		)
			.into_response();
	}

	let session_id: SessionId = match body.session_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_session_id".to_string(),
					message: "Invalid session ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	let status: SessionStatus = match body.status.parse() {
		Ok(s) => s,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(SessionsErrorResponse {
					error: "invalid_status".to_string(),
					message: "Invalid session status".to_string(),
				}),
			)
				.into_response();
		}
	};

	let ended_at = Utc::now();

	if let Err(e) = state
		.sessions_repo
		.end_session(
			&session_id,
			status,
			body.error_count,
			body.crash_count,
			ended_at,
			body.duration_ms,
		)
		.await
	{
		tracing::error!(error = %e, "Failed to end session");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(SessionsErrorResponse {
				error: "internal_error".to_string(),
				message: "Failed to end session".to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(
		session_id = %session_id,
		status = %body.status,
		api_key_id = %api_key.id,
		"Session ended via API key"
	);

	Json(SessionEndResponse { success: true }).into_response()
}
