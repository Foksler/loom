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
	/// Log level filter.
	pub log_level: String,
	/// Directory containing platform binaries.
	pub bin_dir: String,
	/// Whether agent provisioning is enabled.
	pub agent_enabled: bool,
	/// API key for agent endpoints.
	pub agent_api_key: Option<String>,
	/// K8s namespace for agents.
	pub agent_namespace: String,
	/// Cleanup interval in seconds.
	pub agent_cleanup_interval_secs: u64,
	/// Default agent TTL in hours.
	pub agent_default_ttl_hours: u32,
	/// Maximum agent TTL in hours.
	pub agent_max_ttl_hours: u32,
	/// Maximum concurrent agents.
	pub agent_max_concurrent: u32,
	/// Timeout waiting for agent ready state in seconds.
	pub agent_ready_timeout_secs: u64,
	/// Webhooks JSON configuration.
	pub agent_webhooks: String,
}

impl ServerConfig {
	/// Load configuration from environment variables.
	///
	/// Supported variables:
	/// - `LOOM_SERVER_HOST`: Host address (default: 0.0.0.0)
	/// - `LOOM_SERVER_PORT`: Port number (default: 8080)
	/// - `LOOM_SERVER_DATABASE_URL`: SQLite database URL (default:
	///   sqlite:./loom.db)
	/// - `LOOM_SERVER_LOG_LEVEL`: Log level (default: info)
	/// - `LOOM_SERVER_AGENT_ENABLED`: Enable agent provisioning (default: false)
	/// - `LOOM_SERVER_AGENT_API_KEY`: API key for agent endpoints
	/// - `LOOM_SERVER_K8S_NAMESPACE`: K8s namespace (default: loom-agents)
	/// - `LOOM_SERVER_AGENT_CLEANUP_INTERVAL_SECS`: Cleanup interval (default: 1800)
	/// - `LOOM_SERVER_AGENT_DEFAULT_TTL_HOURS`: Default TTL (default: 4)
	/// - `LOOM_SERVER_AGENT_MAX_TTL_HOURS`: Max TTL (default: 48)
	/// - `LOOM_SERVER_AGENT_MAX_CONCURRENT`: Max concurrent agents (default: 64)
	/// - `LOOM_SERVER_AGENT_READY_TIMEOUT_SECS`: Ready timeout (default: 60)
	/// - `LOOM_SERVER_AGENT_WEBHOOKS`: Webhooks JSON (default: [])
	pub fn from_env() -> Result<Self, ConfigError> {
		let host = env::var("LOOM_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

		let port = env::var("LOOM_SERVER_PORT")
			.unwrap_or_else(|_| "8080".to_string())
			.parse::<u16>()
			.map_err(|e| ConfigError::InvalidPort(e.to_string()))?;

		let database_url =
			env::var("LOOM_SERVER_DATABASE_URL").unwrap_or_else(|_| "sqlite:./loom.db".to_string());

		let log_level = env::var("LOOM_SERVER_LOG_LEVEL")
			.unwrap_or_else(|_| "info,tower_http::trace=debug".to_string());

		let bin_dir = env::var("LOOM_SERVER_BIN_DIR").unwrap_or_else(|_| "./bin".to_string());

		let agent_enabled = env::var("LOOM_SERVER_AGENT_ENABLED")
			.map(|v| v.eq_ignore_ascii_case("true") || v == "1")
			.unwrap_or(false);

		let agent_api_key = env::var("LOOM_SERVER_AGENT_API_KEY").ok();

		let agent_namespace =
			env::var("LOOM_SERVER_K8S_NAMESPACE").unwrap_or_else(|_| "loom-agents".to_string());

		let agent_cleanup_interval_secs = env::var("LOOM_SERVER_AGENT_CLEANUP_INTERVAL_SECS")
			.unwrap_or_else(|_| "1800".to_string())
			.parse::<u64>()
			.map_err(|e| ConfigError::InvalidValue("agent_cleanup_interval_secs".into(), e.to_string()))?;

		let agent_default_ttl_hours = env::var("LOOM_SERVER_AGENT_DEFAULT_TTL_HOURS")
			.unwrap_or_else(|_| "4".to_string())
			.parse::<u32>()
			.map_err(|e| ConfigError::InvalidValue("agent_default_ttl_hours".into(), e.to_string()))?;

		let agent_max_ttl_hours = env::var("LOOM_SERVER_AGENT_MAX_TTL_HOURS")
			.unwrap_or_else(|_| "48".to_string())
			.parse::<u32>()
			.map_err(|e| ConfigError::InvalidValue("agent_max_ttl_hours".into(), e.to_string()))?;

		let agent_max_concurrent = env::var("LOOM_SERVER_AGENT_MAX_CONCURRENT")
			.unwrap_or_else(|_| "64".to_string())
			.parse::<u32>()
			.map_err(|e| ConfigError::InvalidValue("agent_max_concurrent".into(), e.to_string()))?;

		let agent_ready_timeout_secs = env::var("LOOM_SERVER_AGENT_READY_TIMEOUT_SECS")
			.unwrap_or_else(|_| "60".to_string())
			.parse::<u64>()
			.map_err(|e| ConfigError::InvalidValue("agent_ready_timeout_secs".into(), e.to_string()))?;

		let agent_webhooks =
			env::var("LOOM_SERVER_AGENT_WEBHOOKS").unwrap_or_else(|_| "[]".to_string());

		Ok(Self {
			host,
			port,
			database_url,
			log_level,
			bin_dir,
			agent_enabled,
			agent_api_key,
			agent_namespace,
			agent_cleanup_interval_secs,
			agent_default_ttl_hours,
			agent_max_ttl_hours,
			agent_max_concurrent,
			agent_ready_timeout_secs,
			agent_webhooks,
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
			log_level: "info,tower_http::trace=debug".to_string(),
			bin_dir: "./bin".to_string(),
			agent_enabled: false,
			agent_api_key: None,
			agent_namespace: "loom-agents".to_string(),
			agent_cleanup_interval_secs: 1800,
			agent_default_ttl_hours: 4,
			agent_max_ttl_hours: 48,
			agent_max_concurrent: 64,
			agent_ready_timeout_secs: 60,
			agent_webhooks: "[]".to_string(),
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
