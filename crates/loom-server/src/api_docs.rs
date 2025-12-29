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
/// Access the interactive documentation at `/api` and the raw JSON spec at
/// `/api/openapi.json`.
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
        (name = "auth", description = "Authentication endpoints (currently stubbed)"),
        (name = "agents", description = "Agent provisioning and management")
    ),
    paths(
        // Thread endpoints
        crate::routes::threads::search_threads,
        crate::routes::threads::upsert_thread,
        crate::routes::threads::get_thread,
        crate::routes::threads::list_threads,
        crate::routes::threads::delete_thread,
        crate::routes::threads::update_thread_visibility,
        // Health endpoints
        crate::routes::health::health_check,
        crate::routes::health::prometheus_metrics,
        // Auth endpoints
        crate::routes::auth::login_stub,
        crate::routes::auth::logout_stub,
        // Google CSE endpoints
        crate::routes::cse::proxy_cse,
        // GitHub endpoints
        crate::routes::github::get_github_app_info,
        crate::routes::github::get_github_installation_by_repo,
        crate::routes::github::proxy_github_search_code,
        crate::routes::github::proxy_github_repo_info,
        crate::routes::github::proxy_github_file_contents,
        // Debug endpoints
        crate::routes::debug::get_query_trace,
        crate::routes::debug::list_query_traces,
        crate::routes::debug::get_trace_stats,
        // Agent endpoints
        crate::routes::agent::create_agent,
        crate::routes::agent::list_agents,
        crate::routes::agent::get_agent,
        crate::routes::agent::delete_agent,
        crate::routes::agent::stream_logs,
        crate::routes::agent::trigger_cleanup,
    ),
    components(
        schemas(
            // API request/response types
            crate::routes::threads::SearchResponse,
            crate::routes::threads::SearchResponseHit,
            crate::routes::threads::UpdateVisibilityRequest,
            crate::routes::threads::ListResponse,
            crate::routes::auth::AuthStubResponse,
            crate::routes::cse::CseProxyRequest,
            crate::routes::cse::CseProxyResponse,
            crate::routes::cse::CseProxyResultItem,
            crate::routes::github::GithubSearchCodeRequest,
            crate::routes::github::GithubRepoInfoRequest,
            crate::routes::github::GithubFileContentsRequest,
            crate::routes::github::GithubRepoInfoResponse,
            crate::routes::github::GithubFileContentsResponse,
            // Health types
            crate::health::HealthResponse,
            crate::health::HealthStatus,
            crate::health::HealthComponents,
            crate::health::DatabaseHealth,
            crate::health::BinDirHealth,
            crate::health::LlmProvidersHealth,
            crate::health::LlmProviderHealth,
            crate::health::AnthropicAccountHealth,
            crate::health::AnthropicAccountStatus,
            crate::health::AnthropicPoolHealth,
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
            // Agent types
            crate::routes::agent::CreateAgentApiRequest,
            crate::routes::agent::AgentApiResponse,
            crate::routes::agent::AgentStatusApi,
            crate::routes::agent::ListAgentsApiResponse,
            crate::routes::agent::CleanupApiResponse,
            crate::routes::agent::ResourceSpecApi,
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

		let expected_tags = [
			"threads",
			"health",
			"llm-proxy",
			"github",
			"google-cse",
			"server-query",
			"debug",
			"auth",
			"agents",
		];
		for tag in expected_tags {
			assert!(json.contains(tag), "Missing tag: {tag}");
		}
	}

	/// Verify all documented endpoints are present in paths.
	#[test]
	fn test_openapi_spec_has_documented_paths() {
		let spec = ApiDoc::openapi();
		let json = serde_json::to_string(&spec).expect("should serialize");

		let expected_paths = [
			"/api/threads",
			"/api/threads/{id}",
			"/api/threads/search",
			"/health",
			"/metrics",
			"/proxy/cse",
			"/api/github/app",
		];
		for path in expected_paths {
			assert!(json.contains(path), "Missing path: {path}");
		}
	}
}
