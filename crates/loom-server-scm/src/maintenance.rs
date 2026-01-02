// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::error::{Result, ScmError};
use crate::git::GitRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaintenanceTask {
	Gc,
	Prune,
	Repack,
	Fsck,
	All,
}

impl MaintenanceTask {
	pub fn as_str(&self) -> &'static str {
		match self {
			MaintenanceTask::Gc => "gc",
			MaintenanceTask::Prune => "prune",
			MaintenanceTask::Repack => "repack",
			MaintenanceTask::Fsck => "fsck",
			MaintenanceTask::All => "all",
		}
	}
}

impl std::str::FromStr for MaintenanceTask {
	type Err = ();
	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s {
			"gc" => Ok(MaintenanceTask::Gc),
			"prune" => Ok(MaintenanceTask::Prune),
			"repack" => Ok(MaintenanceTask::Repack),
			"fsck" => Ok(MaintenanceTask::Fsck),
			"all" => Ok(MaintenanceTask::All),
			_ => Err(()),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaintenanceJobStatus {
	Pending,
	Running,
	Success,
	Failed,
}

impl MaintenanceJobStatus {
	pub fn as_str(&self) -> &'static str {
		match self {
			MaintenanceJobStatus::Pending => "pending",
			MaintenanceJobStatus::Running => "running",
			MaintenanceJobStatus::Success => "success",
			MaintenanceJobStatus::Failed => "failed",
		}
	}
}

impl std::str::FromStr for MaintenanceJobStatus {
	type Err = ();
	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s {
			"pending" => Ok(MaintenanceJobStatus::Pending),
			"running" => Ok(MaintenanceJobStatus::Running),
			"success" => Ok(MaintenanceJobStatus::Success),
			"failed" => Ok(MaintenanceJobStatus::Failed),
			_ => Err(()),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceJob {
	pub id: Uuid,
	pub repo_id: Option<Uuid>,
	pub task: MaintenanceTask,
	pub status: MaintenanceJobStatus,
	pub started_at: Option<DateTime<Utc>>,
	pub finished_at: Option<DateTime<Utc>>,
	pub error: Option<String>,
	pub created_at: DateTime<Utc>,
}

impl MaintenanceJob {
	pub fn new(repo_id: Option<Uuid>, task: MaintenanceTask) -> Self {
		Self {
			id: Uuid::new_v4(),
			repo_id,
			task,
			status: MaintenanceJobStatus::Pending,
			started_at: None,
			finished_at: None,
			error: None,
			created_at: Utc::now(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct MaintenanceResult {
	pub task: MaintenanceTask,
	pub success: bool,
	pub error: Option<String>,
	pub fsck_issues: Vec<String>,
}

#[instrument(skip_all, fields(repo_path = %repo_path.display(), task = ?task))]
pub fn run_maintenance(repo_path: &Path, task: MaintenanceTask) -> Result<MaintenanceResult> {
	let git_repo = GitRepository::open(repo_path)?;

	match task {
		MaintenanceTask::Gc => {
			git_repo.gc()?;
			Ok(MaintenanceResult {
				task,
				success: true,
				error: None,
				fsck_issues: Vec::new(),
			})
		}
		MaintenanceTask::Prune => {
			git_repo.prune()?;
			Ok(MaintenanceResult {
				task,
				success: true,
				error: None,
				fsck_issues: Vec::new(),
			})
		}
		MaintenanceTask::Repack => {
			run_repack(repo_path)?;
			Ok(MaintenanceResult {
				task,
				success: true,
				error: None,
				fsck_issues: Vec::new(),
			})
		}
		MaintenanceTask::Fsck => {
			let issues = git_repo.fsck()?;
			Ok(MaintenanceResult {
				task,
				success: issues.is_empty(),
				error: if issues.is_empty() {
					None
				} else {
					Some(format!("Found {} issues", issues.len()))
				},
				fsck_issues: issues,
			})
		}
		MaintenanceTask::All => {
			git_repo.gc()?;
			git_repo.prune()?;
			run_repack(repo_path)?;
			let issues = git_repo.fsck()?;
			Ok(MaintenanceResult {
				task,
				success: issues.is_empty(),
				error: if issues.is_empty() {
					None
				} else {
					Some(format!("fsck found {} issues", issues.len()))
				},
				fsck_issues: issues,
			})
		}
	}
}

// NOTE: repack uses git subprocess because gitoxide doesn't yet support
// pack maintenance operations. See: https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md
// Track progress at: https://github.com/GitoxideLabs/gitoxide/issues/307
fn run_repack(repo_path: &Path) -> Result<()> {
	let output = std::process::Command::new("git")
		.args(["repack", "-a", "-d"])
		.current_dir(repo_path)
		.output()?;
	if !output.status.success() {
		return Err(ScmError::GitError(
			String::from_utf8_lossy(&output.stderr).to_string(),
		));
	}
	Ok(())
}

#[derive(Debug)]
pub struct RepoMaintenanceResult {
	pub repo_id: Uuid,
	pub repo_path: std::path::PathBuf,
	pub result: std::result::Result<MaintenanceResult, ScmError>,
}

#[instrument(skip_all, fields(repos_dir = %repos_dir.display(), task = ?task, stagger_delay_ms = stagger_delay.as_millis()))]
pub async fn run_global_sweep(
	repos_dir: &Path,
	task: MaintenanceTask,
	stagger_delay: Duration,
	repo_paths: Vec<(Uuid, std::path::PathBuf)>,
) -> Vec<RepoMaintenanceResult> {
	let mut results = Vec::new();

	for (idx, (repo_id, repo_path)) in repo_paths.into_iter().enumerate() {
		if idx > 0 && !stagger_delay.is_zero() {
			tokio::time::sleep(stagger_delay).await;
		}

		let result = tokio::task::spawn_blocking({
			let repo_path = repo_path.clone();
			move || run_maintenance(&repo_path, task)
		})
		.await
		.map_err(|e| ScmError::GitError(format!("Task join error: {}", e)))
		.and_then(|r| r);

		info!(
			repo_id = %repo_id,
			repo_path = %repo_path.display(),
			success = result.is_ok(),
			"Completed maintenance"
		);

		results.push(RepoMaintenanceResult {
			repo_id,
			repo_path,
			result,
		});
	}

	results
}

#[async_trait]
pub trait MaintenanceJobStore: Send + Sync {
	async fn create(&self, job: &MaintenanceJob) -> Result<MaintenanceJob>;
	async fn get_by_id(&self, id: Uuid) -> Result<Option<MaintenanceJob>>;
	async fn list_by_repo(&self, repo_id: Uuid, limit: u32) -> Result<Vec<MaintenanceJob>>;
	async fn list_pending(&self) -> Result<Vec<MaintenanceJob>>;
	async fn update_status(
		&self,
		id: Uuid,
		status: MaintenanceJobStatus,
		error: Option<String>,
	) -> Result<()>;
	async fn mark_started(&self, id: Uuid) -> Result<()>;
	async fn mark_finished(&self, id: Uuid, status: MaintenanceJobStatus, error: Option<String>) -> Result<()>;
}

pub struct SqliteMaintenanceJobStore {
	pool: SqlitePool,
}

impl SqliteMaintenanceJobStore {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	fn row_to_job(&self, row: &sqlx::sqlite::SqliteRow) -> Result<MaintenanceJob> {
		let id_str: String = row.get("id");
		let repo_id_str: Option<String> = row.get("repo_id");
		let task_str: String = row.get("task");
		let status_str: String = row.get("status");
		let started_at_str: Option<String> = row.get("started_at");
		let finished_at_str: Option<String> = row.get("finished_at");
		let created_at_str: String = row.get("created_at");

		Ok(MaintenanceJob {
			id: Uuid::parse_str(&id_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			repo_id: repo_id_str
				.map(|s| Uuid::parse_str(&s))
				.transpose()
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			task: task_str.parse::<MaintenanceTask>().map_err(|_| {
				ScmError::Database(sqlx::Error::Decode(
					format!("invalid task: {}", task_str).into(),
				))
			})?,
			status: status_str.parse::<MaintenanceJobStatus>().map_err(|_| {
				ScmError::Database(sqlx::Error::Decode(
					format!("invalid status: {}", status_str).into(),
				))
			})?,
			started_at: started_at_str
				.map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)))
				.transpose()
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			finished_at: finished_at_str
				.map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)))
				.transpose()
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			error: row.get("error"),
			created_at: DateTime::parse_from_rfc3339(&created_at_str)
				.map(|d| d.with_timezone(&Utc))
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
		})
	}
}

#[async_trait]
impl MaintenanceJobStore for SqliteMaintenanceJobStore {
	async fn create(&self, job: &MaintenanceJob) -> Result<MaintenanceJob> {
		sqlx::query(
			r#"
			INSERT INTO repo_maintenance_jobs (id, repo_id, task, status, started_at, finished_at, error, created_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(job.id.to_string())
		.bind(job.repo_id.map(|id| id.to_string()))
		.bind(job.task.as_str())
		.bind(job.status.as_str())
		.bind(job.started_at.map(|t| t.to_rfc3339()))
		.bind(job.finished_at.map(|t| t.to_rfc3339()))
		.bind(&job.error)
		.bind(job.created_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(job.clone())
	}

	async fn get_by_id(&self, id: Uuid) -> Result<Option<MaintenanceJob>> {
		let row = sqlx::query(
			r#"
			SELECT id, repo_id, task, status, started_at, finished_at, error, created_at
			FROM repo_maintenance_jobs
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_job(&r)).transpose()
	}

	async fn list_by_repo(&self, repo_id: Uuid, limit: u32) -> Result<Vec<MaintenanceJob>> {
		let rows = sqlx::query(
			r#"
			SELECT id, repo_id, task, status, started_at, finished_at, error, created_at
			FROM repo_maintenance_jobs
			WHERE repo_id = ?
			ORDER BY created_at DESC
			LIMIT ?
			"#,
		)
		.bind(repo_id.to_string())
		.bind(limit as i64)
		.fetch_all(&self.pool)
		.await?;

		rows.iter().map(|r| self.row_to_job(r)).collect()
	}

	async fn list_pending(&self) -> Result<Vec<MaintenanceJob>> {
		let rows = sqlx::query(
			r#"
			SELECT id, repo_id, task, status, started_at, finished_at, error, created_at
			FROM repo_maintenance_jobs
			WHERE status = 'pending'
			ORDER BY created_at ASC
			"#,
		)
		.fetch_all(&self.pool)
		.await?;

		rows.iter().map(|r| self.row_to_job(r)).collect()
	}

	async fn update_status(
		&self,
		id: Uuid,
		status: MaintenanceJobStatus,
		error: Option<String>,
	) -> Result<()> {
		let result = sqlx::query(
			r#"
			UPDATE repo_maintenance_jobs
			SET status = ?, error = ?
			WHERE id = ?
			"#,
		)
		.bind(status.as_str())
		.bind(&error)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(ScmError::NotFound);
		}

		Ok(())
	}

	async fn mark_started(&self, id: Uuid) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		let result = sqlx::query(
			r#"
			UPDATE repo_maintenance_jobs
			SET status = 'running', started_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&now)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(ScmError::NotFound);
		}

		Ok(())
	}

	async fn mark_finished(
		&self,
		id: Uuid,
		status: MaintenanceJobStatus,
		error: Option<String>,
	) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		let result = sqlx::query(
			r#"
			UPDATE repo_maintenance_jobs
			SET status = ?, finished_at = ?, error = ?
			WHERE id = ?
			"#,
		)
		.bind(status.as_str())
		.bind(&now)
		.bind(&error)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(ScmError::NotFound);
		}

		Ok(())
	}
}

// Note: Maintenance migrations are now managed by loom-server in:
// - migrations/018_scm_maintenance.sql (repo_maintenance_jobs)

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_maintenance_task_conversion() {
		assert_eq!(MaintenanceTask::Gc.as_str(), "gc");
		assert_eq!(MaintenanceTask::Prune.as_str(), "prune");
		assert_eq!(MaintenanceTask::Repack.as_str(), "repack");
		assert_eq!(MaintenanceTask::Fsck.as_str(), "fsck");
		assert_eq!(MaintenanceTask::All.as_str(), "all");

		assert_eq!("gc".parse::<MaintenanceTask>(), Ok(MaintenanceTask::Gc));
		assert_eq!("prune".parse::<MaintenanceTask>(), Ok(MaintenanceTask::Prune));
		assert_eq!("repack".parse::<MaintenanceTask>(), Ok(MaintenanceTask::Repack));
		assert_eq!("fsck".parse::<MaintenanceTask>(), Ok(MaintenanceTask::Fsck));
		assert_eq!("all".parse::<MaintenanceTask>(), Ok(MaintenanceTask::All));
		assert!("invalid".parse::<MaintenanceTask>().is_err());
	}

	#[test]
	fn test_maintenance_job_status_conversion() {
		assert_eq!(MaintenanceJobStatus::Pending.as_str(), "pending");
		assert_eq!(MaintenanceJobStatus::Running.as_str(), "running");
		assert_eq!(MaintenanceJobStatus::Success.as_str(), "success");
		assert_eq!(MaintenanceJobStatus::Failed.as_str(), "failed");

		assert_eq!("pending".parse::<MaintenanceJobStatus>(), Ok(MaintenanceJobStatus::Pending));
		assert_eq!("running".parse::<MaintenanceJobStatus>(), Ok(MaintenanceJobStatus::Running));
		assert_eq!("success".parse::<MaintenanceJobStatus>(), Ok(MaintenanceJobStatus::Success));
		assert_eq!("failed".parse::<MaintenanceJobStatus>(), Ok(MaintenanceJobStatus::Failed));
		assert!("invalid".parse::<MaintenanceJobStatus>().is_err());
	}

	#[test]
	fn test_maintenance_job_new() {
		let job = MaintenanceJob::new(Some(Uuid::new_v4()), MaintenanceTask::Gc);
		assert_eq!(job.task, MaintenanceTask::Gc);
		assert_eq!(job.status, MaintenanceJobStatus::Pending);
		assert!(job.started_at.is_none());
		assert!(job.finished_at.is_none());
		assert!(job.error.is_none());
	}
}
