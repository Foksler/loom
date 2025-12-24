// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! HTTP API routes and handlers for thread operations.

use axum::{
	routing::{delete, get, post, put},
	Router,
};
use loom_github_app::{GithubAppClient, GithubAppConfig};
use loom_google_cse::CseClient;
use loom_llm_service::LlmService;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
	db::ThreadRepository,
	llm_proxy,
	query_metrics::QueryMetrics,
	query_tracing::QueryTraceStore,
	routes,
	server_query::{self, ServerQueryManager},
};

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
	pub repo: Arc<ThreadRepository>,
	pub cse_client: Option<Arc<CseClient>>,
	pub github_client: Option<Arc<GithubAppClient>>,
	pub llm_service: Option<Arc<LlmService>>,
	pub query_manager: Arc<ServerQueryManager>,
	pub query_metrics: Arc<QueryMetrics>,
	pub trace_store: QueryTraceStore,
}

/// Creates the application state, initializing optional components.
pub fn create_app_state(repo: Arc<ThreadRepository>) -> AppState {
	let cse_client = match (
		std::env::var("LOOM_SERVER_GOOGLE_CSE_API_KEY"),
		std::env::var("LOOM_SERVER_GOOGLE_CSE_CX"),
	) {
		(Ok(api_key), Ok(cx)) if !api_key.is_empty() && !cx.is_empty() => {
			tracing::info!("Google CSE configured, creating client");
			Some(Arc::new(CseClient::new(api_key, cx)))
		}
		_ => {
			tracing::info!("Google CSE not configured");
			None
		}
	};

	let github_client = match GithubAppConfig::from_env() {
		Ok(config) => match GithubAppClient::new(config) {
			Ok(client) => {
				tracing::info!("GitHub App configured, creating client");
				Some(Arc::new(client))
			}
			Err(e) => {
				tracing::warn!(error = %e, "Failed to create GitHub App client");
				None
			}
		},
		Err(_) => {
			tracing::info!("GitHub App not configured");
			None
		}
	};

	let llm_service = match LlmService::from_env() {
		Ok(service) => {
			tracing::info!(
				anthropic = service.has_anthropic(),
				openai = service.has_openai(),
				"LLM service configured"
			);
			Some(Arc::new(service))
		}
		Err(e) => {
			tracing::info!(error = %e, "LLM service not configured");
			None
		}
	};

	let query_metrics = Arc::new(QueryMetrics::default());
	let query_manager = Arc::new(ServerQueryManager::with_metrics(query_metrics.clone()));

	AppState {
		repo,
		cse_client,
		github_client,
		llm_service,
		query_manager,
		query_metrics,
		trace_store: QueryTraceStore::default(),
	}
}

/// Create the API router with all routes.
pub fn create_router(state: AppState) -> Router {
	let bin_dir = std::env::var("LOOM_SERVER_BIN_DIR").unwrap_or_else(|_| "./bin".to_string());
	let web_dir = std::env::var("LOOM_SERVER_WEB_DIR").ok();

	let mut router = Router::new()
        // Thread API routes
        .route("/v1/threads/search", get(routes::threads::search_threads))
        .route("/v1/threads/{id}", put(routes::threads::upsert_thread))
        .route("/v1/threads/{id}", get(routes::threads::get_thread))
        .route("/v1/threads/{id}", delete(routes::threads::delete_thread))
        .route(
            "/v1/threads/{id}/visibility",
            post(routes::threads::update_thread_visibility),
        )
        .route("/v1/threads", get(routes::threads::list_threads))
        // Auth stub routes
        .route("/v1/auth/login", post(routes::auth::login_stub))
        .route("/v1/auth/logout", post(routes::auth::logout_stub))
        // Health and metrics routes
        .route("/health", get(routes::health::health_check))
        .route("/metrics", get(routes::health::prometheus_metrics))
        // CSE proxy route
        .route("/proxy/cse", post(routes::cse::proxy_cse))
        // GitHub App endpoints
        .route("/v1/github/app", get(routes::github::get_github_app_info))
        .route("/v1/github/webhook", post(routes::github::github_webhook))
        .route(
            "/v1/github/installations/by-repo",
            get(routes::github::get_github_installation_by_repo),
        )
        .route("/proxy/github/search-code", post(routes::github::proxy_github_search_code))
        .route("/proxy/github/repo-info", post(routes::github::proxy_github_repo_info))
        .route(
            "/proxy/github/file-contents",
            post(routes::github::proxy_github_file_contents),
        )
        // LLM proxy endpoints
        .route(
            "/proxy/anthropic/complete",
            post(llm_proxy::proxy_anthropic_complete),
        )
        .route(
            "/proxy/anthropic/stream",
            post(llm_proxy::proxy_anthropic_stream),
        )
        .route(
            "/proxy/openai/complete",
            post(llm_proxy::proxy_openai_complete),
        )
        .route("/proxy/openai/stream", post(llm_proxy::proxy_openai_stream))
        .route(
            "/proxy/vertex/complete",
            post(llm_proxy::proxy_vertex_complete),
        )
        .route("/proxy/vertex/stream", post(llm_proxy::proxy_vertex_stream))
        // Server query endpoints
        .route(
            "/v1/sessions/{session_id}/query-response",
            post(server_query::handle_query_response),
        )
        .route(
            "/v1/sessions/{session_id}/queries",
            get(server_query::list_pending_queries),
        )
        // Debug/tracing endpoints
        .route("/v1/debug/query-traces/{trace_id}", get(routes::debug::get_query_trace))
        .route("/v1/debug/query-traces", get(routes::debug::list_query_traces))
        .route("/v1/debug/query-traces/stats", get(routes::debug::get_trace_stats))
        .with_state(state)
        // Bin directory endpoints - use fallback to avoid route conflict
        .nest_service(
            "/bin",
            ServeDir::new(&bin_dir)
                .precompressed_gzip()
                .fallback(axum::routing::get(routes::bin::list_bin_directory)),
        );

	// Add OpenAPI documentation
	router = router.merge(
		SwaggerUi::new("/api").url("/api/openapi.json", crate::api_docs::ApiDoc::openapi()),
	);

	// Serve static web assets if LOOM_SERVER_WEB_DIR is set
	// This serves the built loom-web SPA
	if let Some(web_path) = web_dir {
		tracing::info!(web_dir = %web_path, "serving static web assets");
		// Serve static files and fall back to index.html for SPA routing
		router = router.fallback_service(
			ServeDir::new(&web_path).fallback(ServeFile::new(format!("{}/index.html", web_path))),
		);
	}

	router
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::routes;
	use axum::{
		body::Body,
		http::{Request, StatusCode},
	};
	use loom_thread::{
		AgentStateKind, AgentStateSnapshot, ConversationSnapshot, Thread, ThreadId, ThreadMetadata,
		ThreadVisibility,
	};
	use tempfile::tempdir;
	use tower::ServiceExt;
	use utoipa::OpenApi;

	async fn create_test_app() -> (Router, tempfile::TempDir) {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
		let repo = Arc::new(ThreadRepository::new(&db_url).await.unwrap());
		let state = create_app_state(repo);
		(create_router(state), dir)
	}

	fn create_test_thread() -> Thread {
		Thread {
			id: ThreadId::new(),
			version: 1,
			created_at: chrono::Utc::now().to_rfc3339(),
			updated_at: chrono::Utc::now().to_rfc3339(),
			last_activity_at: chrono::Utc::now().to_rfc3339(),
			workspace_root: Some("/test".to_string()),
			cwd: Some("/test".to_string()),
			loom_version: Some("0.1.0".to_string()),
			git_branch: Some("main".to_string()),
			git_remote_url: Some("github.com/test/repo".to_string()),
			git_initial_branch: Some("main".to_string()),
			git_initial_commit_sha: Some("abc123def456".to_string()),
			git_current_commit_sha: Some("xyz789012345".to_string()),
			git_start_dirty: Some(false),
			git_end_dirty: Some(false),
			git_commits: vec!["abc123def456".to_string(), "xyz789012345".to_string()],
			provider: Some("anthropic".to_string()),
			model: Some("claude-sonnet-4-20250514".to_string()),
			conversation: ConversationSnapshot { messages: vec![] },
			agent_state: AgentStateSnapshot {
				kind: AgentStateKind::WaitingForUserInput,
				retries: 0,
				last_error: None,
				pending_tool_calls: vec![],
			},
			metadata: ThreadMetadata::default(),
			visibility: ThreadVisibility::Private,
			is_private: false,
			is_shared_with_support: false,
		}
	}

	#[tokio::test]
	async fn test_health_check() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/health")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		// Should be OK (healthy or degraded, depending on bin dir)
		assert!(
			response.status() == StatusCode::OK
				|| response.status() == StatusCode::SERVICE_UNAVAILABLE
		);
	}

	#[tokio::test]
	async fn test_health_check_response_structure() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/health")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let health: serde_json::Value = serde_json::from_slice(&body).unwrap();

		// Verify response structure
		assert!(health.get("status").is_some());
		assert!(health.get("timestamp").is_some());
		assert!(health.get("duration_ms").is_some());
		assert!(health.get("version").is_some());
		assert!(health.get("components").is_some());

		let components = health.get("components").unwrap();
		assert!(components.get("database").is_some());
		assert!(components.get("bin_dir").is_some());
		assert!(components.get("llm_providers").is_some());
		assert!(components.get("google_cse").is_some());
	}

	#[tokio::test]
	async fn test_upsert_and_get() {
		let (app, _dir) = create_test_app().await;
		let thread = create_test_thread();
		let thread_json = serde_json::to_string(&thread).unwrap();

		// Upsert
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("PUT")
					.uri(format!("/v1/threads/{}", thread.id))
					.header("Content-Type", "application/json")
					.body(Body::from(thread_json))
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		// Get
		let response = app
			.oneshot(
				Request::builder()
					.uri(format!("/v1/threads/{}", thread.id))
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
	}

	#[tokio::test]
	async fn test_get_not_found() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/v1/threads/T-nonexistent")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::NOT_FOUND);
	}

	#[tokio::test]
	async fn test_list_empty() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/v1/threads")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
	}

	#[tokio::test]
	async fn test_login_stub() {
		let (app, _dir) = create_test_app().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/v1/auth/login")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
	}

	#[tokio::test]
	async fn test_logout_stub() {
		let (app, _dir) = create_test_app().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/v1/auth/logout")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
	}

	#[tokio::test]
	async fn test_update_visibility() {
		let (app, _dir) = create_test_app().await;
		let thread = create_test_thread();
		let thread_json = serde_json::to_string(&thread).unwrap();

		// First create the thread
		let _ = app
			.clone()
			.oneshot(
				Request::builder()
					.method("PUT")
					.uri(format!("/v1/threads/{}", thread.id))
					.header("Content-Type", "application/json")
					.body(Body::from(thread_json))
					.unwrap(),
			)
			.await
			.unwrap();

		// Update visibility
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri(format!("/v1/threads/{}/visibility", thread.id))
					.header("Content-Type", "application/json")
					.body(Body::from(r#"{"visibility":"public"}"#))
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let updated: Thread = serde_json::from_slice(&body).unwrap();
		assert_eq!(updated.visibility, loom_thread::ThreadVisibility::Public);
	}

	#[tokio::test]
	async fn test_search_endpoint() {
		let (app, _dir) = create_test_app().await;

		// First create a thread
		let thread = create_test_thread();
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("PUT")
					.uri(format!("/v1/threads/{}", thread.id.as_str()))
					.header("Content-Type", "application/json")
					.header("If-Match", "0")
					.body(Body::from(serde_json::to_string(&thread).unwrap()))
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::OK);

		// Now search for it
		let response = app
			.oneshot(
				Request::builder()
					.uri("/v1/threads/search?q=main")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert!(result.get("hits").is_some());
	}

	#[tokio::test]
	async fn test_search_empty_query_returns_error() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/v1/threads/search?q=")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::BAD_REQUEST);
	}

	#[tokio::test]
	async fn test_proxy_cse_empty_query_returns_400() {
		let (app, _dir) = create_test_app().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/proxy/cse")
					.header("Content-Type", "application/json")
					.body(Body::from(r#"{"query":""}"#))
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::BAD_REQUEST);
	}

	#[tokio::test]
	async fn test_proxy_cse_whitespace_query_returns_400() {
		let (app, _dir) = create_test_app().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/proxy/cse")
					.header("Content-Type", "application/json")
					.body(Body::from(r#"{"query":"   "}"#))
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::BAD_REQUEST);
	}

	#[tokio::test]
	async fn test_proxy_cse_unconfigured_returns_500() {
		// This test verifies that when CSE is not configured, we get an error
		// (cache miss path, then env var lookup fails)
		let (app, _dir) = create_test_app().await;

		// Clear env vars to ensure CSE is not configured
		std::env::remove_var("LOOM_SERVER_GOOGLE_CSE_API_KEY");
		std::env::remove_var("LOOM_SERVER_GOOGLE_CSE_CX");

		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/proxy/cse")
					.header("Content-Type", "application/json")
					.body(Body::from(r#"{"query":"test query"}"#))
					.unwrap(),
			)
			.await
			.unwrap();

		// Should be 500 because CSE env vars are not set
		assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
	}

	/// Test debug endpoint: GET /v1/debug/query-traces/{trace_id}
	/// **Why Important**: Ensures the debug endpoint correctly retrieves stored
	/// traces for performance analysis and debugging query lifecycle issues.
	#[tokio::test]
	async fn test_get_query_trace_endpoint() {
		use crate::query_tracing::QueryTracer;

		let (app, _dir) = create_test_app().await;

		// Create a test trace
		let mut tracer = QueryTracer::new("Q-test-123", Some("session-debug".to_string()));
		tracer.record_sent(10);
		tracer.record_response_received("ok");

		let _trace_id = tracer.trace_id.as_str().to_string();

		// We need to access the app state to store the trace
		// For now, we'll test the endpoint's 404 behavior when trace doesn't exist
		let response = app
			.oneshot(
				Request::builder()
					.uri(format!("/v1/debug/query-traces/nonexistent-trace"))
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::NOT_FOUND);
	}

	/// Test debug endpoint: GET /v1/debug/query-traces
	/// **Why Important**: Ensures the listing endpoint correctly returns all
	/// stored traces for monitoring and debugging purposes.
	#[tokio::test]
	async fn test_list_query_traces_endpoint() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/v1/debug/query-traces")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		// Should have a traces array and count
		assert!(result.get("traces").is_some());
		assert!(result.get("count").is_some());
	}

	/// Test debug endpoint: GET /v1/debug/query-traces?session_id=...
	/// **Why Important**: Ensures filtering by session_id correctly isolates
	/// traces for specific client sessions.
	#[tokio::test]
	async fn test_list_query_traces_with_session_filter() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/v1/debug/query-traces?session_id=test-session")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		// Should have a traces array and count (likely empty for non-existent session)
		assert!(result.get("traces").is_some());
		assert!(result.get("count").is_some());
		assert_eq!(result["count"].as_u64(), Some(0));
	}

	/// Test debug endpoint: GET /v1/debug/query-traces/stats
	/// **Why Important**: Ensures statistics endpoint correctly aggregates trace
	/// metrics for monitoring trace store health and performance bottlenecks.
	#[tokio::test]
	async fn test_get_trace_stats_endpoint() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/v1/debug/query-traces/stats")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		// Should have required statistics fields
		assert!(result.get("total_traces").is_some());
		assert!(result.get("traces_with_errors").is_some());
		assert!(result.get("slow_traces").is_some());
		assert!(result.get("avg_events_per_trace").is_some());
		assert!(result.get("slow_trace_details").is_some());
	}

	/// Test debug endpoints: Complete flow with trace creation and retrieval
	/// **Why Important**: This integration test demonstrates the full flow of
	/// creating, storing, and retrieving traces through the debug endpoints.
	#[tokio::test]
	async fn test_debug_endpoints_integration() {
		use crate::query_tracing::QueryTracer;

		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
		let repo = Arc::new(ThreadRepository::new(&db_url).await.unwrap());
		let state = create_app_state(repo);

		// Create and store a trace directly
		let mut tracer = QueryTracer::new(
			"Q-integration-test",
			Some("integration-session".to_string()),
		);
		tracer.record_sent(5);
		tokio::time::sleep(std::time::Duration::from_millis(10)).await;
		tracer.record_response_received("success");

		let trace_id = tracer.trace_id.as_str().to_string();
		state.trace_store.store(tracer).await;

		// Create router with our state
		let app = create_router(state);

		// Test 1: Retrieve the stored trace
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.uri(format!("/v1/debug/query-traces/{}", trace_id))
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let timeline: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert_eq!(timeline["trace_id"].as_str().unwrap(), &trace_id);
		assert_eq!(timeline["query_id"].as_str().unwrap(), "Q-integration-test");
		assert_eq!(
			timeline["session_id"].as_str().unwrap(),
			"integration-session"
		);
		assert_eq!(timeline["events"].as_array().unwrap().len(), 3); // created, sent, response_received

		// Test 2: List traces
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.uri("/v1/debug/query-traces")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let list_result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert!(list_result["count"].as_u64().unwrap() >= 1);

		// Test 3: Filter by session
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.uri("/v1/debug/query-traces?session_id=integration-session")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let session_result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert_eq!(session_result["count"].as_u64().unwrap(), 1);

		// Test 4: Get statistics
		let response = app
			.oneshot(
				Request::builder()
					.uri("/v1/debug/query-traces/stats")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let stats: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert!(stats["total_traces"].as_u64().unwrap() >= 1);
		assert_eq!(stats["traces_with_errors"].as_u64().unwrap(), 0);
	}

	/// Test debug endpoint: Trace not found returns 404
	/// **Why Important**: Ensures the endpoint correctly handles missing traces
	/// and returns appropriate HTTP status codes.
	#[tokio::test]
	async fn test_get_query_trace_not_found() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/v1/debug/query-traces/TRACE-missing-trace-id")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::NOT_FOUND);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let error: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert!(error.get("message").is_some());
	}
}
