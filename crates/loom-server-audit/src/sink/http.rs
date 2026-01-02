// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

#![cfg(feature = "sink-http")]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};

use crate::enrichment::EnrichedAuditEvent;
use crate::error::AuditSinkError;
use crate::filter::AuditFilterConfig;
use crate::sink::AuditSink;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSinkConfig {
	pub name: String,
	pub url: String,
	#[serde(default = "default_method")]
	pub method: String,
	#[serde(default)]
	pub headers: Vec<(String, String)>,
	#[serde(default = "default_timeout_ms")]
	pub timeout_ms: u64,
	#[serde(default = "default_retry_max_attempts")]
	pub retry_max_attempts: u32,
	pub filter: AuditFilterConfig,
}

fn default_method() -> String {
	"POST".to_string()
}

fn default_timeout_ms() -> u64 {
	5000
}

fn default_retry_max_attempts() -> u32 {
	3
}

impl HttpSinkConfig {
	pub fn validate(&self) -> Result<(), String> {
		if self.name.is_empty() {
			return Err("name cannot be empty".to_string());
		}
		if self.url.is_empty() {
			return Err("url cannot be empty".to_string());
		}
		if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
			return Err("url must start with http:// or https://".to_string());
		}
		let method_upper = self.method.to_uppercase();
		if !["POST", "PUT", "PATCH"].contains(&method_upper.as_str()) {
			return Err(format!("unsupported method: {}", self.method));
		}
		if self.timeout_ms == 0 {
			return Err("timeout_ms must be greater than 0".to_string());
		}
		Ok(())
	}
}

pub struct HttpAuditSink {
	config: HttpSinkConfig,
	client: Client,
}

impl HttpAuditSink {
	pub fn new(config: HttpSinkConfig) -> Result<Self, AuditSinkError> {
		config
			.validate()
			.map_err(|e| AuditSinkError::Permanent(format!("invalid config: {e}")))?;

		let client = Client::builder()
			.timeout(Duration::from_millis(config.timeout_ms))
			.build()
			.map_err(|e| AuditSinkError::Permanent(format!("failed to create HTTP client: {e}")))?;

		Ok(Self { config, client })
	}

	fn parse_method(&self) -> Method {
		match self.config.method.to_uppercase().as_str() {
			"PUT" => Method::PUT,
			"PATCH" => Method::PATCH,
			_ => Method::POST,
		}
	}

	async fn send_with_retry(&self, body: &str) -> Result<(), AuditSinkError> {
		let method = self.parse_method();
		let mut last_error = None;

		for attempt in 0..self.config.retry_max_attempts {
			if attempt > 0 {
				let backoff = Duration::from_millis(100 * 2u64.pow(attempt - 1));
				tokio::time::sleep(backoff).await;
			}

			let mut request = self.client.request(method.clone(), &self.config.url);

			request = request.header("Content-Type", "application/json");

			for (key, value) in &self.config.headers {
				request = request.header(key, value);
			}

			request = request.body(body.to_string());

			match request.send().await {
				Ok(response) => {
					let status = response.status();
					if status.is_success() {
						return Ok(());
					}

					if is_permanent_status(status) {
						let body = response.text().await.unwrap_or_default();
						return Err(AuditSinkError::Permanent(format!(
							"HTTP {} {}: {}",
							status.as_u16(),
							status.canonical_reason().unwrap_or(""),
							body
						)));
					}

					last_error = Some(AuditSinkError::Transient(format!(
						"HTTP {} {}",
						status.as_u16(),
						status.canonical_reason().unwrap_or("")
					)));
				}
				Err(e) => {
					last_error = Some(AuditSinkError::Transient(format!("request failed: {e}")));
				}
			}
		}

		Err(last_error.unwrap_or_else(|| AuditSinkError::Transient("unknown error".to_string())))
	}
}

fn is_permanent_status(status: StatusCode) -> bool {
	matches!(
		status,
		StatusCode::BAD_REQUEST
			| StatusCode::UNAUTHORIZED
			| StatusCode::FORBIDDEN
			| StatusCode::NOT_FOUND
			| StatusCode::METHOD_NOT_ALLOWED
			| StatusCode::NOT_ACCEPTABLE
			| StatusCode::GONE
			| StatusCode::UNSUPPORTED_MEDIA_TYPE
			| StatusCode::UNPROCESSABLE_ENTITY
	)
}

#[async_trait]
impl AuditSink for HttpAuditSink {
	fn name(&self) -> &str {
		&self.config.name
	}

	fn filter(&self) -> &AuditFilterConfig {
		&self.config.filter
	}

	async fn publish(&self, event: Arc<EnrichedAuditEvent>) -> Result<(), AuditSinkError> {
		let body = serde_json::to_string(&*event)
			.map_err(|e| AuditSinkError::Permanent(format!("failed to serialize event: {e}")))?;

		self.send_with_retry(&body).await
	}

	async fn health_check(&self) -> Result<(), AuditSinkError> {
		let response = self
			.client
			.head(&self.config.url)
			.send()
			.await
			.map_err(|e| AuditSinkError::Transient(format!("health check failed: {e}")))?;

		if response.status().is_server_error() {
			return Err(AuditSinkError::Transient(format!(
				"health check returned {}",
				response.status()
			)));
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn default_filter() -> AuditFilterConfig {
		AuditFilterConfig::default()
	}

	#[test]
	fn test_config_valid() {
		let config = HttpSinkConfig {
			name: "datadog".to_string(),
			url: "https://http-intake.logs.datadoghq.com/api/v2/logs".to_string(),
			method: "POST".to_string(),
			headers: vec![("DD-API-KEY".to_string(), "secret".to_string())],
			timeout_ms: 5000,
			retry_max_attempts: 3,
			filter: default_filter(),
		};
		assert!(config.validate().is_ok());
	}

	#[test]
	fn test_config_empty_name() {
		let config = HttpSinkConfig {
			name: "".to_string(),
			url: "https://example.com".to_string(),
			method: "POST".to_string(),
			headers: vec![],
			timeout_ms: 5000,
			retry_max_attempts: 3,
			filter: default_filter(),
		};
		assert_eq!(config.validate().unwrap_err(), "name cannot be empty");
	}

	#[test]
	fn test_config_empty_url() {
		let config = HttpSinkConfig {
			name: "test".to_string(),
			url: "".to_string(),
			method: "POST".to_string(),
			headers: vec![],
			timeout_ms: 5000,
			retry_max_attempts: 3,
			filter: default_filter(),
		};
		assert_eq!(config.validate().unwrap_err(), "url cannot be empty");
	}

	#[test]
	fn test_config_invalid_url_scheme() {
		let config = HttpSinkConfig {
			name: "test".to_string(),
			url: "ftp://example.com".to_string(),
			method: "POST".to_string(),
			headers: vec![],
			timeout_ms: 5000,
			retry_max_attempts: 3,
			filter: default_filter(),
		};
		assert_eq!(
			config.validate().unwrap_err(),
			"url must start with http:// or https://"
		);
	}

	#[test]
	fn test_config_invalid_method() {
		let config = HttpSinkConfig {
			name: "test".to_string(),
			url: "https://example.com".to_string(),
			method: "DELETE".to_string(),
			headers: vec![],
			timeout_ms: 5000,
			retry_max_attempts: 3,
			filter: default_filter(),
		};
		assert!(config.validate().unwrap_err().contains("unsupported method"));
	}

	#[test]
	fn test_config_zero_timeout() {
		let config = HttpSinkConfig {
			name: "test".to_string(),
			url: "https://example.com".to_string(),
			method: "POST".to_string(),
			headers: vec![],
			timeout_ms: 0,
			retry_max_attempts: 3,
			filter: default_filter(),
		};
		assert_eq!(
			config.validate().unwrap_err(),
			"timeout_ms must be greater than 0"
		);
	}

	#[test]
	fn test_config_defaults() {
		let json = r#"{"name":"test","url":"https://example.com","filter":{"min_severity":"info"}}"#;
		let config: HttpSinkConfig = serde_json::from_str(json).unwrap();
		assert_eq!(config.method, "POST");
		assert_eq!(config.timeout_ms, 5000);
		assert_eq!(config.retry_max_attempts, 3);
		assert!(config.headers.is_empty());
	}

	#[test]
	fn test_sink_creation_valid() {
		let config = HttpSinkConfig {
			name: "splunk".to_string(),
			url: "https://splunk-hec.example.com:8088/services/collector/event".to_string(),
			method: "POST".to_string(),
			headers: vec![("Authorization".to_string(), "Splunk token".to_string())],
			timeout_ms: 5000,
			retry_max_attempts: 3,
			filter: default_filter(),
		};
		let sink = HttpAuditSink::new(config);
		assert!(sink.is_ok());
	}

	#[test]
	fn test_sink_creation_invalid_config() {
		let config = HttpSinkConfig {
			name: "".to_string(),
			url: "https://example.com".to_string(),
			method: "POST".to_string(),
			headers: vec![],
			timeout_ms: 5000,
			retry_max_attempts: 3,
			filter: default_filter(),
		};
		let sink = HttpAuditSink::new(config);
		assert!(sink.is_err());
	}

	#[test]
	fn test_is_permanent_status() {
		assert!(is_permanent_status(StatusCode::BAD_REQUEST));
		assert!(is_permanent_status(StatusCode::UNAUTHORIZED));
		assert!(is_permanent_status(StatusCode::FORBIDDEN));
		assert!(is_permanent_status(StatusCode::NOT_FOUND));
		assert!(!is_permanent_status(StatusCode::OK));
		assert!(!is_permanent_status(StatusCode::INTERNAL_SERVER_ERROR));
		assert!(!is_permanent_status(StatusCode::BAD_GATEWAY));
		assert!(!is_permanent_status(StatusCode::SERVICE_UNAVAILABLE));
	}
}
