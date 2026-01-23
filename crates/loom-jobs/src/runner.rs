// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use crate::error::Result;
use async_trait::async_trait;
use std::time::Instant;

/// Trait for jobs that can be executed.
#[async_trait]
pub trait Job: Send + Sync {
	/// Returns the unique identifier for this job.
	fn id(&self) -> &str;

	/// Returns a human-readable name for this job.
	fn name(&self) -> &str;

	/// Executes the job and returns its result.
	async fn run(&self) -> Result<()>;
}

/// Simple job runner that executes jobs with optional monitoring.
pub struct JobRunner {
	name: String,
	job: Box<dyn Job>,
}

impl JobRunner {
	/// Creates a new job runner for the given job.
	pub fn new(job: Box<dyn Job>) -> Self {
		let name = job.name().to_string();
		Self { name, job }
	}

	/// Executes the job and returns the result.
	///
	/// This method returns the elapsed time in milliseconds for monitoring purposes.
	pub async fn execute(&self) -> Result<Instant> {
		let start = Instant::now();
		self.job.run().await?;
		Ok(start)
	}

	/// Returns the job name.
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Returns the job ID.
	pub fn id(&self) -> &str {
		self.job.id()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct TestJob {
		id: String,
		name: String,
	}

	#[async_trait]
	impl Job for TestJob {
		fn id(&self) -> &str {
			&self.id
		}

		fn name(&self) -> &str {
			&self.name
		}

		async fn run(&self) -> Result<()> {
			Ok(())
		}
	}

	#[tokio::test]
	async fn test_runner_creates() {
		let job = Box::new(TestJob {
			id: "test-job".to_string(),
			name: "Test Job".to_string(),
		});
		let runner = JobRunner::new(job);

		assert_eq!(runner.id(), "test-job");
		assert_eq!(runner.name(), "Test Job");
	}

	#[tokio::test]
	async fn test_runner_executes_successfully() {
		let job = Box::new(TestJob {
			id: "test-job".to_string(),
			name: "Test Job".to_string(),
		});
		let runner = JobRunner::new(job);

		let start = runner.execute().await.unwrap();
		assert!(start.elapsed().as_millis() < 100);
	}
}
