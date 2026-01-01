// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use crate::error::{JobError, Result};
use crate::types::{JobDefinition, JobRun, JobStatus};
use chrono::{DateTime, Utc};
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
        sqlx::query(
            r#"
            INSERT INTO job_definitions (id, name, description, job_type, interval_secs, enabled)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                job_type = excluded.job_type,
                interval_secs = excluded.interval_secs,
                enabled = excluded.enabled
            "#,
        )
        .bind(&def.id)
        .bind(&def.name)
        .bind(&def.description)
        .bind(&def.job_type)
        .bind(def.interval_secs)
        .bind(def.enabled)
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

        Ok(row.map(|(id, name, description, job_type, interval_secs, enabled)| JobDefinition {
            id,
            name,
            description,
            job_type,
            interval_secs,
            enabled,
        }))
    }

    #[instrument(skip(self))]
    pub async fn list_definitions(&self) -> Result<Vec<JobDefinition>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<i64>, bool)>(
            "SELECT id, name, description, job_type, interval_secs, enabled FROM job_definitions ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, name, description, job_type, interval_secs, enabled)| JobDefinition {
                id,
                name,
                description,
                job_type,
                interval_secs,
                enabled,
            })
            .collect())
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

        row.map(|(id, job_id, status, started_at, completed_at, duration_ms, error_message, retry_count, triggered_by, metadata)| {
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
                metadata: metadata.as_deref().and_then(|s| serde_json::from_str(s).ok()),
            })
        }).transpose()
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

        rows.into_iter()
            .map(|(id, job_id, status, started_at, completed_at, duration_ms, error_message, retry_count, triggered_by, metadata)| {
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
                    metadata: metadata.as_deref().and_then(|s| serde_json::from_str(s).ok()),
                })
            })
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

        row.map(|(id, job_id, status, started_at, completed_at, duration_ms, error_message, retry_count, triggered_by, metadata)| {
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
                metadata: metadata.as_deref().and_then(|s| serde_json::from_str(s).ok()),
            })
        }).transpose()
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
