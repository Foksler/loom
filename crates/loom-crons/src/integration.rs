// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Integration with loom-jobs for automatic job monitoring.
//!
//! This module provides utilities to automatically instrument jobs from the
//! loom-jobs crate with crons monitoring.

use crate::{CheckInError, CheckInOk, CronsClient, Result};
use loom_jobs::Job;
use std::sync::Arc;
use std::time::Instant;

/// Wrapper for a job with crons monitoring enabled.
///
/// This struct wraps a `loom_jobs::Job` and automatically reports
/// its execution to the crons monitoring system.
pub struct MonitoredJob {
	inner: Arc<dyn Job>,
	crons_client: CronsClient,
	monitor_slug: String,
}

impl MonitoredJob {
	/// Creates a new monitored job wrapper.
	///
	/// # Arguments
	///
	/// * `job` - The job to monitor
	/// * `crons_client` - The crons client for reporting
	/// * `monitor_slug` - The slug of the monitor to report to
	///
	/// # Example
	///
	/// ```ignore
	/// use loom_crons::MonitoredJob;
	/// use loom_crons::CronsClient;
	/// use loom_jobs::Job;
	///
	/// let job = MyJob::new();
	/// let crons = CronsClient::builder()
	///     .auth_token("token")
	///     .base_url("https://loom.example.com")
	///     .org_id("org_123")
	///     .build()?;
	///
	/// let monitored = MonitoredJob::new(
	///     job,
	///     crons,
	///     "daily-backup"
	/// )?;
	///
	/// monitored.run().await?;
	/// ```
	pub fn new<J>(job: J, crons_client: CronsClient, monitor_slug: impl Into<String>) -> Self
	where
		J: Job + 'static,
	{
		Self {
			inner: Arc::new(job),
			crons_client,
			monitor_slug: monitor_slug.into(),
		}
	}

	/// Executes the job with automatic crons monitoring.
	///
	/// This method:
	/// 1. Starts a check-in with the crons service
	/// 2. Executes the job
	/// 3. Completes the check-in with success or error
	pub async fn run(&self) -> Result<()> {
		let start = Instant::now();

		let checkin_id = self.crons_client.checkin_start(&self.monitor_slug).await?;

		let result = self.inner.run().await;
		let duration_ms = start.elapsed().as_millis() as u64;

		match result {
			Ok(_) => {
				self
					.crons_client
					.checkin_ok(
						checkin_id,
						CheckInOk {
							duration_ms: Some(duration_ms),
							output: None,
						},
					)
					.await?;
				Ok(())
			}
			Err(e) => {
				self
					.crons_client
					.checkin_error(
						checkin_id,
						CheckInError {
							duration_ms: Some(duration_ms),
							exit_code: Some(1),
							output: Some(e.to_string()),
							crash_event_id: None,
						},
					)
					.await?;
				Err(crate::error::CronsSdkError::JobFailed(e.to_string()))
			}
		}
	}

	/// Returns the job ID.
	pub fn id(&self) -> &str {
		self.inner.id()
	}

	/// Returns the job name.
	pub fn name(&self) -> &str {
		self.inner.name()
	}

	/// Returns the monitor slug.
	pub fn monitor_slug(&self) -> &str {
		&self.monitor_slug
	}

	/// Returns a reference to the inner job.
	pub fn inner(&self) -> &Arc<dyn Job> {
		&self.inner
	}

	/// Returns a reference to the crons client.
	pub fn crons_client(&self) -> &CronsClient {
		&self.crons_client
	}
}

/// Extension trait for `loom_jobs::JobRunner` to enable crons monitoring.
///
/// This trait adds a `with_crons_monitoring` method to job runners
/// for automatic instrumentation.
pub trait WithCronsMonitoring {
	/// Wraps the job runner with crons monitoring.
	///
	/// # Arguments
	///
	/// * `crons_client` - The crons client for reporting
	/// * `monitor_slug` - The slug of the monitor to report to
	///
	/// # Returns
	///
	/// A new runner that automatically reports to crons
	fn with_crons_monitoring(
		self,
		crons_client: CronsClient,
		monitor_slug: impl Into<String>,
	) -> MonitoredJob;
}

#[cfg(test)]
mod tests {
	use super::*;

	struct TestJob {
		id: String,
		name: String,
	}

	#[async_trait::async_trait]
	impl Job for TestJob {
		fn id(&self) -> &str {
			&self.id
		}

		fn name(&self) -> &str {
			&self.name
		}

		async fn run(&self) -> loom_jobs::Result<()> {
			Ok(())
		}
	}

	fn create_mock_client() -> CronsClient {
		CronsClient::builder()
			.auth_token("test-token")
			.base_url("https://test.example.com")
			.org_id("test-org")
			.build()
			.unwrap()
	}

	#[tokio::test]
	async fn test_monitored_job_creates() {
		let job = TestJob {
			id: "test-job".to_string(),
			name: "Test Job".to_string(),
		};

		let mock_client = create_mock_client();

		let monitored = MonitoredJob::new(job, mock_client, "test-monitor");

		assert_eq!(monitored.id(), "test-job");
		assert_eq!(monitored.name(), "Test Job");
		assert_eq!(monitored.monitor_slug(), "test-monitor");
		assert_eq!(monitored.inner().id(), "test-job");
		assert!(!monitored.crons_client().is_closed());
	}

	#[tokio::test]
	async fn test_monitored_job_accessors() {
		let job = TestJob {
			id: "test-job".to_string(),
			name: "Test Job".to_string(),
		};

		let mock_client = create_mock_client();

		let monitored = MonitoredJob::new(job, mock_client, "test-monitor");

		assert_eq!(monitored.id(), "test-job");
		assert_eq!(monitored.name(), "Test Job");
		assert_eq!(monitored.monitor_slug(), "test-monitor");
		assert!(!monitored.crons_client().is_closed());
	}
}
