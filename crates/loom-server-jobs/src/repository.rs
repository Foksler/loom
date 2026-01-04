// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use crate::error::{JobError, Result};
use crate::types::{JobDefinition, JobRun, JobStatus};
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::SqlitePool;
use tracing::instrument;

pub struct JobRepository {
	pool: SqlitePool,
}

impl JobRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	#[instrument(skip(self, def), fields(job_id = %def.id))]
	pub async fn upsert_definition(&self, def: &JobDefinition) -> Result<()> {
		let now = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
		sqlx::query(
            r#"
            INSERT INTO job_definitions (id, name, description, job_type, interval_secs, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                job_type = excluded.job_type,
                interval_secs = excluded.interval_secs,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&def.id)
        .bind(&def.name)
        .bind(&def.description)
        .bind(&def.job_type)
        .bind(def.interval_secs)
        .bind(def.enabled)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

		Ok(())
	}

	#[instrument(skip(self))]
	pub async fn get_definition(&self, id: &str) -> Result<Option<JobDefinition>> {
		let row = sqlx::query_as::<_, (String, String, String, String, Option<i64>, bool)>(
            "SELECT id, name, description, job_type, interval_secs, enabled FROM job_definitions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

		Ok(row.map(
			|(id, name, description, job_type, interval_secs, enabled)| JobDefinition {
				id,
				name,
				description,
				job_type,
				interval_secs,
				enabled,
			},
		))
	}

	#[instrument(skip(self))]
	pub async fn list_definitions(&self) -> Result<Vec<JobDefinition>> {
		let rows = sqlx::query_as::<_, (String, String, String, String, Option<i64>, bool)>(
            "SELECT id, name, description, job_type, interval_secs, enabled FROM job_definitions ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;

		Ok(
			rows
				.into_iter()
				.map(
					|(id, name, description, job_type, interval_secs, enabled)| JobDefinition {
						id,
						name,
						description,
						job_type,
						interval_secs,
						enabled,
					},
				)
				.collect(),
		)
	}

	#[instrument(skip(self))]
	pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
		let result = sqlx::query("UPDATE job_definitions SET enabled = ? WHERE id = ?")
			.bind(enabled)
			.bind(id)
			.execute(&self.pool)
			.await?;

		if result.rows_affected() == 0 {
			return Err(JobError::NotFound(id.to_string()));
		}

		Ok(())
	}

	#[instrument(skip(self, run), fields(run_id = %run.id, job_id = %run.job_id))]
	pub async fn record_run_start(&self, run: &JobRun) -> Result<()> {
		sqlx::query(
			r#"
            INSERT INTO job_runs (id, job_id, status, started_at, retry_count, triggered_by)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
		)
		.bind(&run.id)
		.bind(&run.job_id)
		.bind(run.status.as_str())
		.bind(run.started_at)
		.bind(run.retry_count as i64)
		.bind(run.triggered_by.as_str())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self, metadata))]
	pub async fn record_run_complete(
		&self,
		run_id: &str,
		status: JobStatus,
		error: Option<String>,
		metadata: Option<serde_json::Value>,
	) -> Result<()> {
		let now = Utc::now();
		let metadata_str = metadata.map(|m| m.to_string());

		sqlx::query(
			r#"
            UPDATE job_runs
            SET status = ?,
                completed_at = ?,
                duration_ms = CAST((julianday(?) - julianday(started_at)) * 86400000 AS INTEGER),
                error_message = ?,
                metadata = ?
            WHERE id = ?
            "#,
		)
		.bind(status.as_str())
		.bind(now)
		.bind(now)
		.bind(error)
		.bind(metadata_str)
		.bind(run_id)
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self))]
	pub async fn get_run(&self, run_id: &str) -> Result<Option<JobRun>> {
		let row = sqlx::query_as::<_, (String, String, String, DateTime<Utc>, Option<DateTime<Utc>>, Option<i64>, Option<String>, i64, String, Option<String>)>(
            r#"
            SELECT id, job_id, status, started_at, completed_at, duration_ms, error_message, retry_count, triggered_by, metadata
            FROM job_runs
            WHERE id = ?
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

		row
			.map(
				|(
					id,
					job_id,
					status,
					started_at,
					completed_at,
					duration_ms,
					error_message,
					retry_count,
					triggered_by,
					metadata,
				)| {
					Ok(JobRun {
						id,
						job_id,
						status: status.parse().map_err(JobError::Repository)?,
						started_at,
						completed_at,
						duration_ms,
						error_message,
						retry_count: retry_count as u32,
						triggered_by: triggered_by.parse().map_err(JobError::Repository)?,
						metadata: metadata
							.as_deref()
							.and_then(|s| serde_json::from_str(s).ok()),
					})
				},
			)
			.transpose()
	}

	#[instrument(skip(self))]
	pub async fn list_runs(&self, job_id: &str, limit: u32, offset: u32) -> Result<Vec<JobRun>> {
		let rows = sqlx::query_as::<_, (String, String, String, DateTime<Utc>, Option<DateTime<Utc>>, Option<i64>, Option<String>, i64, String, Option<String>)>(
            r#"
            SELECT id, job_id, status, started_at, completed_at, duration_ms, error_message, retry_count, triggered_by, metadata
            FROM job_runs
            WHERE job_id = ?
            ORDER BY started_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(job_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

		rows
			.into_iter()
			.map(
				|(
					id,
					job_id,
					status,
					started_at,
					completed_at,
					duration_ms,
					error_message,
					retry_count,
					triggered_by,
					metadata,
				)| {
					Ok(JobRun {
						id,
						job_id,
						status: status.parse().map_err(JobError::Repository)?,
						started_at,
						completed_at,
						duration_ms,
						error_message,
						retry_count: retry_count as u32,
						triggered_by: triggered_by.parse().map_err(JobError::Repository)?,
						metadata: metadata
							.as_deref()
							.and_then(|s| serde_json::from_str(s).ok()),
					})
				},
			)
			.collect()
	}

	#[instrument(skip(self))]
	pub async fn get_last_run(&self, job_id: &str) -> Result<Option<JobRun>> {
		let row = sqlx::query_as::<_, (String, String, String, DateTime<Utc>, Option<DateTime<Utc>>, Option<i64>, Option<String>, i64, String, Option<String>)>(
            r#"
            SELECT id, job_id, status, started_at, completed_at, duration_ms, error_message, retry_count, triggered_by, metadata
            FROM job_runs
            WHERE job_id = ?
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

		row
			.map(
				|(
					id,
					job_id,
					status,
					started_at,
					completed_at,
					duration_ms,
					error_message,
					retry_count,
					triggered_by,
					metadata,
				)| {
					Ok(JobRun {
						id,
						job_id,
						status: status.parse().map_err(JobError::Repository)?,
						started_at,
						completed_at,
						duration_ms,
						error_message,
						retry_count: retry_count as u32,
						triggered_by: triggered_by.parse().map_err(JobError::Repository)?,
						metadata: metadata
							.as_deref()
							.and_then(|s| serde_json::from_str(s).ok()),
					})
				},
			)
			.transpose()
	}

	#[instrument(skip(self))]
	pub async fn count_consecutive_failures(&self, job_id: &str) -> Result<u32> {
		let row = sqlx::query_as::<_, (i64,)>(
			r#"
            WITH ranked AS (
                SELECT status,
                       ROW_NUMBER() OVER (ORDER BY started_at DESC) as rn
                FROM job_runs
                WHERE job_id = ?
            )
            SELECT COUNT(*) as count
            FROM ranked
            WHERE status = 'failed'
              AND rn <= (
                  SELECT COALESCE(MIN(rn) - 1, (SELECT COUNT(*) FROM ranked))
                  FROM ranked
                  WHERE status != 'failed'
              )
            "#,
		)
		.bind(job_id)
		.fetch_one(&self.pool)
		.await?;

		Ok(row.0 as u32)
	}

	#[instrument(skip(self))]
	pub async fn delete_old_runs(&self, before: DateTime<Utc>) -> Result<u64> {
		let result = sqlx::query("DELETE FROM job_runs WHERE completed_at < ?")
			.bind(before)
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected())
	}

	#[instrument(skip(self))]
	pub async fn cleanup_old_runs(&self, retention_days: u32) -> Result<u64> {
		let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
		self.delete_old_runs(cutoff).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::{JobDefinition, JobRun, JobStatus, TriggerSource};

	async fn setup_db() -> SqlitePool {
		let pool = SqlitePool::connect(":memory:").await.unwrap();

		sqlx::query(
			r#"
            CREATE TABLE IF NOT EXISTS job_definitions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                job_type TEXT NOT NULL,
                interval_secs INTEGER,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
		)
		.execute(&pool)
		.await
		.unwrap();

		sqlx::query(
			r#"
            CREATE TABLE IF NOT EXISTS job_runs (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL REFERENCES job_definitions(id),
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                duration_ms INTEGER,
                error_message TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                triggered_by TEXT NOT NULL,
                metadata TEXT
            )
            "#,
		)
		.execute(&pool)
		.await
		.unwrap();

		pool
	}

	fn make_definition(id: &str, name: &str) -> JobDefinition {
		JobDefinition {
			id: id.to_string(),
			name: name.to_string(),
			description: "Test job".to_string(),
			job_type: "periodic".to_string(),
			interval_secs: Some(60),
			enabled: true,
		}
	}

	fn make_run(id: &str, job_id: &str, status: JobStatus) -> JobRun {
		JobRun {
			id: id.to_string(),
			job_id: job_id.to_string(),
			status,
			started_at: Utc::now(),
			completed_at: None,
			duration_ms: None,
			error_message: None,
			retry_count: 0,
			triggered_by: TriggerSource::Schedule,
			metadata: None,
		}
	}

	#[tokio::test]
	async fn test_upsert_definition_insert() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let def = make_definition("job-1", "Test Job");
		repo.upsert_definition(&def).await.unwrap();

		let fetched = repo.get_definition("job-1").await.unwrap().unwrap();
		assert_eq!(fetched.id, "job-1");
		assert_eq!(fetched.name, "Test Job");
		assert!(fetched.enabled);
	}

	#[tokio::test]
	async fn test_upsert_definition_update() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let def = make_definition("job-1", "Original Name");
		repo.upsert_definition(&def).await.unwrap();

		let mut updated_def = def.clone();
		updated_def.name = "Updated Name".to_string();
		updated_def.enabled = false;
		repo.upsert_definition(&updated_def).await.unwrap();

		let fetched = repo.get_definition("job-1").await.unwrap().unwrap();
		assert_eq!(fetched.name, "Updated Name");
		assert!(!fetched.enabled);
	}

	#[tokio::test]
	async fn test_record_run_start_and_complete() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let def = make_definition("job-1", "Test Job");
		repo.upsert_definition(&def).await.unwrap();

		let run = make_run("run-1", "job-1", JobStatus::Running);
		repo.record_run_start(&run).await.unwrap();

		let fetched = repo.get_run("run-1").await.unwrap().unwrap();
		assert_eq!(fetched.status, JobStatus::Running);
		assert!(fetched.completed_at.is_none());

		repo
			.record_run_complete("run-1", JobStatus::Succeeded, None, None)
			.await
			.unwrap();

		let completed = repo.get_run("run-1").await.unwrap().unwrap();
		assert_eq!(completed.status, JobStatus::Succeeded);
		assert!(completed.completed_at.is_some());
	}

	#[tokio::test]
	async fn test_record_run_complete_with_error() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let def = make_definition("job-1", "Test Job");
		repo.upsert_definition(&def).await.unwrap();

		let run = make_run("run-1", "job-1", JobStatus::Running);
		repo.record_run_start(&run).await.unwrap();

		repo
			.record_run_complete(
				"run-1",
				JobStatus::Failed,
				Some("Something went wrong".to_string()),
				None,
			)
			.await
			.unwrap();

		let completed = repo.get_run("run-1").await.unwrap().unwrap();
		assert_eq!(completed.status, JobStatus::Failed);
		assert_eq!(
			completed.error_message.as_deref(),
			Some("Something went wrong")
		);
	}

	#[tokio::test]
	async fn test_get_last_run() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let def = make_definition("job-1", "Test Job");
		repo.upsert_definition(&def).await.unwrap();

		assert!(repo.get_last_run("job-1").await.unwrap().is_none());

		let run1 = JobRun {
			id: "run-1".to_string(),
			job_id: "job-1".to_string(),
			status: JobStatus::Running,
			started_at: Utc::now() - chrono::Duration::hours(1),
			completed_at: None,
			duration_ms: None,
			error_message: None,
			retry_count: 0,
			triggered_by: TriggerSource::Schedule,
			metadata: None,
		};
		repo.record_run_start(&run1).await.unwrap();

		let run2 = JobRun {
			id: "run-2".to_string(),
			job_id: "job-1".to_string(),
			status: JobStatus::Running,
			started_at: Utc::now(),
			completed_at: None,
			duration_ms: None,
			error_message: None,
			retry_count: 0,
			triggered_by: TriggerSource::Schedule,
			metadata: None,
		};
		repo.record_run_start(&run2).await.unwrap();

		let last = repo.get_last_run("job-1").await.unwrap().unwrap();
		assert_eq!(last.id, "run-2");
	}

	#[tokio::test]
	async fn test_count_consecutive_failures_all_failed() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let def = make_definition("job-1", "Test Job");
		repo.upsert_definition(&def).await.unwrap();

		for i in 0..3 {
			let run = JobRun {
				id: format!("run-{i}"),
				job_id: "job-1".to_string(),
				status: JobStatus::Failed,
				started_at: Utc::now() - chrono::Duration::minutes(3 - i),
				completed_at: Some(Utc::now() - chrono::Duration::minutes(3 - i)),
				duration_ms: Some(100),
				error_message: Some("Error".to_string()),
				retry_count: 0,
				triggered_by: TriggerSource::Schedule,
				metadata: None,
			};
			repo.record_run_start(&run).await.unwrap();
			repo
				.record_run_complete(&run.id, JobStatus::Failed, Some("Error".to_string()), None)
				.await
				.unwrap();
		}

		let count = repo.count_consecutive_failures("job-1").await.unwrap();
		assert_eq!(count, 3);
	}

	#[tokio::test]
	async fn test_count_consecutive_failures_with_success() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let def = make_definition("job-1", "Test Job");
		repo.upsert_definition(&def).await.unwrap();

		let success_run = JobRun {
			id: "run-0".to_string(),
			job_id: "job-1".to_string(),
			status: JobStatus::Succeeded,
			started_at: Utc::now() - chrono::Duration::minutes(10),
			completed_at: None,
			duration_ms: None,
			error_message: None,
			retry_count: 0,
			triggered_by: TriggerSource::Schedule,
			metadata: None,
		};
		repo.record_run_start(&success_run).await.unwrap();
		repo
			.record_run_complete("run-0", JobStatus::Succeeded, None, None)
			.await
			.unwrap();

		for i in 1..=2 {
			let run = JobRun {
				id: format!("run-{i}"),
				job_id: "job-1".to_string(),
				status: JobStatus::Failed,
				started_at: Utc::now() - chrono::Duration::minutes(5 - i as i64),
				completed_at: None,
				duration_ms: None,
				error_message: None,
				retry_count: 0,
				triggered_by: TriggerSource::Schedule,
				metadata: None,
			};
			repo.record_run_start(&run).await.unwrap();
			repo
				.record_run_complete(&run.id, JobStatus::Failed, Some("Error".to_string()), None)
				.await
				.unwrap();
		}

		let count = repo.count_consecutive_failures("job-1").await.unwrap();
		assert_eq!(count, 2);
	}

	#[tokio::test]
	async fn test_count_consecutive_failures_no_runs() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let count = repo
			.count_consecutive_failures("nonexistent")
			.await
			.unwrap();
		assert_eq!(count, 0);
	}

	#[tokio::test]
	async fn test_cleanup_old_runs() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let def = make_definition("job-1", "Test Job");
		repo.upsert_definition(&def).await.unwrap();

		let old_run = JobRun {
			id: "old-run".to_string(),
			job_id: "job-1".to_string(),
			status: JobStatus::Running,
			started_at: Utc::now() - chrono::Duration::days(10),
			completed_at: None,
			duration_ms: None,
			error_message: None,
			retry_count: 0,
			triggered_by: TriggerSource::Schedule,
			metadata: None,
		};
		repo.record_run_start(&old_run).await.unwrap();
		repo
			.record_run_complete("old-run", JobStatus::Succeeded, None, None)
			.await
			.unwrap();

		sqlx::query("UPDATE job_runs SET completed_at = ? WHERE id = ?")
			.bind(Utc::now() - chrono::Duration::days(10))
			.bind("old-run")
			.execute(&repo.pool)
			.await
			.unwrap();

		let new_run = JobRun {
			id: "new-run".to_string(),
			job_id: "job-1".to_string(),
			status: JobStatus::Running,
			started_at: Utc::now(),
			completed_at: None,
			duration_ms: None,
			error_message: None,
			retry_count: 0,
			triggered_by: TriggerSource::Schedule,
			metadata: None,
		};
		repo.record_run_start(&new_run).await.unwrap();
		repo
			.record_run_complete("new-run", JobStatus::Succeeded, None, None)
			.await
			.unwrap();

		let deleted = repo.cleanup_old_runs(7).await.unwrap();
		assert_eq!(deleted, 1);

		assert!(repo.get_run("old-run").await.unwrap().is_none());
		assert!(repo.get_run("new-run").await.unwrap().is_some());
	}

	#[tokio::test]
	async fn test_delete_old_runs_before_cutoff() {
		let pool = setup_db().await;
		let repo = JobRepository::new(pool);

		let def = make_definition("job-1", "Test Job");
		repo.upsert_definition(&def).await.unwrap();

		for i in 0..5 {
			let run = JobRun {
				id: format!("run-{i}"),
				job_id: "job-1".to_string(),
				status: JobStatus::Running,
				started_at: Utc::now() - chrono::Duration::days(i + 1),
				completed_at: None,
				duration_ms: None,
				error_message: None,
				retry_count: 0,
				triggered_by: TriggerSource::Schedule,
				metadata: None,
			};
			repo.record_run_start(&run).await.unwrap();
			repo
				.record_run_complete(&run.id, JobStatus::Succeeded, None, None)
				.await
				.unwrap();

			sqlx::query("UPDATE job_runs SET completed_at = ? WHERE id = ?")
				.bind(Utc::now() - chrono::Duration::days(i + 1))
				.bind(&run.id)
				.execute(&repo.pool)
				.await
				.unwrap();
		}

		let cutoff = Utc::now() - chrono::Duration::days(3);
		let deleted = repo.delete_old_runs(cutoff).await.unwrap();
		assert_eq!(deleted, 3);

		let remaining = repo.list_runs("job-1", 10, 0).await.unwrap();
		assert_eq!(remaining.len(), 2);
	}
}
