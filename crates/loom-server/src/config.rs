// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Server configuration from environment variables.

use std::env;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
	/// Host address to bind to.
	pub host: String,
	/// Port to listen on.
	pub port: u16,
	/// SQLite database URL.
	pub database_url: String,
	/// Public base URL for links (e.g., magic link verification).
	pub base_url: String,
	/// Log level filter.
	pub log_level: String,
	/// Directory containing platform binaries.
	pub bin_dir: String,
	/// Whether weaver provisioning is enabled.
	pub weaver_enabled: bool,
	/// K8s namespace for weavers.
	pub weaver_namespace: String,
	/// Cleanup interval in seconds.
	pub weaver_cleanup_interval_secs: u64,
	/// Default weaver TTL in hours.
	pub weaver_default_ttl_hours: u32,
	/// Maximum weaver TTL in hours.
	pub weaver_max_ttl_hours: u32,
	/// Maximum concurrent weavers.
	pub weaver_max_concurrent: u32,
	/// Timeout waiting for weaver ready state in seconds.
	pub weaver_ready_timeout_secs: u64,
	/// Webhooks JSON configuration.
	pub weaver_webhooks: String,
	/// Comma-separated list of image pull secret names for private registries.
	pub weaver_image_pull_secrets: String,
	/// SMTP server hostname.
	pub smtp_host: Option<String>,
	/// SMTP server port.
	pub smtp_port: u16,
	/// SMTP username for authentication.
	pub smtp_username: Option<String>,
	/// SMTP password for authentication.
	pub smtp_password: Option<String>,
	/// Email address to send from.
	pub smtp_from_address: Option<String>,
	/// Display name for sent emails.
	pub smtp_from_name: String,
	/// Whether to use TLS for SMTP connection.
	pub smtp_use_tls: bool,
	/// Default locale for emails and API responses.
	/// Used when user has no preference set.
	pub default_locale: String,
	/// Enable email alerts for job failures.
	pub job_alert_enabled: bool,
	/// Comma-separated list of email recipients for job alerts.
	pub job_alert_recipients: Vec<String>,
	/// Number of days to retain job run history.
	pub job_history_retention_days: u32,
	/// Session cleanup interval in seconds.
	pub session_cleanup_interval_secs: u64,
	/// OAuth state cleanup interval in seconds.
	pub oauth_state_cleanup_interval_secs: u64,
}

impl ServerConfig {
	/// Load configuration from environment variables.
	///
	/// Supported variables:
	/// - `LOOM_SERVER_HOST`: Host address (default: 0.0.0.0)
	/// - `LOOM_SERVER_PORT`: Port number (default: 8080)
	/// - `LOOM_SERVER_DATABASE_URL`: SQLite database URL (default:
	///   sqlite:./loom.db)
	/// - `LOOM_SERVER_BASE_URL`: Public base URL (default: http://localhost:8080)
	/// - `LOOM_SERVER_LOG_LEVEL`: Log level (default: info)
	/// - `LOOM_SERVER_WEAVER_ENABLED`: Enable weaver provisioning (default: false)
	/// - `LOOM_SERVER_WEAVER_K8S_NAMESPACE`: K8s namespace (default: loom-weavers)
	/// - `LOOM_SERVER_WEAVER_CLEANUP_INTERVAL_SECS`: Cleanup interval (default: 1800)
	/// - `LOOM_SERVER_WEAVER_DEFAULT_TTL_HOURS`: Default TTL (default: 4)
	/// - `LOOM_SERVER_WEAVER_MAX_TTL_HOURS`: Max TTL (default: 48)
	/// - `LOOM_SERVER_WEAVER_MAX_CONCURRENT`: Max concurrent weavers (default: 64)
	/// - `LOOM_SERVER_WEAVER_READY_TIMEOUT_SECS`: Ready timeout (default: 60)
	/// - `LOOM_SERVER_WEAVER_WEBHOOKS`: Webhooks JSON (default: [])
	/// - `LOOM_SERVER_WEAVER_IMAGE_PULL_SECRETS`: Comma-separated secret names (default: "")
	/// - `LOOM_SERVER_DEFAULT_LOCALE`: Default locale for emails (default: en)
	/// - `LOOM_SERVER_JOB_ALERT_ENABLED`: Enable job failure alerts (default: false)
	/// - `LOOM_SERVER_JOB_ALERT_RECIPIENTS`: Comma-separated alert recipients (default: "")
	/// - `LOOM_SERVER_JOB_HISTORY_RETENTION_DAYS`: Job history retention days (default: 90)
	/// - `LOOM_SERVER_SESSION_CLEANUP_INTERVAL_SECS`: Session cleanup interval (default: 3600)
	/// - `LOOM_SERVER_OAUTH_STATE_CLEANUP_INTERVAL_SECS`: OAuth state cleanup interval (default: 900)
	pub fn from_env() -> Result<Self, ConfigError> {
		let host = env::var("LOOM_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

		let port = env::var("LOOM_SERVER_PORT")
			.unwrap_or_else(|_| "8080".to_string())
			.parse::<u16>()
			.map_err(|e| ConfigError::InvalidPort(e.to_string()))?;

		let database_url =
			env::var("LOOM_SERVER_DATABASE_URL").unwrap_or_else(|_| "sqlite:./loom.db".to_string());

		let base_url =
			env::var("LOOM_SERVER_BASE_URL").unwrap_or_else(|_| format!("http://localhost:{port}"));

		let log_level = env::var("LOOM_SERVER_LOG_LEVEL")
			.unwrap_or_else(|_| "info,tower_http::trace=debug".to_string());

		let bin_dir = env::var("LOOM_SERVER_BIN_DIR").unwrap_or_else(|_| "./bin".to_string());

		let weaver_enabled = env::var("LOOM_SERVER_WEAVER_ENABLED")
			.map(|v| v.eq_ignore_ascii_case("true") || v == "1")
			.unwrap_or(false);

		let weaver_namespace =
			env::var("LOOM_SERVER_WEAVER_K8S_NAMESPACE").unwrap_or_else(|_| "loom-weavers".to_string());

		let weaver_cleanup_interval_secs = env::var("LOOM_SERVER_WEAVER_CLEANUP_INTERVAL_SECS")
			.unwrap_or_else(|_| "1800".to_string())
			.parse::<u64>()
			.map_err(|e| {
				ConfigError::InvalidValue("weaver_cleanup_interval_secs".into(), e.to_string())
			})?;

		let weaver_default_ttl_hours = env::var("LOOM_SERVER_WEAVER_DEFAULT_TTL_HOURS")
			.unwrap_or_else(|_| "4".to_string())
			.parse::<u32>()
			.map_err(|e| ConfigError::InvalidValue("weaver_default_ttl_hours".into(), e.to_string()))?;

		let weaver_max_ttl_hours = env::var("LOOM_SERVER_WEAVER_MAX_TTL_HOURS")
			.unwrap_or_else(|_| "48".to_string())
			.parse::<u32>()
			.map_err(|e| ConfigError::InvalidValue("weaver_max_ttl_hours".into(), e.to_string()))?;

		let weaver_max_concurrent = env::var("LOOM_SERVER_WEAVER_MAX_CONCURRENT")
			.unwrap_or_else(|_| "64".to_string())
			.parse::<u32>()
			.map_err(|e| ConfigError::InvalidValue("weaver_max_concurrent".into(), e.to_string()))?;

		let weaver_ready_timeout_secs = env::var("LOOM_SERVER_WEAVER_READY_TIMEOUT_SECS")
			.unwrap_or_else(|_| "60".to_string())
			.parse::<u64>()
			.map_err(|e| ConfigError::InvalidValue("weaver_ready_timeout_secs".into(), e.to_string()))?;

		let weaver_webhooks =
			env::var("LOOM_SERVER_WEAVER_WEBHOOKS").unwrap_or_else(|_| "[]".to_string());

		let weaver_image_pull_secrets =
			env::var("LOOM_SERVER_WEAVER_IMAGE_PULL_SECRETS").unwrap_or_default();

		let smtp_host = env::var("LOOM_SERVER_SMTP_HOST").ok();

		let smtp_port = env::var("LOOM_SERVER_SMTP_PORT")
			.unwrap_or_else(|_| "587".to_string())
			.parse::<u16>()
			.map_err(|e| ConfigError::InvalidValue("smtp_port".into(), e.to_string()))?;

		let smtp_username = env::var("LOOM_SERVER_SMTP_USERNAME").ok();
		let smtp_password = env::var("LOOM_SERVER_SMTP_PASSWORD").ok();
		let smtp_from_address = env::var("LOOM_SERVER_SMTP_FROM_ADDRESS").ok();

		let smtp_from_name =
			env::var("LOOM_SERVER_SMTP_FROM_NAME").unwrap_or_else(|_| "Loom".to_string());

		let smtp_use_tls = env::var("LOOM_SERVER_SMTP_USE_TLS")
			.map(|v| v.eq_ignore_ascii_case("true") || v == "1")
			.unwrap_or(true);

		let default_locale =
			env::var("LOOM_SERVER_DEFAULT_LOCALE").unwrap_or_else(|_| "en".to_string());

		let job_alert_enabled = env::var("LOOM_SERVER_JOB_ALERT_ENABLED")
			.map(|v| v.eq_ignore_ascii_case("true") || v == "1")
			.unwrap_or(false);

		let job_alert_recipients = env::var("LOOM_SERVER_JOB_ALERT_RECIPIENTS")
			.unwrap_or_default()
			.split(',')
			.map(|s| s.trim().to_string())
			.filter(|s| !s.is_empty())
			.collect();

		let job_history_retention_days = env::var("LOOM_SERVER_JOB_HISTORY_RETENTION_DAYS")
			.unwrap_or_else(|_| "90".to_string())
			.parse::<u32>()
			.map_err(|e| ConfigError::InvalidValue("job_history_retention_days".into(), e.to_string()))?;

		let session_cleanup_interval_secs = env::var("LOOM_SERVER_SESSION_CLEANUP_INTERVAL_SECS")
			.unwrap_or_else(|_| "3600".to_string())
			.parse::<u64>()
			.map_err(|e| {
				ConfigError::InvalidValue("session_cleanup_interval_secs".into(), e.to_string())
			})?;

		let oauth_state_cleanup_interval_secs =
			env::var("LOOM_SERVER_OAUTH_STATE_CLEANUP_INTERVAL_SECS")
				.unwrap_or_else(|_| "900".to_string())
				.parse::<u64>()
				.map_err(|e| {
					ConfigError::InvalidValue("oauth_state_cleanup_interval_secs".into(), e.to_string())
				})?;

		Ok(Self {
			host,
			port,
			database_url,
			base_url,
			log_level,
			bin_dir,
			weaver_enabled,
			weaver_namespace,
			weaver_cleanup_interval_secs,
			weaver_default_ttl_hours,
			weaver_max_ttl_hours,
			weaver_max_concurrent,
			weaver_ready_timeout_secs,
			weaver_webhooks,
			weaver_image_pull_secrets,
			smtp_host,
			smtp_port,
			smtp_username,
			smtp_password,
			smtp_from_address,
			smtp_from_name,
			smtp_use_tls,
			default_locale,
			job_alert_enabled,
			job_alert_recipients,
			job_history_retention_days,
			session_cleanup_interval_secs,
			oauth_state_cleanup_interval_secs,
		})
	}

	/// Get the socket address string.
	pub fn socket_addr(&self) -> String {
		format!("{}:{}", self.host, self.port)
	}
}

impl Default for ServerConfig {
	fn default() -> Self {
		Self {
			host: "0.0.0.0".to_string(),
			port: 8080,
			database_url: "sqlite:./loom.db".to_string(),
			base_url: "http://localhost:8080".to_string(),
			log_level: "info,tower_http::trace=debug".to_string(),
			bin_dir: "./bin".to_string(),
			weaver_enabled: false,
			weaver_namespace: "loom-weavers".to_string(),
			weaver_cleanup_interval_secs: 1800,
			weaver_default_ttl_hours: 4,
			weaver_max_ttl_hours: 48,
			weaver_max_concurrent: 64,
			weaver_ready_timeout_secs: 60,
			weaver_webhooks: "[]".to_string(),
			weaver_image_pull_secrets: String::new(),
			smtp_host: None,
			smtp_port: 587,
			smtp_username: None,
			smtp_password: None,
			smtp_from_address: None,
			smtp_from_name: "Loom".to_string(),
			smtp_use_tls: true,
			default_locale: "en".to_string(),
			job_alert_enabled: false,
			job_alert_recipients: Vec::new(),
			job_history_retention_days: 90,
			session_cleanup_interval_secs: 3600,
			oauth_state_cleanup_interval_secs: 900,
		}
	}
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
	#[error("Invalid port: {0}")]
	InvalidPort(String),
	#[error("Invalid value for {0}: {1}")]
	InvalidValue(String, String),
}
