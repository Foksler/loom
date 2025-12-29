// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Background cleanup task for expired agents.

use std::sync::Arc;
use std::time::Duration;

use crate::provisioner::Provisioner;

/// Start the background cleanup task that periodically removes expired agents.
///
/// This task runs cleanup immediately on start (for reconciliation after restart),
/// then loops at the configured interval.
pub async fn start_cleanup_task(provisioner: Arc<Provisioner>) {
    tracing::info!("Starting cleanup task");

    // Run cleanup immediately on start for reconciliation
    run_cleanup(&provisioner).await;

    let interval = Duration::from_secs(provisioner.cleanup_interval_secs());

    loop {
        tokio::time::sleep(interval).await;
        run_cleanup(&provisioner).await;
    }
}

async fn run_cleanup(provisioner: &Provisioner) {
    tracing::debug!("Running expired agent cleanup");

    match provisioner.cleanup_expired_agents().await {
        Ok(result) => {
            if result.count > 0 {
                tracing::info!(count = result.count, "Cleanup completed, deleted expired agents");
            } else {
                tracing::debug!("Cleanup completed, no expired agents found");
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Cleanup task failed");
        }
    }
}
