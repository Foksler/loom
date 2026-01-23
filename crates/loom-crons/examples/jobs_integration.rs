// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Example demonstrating loom-jobs auto-instrumentation with loom-crons.

use loom_crons::MonitoredJob;
use loom_jobs::Job;
use std::time::Duration;

/// Example job that simulates a daily backup task.
struct BackupJob {
	id: String,
	name: String,
}

impl BackupJob {
	fn new() -> Self {
		Self {
			id: "daily-backup".to_string(),
			name: "Daily Backup".to_string(),
		}
	}
}

#[async_trait::async_trait]
impl Job for BackupJob {
	fn id(&self) -> &str {
		&self.id
	}

	fn name(&self) -> &str {
		&self.name
	}

	async fn run(&self) -> loom_jobs::Result<()> {
		tokio::time::sleep(Duration::from_millis(100)).await;
		Ok(())
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let job = BackupJob::new();

	let crons_client = loom_crons::CronsClient::builder()
		.auth_token("your_auth_token")
		.base_url("https://loom.ghuntley.com")
		.org_id("your_org_id")
		.environment("production")
		.release("1.0.0")
		.build()?;

	let monitored_job = MonitoredJob::new(job, crons_client, "daily-backup");

	monitored_job.run().await?;

	Ok(())
}
