//! Error types for the LLM service.

use loom_core::LlmError;

/// Errors that can occur when configuring or using the LLM service.
#[derive(Debug, thiserror::Error)]
pub enum LlmServiceError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),
}

/// Errors that can occur when loading configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid value for {key}: {message}")]
    InvalidValue { key: String, message: String },
}
