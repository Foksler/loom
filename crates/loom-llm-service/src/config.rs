//! LLM service configuration.

use std::env;

use loom_config_common::{load_secret_env, Secret, SecretString};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::ConfigError;

/// Available LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    #[default]
    Anthropic,
    OpenAi,
    Vertex,
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProvider::Anthropic => write!(f, "anthropic"),
            LlmProvider::OpenAi => write!(f, "openai"),
            LlmProvider::Vertex => write!(f, "vertex"),
        }
    }
}

impl std::str::FromStr for LlmProvider {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(LlmProvider::Anthropic),
            "openai" => Ok(LlmProvider::OpenAi),
            "vertex" => Ok(LlmProvider::Vertex),
            _ => Err(ConfigError::InvalidValue {
                key: "provider".to_string(),
                message: format!(
                    "unknown provider '{}', expected 'anthropic', 'openai', or 'vertex'",
                    s
                ),
            }),
        }
    }
}

/// Configuration for the LLM service.
///
/// API keys are stored as [`SecretString`] to prevent accidental logging.
/// Use `.expose()` to access the actual key value when needed.
#[derive(Clone, Default)]
pub struct LlmServiceConfig {
    pub provider: LlmProvider,
    pub anthropic_api_key: Option<SecretString>,
    pub anthropic_model: Option<String>,
    pub openai_api_key: Option<SecretString>,
    pub openai_model: Option<String>,
    pub openai_organization: Option<String>,
    pub vertex_project: Option<String>,
    pub vertex_location: Option<String>,
    pub vertex_model: Option<String>,
}

impl std::fmt::Debug for LlmServiceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmServiceConfig")
            .field("provider", &self.provider)
            .field("anthropic_api_key", &self.anthropic_api_key)
            .field("anthropic_model", &self.anthropic_model)
            .field("openai_api_key", &self.openai_api_key)
            .field("openai_model", &self.openai_model)
            .field("openai_organization", &self.openai_organization)
            .field("vertex_project", &self.vertex_project)
            .field("vertex_location", &self.vertex_location)
            .field("vertex_model", &self.vertex_model)
            .finish()
    }
}

impl LlmServiceConfig {
    /// Creates a new configuration with the specified provider.
    pub fn new(provider: LlmProvider) -> Self {
        Self {
            provider,
            ..Default::default()
        }
    }

    /// Loads configuration from environment variables.
    ///
    /// Supports both direct environment variables and file-based secrets:
    /// - `LOOM_SERVER_ANTHROPIC_API_KEY`: Anthropic API key (or `_FILE` suffix for file path)
    /// - `LOOM_SERVER_OPENAI_API_KEY`: OpenAI API key (or `_FILE` suffix for file path)
    ///
    /// Other environment variables:
    /// - `LOOM_SERVER_LLM_PROVIDER`: Provider to use ("anthropic", "openai", or "vertex")
    /// - `LOOM_SERVER_ANTHROPIC_MODEL`: Anthropic model name
    /// - `LOOM_SERVER_OPENAI_MODEL`: OpenAI model name
    /// - `LOOM_SERVER_OPENAI_ORGANIZATION`: OpenAI organization ID
    /// - `LOOM_SERVER_VERTEX_PROJECT`: GCP project ID for Vertex AI
    /// - `LOOM_SERVER_VERTEX_LOCATION`: GCP region for Vertex AI (e.g., "us-central1")
    /// - `LOOM_SERVER_VERTEX_MODEL`: Vertex AI model name (e.g., "gemini-1.5-pro")
    pub fn from_env() -> Result<Self, ConfigError> {
        debug!("Loading LLM service configuration from environment");

        let provider = match env::var("LOOM_SERVER_LLM_PROVIDER") {
            Ok(p) => {
                debug!(provider = %p, "Found LOOM_SERVER_LLM_PROVIDER");
                p.parse()?
            }
            Err(_) => {
                debug!("LOOM_SERVER_LLM_PROVIDER not set, using default");
                LlmProvider::default()
            }
        };

        let anthropic_api_key = load_secret_env("LOOM_SERVER_ANTHROPIC_API_KEY")?;
        let anthropic_model = env::var("LOOM_SERVER_ANTHROPIC_MODEL").ok();
        let openai_api_key = load_secret_env("LOOM_SERVER_OPENAI_API_KEY")?;
        let openai_model = env::var("LOOM_SERVER_OPENAI_MODEL").ok();
        let openai_organization = env::var("LOOM_SERVER_OPENAI_ORGANIZATION").ok();
        let vertex_project = env::var("LOOM_SERVER_VERTEX_PROJECT").ok();
        let vertex_location = env::var("LOOM_SERVER_VERTEX_LOCATION").ok();
        let vertex_model = env::var("LOOM_SERVER_VERTEX_MODEL").ok();

        info!(
            provider = %provider,
            anthropic_configured = anthropic_api_key.is_some(),
            openai_configured = openai_api_key.is_some(),
            vertex_configured = vertex_project.is_some() && vertex_location.is_some(),
            "Loaded LLM service configuration"
        );

        Ok(Self {
            provider,
            anthropic_api_key,
            anthropic_model,
            openai_api_key,
            openai_model,
            openai_organization,
            vertex_project,
            vertex_location,
            vertex_model,
        })
    }

    /// Sets the Anthropic API key.
    pub fn with_anthropic_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.anthropic_api_key = Some(Secret::new(api_key.into()));
        self
    }

    /// Sets the Anthropic model.
    pub fn with_anthropic_model(mut self, model: impl Into<String>) -> Self {
        self.anthropic_model = Some(model.into());
        self
    }

    /// Sets the OpenAI API key.
    pub fn with_openai_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.openai_api_key = Some(Secret::new(api_key.into()));
        self
    }

    /// Sets the OpenAI model.
    pub fn with_openai_model(mut self, model: impl Into<String>) -> Self {
        self.openai_model = Some(model.into());
        self
    }

    /// Sets the OpenAI organization.
    pub fn with_openai_organization(mut self, org: impl Into<String>) -> Self {
        self.openai_organization = Some(org.into());
        self
    }

    /// Sets the Vertex AI project ID.
    pub fn with_vertex_project(mut self, project: impl Into<String>) -> Self {
        self.vertex_project = Some(project.into());
        self
    }

    /// Sets the Vertex AI location (GCP region).
    pub fn with_vertex_location(mut self, location: impl Into<String>) -> Self {
        self.vertex_location = Some(location.into());
        self
    }

    /// Sets the Vertex AI model.
    pub fn with_vertex_model(mut self, model: impl Into<String>) -> Self {
        self.vertex_model = Some(model.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    mod llm_provider {
        use super::*;

        /// Verifies that LlmProvider parsing is case-insensitive and consistent.
        /// This is important because environment variables may have varying case.
        #[test]
        fn parsing_is_case_insensitive() {
            assert_eq!(
                "anthropic".parse::<LlmProvider>().unwrap(),
                LlmProvider::Anthropic
            );
            assert_eq!(
                "ANTHROPIC".parse::<LlmProvider>().unwrap(),
                LlmProvider::Anthropic
            );
            assert_eq!(
                "Anthropic".parse::<LlmProvider>().unwrap(),
                LlmProvider::Anthropic
            );
            assert_eq!(
                "openai".parse::<LlmProvider>().unwrap(),
                LlmProvider::OpenAi
            );
            assert_eq!(
                "OPENAI".parse::<LlmProvider>().unwrap(),
                LlmProvider::OpenAi
            );
            assert_eq!(
                "OpenAI".parse::<LlmProvider>().unwrap(),
                LlmProvider::OpenAi
            );
            assert_eq!(
                "vertex".parse::<LlmProvider>().unwrap(),
                LlmProvider::Vertex
            );
            assert_eq!(
                "VERTEX".parse::<LlmProvider>().unwrap(),
                LlmProvider::Vertex
            );
            assert_eq!(
                "Vertex".parse::<LlmProvider>().unwrap(),
                LlmProvider::Vertex
            );
        }

        /// Verifies that invalid provider strings produce appropriate errors.
        /// This is important for user feedback when configuration is wrong.
        #[test]
        fn invalid_provider_returns_error() {
            let result = "invalid".parse::<LlmProvider>();
            assert!(result.is_err());
        }

        proptest! {
            /// Verifies that Display and FromStr are consistent for valid providers.
            /// This is important for round-trip serialization in config files.
            #[test]
            fn display_and_parse_roundtrip(provider in prop_oneof![
                Just(LlmProvider::Anthropic),
                Just(LlmProvider::OpenAi),
                Just(LlmProvider::Vertex),
            ]) {
                let displayed = provider.to_string();
                let parsed: LlmProvider = displayed.parse().unwrap();
                prop_assert_eq!(provider, parsed);
            }
        }
    }

    mod llm_service_config {
        use super::*;

        /// Verifies that default configuration uses Anthropic as the provider.
        /// This is important because it documents the expected default behavior.
        #[test]
        fn default_uses_anthropic() {
            let config = LlmServiceConfig::default();
            assert_eq!(config.provider, LlmProvider::Anthropic);
        }

        /// Verifies that builder methods correctly set configuration values.
        /// This is important for programmatic configuration.
        #[test]
        fn builder_methods_set_values() {
            let config = LlmServiceConfig::new(LlmProvider::OpenAi)
                .with_anthropic_api_key("anthropic-key")
                .with_anthropic_model("claude-3")
                .with_openai_api_key("openai-key")
                .with_openai_model("gpt-4")
                .with_openai_organization("org-123");

            assert_eq!(config.provider, LlmProvider::OpenAi);
            assert_eq!(
                config
                    .anthropic_api_key
                    .as_ref()
                    .map(|s| s.expose().as_str()),
                Some("anthropic-key")
            );
            assert_eq!(config.anthropic_model, Some("claude-3".to_string()));
            assert_eq!(
                config.openai_api_key.as_ref().map(|s| s.expose().as_str()),
                Some("openai-key")
            );
            assert_eq!(config.openai_model, Some("gpt-4".to_string()));
            assert_eq!(config.openai_organization, Some("org-123".to_string()));
        }

        /// Verifies that Debug output never contains API key values.
        /// This is critical for security - keys must never appear in logs.
        #[test]
        fn debug_redacts_api_keys() {
            let config = LlmServiceConfig::new(LlmProvider::OpenAi)
                .with_anthropic_api_key("sk-ant-super-secret")
                .with_openai_api_key("sk-openai-super-secret");

            let debug_output = format!("{:?}", config);

            assert!(!debug_output.contains("sk-ant-super-secret"));
            assert!(!debug_output.contains("sk-openai-super-secret"));
            assert!(debug_output.contains("[REDACTED]"));
        }
    }
}
