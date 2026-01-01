// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Routes for repository maintenance operations.
//!
//! Provides endpoints for:
//! - Triggering maintenance for a specific repository
//! - Triggering global maintenance sweep (admin only)
//! - Listing maintenance jobs for a repository

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	routes::admin::AdminErrorResponse,
};
use loom_scm::{MaintenanceJob, MaintenanceJobStatus, MaintenanceJobStore, MaintenanceTask, RepoStore};

#[derive(Debug, Deserialize, ToSchema)]
pub struct TriggerMaintenanceRequest {
	#[serde(default = "default_task")]
	pub task: MaintenanceTaskApi,
}

fn default_task() -> MaintenanceTaskApi {
	MaintenanceTaskApi::Gc
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MaintenanceTaskApi {
	Gc,
	Prune,
	Repack,
	Fsck,
	All,
}

impl From<MaintenanceTaskApi> for MaintenanceTask {
	fn from(api: MaintenanceTaskApi) -> Self {
		match api {
			MaintenanceTaskApi::Gc => MaintenanceTask::Gc,
			MaintenanceTaskApi::Prune => MaintenanceTask::Prune,
			MaintenanceTaskApi::Repack => MaintenanceTask::Repack,
			MaintenanceTaskApi::Fsck => MaintenanceTask::Fsck,
			MaintenanceTaskApi::All => MaintenanceTask::All,
		}
	}
}

impl From<MaintenanceTask> for MaintenanceTaskApi {
	fn from(task: MaintenanceTask) -> Self {
		match task {
			MaintenanceTask::Gc => MaintenanceTaskApi::Gc,
			MaintenanceTask::Prune => MaintenanceTaskApi::Prune,
			MaintenanceTask::Repack => MaintenanceTaskApi::Repack,
			MaintenanceTask::Fsck => MaintenanceTaskApi::Fsck,
			MaintenanceTask::All => MaintenanceTaskApi::All,
		}
	}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MaintenanceJobStatusApi {
	Pending,
	Running,
	Success,
	Failed,
}

impl From<MaintenanceJobStatus> for MaintenanceJobStatusApi {
	fn from(status: MaintenanceJobStatus) -> Self {
		match status {
			MaintenanceJobStatus::Pending => MaintenanceJobStatusApi::Pending,
			MaintenanceJobStatus::Running => MaintenanceJobStatusApi::Running,
			MaintenanceJobStatus::Success => MaintenanceJobStatusApi::Success,
			MaintenanceJobStatus::Failed => MaintenanceJobStatusApi::Failed,
		}
	}
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MaintenanceJobResponse {
	pub id: String,
	pub repo_id: Option<String>,
	pub task: MaintenanceTaskApi,
	pub status: MaintenanceJobStatusApi,
	pub started_at: Option<String>,
	pub finished_at: Option<String>,
	pub error: Option<String>,
	pub created_at: String,
}

impl From<MaintenanceJob> for MaintenanceJobResponse {
	fn from(job: MaintenanceJob) -> Self {
		Self {
			id: job.id.to_string(),
			repo_id: job.repo_id.map(|id| id.to_string()),
			task: job.task.into(),
			status: job.status.into(),
			started_at: job.started_at.map(|t| t.to_rfc3339()),
			finished_at: job.finished_at.map(|t| t.to_rfc3339()),
			error: job.error,
			created_at: job.created_at.to_rfc3339(),
		}
	}
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TriggerMaintenanceResponse {
	pub job_id: String,
	pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MaintenanceErrorResponse {
	pub error: String,
	pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListMaintenanceJobsResponse {
	pub jobs: Vec<MaintenanceJobResponse>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListMaintenanceJobsQuery {
	#[serde(default = "default_limit")]
	pub limit: u32,
}

fn default_limit() -> u32 {
	50
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TriggerGlobalSweepRequest {
	#[serde(default = "default_task")]
	pub task: MaintenanceTaskApi,
	#[serde(default = "default_stagger_ms")]
	pub stagger_ms: u64,
}

fn default_stagger_ms() -> u64 {
	1000
}

/// Trigger maintenance for a repository.
///
/// # Authorization
///
/// Requires `repo:admin` role on the repository.
#[utoipa::path(
	post,
	path = "/api/v1/repos/{id}/maintenance",
	params(("id" = String, Path, description = "Repository ID")),
	request_body = TriggerMaintenanceRequest,
	responses(
		(status = 202, description = "Maintenance job queued", body = TriggerMaintenanceResponse),
		(status = 400, description = "Invalid request", body = MaintenanceErrorResponse),
		(status = 401, description = "Not authenticated", body = MaintenanceErrorResponse),
		(status = 403, description = "Not authorized", body = MaintenanceErrorResponse),
		(status = 404, description = "Repository not found", body = MaintenanceErrorResponse)
	),
	tag = "repos-maintenance"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id, repo_id = %repo_id))]
pub async fn trigger_repo_maintenance(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(repo_id): Path<Uuid>,
	Json(request): Json<TriggerMaintenanceRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let maintenance_store = match &state.scm_maintenance_store {
		Some(s) => s,
		None => {
			return (
				StatusCode::NOT_IMPLEMENTED,
				Json(MaintenanceErrorResponse {
					error: "not_implemented".to_string(),
					message: "Maintenance not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let repo_store = match &state.scm_repo_store {
		Some(s) => s,
		None => {
			return (
				StatusCode::NOT_IMPLEMENTED,
				Json(MaintenanceErrorResponse {
					error: "not_implemented".to_string(),
					message: "SCM not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let _repo = match repo_store.get_by_id(repo_id).await {
		Ok(Some(r)) => r,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(MaintenanceErrorResponse {
					error: "not_found".to_string(),
					message: "Repository not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get repository");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(MaintenanceErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let task: MaintenanceTask = request.task.into();
	let job = MaintenanceJob::new(Some(repo_id), task);

	match maintenance_store.create(&job).await {
		Ok(created_job) => {
			tracing::info!(
				actor_id = %current_user.user.id,
				repo_id = %repo_id,
				job_id = %created_job.id,
				task = ?task,
				"Maintenance job created"
			);

			(
				StatusCode::ACCEPTED,
				Json(TriggerMaintenanceResponse {
					job_id: created_job.id.to_string(),
					message: format!(
						"Maintenance task '{}' queued for repository",
						task.as_str()
					),
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to create maintenance job");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(MaintenanceErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

/// Trigger global maintenance sweep.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	post,
	path = "/api/v1/admin/maintenance/sweep",
	request_body = TriggerGlobalSweepRequest,
	responses(
		(status = 202, description = "Global sweep queued", body = TriggerMaintenanceResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse)
	),
	tag = "admin-maintenance"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id))]
pub async fn trigger_global_sweep(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Json(request): Json<TriggerGlobalSweepRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized global sweep attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let maintenance_store = match &state.scm_maintenance_store {
		Some(s) => s,
		None => {
			return (
				StatusCode::NOT_IMPLEMENTED,
				Json(AdminErrorResponse {
					error: "not_implemented".to_string(),
					message: "Maintenance not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let task: MaintenanceTask = request.task.into();
	let job = MaintenanceJob::new(None, task);

	match maintenance_store.create(&job).await {
		Ok(created_job) => {
			tracing::info!(
				actor_id = %current_user.user.id,
				job_id = %created_job.id,
				task = ?task,
				stagger_ms = request.stagger_ms,
				"Global maintenance sweep queued"
			);

			(
				StatusCode::ACCEPTED,
				Json(TriggerMaintenanceResponse {
					job_id: created_job.id.to_string(),
					message: format!(
						"Global maintenance sweep '{}' queued",
						task.as_str()
					),
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to create global maintenance job");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AdminErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

/// List maintenance jobs for a repository.
#[utoipa::path(
	get,
	path = "/api/v1/repos/{id}/maintenance/jobs",
	params(
		("id" = String, Path, description = "Repository ID"),
		ListMaintenanceJobsQuery
	),
	responses(
		(status = 200, description = "List of maintenance jobs", body = ListMaintenanceJobsResponse),
		(status = 401, description = "Not authenticated", body = MaintenanceErrorResponse),
		(status = 403, description = "Not authorized", body = MaintenanceErrorResponse),
		(status = 404, description = "Repository not found", body = MaintenanceErrorResponse)
	),
	tag = "repos-maintenance"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id, repo_id = %repo_id))]
pub async fn list_repo_maintenance_jobs(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(repo_id): Path<Uuid>,
	Query(query): Query<ListMaintenanceJobsQuery>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let maintenance_store = match &state.scm_maintenance_store {
		Some(s) => s,
		None => {
			return (
				StatusCode::NOT_IMPLEMENTED,
				Json(MaintenanceErrorResponse {
					error: "not_implemented".to_string(),
					message: "Maintenance not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	match maintenance_store.list_by_repo(repo_id, query.limit).await {
		Ok(jobs) => {
			let job_responses: Vec<MaintenanceJobResponse> =
				jobs.into_iter().map(MaintenanceJobResponse::from).collect();

			tracing::info!(
				actor_id = %current_user.user.id,
				repo_id = %repo_id,
				job_count = job_responses.len(),
				"Listed maintenance jobs"
			);

			(
				StatusCode::OK,
				Json(ListMaintenanceJobsResponse { jobs: job_responses }),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list maintenance jobs");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(MaintenanceErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}
