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
}

impl ServerConfig {
    /// Load configuration from environment variables.
    ///
    /// Supported variables:
    /// - `LOOM_SERVER_HOST`: Host address (default: 127.0.0.1)
    /// - `LOOM_SERVER_PORT`: Port number (default: 8080)
    /// - `LOOM_SERVER_DATABASE_URL`: SQLite database URL (default: sqlite:./loom.db)
    /// - `LOOM_SERVER_LOG_LEVEL`: Log level (default: info)
    pub fn from_env() -> Result<Self, ConfigError> {
        let host = env::var("LOOM_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let port = env::var("LOOM_SERVER_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|e| ConfigError::InvalidPort(e.to_string()))?;

        let database_url =
            env::var("LOOM_SERVER_DATABASE_URL").unwrap_or_else(|_| "sqlite:./loom.db".to_string());

        let log_level = env::var("LOOM_SERVER_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            host,
            port,
            database_url,
            log_level,
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
            host: "127.0.0.1".to_string(),
            port: 8080,
            database_url: "sqlite:./loom.db".to_string(),
            log_level: "info".to_string(),
        }
    }
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid port: {0}")]
    InvalidPort(String),
}
