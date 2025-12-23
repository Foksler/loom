// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! OpenAPI documentation for loom-server.
//!
//! This module provides the OpenAPI 3.0 specification for the Loom Server API,
//! generated from Rust types using utoipa.

use utoipa::OpenApi;

/// Main OpenAPI documentation struct.
///
/// This generates the complete OpenAPI specification for the Loom Server API.
/// Access the interactive documentation at `/docs` and the raw JSON spec at
/// `/docs/openapi.json`.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Loom Server API",
        version = "1.0.0",
        description = "AI-powered coding assistant server API. Loom provides thread persistence, LLM proxy endpoints, GitHub integration, and web search capabilities.",
        license(name = "Proprietary"),
        contact(
            name = "Geoffrey Huntley",
            email = "ghuntley@ghuntley.com",
            url = "https://ghuntley.com"
        )
    ),
    servers(
        (url = "/", description = "Local server")
    ),
    tags(
        (name = "threads", description = "Thread CRUD and search operations for conversation persistence"),
        (name = "health", description = "Health checks, metrics, and system status"),
        (name = "llm-proxy", description = "LLM provider proxy endpoints (Anthropic, OpenAI, Vertex)"),
        (name = "github", description = "GitHub App integration and code search"),
        (name = "google-cse", description = "Google Custom Search Engine proxy"),
        (name = "server-query", description = "Server query orchestration for client-server communication"),
        (name = "debug", description = "Debug and tracing endpoints for development"),
        (name = "auth", description = "Authentication endpoints (currently stubbed)")
    ),
    paths(
        // Thread endpoints
        crate::api::search_threads,
        crate::api::upsert_thread,
        crate::api::get_thread,
        crate::api::list_threads,
        crate::api::delete_thread,
        crate::api::update_thread_visibility,
        // Health endpoints
        crate::api::health_check,
        crate::api::prometheus_metrics,
        // Auth endpoints
        crate::api::login_stub,
        crate::api::logout_stub,
        // Google CSE endpoints
        crate::api::proxy_cse,
        // GitHub endpoints
        crate::api::get_github_app_info,
        crate::api::get_github_installation_by_repo,
        crate::api::proxy_github_search_code,
        crate::api::proxy_github_repo_info,
        crate::api::proxy_github_file_contents,
        // Debug endpoints
        crate::api::get_query_trace,
        crate::api::list_query_traces,
        crate::api::get_trace_stats,
    ),
    components(
        schemas(
            // API request/response types
            crate::api::SearchResponse,
            crate::api::SearchResponseHit,
            crate::api::UpdateVisibilityRequest,
            crate::api::ListResponse,
            crate::api::AuthStubResponse,
            crate::api::CseProxyRequest,
            crate::api::CseProxyResponse,
            crate::api::CseProxyResultItem,
            crate::api::GithubSearchCodeRequest,
            crate::api::GithubRepoInfoRequest,
            crate::api::GithubFileContentsRequest,
            crate::api::GithubRepoInfoResponse,
            crate::api::GithubFileContentsResponse,
            // Health types
            crate::health::HealthResponse,
            crate::health::HealthStatus,
            crate::health::HealthComponents,
            crate::health::DatabaseHealth,
            crate::health::BinDirHealth,
            crate::health::LlmProvidersHealth,
            crate::health::LlmProviderHealth,
            crate::health::GoogleCseHealth,
            crate::health::GithubAppHealth,
            // Error types
            crate::error::ErrorResponse,
            // Thread types (from loom-thread crate)
            loom_thread::Thread,
            loom_thread::ThreadId,
            loom_thread::ThreadSummary,
            loom_thread::ThreadVisibility,
            loom_thread::MessageSnapshot,
            loom_thread::MessageRole,
            loom_thread::ConversationSnapshot,
            loom_thread::AgentStateSnapshot,
            loom_thread::AgentStateKind,
            loom_thread::ThreadMetadata,
            // GitHub types (from loom-github-app crate)
            loom_github_app::AppInfoResponse,
            loom_github_app::InstallationStatusResponse,
            loom_github_app::CodeSearchRequest,
            loom_github_app::CodeSearchResponse,
            loom_github_app::CodeSearchItem,
            // Google CSE types (from loom-google-cse crate)
            loom_google_cse::CseRequest,
            loom_google_cse::CseResponse,
            loom_google_cse::CseResultItem,
        )
    )
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the OpenAPI spec generates valid JSON.
    #[test]
    fn test_openapi_spec_generates_valid_json() {
        let spec = ApiDoc::openapi();
        let json = serde_json::to_string_pretty(&spec).expect("should serialize to JSON");
        
        assert!(!json.is_empty());
        assert!(json.contains("\"openapi\""));
        assert!(json.contains("\"3.1"));
        assert!(json.contains("Loom Server API"));
    }

    /// Verify all expected tags are present.
    #[test]
    fn test_openapi_spec_has_all_tags() {
        let spec = ApiDoc::openapi();
        let json = serde_json::to_string(&spec).expect("should serialize");
        
        let expected_tags = ["threads", "health", "llm-proxy", "github", "google-cse", "server-query", "debug", "auth"];
        for tag in expected_tags {
            assert!(json.contains(tag), "Missing tag: {}", tag);
        }
    }

    /// Verify all documented endpoints are present in paths.
    #[test]
    fn test_openapi_spec_has_documented_paths() {
        let spec = ApiDoc::openapi();
        let json = serde_json::to_string(&spec).expect("should serialize");
        
        let expected_paths = [
            "/v1/threads",
            "/v1/threads/{id}",
            "/v1/threads/search",
            "/health",
            "/metrics",
            "/proxy/cse",
            "/v1/github/app",
        ];
        for path in expected_paths {
            assert!(json.contains(path), "Missing path: {}", path);
        }
    }
}
