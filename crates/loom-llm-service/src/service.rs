//! LLM service implementation.

use std::sync::Arc;

use loom_core::{LlmClient, LlmError, LlmRequest, LlmResponse, LlmStream};
use loom_llm_anthropic::AnthropicClient;
use loom_llm_openai::OpenAIClient;
use tracing::{debug, info, instrument};

use crate::config::{LlmProvider, LlmServiceConfig};
use crate::error::LlmServiceError;

/// Service for managing LLM provider interactions.
///
/// This service abstracts over different LLM providers (Anthropic, OpenAI)
/// and provides a unified interface for making completion requests.
#[derive(Clone)]
pub struct LlmService {
    provider: LlmProvider,
    client: Arc<dyn LlmClient>,
}

impl LlmService {
    /// Creates a new LLM service with the given configuration.
    pub fn new(config: LlmServiceConfig) -> Result<Self, LlmServiceError> {
        info!(provider = %config.provider, "Initializing LLM service");

        let client: Arc<dyn LlmClient> = match config.provider {
            LlmProvider::Anthropic => {
                let api_key = config.anthropic_api_key.ok_or_else(|| {
                    LlmServiceError::ProviderNotConfigured(
                        "Anthropic API key not configured".to_string(),
                    )
                })?;

                let mut anthropic_config = loom_llm_anthropic::AnthropicConfig::new(api_key);
                if let Some(model) = config.anthropic_model {
                    debug!(model = %model, "Using custom Anthropic model");
                    anthropic_config = anthropic_config.with_model(model);
                }

                let client = AnthropicClient::new(anthropic_config)
                    .map_err(|e| LlmServiceError::Config(e.to_string()))?;
                Arc::new(client)
            }
            LlmProvider::OpenAi => {
                let api_key = config.openai_api_key.ok_or_else(|| {
                    LlmServiceError::ProviderNotConfigured(
                        "OpenAI API key not configured".to_string(),
                    )
                })?;

                let mut openai_config = loom_llm_openai::OpenAIConfig::new(api_key);
                if let Some(model) = config.openai_model {
                    debug!(model = %model, "Using custom OpenAI model");
                    openai_config = openai_config.with_model(model);
                }
                if let Some(org) = config.openai_organization {
                    debug!(organization = %org, "Using OpenAI organization");
                    openai_config = openai_config.with_organization(org);
                }

                let client = OpenAIClient::new(openai_config)
                    .map_err(|e| LlmServiceError::Config(e.to_string()))?;
                Arc::new(client)
            }
        };

        info!(provider = %config.provider, "LLM service initialized");

        Ok(Self {
            provider: config.provider,
            client,
        })
    }

    /// Creates a new LLM service from environment variables.
    pub fn from_env() -> Result<Self, LlmServiceError> {
        let config = LlmServiceConfig::from_env()
            .map_err(|e| LlmServiceError::Config(e.to_string()))?;
        Self::new(config)
    }

    /// Returns the active LLM provider.
    pub fn provider(&self) -> LlmProvider {
        self.provider
    }

    /// Sends a completion request and waits for the full response.
    #[instrument(skip(self, request), fields(provider = %self.provider))]
    pub async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        debug!(
            model = %request.model,
            message_count = request.messages.len(),
            tool_count = request.tools.len(),
            "Sending completion request"
        );

        self.client.complete(request).await
    }

    /// Sends a completion request and returns a stream of events.
    #[instrument(skip(self, request), fields(provider = %self.provider))]
    pub async fn complete_streaming(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        debug!(
            model = %request.model,
            message_count = request.messages.len(),
            tool_count = request.tools.len(),
            "Sending streaming completion request"
        );

        self.client.complete_streaming(request).await
    }
}

impl std::fmt::Debug for LlmService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmService")
            .field("provider", &self.provider)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that service creation fails without required API keys.
    /// This is important to fail fast with clear error messages.
    #[test]
    fn new_fails_without_api_key() {
        let config = LlmServiceConfig::new(LlmProvider::Anthropic);
        let result = LlmService::new(config);
        assert!(matches!(result, Err(LlmServiceError::ProviderNotConfigured(_))));

        let config = LlmServiceConfig::new(LlmProvider::OpenAi);
        let result = LlmService::new(config);
        assert!(matches!(result, Err(LlmServiceError::ProviderNotConfigured(_))));
    }

    /// Verifies that provider() returns the configured provider.
    /// This is important for consumers that need to know which provider is active.
    #[test]
    fn provider_returns_configured_provider() {
        let config = LlmServiceConfig::new(LlmProvider::Anthropic)
            .with_anthropic_api_key("test-key");
        let service = LlmService::new(config).unwrap();
        assert_eq!(service.provider(), LlmProvider::Anthropic);

        let config = LlmServiceConfig::new(LlmProvider::OpenAi)
            .with_openai_api_key("test-key");
        let service = LlmService::new(config).unwrap();
        assert_eq!(service.provider(), LlmProvider::OpenAi);
    }

    /// Verifies that Debug implementation doesn't expose sensitive data.
    /// This is important for security - API keys should never appear in logs.
    #[test]
    fn debug_does_not_expose_secrets() {
        let config = LlmServiceConfig::new(LlmProvider::Anthropic)
            .with_anthropic_api_key("super-secret-key");
        let service = LlmService::new(config).unwrap();
        let debug_output = format!("{:?}", service);
        assert!(!debug_output.contains("super-secret-key"));
    }
}
