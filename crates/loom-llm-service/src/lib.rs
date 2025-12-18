//! Server-side LLM provider abstraction for Loom.
//!
//! This crate provides a unified interface for interacting with different LLM providers
//! (Anthropic, OpenAI) on the server side. It owns all provider clients and API keys.

mod config;
mod error;
mod service;

pub use config::{LlmProvider, LlmServiceConfig};
pub use error::{ConfigError, LlmServiceError};
pub use service::LlmService;

pub use loom_core::{LlmClient, LlmError, LlmEvent, LlmRequest, LlmResponse, LlmStream, Usage};
