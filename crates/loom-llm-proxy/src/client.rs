//! HTTP client for LLM proxy communication.

use async_trait::async_trait;
use tracing::{debug, info, instrument};

use loom_core::{LlmClient, LlmError, LlmRequest, LlmResponse, LlmStream};

use crate::stream::ProxyLlmStream;
use crate::types::LlmProxyResponse;

/// LLM client that communicates with a server proxy instead of direct LLM providers.
///
/// This client sends requests to the server's proxy endpoints, which handle
/// authentication and communication with the actual LLM providers.
pub struct ProxyLlmClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl ProxyLlmClient {
    /// Creates a new proxy client with the given base URL.
    ///
    /// Uses a default HTTP client with reasonable timeouts.
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        info!(base_url = %base_url, "creating ProxyLlmClient");
        Self {
            base_url,
            http_client: reqwest::Client::new(),
        }
    }

    /// Creates a new proxy client with a custom HTTP client.
    ///
    /// Use this when you need custom timeouts, retry policies, or other HTTP configuration.
    pub fn with_http_client(base_url: impl Into<String>, http_client: reqwest::Client) -> Self {
        let base_url = base_url.into();
        info!(base_url = %base_url, "creating ProxyLlmClient with custom HTTP client");
        Self {
            base_url,
            http_client,
        }
    }

    fn complete_url(&self) -> String {
        format!("{}/proxy/llm/complete", self.base_url.trim_end_matches('/'))
    }

    fn stream_url(&self) -> String {
        format!("{}/proxy/llm/stream", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl LlmClient for ProxyLlmClient {
    /// Sends a completion request to the proxy and waits for the full response.
    #[instrument(skip(self, request), fields(model = %request.model))]
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let url = self.complete_url();
        debug!(url = %url, "sending completion request to proxy");

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                debug!(error = %e, "HTTP request failed");
                LlmError::Http(e.to_string())
            })?;

        let status = response.status();
        debug!(status = %status, "received response from proxy");

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            debug!(status = %status, body = %error_body, "proxy returned error status");

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(LlmError::RateLimited {
                    retry_after_secs: None,
                });
            }

            return Err(LlmError::Api(format!(
                "proxy returned status {}: {}",
                status, error_body
            )));
        }

        let proxy_response: LlmProxyResponse = response.json().await.map_err(|e| {
            debug!(error = %e, "failed to parse proxy response");
            LlmError::InvalidResponse(format!("failed to parse response: {}", e))
        })?;

        debug!("successfully parsed proxy response");
        Ok(proxy_response.into())
    }

    /// Sends a streaming completion request to the proxy.
    #[instrument(skip(self, request), fields(model = %request.model))]
    async fn complete_streaming(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        let url = self.stream_url();
        debug!(url = %url, "sending streaming request to proxy");

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                debug!(error = %e, "HTTP request failed");
                LlmError::Http(e.to_string())
            })?;

        let status = response.status();
        debug!(status = %status, "received streaming response from proxy");

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            debug!(status = %status, body = %error_body, "proxy returned error status");

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(LlmError::RateLimited {
                    retry_after_secs: None,
                });
            }

            return Err(LlmError::Api(format!(
                "proxy returned status {}: {}",
                status, error_body
            )));
        }

        let byte_stream = response.bytes_stream();
        let proxy_stream = ProxyLlmStream::new(Box::pin(byte_stream));

        debug!("created streaming response");
        Ok(LlmStream::new(Box::pin(proxy_stream)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_correct_urls() {
        let client = ProxyLlmClient::new("http://localhost:8080");
        assert_eq!(client.complete_url(), "http://localhost:8080/proxy/llm/complete");
        assert_eq!(client.stream_url(), "http://localhost:8080/proxy/llm/stream");
    }

    #[test]
    fn handles_trailing_slash_in_base_url() {
        let client = ProxyLlmClient::new("http://localhost:8080/");
        assert_eq!(client.complete_url(), "http://localhost:8080/proxy/llm/complete");
        assert_eq!(client.stream_url(), "http://localhost:8080/proxy/llm/stream");
    }
}
