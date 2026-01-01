// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use loom_jobs::{Job, JobContext, JobError, JobOutput};
use sqlx::SqlitePool;
use tracing::instrument;

pub struct SessionCleanupJob {
	pool: SqlitePool,
}

impl SessionCleanupJob {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

#[async_trait]
impl Job for SessionCleanupJob {
	fn id(&self) -> &str {
		"session-cleanup"
	}

	fn name(&self) -> &str {
		"Session Cleanup"
	}

	fn description(&self) -> &str {
		"Delete expired user sessions from database"
	}

	#[instrument(skip(self, ctx), fields(job_id = "session-cleanup"))]
	async fn run(&self, ctx: &JobContext) -> Result<JobOutput, JobError> {
		if ctx.cancellation_token.is_cancelled() {
			return Err(JobError::Cancelled);
		}

		let now = chrono::Utc::now().to_rfc3339();

		let sessions_result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
			.bind(&now)
			.execute(&self.pool)
			.await
			.map_err(|e| JobError::Failed {
				message: e.to_string(),
				retryable: true,
			})?;

		let tokens_result =
			sqlx::query("DELETE FROM access_tokens WHERE expires_at < ? AND revoked_at IS NULL")
				.bind(&now)
				.execute(&self.pool)
				.await
				.map_err(|e| JobError::Failed {
					message: e.to_string(),
					retryable: true,
				})?;

		let device_result = sqlx::query("DELETE FROM device_codes WHERE expires_at < ?")
			.bind(&now)
			.execute(&self.pool)
			.await
			.map_err(|e| JobError::Failed {
				message: e.to_string(),
				retryable: true,
			})?;

		let magic_result = sqlx::query("DELETE FROM magic_links WHERE expires_at < ?")
			.bind(&now)
			.execute(&self.pool)
			.await
			.map_err(|e| JobError::Failed {
				message: e.to_string(),
				retryable: true,
			})?;

		let sessions_deleted = sessions_result.rows_affected();
		let tokens_deleted = tokens_result.rows_affected();
		let device_codes_deleted = device_result.rows_affected();
		let magic_links_deleted = magic_result.rows_affected();
		let total = sessions_deleted + tokens_deleted + device_codes_deleted + magic_links_deleted;

		tracing::info!(
			sessions_deleted,
			tokens_deleted,
			device_codes_deleted,
			magic_links_deleted,
			total,
			"Session cleanup completed"
		);

		Ok(JobOutput {
			message: format!("Deleted {} expired auth records", total),
			metadata: Some(serde_json::json!({
				"sessions_deleted": sessions_deleted,
				"tokens_deleted": tokens_deleted,
				"device_codes_deleted": device_codes_deleted,
				"magic_links_deleted": magic_links_deleted,
			})),
		})
	}
}
