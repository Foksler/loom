// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! LLM service implementation.

use std::env;
use std::sync::Arc;

use loom_core::{LlmClient, LlmError, LlmRequest, LlmResponse, LlmStream};
use std::time::Duration;

use loom_llm_anthropic::{
	AnthropicClient, AnthropicConfig, AnthropicPool, AnthropicPoolConfig, MemoryCredentialStore,
};

pub use loom_llm_anthropic::{
	AccountDetails, AccountHealthInfo, AccountHealthStatus, OAuthCredentials, PoolStatus,
};
use loom_llm_openai::OpenAIClient;
use loom_llm_vertex::VertexClient;
use tracing::{debug, info, instrument};

use crate::config::{AnthropicAuthConfig, LlmServiceConfig};
use crate::error::LlmServiceError;

/// Health information for Anthropic client.
#[derive(Debug, Clone)]
pub enum AnthropicHealthInfo {
	/// API key mode - just indicates if configured.
	ApiKey { configured: bool },
	/// OAuth pool mode - detailed pool status.
	Pool(PoolStatus),
}

/// Wrapper for Anthropic client that can use either API key or OAuth pool.
enum AnthropicClientWrapper {
	ApiKey(Arc<AnthropicClient<MemoryCredentialStore>>),
	Pool(Arc<AnthropicPool>),
}

impl Clone for AnthropicClientWrapper {
	fn clone(&self) -> Self {
		match self {
			AnthropicClientWrapper::ApiKey(c) => AnthropicClientWrapper::ApiKey(Arc::clone(c)),
			AnthropicClientWrapper::Pool(p) => AnthropicClientWrapper::Pool(Arc::clone(p)),
		}
	}
}

impl AnthropicClientWrapper {
	async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
		match self {
			AnthropicClientWrapper::ApiKey(c) => c.complete(request).await,
			AnthropicClientWrapper::Pool(p) => p.complete(request).await,
		}
	}

	async fn complete_streaming(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
		match self {
			AnthropicClientWrapper::ApiKey(c) => c.complete_streaming(request).await,
			AnthropicClientWrapper::Pool(p) => p.complete_streaming(request).await,
		}
	}
}

/// Service for managing LLM provider interactions.
///
/// This service holds clients for multiple LLM providers (Anthropic, OpenAI,
/// Vertex) and provides provider-specific methods for making completion
/// requests.
pub struct LlmService {
	anthropic_client: Option<AnthropicClientWrapper>,
	openai_client: Option<Arc<OpenAIClient>>,
	vertex_client: Option<Arc<VertexClient>>,
	#[allow(dead_code)] // Stored for future graceful shutdown
	refresh_task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl LlmService {
	/// Creates a new LLM service with the given configuration.
	///
	/// Initializes clients for all providers that have API keys configured.
	/// This is async because OAuth credential loading requires file I/O.
	pub async fn new(config: LlmServiceConfig) -> Result<Self, LlmServiceError> {
		info!("Initializing LLM service");

		let anthropic_client = match &config.anthropic_auth {
			Some(AnthropicAuthConfig::ApiKey(api_key)) => {
				let mut anthropic_config = AnthropicConfig::new(api_key.expose().clone());
				if let Some(ref model) = config.anthropic_model {
					debug!(model = %model, "Using custom Anthropic model");
					anthropic_config = anthropic_config.with_model(model.clone());
				}

				let client = AnthropicClient::new(anthropic_config)
					.map_err(|e| LlmServiceError::Config(e.to_string()))?;
				info!("Anthropic client initialized with API key");
				Some(AnthropicClientWrapper::ApiKey(Arc::new(client)))
			}
			Some(AnthropicAuthConfig::OAuthPool {
				credential_file,
				provider_ids,
				cooldown_secs,
			}) => {
				let pool_config = AnthropicPoolConfig {
					cooldown: Duration::from_secs(*cooldown_secs),
					..Default::default()
				};

				debug!(
					providers = ?provider_ids,
					credential_file = ?credential_file,
					cooldown_secs = cooldown_secs,
					"Creating Anthropic OAuth pool"
				);

				let pool = Arc::new(
					AnthropicPool::new(
						credential_file,
						provider_ids.clone(),
						config.anthropic_model.clone(),
						pool_config,
					)
					.await
					.map_err(|e| LlmServiceError::Config(format!("Failed to create OAuth pool: {e}")))?,
				);

				info!(
					account_count = provider_ids.len(),
					"Anthropic OAuth pool initialized"
				);
				Some(AnthropicClientWrapper::Pool(pool))
			}
			None => {
				debug!("Anthropic auth not configured");
				None
			}
		};

		let openai_client = if let Some(ref api_key) = config.openai_api_key {
			let mut openai_config = loom_llm_openai::OpenAIConfig::new(api_key.expose().clone());
			if let Some(model) = config.openai_model {
				debug!(model = %model, "Using custom OpenAI model");
				openai_config = openai_config.with_model(model);
			}
			if let Some(org) = config.openai_organization {
				debug!(organization = %org, "Using OpenAI organization");
				openai_config = openai_config.with_organization(org);
			}

			let client =
				OpenAIClient::new(openai_config).map_err(|e| LlmServiceError::Config(e.to_string()))?;
			info!("OpenAI client initialized");
			Some(Arc::new(client))
		} else {
			debug!("OpenAI API key not configured");
			None
		};

		let vertex_client =
			if let (Some(project), Some(location)) = (config.vertex_project, config.vertex_location) {
				let mut vertex_config = loom_llm_vertex::VertexConfig::new(project, location);
				if let Some(model) = config.vertex_model {
					debug!(model = %model, "Using custom Vertex model");
					vertex_config = vertex_config.with_model(model);
				}

				let client =
					VertexClient::new(vertex_config).map_err(|e| LlmServiceError::Config(e.to_string()))?;
				info!("Vertex client initialized");
				Some(Arc::new(client))
			} else {
				debug!("Vertex project/location not configured");
				None
			};

		if anthropic_client.is_none() && openai_client.is_none() && vertex_client.is_none() {
			return Err(LlmServiceError::ProviderNotConfigured(
				"No LLM providers configured. Set LOOM_SERVER_ANTHROPIC_API_KEY, \
				 LOOM_SERVER_OPENAI_API_KEY, or LOOM_SERVER_VERTEX_PROJECT + LOOM_SERVER_VERTEX_LOCATION"
					.to_string(),
			));
		}

		let refresh_task_handle =
			if let Some(AnthropicClientWrapper::Pool(ref pool)) = anthropic_client {
				let refresh_interval_secs: u64 = env::var("LOOM_SERVER_ANTHROPIC_REFRESH_INTERVAL_SECS")
					.ok()
					.and_then(|s| s.parse().ok())
					.unwrap_or(300);
				let refresh_threshold_secs: u64 =
					env::var("LOOM_SERVER_ANTHROPIC_REFRESH_THRESHOLD_SECS")
						.ok()
						.and_then(|s| s.parse().ok())
						.unwrap_or(900);

				let handle = Arc::clone(pool).spawn_refresh_task(
					Duration::from_secs(refresh_interval_secs),
					Duration::from_secs(refresh_threshold_secs),
				);
				info!(
					refresh_interval_secs,
					refresh_threshold_secs, "Started Anthropic OAuth token refresh task"
				);
				Some(handle)
			} else {
				None
			};

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
			refresh_task_handle,
		})
	}

	/// Creates a new LLM service from environment variables.
	pub async fn from_env() -> Result<Self, LlmServiceError> {
		let config =
			LlmServiceConfig::from_env().map_err(|e| LlmServiceError::Config(e.to_string()))?;
		Self::new(config).await
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

	/// Get Anthropic health status.
	pub async fn anthropic_health(&self) -> Option<AnthropicHealthInfo> {
		match &self.anthropic_client {
			Some(AnthropicClientWrapper::ApiKey(_)) => Some(AnthropicHealthInfo::ApiKey { configured: true }),
			Some(AnthropicClientWrapper::Pool(pool)) => {
				Some(AnthropicHealthInfo::Pool(pool.pool_status().await))
			}
			None => None,
		}
	}

	/// Returns whether Anthropic is configured in OAuth pool mode.
	pub fn is_anthropic_oauth_pool(&self) -> bool {
		matches!(&self.anthropic_client, Some(AnthropicClientWrapper::Pool(_)))
	}

	/// Add an Anthropic OAuth account to the pool.
	pub async fn add_anthropic_account(
		&self,
		account_id: String,
		credentials: OAuthCredentials,
	) -> Result<(), LlmServiceError> {
		match &self.anthropic_client {
			Some(AnthropicClientWrapper::Pool(pool)) => pool
				.add_account(account_id, credentials)
				.await
				.map_err(|e| LlmServiceError::Config(format!("Failed to add account: {e}"))),
			Some(AnthropicClientWrapper::ApiKey(_)) => Err(LlmServiceError::Config(
				"Cannot add accounts: Anthropic is configured with API key, not OAuth pool".to_string(),
			)),
			None => Err(LlmServiceError::ProviderNotConfigured(
				"Anthropic provider not configured".to_string(),
			)),
		}
	}

	/// Remove an Anthropic OAuth account from the pool.
	pub async fn remove_anthropic_account(&self, account_id: &str) -> Result<(), LlmServiceError> {
		match &self.anthropic_client {
			Some(AnthropicClientWrapper::Pool(pool)) => pool
				.remove_account(account_id)
				.await
				.map_err(|e| LlmServiceError::Config(format!("Failed to remove account: {e}"))),
			Some(AnthropicClientWrapper::ApiKey(_)) => Err(LlmServiceError::Config(
				"Cannot remove accounts: Anthropic is configured with API key, not OAuth pool"
					.to_string(),
			)),
			None => Err(LlmServiceError::ProviderNotConfigured(
				"Anthropic provider not configured".to_string(),
			)),
		}
	}

	/// Get detailed account info for admin API.
	pub async fn anthropic_account_details(&self) -> Option<Vec<AccountDetails>> {
		match &self.anthropic_client {
			Some(AnthropicClientWrapper::Pool(pool)) => Some(pool.account_details().await),
			_ => None,
		}
	}

	/// Sends a completion request to Anthropic.
	#[instrument(skip(self, request), fields(provider = "anthropic"))]
	pub async fn complete_anthropic(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
		let client = self
			.anthropic_client
			.as_ref()
			.ok_or_else(|| LlmError::Api("Anthropic provider not configured".to_string()))?;

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
	pub async fn complete_streaming_anthropic(
		&self,
		request: LlmRequest,
	) -> Result<LlmStream, LlmError> {
		let client = self
			.anthropic_client
			.as_ref()
			.ok_or_else(|| LlmError::Api("Anthropic provider not configured".to_string()))?;

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
		let client = self
			.openai_client
			.as_ref()
			.ok_or_else(|| LlmError::Api("OpenAI provider not configured".to_string()))?;

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
	pub async fn complete_streaming_openai(
		&self,
		request: LlmRequest,
	) -> Result<LlmStream, LlmError> {
		let client = self
			.openai_client
			.as_ref()
			.ok_or_else(|| LlmError::Api("OpenAI provider not configured".to_string()))?;

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
		let client = self
			.vertex_client
			.as_ref()
			.ok_or_else(|| LlmError::Api("Vertex provider not configured".to_string()))?;

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
	pub async fn complete_streaming_vertex(
		&self,
		request: LlmRequest,
	) -> Result<LlmStream, LlmError> {
		let client = self
			.vertex_client
			.as_ref()
			.ok_or_else(|| LlmError::Api("Vertex provider not configured".to_string()))?;

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
	#[tokio::test]
	async fn new_fails_without_any_api_key() {
		let config = LlmServiceConfig::new(LlmProvider::Anthropic);
		let result = LlmService::new(config).await;
		assert!(matches!(
			result,
			Err(LlmServiceError::ProviderNotConfigured(_))
		));
	}

	/// Verifies that has_anthropic() returns true when configured.
	#[tokio::test]
	async fn has_anthropic_returns_true_when_configured() {
		let config = LlmServiceConfig::new(LlmProvider::Anthropic).with_anthropic_api_key("test-key");
		let service = LlmService::new(config).await.unwrap();
		assert!(service.has_anthropic());
		assert!(!service.has_openai());
	}

	/// Verifies that has_openai() returns true when configured.
	#[tokio::test]
	async fn has_openai_returns_true_when_configured() {
		let config = LlmServiceConfig::new(LlmProvider::OpenAi).with_openai_api_key("test-key");
		let service = LlmService::new(config).await.unwrap();
		assert!(!service.has_anthropic());
		assert!(service.has_openai());
	}

	/// Verifies that both providers can be configured simultaneously.
	#[tokio::test]
	async fn both_providers_can_be_configured() {
		let config = LlmServiceConfig::new(LlmProvider::Anthropic)
			.with_anthropic_api_key("anthropic-key")
			.with_openai_api_key("openai-key");
		let service = LlmService::new(config).await.unwrap();
		assert!(service.has_anthropic());
		assert!(service.has_openai());
	}

	/// Verifies that Debug implementation doesn't expose sensitive data.
	/// This is important for security - API keys should never appear in logs.
	#[tokio::test]
	async fn debug_does_not_expose_secrets() {
		let config =
			LlmServiceConfig::new(LlmProvider::Anthropic).with_anthropic_api_key("super-secret-key");
		let service = LlmService::new(config).await.unwrap();
		let debug_output = format!("{service:?}");
		assert!(!debug_output.contains("super-secret-key"));
	}
}
