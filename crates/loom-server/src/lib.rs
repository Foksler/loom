// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Loom thread persistence server.
//!
//! This crate provides an HTTP server for persisting and syncing Loom threads
//! to a SQLite database.

pub mod api;
pub mod api_docs;
pub mod config;
pub mod db;
pub mod error;
pub mod health;
pub mod llm_proxy;
pub mod llm_query_handler;
pub mod llm_query_processor;
pub mod query_metrics;
pub mod query_security;
pub mod query_tracing;
pub mod routes;
pub mod server_query;
pub mod websocket;

#[cfg(test)]
mod tests;

pub use api::{create_app_state, create_router, AppState};
pub use api_docs::ApiDoc;
pub use config::ServerConfig;
pub use db::{GithubInstallation, GithubInstallationInfo, GithubRepo, ThreadRepository};
pub use error::ServerError;
pub use llm_query_handler::{LlmQueryHandler, SimpleRegexDetector};
pub use llm_query_processor::LlmQueryProcessor;
pub use query_metrics::QueryMetrics;
pub use query_security::{
	PathSanitizer, QueryValidator, RateLimiter, ResultValidator, SecurityError,
};
pub use query_tracing::{QueryTraceStore, QueryTracer, TraceEvent, TraceId, TraceTimeline};
pub use server_query::ServerQueryManager;
