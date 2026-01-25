// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Self-monitoring for Loom itself.
//!
//! Automatically creates an internal organization, projects, and API keys
//! for crash monitoring of loom-server, loom-web, and loom-cli.

use loom_crash::{CrashClient, CrashClientBuilder};
use loom_crash_core::{OrgId, Platform, ProjectId};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::info;

/// Well-known UUID for the Loom Internal organization.
/// This is a fixed UUID so it's consistent across installations.
pub const LOOM_INTERNAL_ORG_ID: &str = "00000000-0000-0000-0000-000000000001";

/// Well-known UUID for the loom-server project.
pub const LOOM_SERVER_PROJECT_ID: &str = "00000000-0000-0000-0000-000000000002";

/// Well-known UUID for the loom-web project.
pub const LOOM_WEB_PROJECT_ID: &str = "00000000-0000-0000-0000-000000000003";

/// Well-known UUID for the loom-cli project.
pub const LOOM_CLI_PROJECT_ID: &str = "00000000-0000-0000-0000-000000000004";

/// System user ID for internal operations.
pub const SYSTEM_USER_ID: &str = "00000000-0000-0000-0000-000000000000";

/// Configuration for self-monitoring.
#[derive(Debug, Clone)]
pub struct SelfMonitoringConfig {
	pub enabled: bool,
	pub server_api_key: Option<String>,
	pub web_api_key: Option<String>,
	pub cli_api_key: Option<String>,
	/// Analytics API key for loom-web (write-only)
	pub analytics_api_key: Option<String>,
}

impl Default for SelfMonitoringConfig {
	fn default() -> Self {
		Self {
			enabled: true,
			server_api_key: None,
			web_api_key: None,
			cli_api_key: None,
			analytics_api_key: None,
		}
	}
}

/// Global crash client for loom-server itself.
static SERVER_CRASH_CLIENT: OnceCell<Arc<CrashClient>> = OnceCell::const_new();

/// Global self-monitoring configuration.
static SELF_MONITORING_CONFIG: OnceCell<SelfMonitoringConfig> = OnceCell::const_new();

/// Initialize self-monitoring infrastructure.
///
/// This creates the internal organization, projects, and API keys if they don't exist,
/// then initializes the crash client for loom-server.
pub async fn initialize_self_monitoring(
	pool: &SqlitePool,
	base_url: &str,
	release: &str,
	environment: &str,
) -> Result<SelfMonitoringConfig, SelfMonitoringError> {
	info!("Initializing Loom self-monitoring");

	// Ensure internal org exists
	let org_id: OrgId = LOOM_INTERNAL_ORG_ID
		.parse()
		.map_err(|_| SelfMonitoringError::InvalidId)?;
	ensure_internal_org(pool, &org_id).await?;

	// Ensure projects exist
	let server_project_id: ProjectId = LOOM_SERVER_PROJECT_ID
		.parse()
		.map_err(|_| SelfMonitoringError::InvalidId)?;
	let web_project_id: ProjectId = LOOM_WEB_PROJECT_ID
		.parse()
		.map_err(|_| SelfMonitoringError::InvalidId)?;
	let cli_project_id: ProjectId = LOOM_CLI_PROJECT_ID
		.parse()
		.map_err(|_| SelfMonitoringError::InvalidId)?;

	ensure_internal_project(pool, &org_id, &server_project_id, "loom-server", Platform::Rust)
		.await?;
	ensure_internal_project(pool, &org_id, &web_project_id, "loom-web", Platform::JavaScript)
		.await?;
	ensure_internal_project(pool, &org_id, &cli_project_id, "loom-cli", Platform::Rust).await?;

	// Get or create API keys
	let server_api_key = ensure_api_key(pool, &server_project_id, "loom-server-internal").await?;
	let web_api_key = ensure_api_key(pool, &web_project_id, "loom-web-internal").await?;
	let cli_api_key = ensure_api_key(pool, &cli_project_id, "loom-cli-internal").await?;

	// Get or create analytics API key for loom-web
	let analytics_api_key = ensure_analytics_api_key(pool, &org_id, "loom-web-analytics").await?;

	// Initialize loom-server crash client
	let crash_client = CrashClientBuilder::new()
		.base_url(base_url)
		.project_id(&server_project_id.to_string())
		.auth_token(&server_api_key)
		.release(release)
		.environment(environment)
		.build()
		.map_err(SelfMonitoringError::CrashClientInit)?;

	// Install panic hook
	crash_client.install_panic_hook();

	// Store globally
	let client = Arc::new(crash_client);
	SERVER_CRASH_CLIENT
		.set(client)
		.map_err(|_| SelfMonitoringError::AlreadyInitialized)?;

	let config = SelfMonitoringConfig {
		enabled: true,
		server_api_key: Some(server_api_key),
		web_api_key: Some(web_api_key),
		cli_api_key: Some(cli_api_key),
		analytics_api_key: Some(analytics_api_key),
	};

	// Store config globally for API access
	let _ = SELF_MONITORING_CONFIG.set(config.clone());

	info!(
		org_id = %LOOM_INTERNAL_ORG_ID,
		server_project = %LOOM_SERVER_PROJECT_ID,
		web_project = %LOOM_WEB_PROJECT_ID,
		cli_project = %LOOM_CLI_PROJECT_ID,
		"Self-monitoring initialized"
	);

	Ok(config)
}

/// Get the server crash client for manual error capture.
pub fn get_server_crash_client() -> Option<Arc<CrashClient>> {
	SERVER_CRASH_CLIENT.get().cloned()
}

/// Get the self-monitoring configuration.
pub fn get_self_monitoring_config() -> Option<SelfMonitoringConfig> {
	SELF_MONITORING_CONFIG.get().cloned()
}

/// Configuration for web crash SDK initialization.
/// This is a subset of SelfMonitoringConfig suitable for the web frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WebCrashConfig {
	pub project_id: String,
	pub api_key: String,
	pub release: String,
	pub environment: String,
}

/// Get the web crash SDK configuration.
/// Returns None if self-monitoring is not initialized or web API key is not available.
pub fn get_web_crash_config() -> Option<WebCrashConfig> {
	let config = SELF_MONITORING_CONFIG.get()?;
	let api_key = config.web_api_key.clone()?;
	let release = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
	let environment =
		std::env::var("LOOM_ENVIRONMENT").unwrap_or_else(|_| "production".to_string());

	Some(WebCrashConfig {
		project_id: LOOM_WEB_PROJECT_ID.to_string(),
		api_key,
		release,
		environment,
	})
}

/// Configuration for CLI crash SDK initialization.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CliCrashConfig {
	pub project_id: String,
	pub api_key: String,
	pub release: String,
	pub environment: String,
}

/// Get the CLI crash SDK configuration.
/// Returns None if self-monitoring is not initialized or CLI API key is not available.
pub fn get_cli_crash_config() -> Option<CliCrashConfig> {
	let config = SELF_MONITORING_CONFIG.get()?;
	let api_key = config.cli_api_key.clone()?;
	let release = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
	let environment =
		std::env::var("LOOM_ENVIRONMENT").unwrap_or_else(|_| "production".to_string());

	Some(CliCrashConfig {
		project_id: LOOM_CLI_PROJECT_ID.to_string(),
		api_key,
		release,
		environment,
	})
}

/// Configuration for web analytics SDK initialization.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WebAnalyticsConfig {
	pub api_key: String,
	pub release: String,
	pub environment: String,
}

/// Get the web analytics SDK configuration.
/// Returns None if self-monitoring is not initialized or analytics API key is not available.
pub fn get_web_analytics_config() -> Option<WebAnalyticsConfig> {
	let config = SELF_MONITORING_CONFIG.get()?;
	let api_key = config.analytics_api_key.clone()?;
	let release = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
	let environment =
		std::env::var("LOOM_ENVIRONMENT").unwrap_or_else(|_| "production".to_string());

	Some(WebAnalyticsConfig {
		api_key,
		release,
		environment,
	})
}

/// Ensure the internal organization exists.
async fn ensure_internal_org(pool: &SqlitePool, org_id: &OrgId) -> Result<(), SelfMonitoringError> {
	let org_id_str = org_id.to_string();
	let now = chrono::Utc::now();

	// Check if org exists
	let existing: Option<(String,)> =
		sqlx::query_as("SELECT id FROM organizations WHERE id = ?")
			.bind(&org_id_str)
			.fetch_optional(pool)
			.await
			.map_err(SelfMonitoringError::Database)?;

	if existing.is_some() {
		return Ok(());
	}

	// Create org
	sqlx::query(
		r#"
		INSERT INTO organizations (id, name, slug, visibility, is_personal, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		"#,
	)
	.bind(&org_id_str)
	.bind("Loom Internal")
	.bind("loom-internal")
	.bind("private")
	.bind(false)
	.bind(now)
	.bind(now)
	.execute(pool)
	.await
	.map_err(SelfMonitoringError::Database)?;

	info!(org_id = %org_id_str, "Created Loom Internal organization");
	Ok(())
}

/// Ensure an internal crash project exists.
async fn ensure_internal_project(
	pool: &SqlitePool,
	org_id: &OrgId,
	project_id: &ProjectId,
	name: &str,
	platform: Platform,
) -> Result<(), SelfMonitoringError> {
	let project_id_str = project_id.to_string();
	let org_id_str = org_id.to_string();
	let now = chrono::Utc::now();

	// Check if project exists
	let existing: Option<(String,)> =
		sqlx::query_as("SELECT id FROM crash_projects WHERE id = ?")
			.bind(&project_id_str)
			.fetch_optional(pool)
			.await
			.map_err(SelfMonitoringError::Database)?;

	if existing.is_some() {
		return Ok(());
	}

	// Create project
	let slug = name.to_lowercase().replace(' ', "-");
	let platform_str = match platform {
		Platform::JavaScript => "javascript",
		Platform::Rust => "rust",
		_ => "other",
	};

	sqlx::query(
		r#"
		INSERT INTO crash_projects (id, org_id, name, slug, platform, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		"#,
	)
	.bind(&project_id_str)
	.bind(&org_id_str)
	.bind(name)
	.bind(&slug)
	.bind(platform_str)
	.bind(now)
	.bind(now)
	.execute(pool)
	.await
	.map_err(SelfMonitoringError::Database)?;

	info!(project_id = %project_id_str, name = %name, "Created internal crash project");
	Ok(())
}

/// Ensure an API key exists for a project, or create one.
async fn ensure_api_key(
	pool: &SqlitePool,
	project_id: &ProjectId,
	name: &str,
) -> Result<String, SelfMonitoringError> {
	let project_id_str = project_id.to_string();

	// Check for existing non-revoked key with this name
	let existing: Option<(String,)> = sqlx::query_as(
		r#"
		SELECT key_hash FROM crash_api_keys
		WHERE project_id = ? AND name = ? AND revoked_at IS NULL
		"#,
	)
	.bind(&project_id_str)
	.bind(name)
	.fetch_optional(pool)
	.await
	.map_err(SelfMonitoringError::Database)?;

	if let Some((key_hash,)) = existing {
		// Return the stored key (for internal keys, we store the actual key in key_hash)
		return Ok(key_hash);
	}

	// Generate new API key
	let key = format!(
		"lk_{}",
		uuid::Uuid::new_v4().to_string().replace('-', "")
	);
	let key_id = uuid::Uuid::new_v4().to_string();
	let now = chrono::Utc::now();

	// For internal keys, we store the actual key (not hashed) for retrieval
	// This is safe because these are system-managed keys
	// created_by uses a special system user ID to indicate auto-generated keys
	let system_user_id = "00000000-0000-0000-0000-000000000000";
	sqlx::query(
		r#"
		INSERT INTO crash_api_keys (id, project_id, name, key_prefix, key_hash, key_type, created_by, created_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?)
		"#,
	)
	.bind(&key_id)
	.bind(&project_id_str)
	.bind(name)
	.bind(&key[..10]) // Store prefix for display
	.bind(&key) // Store full key for internal use
	.bind("capture")
	.bind(system_user_id)
	.bind(now)
	.execute(pool)
	.await
	.map_err(SelfMonitoringError::Database)?;

	info!(project_id = %project_id_str, name = %name, "Created internal API key");
	Ok(key)
}

/// Ensure an analytics API key exists for the internal organization, or create one.
async fn ensure_analytics_api_key(
	pool: &SqlitePool,
	org_id: &OrgId,
	name: &str,
) -> Result<String, SelfMonitoringError> {
	let org_id_str = org_id.to_string();

	// Check for existing non-revoked key with this name
	let existing: Option<(String,)> = sqlx::query_as(
		r#"
		SELECT key_hash FROM analytics_api_keys
		WHERE org_id = ? AND name = ? AND revoked_at IS NULL
		"#,
	)
	.bind(&org_id_str)
	.bind(name)
	.fetch_optional(pool)
	.await
	.map_err(SelfMonitoringError::Database)?;

	if let Some((key_hash,)) = existing {
		// Return the stored key (for internal keys, we store the actual key in key_hash)
		return Ok(key_hash);
	}

	// Generate new analytics API key (write-only for web frontend)
	let random = uuid::Uuid::new_v4().to_string().replace('-', "");
	let key = format!("loom_analytics_write_{}", random);
	let key_id = uuid::Uuid::new_v4().to_string();
	let now = chrono::Utc::now();

	// For internal keys, we store the actual key (not hashed) for retrieval
	// This is safe because these are system-managed keys
	sqlx::query(
		r#"
		INSERT INTO analytics_api_keys (id, org_id, name, key_type, key_hash, created_by, created_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		"#,
	)
	.bind(&key_id)
	.bind(&org_id_str)
	.bind(name)
	.bind("write")
	.bind(&key) // Store full key for internal use
	.bind(SYSTEM_USER_ID)
	.bind(now)
	.execute(pool)
	.await
	.map_err(SelfMonitoringError::Database)?;

	info!(org_id = %org_id_str, name = %name, "Created internal analytics API key");
	Ok(key)
}

/// Errors that can occur during self-monitoring initialization.
#[derive(Debug, thiserror::Error)]
pub enum SelfMonitoringError {
	#[error("Database error: {0}")]
	Database(#[from] sqlx::Error),

	#[error("Invalid internal ID")]
	InvalidId,

	#[error("Crash client initialization failed: {0}")]
	CrashClientInit(loom_crash::CrashSdkError),

	#[error("Self-monitoring already initialized")]
	AlreadyInitialized,
}
