// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Anthropic client implementation.

use async_trait::async_trait;
use loom_core::{LlmClient, LlmError, LlmRequest, LlmResponse, LlmStream};
use loom_http_retry::{retry, RetryConfig, RetryableError};
use reqwest::Client;
use tracing::{debug, error, info, instrument, trace};

use crate::stream::parse_sse_stream;
use crate::types::{AnthropicConfig, AnthropicError, AnthropicRequest, AnthropicResponse};

const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug)]
pub struct ClientError {
	message: String,
	retryable: bool,
}

impl std::fmt::Display for ClientError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.message)
	}
}

impl std::error::Error for ClientError {}

impl RetryableError for ClientError {
	fn is_retryable(&self) -> bool {
		self.retryable
	}
}

impl From<ClientError> for LlmError {
	fn from(err: ClientError) -> Self {
		LlmError::Http(err.message)
	}
}

/// Client for interacting with Anthropic's Claude API.
#[derive(Debug, Clone)]
pub struct AnthropicClient {
	config: AnthropicConfig,
	http_client: Client,
	retry_config: RetryConfig,
}

impl AnthropicClient {
	pub fn new(config: AnthropicConfig) -> Result<Self, LlmError> {
		let http_client = Client::builder()
			.build()
			.map_err(|e| LlmError::Http(format!("Failed to create HTTP client: {}", e)))?;

		Ok(Self {
			config,
			http_client,
			retry_config: RetryConfig::default(),
		})
	}

	pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
		self.retry_config = retry_config;
		self
	}

	fn messages_url(&self) -> String {
		format!("{}/v1/messages", self.config.base_url)
	}

	#[instrument(skip(self, request), fields(model = %self.config.model))]
	async fn send_request(
		&self,
		request: &AnthropicRequest,
	) -> Result<reqwest::Response, ClientError> {
		let url = self.messages_url();
		debug!(url = %url, "Sending request to Anthropic API");
		trace!(request = ?request, "Request payload");

		let response = self
			.http_client
			.post(&url)
			.header("x-api-key", &self.config.api_key)
			.header("anthropic-version", ANTHROPIC_VERSION)
			.header("content-type", "application/json")
			.json(request)
			.send()
			.await
			.map_err(|e| {
				let retryable = e.is_timeout() || e.is_connect();
				error!(error = %e, retryable = retryable, "HTTP request failed");
				ClientError {
					message: e.to_string(),
					retryable,
				}
			})?;

		let status = response.status();
		debug!(status = %status, "Received response");

		if !status.is_success() {
			let retryable = matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504);
			let error_body = response.text().await.unwrap_or_default();
			error!(status = %status, body = %error_body, retryable = retryable, "API error response");

			let message = if let Ok(api_error) = serde_json::from_str::<AnthropicError>(&error_body) {
				api_error.error.message
			} else {
				error_body
			};

			return Err(ClientError { message, retryable });
		}

		Ok(response)
	}
}

#[async_trait]
impl LlmClient for AnthropicClient {
	#[instrument(skip(self, request), fields(model = %self.config.model))]
	async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
		info!("Starting non-streaming completion request");

		let mut anthropic_request = AnthropicRequest::from(&request);
		anthropic_request.model = self.config.model.clone();
		anthropic_request.stream = Some(false);

		let client = self.clone();
		let anthropic_request_clone = anthropic_request.clone();
		let response = retry(&self.retry_config, || {
			let req = anthropic_request_clone.clone();
			let c = client.clone();
			async move { c.send_request(&req).await }
		})
		.await
		.map_err(LlmError::from)?;

		let response_body = response.text().await.map_err(|e| {
			error!(error = %e, "Failed to read response body");
			LlmError::Http(e.to_string())
		})?;

		trace!(body = %response_body, "Response body");

		let anthropic_response: AnthropicResponse =
			serde_json::from_str(&response_body).map_err(|e| {
				error!(error = %e, body = %response_body, "Failed to parse response");
				LlmError::InvalidResponse(format!("Failed to parse response: {}", e))
			})?;

		let llm_response = LlmResponse::try_from(anthropic_response)?;
		info!(
				finish_reason = ?llm_response.finish_reason,
				"Completion request finished"
		);

		Ok(llm_response)
	}

	#[instrument(skip(self, request), fields(model = %self.config.model))]
	async fn complete_streaming(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
		info!("Starting streaming completion request");

		let mut anthropic_request = AnthropicRequest::from(&request);
		anthropic_request.model = self.config.model.clone();
		anthropic_request.stream = Some(true);

		let client = self.clone();
		let anthropic_request_clone = anthropic_request.clone();
		let response = retry(&self.retry_config, || {
			let req = anthropic_request_clone.clone();
			let c = client.clone();
			async move { c.send_request(&req).await }
		})
		.await
		.map_err(LlmError::from)?;

		debug!("Stream connection established");
		let stream = parse_sse_stream(response.bytes_stream());

		Ok(LlmStream::new(Box::pin(stream)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_client_creation() {
		let config = AnthropicConfig::new("test-key");
		let client = AnthropicClient::new(config);
		assert!(client.is_ok());
	}

	#[test]
	fn test_messages_url() {
		let config = AnthropicConfig::new("test-key").with_base_url("https://custom.api.com");
		let client = AnthropicClient::new(config).unwrap();
		assert_eq!(client.messages_url(), "https://custom.api.com/v1/messages");
	}
}
