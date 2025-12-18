//! LLM service implementation.

use std::sync::Arc;

use loom_core::{LlmClient, LlmError, LlmRequest, LlmResponse, LlmStream};
use loom_llm_anthropic::AnthropicClient;
use loom_llm_openai::OpenAIClient;
use loom_llm_vertex::VertexClient;
use tracing::{debug, info, instrument};

use crate::config::LlmServiceConfig;
use crate::error::LlmServiceError;

/// Service for managing LLM provider interactions.
///
/// This service holds clients for multiple LLM providers (Anthropic, OpenAI, Vertex)
/// and provides provider-specific methods for making completion requests.
#[derive(Clone)]
pub struct LlmService {
    anthropic_client: Option<Arc<AnthropicClient>>,
    openai_client: Option<Arc<OpenAIClient>>,
    vertex_client: Option<Arc<VertexClient>>,
}

impl LlmService {
    /// Creates a new LLM service with the given configuration.
    ///
    /// Initializes clients for all providers that have API keys configured.
    pub fn new(config: LlmServiceConfig) -> Result<Self, LlmServiceError> {
        info!("Initializing LLM service");

        let anthropic_client = if let Some(api_key) = config.anthropic_api_key {
            let mut anthropic_config = loom_llm_anthropic::AnthropicConfig::new(api_key);
            if let Some(model) = config.anthropic_model {
                debug!(model = %model, "Using custom Anthropic model");
                anthropic_config = anthropic_config.with_model(model);
            }

            let client = AnthropicClient::new(anthropic_config)
                .map_err(|e| LlmServiceError::Config(e.to_string()))?;
            info!("Anthropic client initialized");
            Some(Arc::new(client))
        } else {
            debug!("Anthropic API key not configured");
            None
        };

        let openai_client = if let Some(api_key) = config.openai_api_key {
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
            info!("OpenAI client initialized");
            Some(Arc::new(client))
        } else {
            debug!("OpenAI API key not configured");
            None
        };

        let vertex_client =
            if let (Some(project), Some(location)) = (config.vertex_project, config.vertex_location)
            {
                let mut vertex_config = loom_llm_vertex::VertexConfig::new(project, location);
                if let Some(model) = config.vertex_model {
                    debug!(model = %model, "Using custom Vertex model");
                    vertex_config = vertex_config.with_model(model);
                }

                let client = VertexClient::new(vertex_config)
                    .map_err(|e| LlmServiceError::Config(e.to_string()))?;
                info!("Vertex client initialized");
                Some(Arc::new(client))
            } else {
                debug!("Vertex project/location not configured");
                None
            };

        if anthropic_client.is_none() && openai_client.is_none() && vertex_client.is_none() {
            return Err(LlmServiceError::ProviderNotConfigured(
                "No LLM providers configured. Set LOOM_SERVER_ANTHROPIC_API_KEY, LOOM_SERVER_OPENAI_API_KEY, or LOOM_SERVER_VERTEX_PROJECT + LOOM_SERVER_VERTEX_LOCATION".to_string(),
            ));
        }

        info!(
            anthropic = anthropic_client.is_some(),
            openai = openai_client.is_some(),
            vertex = vertex_client.is_some(),
            "LLM service initialized"
        );

        Ok(Self {
            anthropic_client,
            openai_client,
            vertex_client,
        })
    }

    /// Creates a new LLM service from environment variables.
    pub fn from_env() -> Result<Self, LlmServiceError> {
        let config = LlmServiceConfig::from_env()
            .map_err(|e| LlmServiceError::Config(e.to_string()))?;
        Self::new(config)
    }

    /// Returns whether the Anthropic provider is configured.
    pub fn has_anthropic(&self) -> bool {
        self.anthropic_client.is_some()
    }

    /// Returns whether the OpenAI provider is configured.
    pub fn has_openai(&self) -> bool {
        self.openai_client.is_some()
    }

    /// Returns whether the Vertex AI provider is configured.
    pub fn has_vertex(&self) -> bool {
        self.vertex_client.is_some()
    }

    /// Sends a completion request to Anthropic.
    #[instrument(skip(self, request), fields(provider = "anthropic"))]
    pub async fn complete_anthropic(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let client = self.anthropic_client.as_ref().ok_or_else(|| {
            LlmError::Api("Anthropic provider not configured".to_string())
        })?;

        debug!(
            model = %request.model,
            message_count = request.messages.len(),
            tool_count = request.tools.len(),
            "Sending Anthropic completion request"
        );

        client.complete(request).await
    }

    /// Sends a streaming completion request to Anthropic.
    #[instrument(skip(self, request), fields(provider = "anthropic"))]
    pub async fn complete_streaming_anthropic(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        let client = self.anthropic_client.as_ref().ok_or_else(|| {
            LlmError::Api("Anthropic provider not configured".to_string())
        })?;

        debug!(
            model = %request.model,
            message_count = request.messages.len(),
            tool_count = request.tools.len(),
            "Sending Anthropic streaming completion request"
        );

        client.complete_streaming(request).await
    }

    /// Sends a completion request to OpenAI.
    #[instrument(skip(self, request), fields(provider = "openai"))]
    pub async fn complete_openai(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let client = self.openai_client.as_ref().ok_or_else(|| {
            LlmError::Api("OpenAI provider not configured".to_string())
        })?;

        debug!(
            model = %request.model,
            message_count = request.messages.len(),
            tool_count = request.tools.len(),
            "Sending OpenAI completion request"
        );

        client.complete(request).await
    }

    /// Sends a streaming completion request to OpenAI.
    #[instrument(skip(self, request), fields(provider = "openai"))]
    pub async fn complete_streaming_openai(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        let client = self.openai_client.as_ref().ok_or_else(|| {
            LlmError::Api("OpenAI provider not configured".to_string())
        })?;

        debug!(
            model = %request.model,
            message_count = request.messages.len(),
            tool_count = request.tools.len(),
            "Sending OpenAI streaming completion request"
        );

        client.complete_streaming(request).await
    }

    /// Sends a completion request to Vertex AI.
    #[instrument(skip(self, request), fields(provider = "vertex"))]
    pub async fn complete_vertex(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let client = self.vertex_client.as_ref().ok_or_else(|| {
            LlmError::Api("Vertex provider not configured".to_string())
        })?;

        debug!(
            model = %request.model,
            message_count = request.messages.len(),
            tool_count = request.tools.len(),
            "Sending Vertex completion request"
        );

        client.complete(request).await
    }

    /// Sends a streaming completion request to Vertex AI.
    #[instrument(skip(self, request), fields(provider = "vertex"))]
    pub async fn complete_streaming_vertex(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        let client = self.vertex_client.as_ref().ok_or_else(|| {
            LlmError::Api("Vertex provider not configured".to_string())
        })?;

        debug!(
            model = %request.model,
            message_count = request.messages.len(),
            tool_count = request.tools.len(),
            "Sending Vertex streaming completion request"
        );

        client.complete_streaming(request).await
    }
}

impl std::fmt::Debug for LlmService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmService")
            .field("anthropic_configured", &self.anthropic_client.is_some())
            .field("openai_configured", &self.openai_client.is_some())
            .field("vertex_configured", &self.vertex_client.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LlmProvider;

    /// Verifies that service creation fails without any API keys.
    /// This is important to fail fast with clear error messages.
    #[test]
    fn new_fails_without_any_api_key() {
        let config = LlmServiceConfig::new(LlmProvider::Anthropic);
        let result = LlmService::new(config);
        assert!(matches!(result, Err(LlmServiceError::ProviderNotConfigured(_))));
    }

    /// Verifies that has_anthropic() returns true when configured.
    #[test]
    fn has_anthropic_returns_true_when_configured() {
        let config = LlmServiceConfig::new(LlmProvider::Anthropic)
            .with_anthropic_api_key("test-key");
        let service = LlmService::new(config).unwrap();
        assert!(service.has_anthropic());
        assert!(!service.has_openai());
    }

    /// Verifies that has_openai() returns true when configured.
    #[test]
    fn has_openai_returns_true_when_configured() {
        let config = LlmServiceConfig::new(LlmProvider::OpenAi)
            .with_openai_api_key("test-key");
        let service = LlmService::new(config).unwrap();
        assert!(!service.has_anthropic());
        assert!(service.has_openai());
    }

    /// Verifies that both providers can be configured simultaneously.
    #[test]
    fn both_providers_can_be_configured() {
        let config = LlmServiceConfig::new(LlmProvider::Anthropic)
            .with_anthropic_api_key("anthropic-key")
            .with_openai_api_key("openai-key");
        let service = LlmService::new(config).unwrap();
        assert!(service.has_anthropic());
        assert!(service.has_openai());
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
