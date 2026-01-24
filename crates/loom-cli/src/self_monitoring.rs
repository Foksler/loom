// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Self-monitoring for loom-cli.
//!
//! Automatically initializes crash monitoring for the CLI by fetching
//! configuration from the server. This enables Loom to monitor itself
//! without any user configuration.

use std::sync::Arc;

use anyhow::Result;
use loom_crash::{CrashClient, CrashClientBuilder};
use serde::Deserialize;
use tokio::sync::OnceCell;
use tracing::{debug, warn};

/// Well-known project ID for loom-cli (from loom-server self_monitoring.rs).
const LOOM_CLI_PROJECT_ID: &str = "00000000-0000-0000-0000-000000000004";

/// Global crash client for loom-cli.
static CLI_CRASH_CLIENT: OnceCell<Arc<CrashClient>> = OnceCell::const_new();

/// Configuration returned by the self-monitoring API.
#[derive(Debug, Deserialize)]
struct CliCrashConfig {
	project_id: String,
	api_key: String,
	release: String,
	environment: String,
}

/// Initialize self-monitoring for loom-cli.
///
/// This fetches the CLI API key from the server's self-monitoring endpoint
/// and initializes the crash client with a panic hook.
///
/// Call this once during CLI startup. Failures are logged but do not
/// prevent the CLI from running.
pub async fn initialize_self_monitoring(server_url: &str) -> Option<Arc<CrashClient>> {
	// Don't initialize twice
	if let Some(client) = CLI_CRASH_CLIENT.get() {
		return Some(client.clone());
	}

	match fetch_and_initialize(server_url).await {
		Ok(client) => {
			let client = Arc::new(client);
			let _ = CLI_CRASH_CLIENT.set(client.clone());
			debug!("Self-monitoring initialized for loom-cli");
			Some(client)
		}
		Err(e) => {
			warn!("Failed to initialize self-monitoring: {}", e);
			None
		}
	}
}

async fn fetch_and_initialize(server_url: &str) -> Result<CrashClient> {
	// Fetch CLI config from server
	let http = loom_common_http::new_client();
	let url = format!(
		"{}/api/self-monitoring/cli-config",
		server_url.trim_end_matches('/')
	);

	let response = http.get(&url).send().await?;

	if !response.status().is_success() {
		anyhow::bail!(
			"Self-monitoring not available: {} {}",
			response.status(),
			response.status().canonical_reason().unwrap_or("")
		);
	}

	let config: CliCrashConfig = response.json().await?;

	let client = CrashClientBuilder::new()
		.base_url(server_url)
		.project_id(&config.project_id)
		.auth_token(&config.api_key)
		.release(&config.release)
		.environment(&config.environment)
		.build()?;

	// Install panic hook for automatic crash reporting
	client.install_panic_hook();

	Ok(client)
}

/// Get the CLI crash client for manual error capture.
pub fn get_crash_client() -> Option<Arc<CrashClient>> {
	CLI_CRASH_CLIENT.get().cloned()
}

/// Capture an exception using the self-monitoring crash client.
///
/// This is a convenience function that handles the case where
/// self-monitoring is not initialized.
#[allow(dead_code)]
pub fn capture_exception(error: &dyn std::error::Error) -> Option<String> {
	if let Some(client) = get_crash_client() {
		// Use blocking capture in sync context
		// For now, just log since async capture would require runtime
		warn!("CLI crash captured (async send not implemented): {}", error);
		return None;
	}
	None
}

/// Shutdown the CLI crash client.
///
/// Call this during CLI shutdown to flush pending events.
pub async fn shutdown_self_monitoring() {
	if let Some(client) = CLI_CRASH_CLIENT.get() {
		if let Err(e) = client.shutdown().await {
			warn!("Failed to shutdown crash client: {}", e);
		}
	}
}
