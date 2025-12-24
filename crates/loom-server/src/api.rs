// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! HTTP API routes and handlers for thread operations.

use axum::{
	body::Bytes,
	extract::{Path, Query, State},
	http::{HeaderMap, StatusCode},
	response::IntoResponse,
	routing::{delete, get, post, put},
	Json, Router,
};
use loom_github_app::{
	AppInfoResponse, CodeSearchRequest, CodeSearchResponse, GithubAppClient, GithubAppConfig,
	GithubAppError, InstallationStatusResponse,
};
use loom_google_cse::{CseClient, CseError, CseRequest};
use loom_llm_service::LlmService;
use loom_thread::{Thread, ThreadId, ThreadSummary};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use tower_http::services::{ServeDir, ServeFile};

use crate::llm_proxy;

use crate::{
	db::{GithubInstallation, GithubRepo, ThreadRepository},
	error::{ErrorResponse, ServerError},
	health::{self, HealthComponents, HealthResponse, HealthStatus},
	query_metrics::QueryMetrics,
	query_tracing::QueryTraceStore,
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
        // Server API routes
        .route("/v1/threads/search", get(search_threads))
        .route("/v1/threads/{id}", put(upsert_thread))
        .route("/v1/threads/{id}", get(get_thread))
        .route("/v1/threads/{id}", delete(delete_thread))
        .route(
            "/v1/threads/{id}/visibility",
            post(update_thread_visibility),
        )
        .route("/v1/threads", get(list_threads))
        .route("/v1/auth/login", post(login_stub))
        .route("/v1/auth/logout", post(logout_stub))
        .route("/health", get(health_check))
        .route("/metrics", get(prometheus_metrics))
        .route("/proxy/cse", post(proxy_cse))
        // GitHub App endpoints
        .route("/v1/github/app", get(get_github_app_info))
        .route("/v1/github/webhook", post(github_webhook))
        .route(
            "/v1/github/installations/by-repo",
            get(get_github_installation_by_repo),
        )
        .route("/proxy/github/search-code", post(proxy_github_search_code))
        .route("/proxy/github/repo-info", post(proxy_github_repo_info))
        .route(
            "/proxy/github/file-contents",
            post(proxy_github_file_contents),
        )
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
        .route("/v1/debug/query-traces/{trace_id}", get(get_query_trace))
        .route("/v1/debug/query-traces", get(list_query_traces))
        .route("/v1/debug/query-traces/stats", get(get_trace_stats))
        .nest_service("/bin", ServeDir::new(&bin_dir).precompressed_gzip())
        .route("/bin", get(list_bin_directory))
        .route("/bin/", get(list_bin_directory))
        .with_state(state);

	// Add OpenAPI documentation
	router = router.merge(
		SwaggerUi::new("/docs")
			.url("/docs/openapi.json", crate::api_docs::ApiDoc::openapi())
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

/// Handler to list files in the /bin directory
async fn list_bin_directory() -> impl IntoResponse {
	let bin_dir = std::env::var("LOOM_SERVER_BIN_DIR").unwrap_or_else(|_| "./bin".to_string());
	let path = std::path::Path::new(&bin_dir);

	let mut entries = Vec::new();

	if path.exists() && path.is_dir() {
		if let Ok(read_dir) = std::fs::read_dir(path) {
			for entry in read_dir.flatten() {
				let name = entry.file_name().to_string_lossy().to_string();
				let metadata = entry.metadata().ok();
				let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
				let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
				let modified = metadata
					.as_ref()
					.and_then(|m| m.modified().ok())
					.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
					.map(|d| {
						let secs = d.as_secs();
						let dt = chrono::DateTime::from_timestamp(secs as i64, 0)
							.unwrap_or_default();
						dt.format("%Y-%m-%d %H:%M").to_string()
					})
					.unwrap_or_else(|| "-".to_string());

				entries.push((name, is_dir, size, modified));
			}
		}
	}

	entries.sort_by(|a, b| {
		match (a.1, b.1) {
			(true, false) => std::cmp::Ordering::Less,
			(false, true) => std::cmp::Ordering::Greater,
			_ => a.0.cmp(&b.0),
		}
	});

	let mut html = String::from(
		r#"<!DOCTYPE html>
<html>
<head>
<title>Index of /bin/</title>
<style>
body { font-family: monospace; margin: 2em; }
table { border-collapse: collapse; }
th, td { text-align: left; padding: 0.25em 1em; }
th { border-bottom: 1px solid #ccc; }
a { text-decoration: none; }
a:hover { text-decoration: underline; }
.dir { font-weight: bold; }
.size { text-align: right; }
</style>
</head>
<body>
<h1>Index of /bin/</h1>
<table>
<tr><th>Name</th><th>Size</th><th>Modified</th></tr>
"#,
	);

	for (name, is_dir, size, modified) in entries {
		let display_name = if is_dir {
			format!("{}/", name)
		} else {
			name.clone()
		};
		let size_str = if is_dir {
			"-".to_string()
		} else {
			format_size(size)
		};
		let class = if is_dir { " class=\"dir\"" } else { "" };
		html.push_str(&format!(
			r#"<tr><td{class}><a href="/bin/{name}">{display_name}</a></td><td class="size">{size_str}</td><td>{modified}</td></tr>
"#
		));
	}

	html.push_str("</table>\n</body>\n</html>");

	(
		StatusCode::OK,
		[("Content-Type", "text/html; charset=utf-8")],
		html,
	)
}

fn format_size(size: u64) -> String {
	const KB: u64 = 1024;
	const MB: u64 = KB * 1024;
	const GB: u64 = MB * 1024;

	if size >= GB {
		format!("{:.1}G", size as f64 / GB as f64)
	} else if size >= MB {
		format!("{:.1}M", size as f64 / MB as f64)
	} else if size >= KB {
		format!("{:.1}K", size as f64 / KB as f64)
	} else {
		format!("{size}")
	}
}

/// Query parameters for listing threads.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListParams {
	/// Filter by workspace root.
	pub workspace: Option<String>,
	/// Maximum number of results (default: 50).
	#[serde(default = "default_limit")]
	pub limit: u32,
	/// Pagination offset (default: 0).
	#[serde(default)]
	pub offset: u32,
}

fn default_limit() -> u32 {
	50
}

fn default_search_limit() -> u32 {
	50
}

/// Query parameters for search endpoint
#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchParams {
	/// Search query
	pub q: String,
	/// Optional workspace filter
	pub workspace: Option<String>,
	/// Maximum results (default: 50)
	#[serde(default = "default_search_limit")]
	pub limit: u32,
	/// Pagination offset (default: 0)
	#[serde(default)]
	pub offset: u32,
}

/// Search response
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
	pub hits: Vec<SearchResponseHit>,
	pub limit: u32,
	pub offset: u32,
}

/// Single search hit in the response
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponseHit {
	#[serde(flatten)]
	pub summary: ThreadSummary,
	pub score: f64,
}

/// Request body for updating thread visibility.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateVisibilityRequest {
	pub visibility: loom_thread::ThreadVisibility,
}

/// Response for list endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListResponse {
	pub threads: Vec<ThreadSummary>,
	pub total: u64,
	pub limit: u32,
	pub offset: u32,
}

/// Response for authentication stub endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthStubResponse {
	pub status: String,
	pub message: String,
}

/// Request body for CSE proxy endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CseProxyRequest {
	pub query: String,
	pub max_results: Option<u32>,
}

/// Response for CSE proxy endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct CseProxyResponse {
	pub query: String,
	pub results: Vec<CseProxyResultItem>,
}

/// Single result item in CSE proxy response.
#[derive(Debug, Serialize, ToSchema)]
pub struct CseProxyResultItem {
	pub title: String,
	pub url: String,
	pub snippet: String,
	pub display_link: Option<String>,
	pub rank: u32,
}

/// Query parameters for GitHub installation lookup by repo.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GithubInstallationByRepoQuery {
	pub owner: String,
	pub repo: String,
}

/// Request body for GitHub code search proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct GithubSearchCodeRequest {
	pub owner: String,
	pub repo: String,
	pub query: String,
	#[serde(default = "default_github_per_page")]
	pub per_page: u32,
	#[serde(default = "default_github_page")]
	pub page: u32,
}

fn default_github_per_page() -> u32 {
	30
}

fn default_github_page() -> u32 {
	1
}

/// Request body for GitHub repo info proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct GithubRepoInfoRequest {
	pub owner: String,
	pub repo: String,
}

/// Request body for GitHub file contents proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct GithubFileContentsRequest {
	pub owner: String,
	pub repo: String,
	pub path: String,
	#[serde(rename = "ref")]
	pub git_ref: Option<String>,
}

/// Simplified repository info response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GithubRepoInfoResponse {
	pub id: i64,
	pub full_name: String,
	pub description: Option<String>,
	pub private: bool,
	pub default_branch: String,
	pub language: Option<String>,
	pub stargazers_count: u32,
	pub html_url: String,
}

/// Simplified file contents response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GithubFileContentsResponse {
	pub name: String,
	pub path: String,
	pub sha: String,
	pub size: u64,
	pub encoding: String,
	pub content: String,
}

/// PUT /v1/threads/{id} - Create or update a thread.
///
/// Supports optimistic concurrency via If-Match header.
#[utoipa::path(
    put,
    path = "/v1/threads/{id}",
    params(
        ("id" = String, Path, description = "Thread ID")
    ),
    request_body = Thread,
    responses(
        (status = 200, description = "Thread created or updated", body = Thread),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 409, description = "Version conflict", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "threads"
)]
#[axum::debug_handler]
async fn upsert_thread(
	State(state): State<AppState>,
	Path(id): Path<String>,
	headers: HeaderMap,
	Json(mut thread): Json<Thread>,
) -> Result<impl IntoResponse, ServerError> {
	// Validate ID matches path
	if thread.id.as_str() != id {
		return Err(ServerError::BadRequest(format!(
			"Thread ID in body ({}) does not match path ({})",
			thread.id, id
		)));
	}

	// Parse If-Match header for optimistic concurrency
	let expected_version = headers
		.get("If-Match")
		.and_then(|v| v.to_str().ok())
		.and_then(|s| s.parse::<u64>().ok());

	tracing::debug!(
			thread_id = %id,
			version = thread.version,
			expected_version = ?expected_version,
			"upserting thread"
	);

	// Update timestamp
	thread.updated_at = chrono::Utc::now().to_rfc3339();

	let stored = state.repo.upsert(&thread, expected_version).await?;

	tracing::info!(
			thread_id = %id,
			version = stored.version,
			"thread upserted"
	);

	Ok((StatusCode::OK, Json(stored)))
}

/// GET /v1/threads/{id} - Get a thread by ID.
#[utoipa::path(
    get,
    path = "/v1/threads/{id}",
    params(
        ("id" = String, Path, description = "Thread ID")
    ),
    responses(
        (status = 200, description = "Thread found", body = Thread),
        (status = 404, description = "Thread not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "threads"
)]
#[axum::debug_handler]
async fn get_thread(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
	let thread_id = ThreadId::from_string(id.clone());

	tracing::debug!(thread_id = %id, "getting thread");

	let thread = state
		.repo
		.get(&thread_id)
		.await?
		.ok_or_else(|| ServerError::NotFound(id.clone()))?;

	Ok(Json(thread))
}

/// GET /v1/threads - List threads.
#[utoipa::path(
    get,
    path = "/v1/threads",
    params(ListParams),
    responses(
        (status = 200, description = "List of threads", body = ListResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "threads"
)]
#[axum::debug_handler]
async fn list_threads(
	State(state): State<AppState>,
	Query(params): Query<ListParams>,
) -> Result<impl IntoResponse, ServerError> {
	tracing::debug!(
			workspace = ?params.workspace,
			limit = params.limit,
			offset = params.offset,
			"listing threads"
	);

	let threads = state
		.repo
		.list(params.workspace.as_deref(), params.limit, params.offset)
		.await?;

	let total = state.repo.count(params.workspace.as_deref()).await?;

	let response = ListResponse {
		threads,
		total,
		limit: params.limit,
		offset: params.offset,
	};

	Ok(Json(response))
}

/// DELETE /v1/threads/{id} - Soft-delete a thread.
#[utoipa::path(
    delete,
    path = "/v1/threads/{id}",
    params(
        ("id" = String, Path, description = "Thread ID")
    ),
    responses(
        (status = 204, description = "Thread deleted"),
        (status = 404, description = "Thread not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "threads"
)]
#[axum::debug_handler]
async fn delete_thread(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
	let thread_id = ThreadId::from_string(id.clone());

	tracing::debug!(thread_id = %id, "deleting thread");

	let deleted = state.repo.delete(&thread_id).await?;

	if deleted {
		tracing::info!(thread_id = %id, "thread deleted");
		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(ServerError::NotFound(id))
	}
}

/// POST /v1/threads/{id}/visibility - Update thread visibility.
///
/// Allows changing the visibility of a thread without syncing the full thread
/// content. Supports optimistic concurrency via If-Match header.
#[utoipa::path(
    post,
    path = "/v1/threads/{id}/visibility",
    params(
        ("id" = String, Path, description = "Thread ID")
    ),
    request_body = UpdateVisibilityRequest,
    responses(
        (status = 200, description = "Visibility updated", body = Thread),
        (status = 404, description = "Thread not found", body = ErrorResponse),
        (status = 409, description = "Version conflict", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "threads"
)]
#[axum::debug_handler]
async fn update_thread_visibility(
	State(state): State<AppState>,
	Path(id): Path<String>,
	headers: HeaderMap,
	Json(body): Json<UpdateVisibilityRequest>,
) -> Result<impl IntoResponse, ServerError> {
	let thread_id = ThreadId::from_string(id.clone());

	let expected_version = headers
		.get("If-Match")
		.and_then(|v| v.to_str().ok())
		.and_then(|s| s.parse::<u64>().ok());

	tracing::debug!(
			thread_id = %id,
			visibility = ?body.visibility,
			expected_version = ?expected_version,
			"updating thread visibility"
	);

	let mut thread = state
		.repo
		.get(&thread_id)
		.await?
		.ok_or_else(|| ServerError::NotFound(id.clone()))?;

	if let Some(expected) = expected_version {
		if thread.version != expected {
			return Err(ServerError::Conflict {
				expected: thread.version,
				actual: expected,
			});
		}
	}

	thread.visibility = body.visibility;
	thread.updated_at = chrono::Utc::now().to_rfc3339();
	thread.version += 1;

	let stored = state.repo.upsert(&thread, None).await?;

	tracing::info!(
			thread_id = %id,
			version = stored.version,
			visibility = ?stored.visibility,
			"thread visibility updated"
	);

	Ok((StatusCode::OK, Json(stored)))
}

/// GET /v1/threads/search - Search threads.
#[utoipa::path(
    get,
    path = "/v1/threads/search",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 400, description = "Invalid search query", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "threads"
)]
#[axum::debug_handler]
async fn search_threads(
	State(state): State<AppState>,
	Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, ServerError> {
	let query = params.q.trim();

	if query.is_empty() {
		return Err(ServerError::BadRequest("Empty search query".into()));
	}

	tracing::debug!(
			query = %query,
			workspace = ?params.workspace,
			limit = params.limit,
			offset = params.offset,
			"searching threads"
	);

	let hits = state
		.repo
		.search(
			query,
			params.workspace.as_deref(),
			params.limit,
			params.offset,
		)
		.await?;

	let response_hits = hits
		.into_iter()
		.map(|h| SearchResponseHit {
			summary: h.summary,
			score: h.score,
		})
		.collect();

	Ok(Json(SearchResponse {
		hits: response_hits,
		limit: params.limit,
		offset: params.offset,
	}))
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "System is healthy", body = HealthResponse),
        (status = 503, description = "System is unhealthy", body = HealthResponse)
    ),
    tag = "health"
)]
/// GET /health - Comprehensive health check endpoint.
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
	use tokio::time::Instant;

	let overall_start = Instant::now();

	// Run checks in parallel
	let (database, bin_dir, google_cse, github_app) = tokio::join!(
		health::check_database(&state.repo),
		async { health::check_bin_dir() },
		health::check_google_cse(),
		health::check_github_app(state.github_client.clone())
	);

	let llm_providers = health::check_llm_providers(state.llm_service.as_deref());

	let components = HealthComponents {
		database,
		bin_dir,
		llm_providers,
		google_cse,
		github_app,
	};

	let status = health::aggregate_status(&components);
	let duration_ms = overall_start.elapsed().as_millis() as u64;

	let response = HealthResponse {
		status,
		timestamp: chrono::Utc::now().to_rfc3339(),
		duration_ms,
		version: loom_version::HealthVersionInfo::current(),
		components,
	};

	let http_status = match status {
		HealthStatus::Healthy | HealthStatus::Degraded => StatusCode::OK,
		HealthStatus::Unhealthy | HealthStatus::Unknown => StatusCode::SERVICE_UNAVAILABLE,
	};

	(http_status, Json(response))
}

#[utoipa::path(
    post,
    path = "/v1/auth/login",
    responses(
        (status = 501, description = "Not implemented", body = AuthStubResponse)
    ),
    tag = "auth"
)]
/// POST /v1/auth/login - Stub login endpoint.
async fn login_stub() -> impl IntoResponse {
	(
		StatusCode::NOT_IMPLEMENTED,
		Json(AuthStubResponse {
			status: "not_implemented".to_string(),
			message: "Authentication is not implemented yet.".to_string(),
		}),
	)
}

#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    responses(
        (status = 501, description = "Not implemented", body = AuthStubResponse)
    ),
    tag = "auth"
)]
/// POST /v1/auth/logout - Stub logout endpoint.
async fn logout_stub() -> impl IntoResponse {
	(
		StatusCode::NOT_IMPLEMENTED,
		Json(AuthStubResponse {
			status: "not_implemented".to_string(),
			message: "Logout is not implemented yet.".to_string(),
		}),
	)
}

#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, description = "Prometheus metrics", content_type = "text/plain")
    ),
    tag = "health"
)]
/// GET /metrics - Prometheus metrics export endpoint.
///
/// Returns all query metrics in Prometheus text format. Includes:
/// - Total queries sent/succeeded/failed
/// - Query latency histogram
/// - Pending queries gauge
/// - Metrics by query type and session
/// - Timeout counters by query type
async fn prometheus_metrics(
	State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
	match state.query_metrics.gather_metrics() {
		Ok(metrics) => {
			tracing::debug!("prometheus_metrics: gathering metrics");
			Ok((
				StatusCode::OK,
				[(
					axum::http::header::CONTENT_TYPE,
					"text/plain; version=0.0.4; charset=utf-8",
				)],
				metrics,
			))
		}
		Err(e) => {
			tracing::error!(error = %e, "prometheus_metrics: failed to gather metrics");
			Err(ServerError::Internal(format!(
				"Failed to gather metrics: {}",
				e
			)))
		}
	}
}

#[utoipa::path(
    post,
    path = "/proxy/cse",
    request_body = CseProxyRequest,
    responses(
        (status = 200, description = "Search results", body = CseProxyResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "CSE not configured or error", body = ErrorResponse)
    ),
    tag = "google-cse"
)]
/// POST /proxy/cse - Proxy requests to Google Custom Search Engine.
#[axum::debug_handler]
async fn proxy_cse(
	State(state): State<AppState>,
	Json(body): Json<CseProxyRequest>,
) -> Result<impl IntoResponse, ServerError> {
	let query = body.query.trim().to_string();
	let max_results = body.max_results.unwrap_or(5).clamp(1, 10);

	if query.is_empty() {
		tracing::warn!("proxy_cse: empty query");
		return Err(ServerError::BadRequest("query must not be empty".into()));
	}

	// Try cache first
	if let Some(cached) = state.repo.get_cse_cache(&query, max_results).await? {
		tracing::info!(
				query = %query,
				max_results = max_results,
				results_count = cached.results.len(),
				"proxy_cse: returning cached response"
		);

		let response = CseProxyResponse {
			query: cached.query,
			results: cached
				.results
				.into_iter()
				.map(|item| CseProxyResultItem {
					title: item.title,
					url: item.url,
					snippet: item.snippet,
					display_link: item.display_link,
					rank: item.rank,
				})
				.collect(),
		};

		return Ok((StatusCode::OK, Json(response)));
	}

	tracing::debug!(
			query = %query,
			max_results = max_results,
			"proxy_cse: cache miss, calling Google CSE"
	);

	// Get CSE client from state, or return error if not configured
	let client = state.cse_client.as_ref().ok_or_else(|| {
		tracing::error!("proxy_cse: Google CSE not configured");
		ServerError::Internal("Google CSE is not configured on the server".to_string())
	})?;

	let request = CseRequest::new(query.clone(), max_results);

	let cse_response = client.search(request).await.map_err(|e| match e {
		CseError::Timeout => {
			tracing::warn!("proxy_cse: timeout contacting Google CSE");
			ServerError::UpstreamTimeout("Google CSE request timed out".into())
		}
		CseError::RateLimited => {
			tracing::warn!("proxy_cse: rate limited by Google CSE");
			ServerError::ServiceUnavailable("Google CSE rate limit exceeded; try again later".into())
		}
		CseError::Unauthorized => {
			tracing::error!("proxy_cse: invalid API key or CSE ID");
			ServerError::Internal("Google CSE authentication failed".into())
		}
		CseError::Network(e) => {
			tracing::error!(error = %e, "proxy_cse: network error");
			ServerError::UpstreamError(format!("Failed to contact Google CSE: {}", e))
		}
		CseError::InvalidResponse(msg) => {
			tracing::error!(error = %msg, "proxy_cse: invalid response");
			ServerError::UpstreamError(format!("Invalid Google CSE response: {}", msg))
		}
		CseError::ApiError { status, message } => {
			tracing::warn!(status = status, message = %message, "proxy_cse: API error");
			ServerError::UpstreamError(format!("Google CSE error: {} - {}", status, message))
		}
	})?;

	// Store in cache
	if let Err(e) = state.repo.put_cse_cache(&cse_response, max_results).await {
		tracing::warn!(error = %e, "proxy_cse: failed to write to cache");
	}

	tracing::info!(
			query = %query,
			results_count = cse_response.results.len(),
			"proxy_cse: returning results"
	);

	let response = CseProxyResponse {
		query: cse_response.query,
		results: cse_response
			.results
			.into_iter()
			.map(|item| CseProxyResultItem {
				title: item.title,
				url: item.url,
				snippet: item.snippet,
				display_link: item.display_link,
				rank: item.rank,
			})
			.collect(),
	};

	Ok((StatusCode::OK, Json(response)))
}

// ============================================================================
// GitHub App Handlers
// ============================================================================

#[utoipa::path(
    get,
    path = "/v1/github/app",
    responses(
        (status = 200, description = "GitHub App info", body = AppInfoResponse),
        (status = 500, description = "GitHub App not configured", body = ErrorResponse)
    ),
    tag = "github"
)]
/// GET /v1/github/app - Get GitHub App configuration info.
#[axum::debug_handler]
async fn get_github_app_info(State(state): State<AppState>) -> impl IntoResponse {
	match &state.github_client {
		Some(client) => Json(AppInfoResponse {
			configured: true,
			app_slug: Some(client.app_slug().to_string()),
			installation_url: Some(client.installation_url()),
		}),
		None => Json(AppInfoResponse {
			configured: false,
			app_slug: None,
			installation_url: None,
		}),
	}
}

#[utoipa::path(
    get,
    path = "/v1/github/installations/by-repo",
    params(GithubInstallationByRepoQuery),
    responses(
        (status = 200, description = "Installation status", body = InstallationStatusResponse),
        (status = 500, description = "GitHub App not configured", body = ErrorResponse)
    ),
    tag = "github"
)]
/// GET /v1/github/installations/by-repo - Check if app is installed for a repo.
#[axum::debug_handler]
async fn get_github_installation_by_repo(
	State(state): State<AppState>,
	Query(params): Query<GithubInstallationByRepoQuery>,
) -> Result<Json<InstallationStatusResponse>, ServerError> {
	match state
		.repo
		.get_github_installation_for_repo(&params.owner, &params.repo)
		.await?
	{
		Some(info) => Ok(Json(InstallationStatusResponse::installed(
			info.installation_id,
			info.account_login,
			info.account_type,
			info.repositories_selection,
		))),
		None => Ok(Json(InstallationStatusResponse::not_installed())),
	}
}

/// POST /v1/github/webhook - Handle GitHub App webhook events.
///
/// Security: Requires webhook secret to be configured and valid signature.
#[axum::debug_handler]
async fn github_webhook(
	State(state): State<AppState>,
	headers: HeaderMap,
	body: Bytes,
) -> Result<impl IntoResponse, ServerError> {
	let event_type = headers
		.get("X-GitHub-Event")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("unknown");

	tracing::debug!(event_type = %event_type, "github_webhook: received event");

	// 1. Ensure GitHub App is configured
	let client = state.github_client.as_ref().ok_or_else(|| {
		tracing::error!("github_webhook: GitHub App not configured");
		ServerError::ServiceUnavailable("GitHub App is not configured on the server".into())
	})?;

	// 2. Enforce webhook secret presence (security requirement)
	let secret = client.webhook_secret().ok_or_else(|| {
		tracing::error!("github_webhook: webhook secret not configured");
		ServerError::Internal("GitHub webhook secret is not configured on the server".into())
	})?;

	// 3. Extract signature header (required)
	let sig_header = headers
		.get("X-Hub-Signature-256")
		.and_then(|v| v.to_str().ok())
		.ok_or_else(|| {
			tracing::warn!("github_webhook: missing X-Hub-Signature-256 header");
			ServerError::BadRequest("Missing X-Hub-Signature-256 header".into())
		})?;

	// 4. Verify signature
	if let Err(e) = loom_github_app::verify_webhook_signature(secret, sig_header, &body) {
		tracing::warn!(error = %e, "github_webhook: signature verification failed");
		return Err(ServerError::Unauthorized(
			"Invalid webhook signature".into(),
		));
	}

	// 5. Process the event
	match event_type {
		"installation" => handle_installation_webhook(&state, &body).await?,
		"installation_repositories" => handle_installation_repos_webhook(&state, &body).await?,
		_ => {
			tracing::debug!(event_type = %event_type, "github_webhook: ignoring event");
		}
	}

	Ok(StatusCode::OK)
}

/// Handle installation webhook events.
async fn handle_installation_webhook(state: &AppState, body: &[u8]) -> Result<(), ServerError> {
	use loom_github_app::types::InstallationWebhookPayload;

	let payload: InstallationWebhookPayload = serde_json::from_slice(body)
		.map_err(|e| ServerError::BadRequest(format!("Invalid webhook payload: {}", e)))?;

	tracing::info!(
			action = %payload.action,
			installation_id = payload.installation.id,
			account_login = %payload.installation.account.login,
			"github_webhook: installation event"
	);

	let now = chrono::Utc::now().to_rfc3339();

	match payload.action.as_str() {
		"created" => {
			let installation = GithubInstallation {
				installation_id: payload.installation.id,
				account_id: payload.installation.account.id,
				account_login: payload.installation.account.login.clone(),
				account_type: payload.installation.account.account_type.clone(),
				app_slug: None,
				repositories_selection: payload.installation.repository_selection.clone(),
				suspended_at: None,
				created_at: now.clone(),
				updated_at: now.clone(),
			};
			state.repo.upsert_github_installation(&installation).await?;

			let repos: Vec<GithubRepo> = payload
				.repositories
				.iter()
				.map(|r| {
					let (owner, name) = r.full_name.split_once('/').unwrap_or(("", &r.name));
					GithubRepo {
						repository_id: r.id,
						owner: owner.to_string(),
						name: name.to_string(),
						full_name: r.full_name.clone(),
						private: r.private,
						default_branch: None,
					}
				})
				.collect();

			if !repos.is_empty() {
				state
					.repo
					.add_github_installation_repos(payload.installation.id, &repos)
					.await?;
			}
		}
		"deleted" => {
			state
				.repo
				.delete_github_installation(payload.installation.id)
				.await?;
		}
		"suspended" => {
			state
				.repo
				.update_github_installation_suspension(payload.installation.id, Some(&now))
				.await?;
		}
		"unsuspended" => {
			state
				.repo
				.update_github_installation_suspension(payload.installation.id, None)
				.await?;
		}
		_ => {
			tracing::debug!(action = %payload.action, "github_webhook: ignoring installation action");
		}
	}

	Ok(())
}

/// Handle installation_repositories webhook events.
async fn handle_installation_repos_webhook(
	state: &AppState,
	body: &[u8],
) -> Result<(), ServerError> {
	use loom_github_app::types::InstallationWebhookPayload;

	let payload: InstallationWebhookPayload = serde_json::from_slice(body)
		.map_err(|e| ServerError::BadRequest(format!("Invalid webhook payload: {}", e)))?;

	tracing::info!(
			action = %payload.action,
			installation_id = payload.installation.id,
			added = payload.repositories_added.len(),
			removed = payload.repositories_removed.len(),
			"github_webhook: installation_repositories event"
	);

	if !payload.repositories_added.is_empty() {
		let repos: Vec<GithubRepo> = payload
			.repositories_added
			.iter()
			.map(|r| {
				let (owner, name) = r.full_name.split_once('/').unwrap_or(("", &r.name));
				GithubRepo {
					repository_id: r.id,
					owner: owner.to_string(),
					name: name.to_string(),
					full_name: r.full_name.clone(),
					private: r.private,
					default_branch: None,
				}
			})
			.collect();
		state
			.repo
			.add_github_installation_repos(payload.installation.id, &repos)
			.await?;
	}

	if !payload.repositories_removed.is_empty() {
		let repo_ids: Vec<i64> = payload.repositories_removed.iter().map(|r| r.id).collect();
		state
			.repo
			.remove_github_installation_repos(&repo_ids)
			.await?;
	}

	Ok(())
}

#[utoipa::path(
    post,
    path = "/proxy/github/search-code",
    request_body = GithubSearchCodeRequest,
    responses(
        (status = 200, description = "Code search results", body = CodeSearchResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "GitHub error", body = ErrorResponse)
    ),
    tag = "github"
)]
/// POST /proxy/github/search-code - Proxy code search requests.
#[axum::debug_handler]
async fn proxy_github_search_code(
	State(state): State<AppState>,
	Json(body): Json<GithubSearchCodeRequest>,
) -> Result<impl IntoResponse, ServerError> {
	let client = state.github_client.as_ref().ok_or_else(|| {
		tracing::error!("proxy_github_search_code: GitHub App not configured");
		ServerError::ServiceUnavailable("GitHub App is not configured on the server".into())
	})?;

	let installation = state
		.repo
		.get_github_installation_for_repo(&body.owner, &body.repo)
		.await?
		.ok_or_else(|| {
			ServerError::NotFound(format!(
				"GitHub App not installed for {}/{}",
				body.owner, body.repo
			))
		})?;

	tracing::debug!(
			owner = %body.owner,
			repo = %body.repo,
			query = %body.query,
			installation_id = installation.installation_id,
			"proxy_github_search_code: searching"
	);

	let request = CodeSearchRequest::new(&body.query, &body.owner, &body.repo)
		.with_per_page(body.per_page)
		.with_page(body.page);

	let response = client
		.search_code(installation.installation_id, request)
		.await
		.map_err(map_github_error)?;

	tracing::info!(
			owner = %body.owner,
			repo = %body.repo,
			total_count = response.total_count,
			"proxy_github_search_code: returning results"
	);

	Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    post,
    path = "/proxy/github/repo-info",
    request_body = GithubRepoInfoRequest,
    responses(
        (status = 200, description = "Repository info", body = GithubRepoInfoResponse),
        (status = 404, description = "Repository not found", body = ErrorResponse),
        (status = 500, description = "GitHub error", body = ErrorResponse)
    ),
    tag = "github"
)]
/// POST /proxy/github/repo-info - Get repository metadata.
#[axum::debug_handler]
async fn proxy_github_repo_info(
	State(state): State<AppState>,
	Json(body): Json<GithubRepoInfoRequest>,
) -> Result<impl IntoResponse, ServerError> {
	let client = state.github_client.as_ref().ok_or_else(|| {
		tracing::error!("proxy_github_repo_info: GitHub App not configured");
		ServerError::ServiceUnavailable("GitHub App is not configured on the server".into())
	})?;

	let installation = state
		.repo
		.get_github_installation_for_repo(&body.owner, &body.repo)
		.await?
		.ok_or_else(|| {
			ServerError::NotFound(format!(
				"GitHub App not installed for {}/{}",
				body.owner, body.repo
			))
		})?;

	tracing::debug!(
			owner = %body.owner,
			repo = %body.repo,
			installation_id = installation.installation_id,
			"proxy_github_repo_info: fetching"
	);

	let repo = client
		.get_repository(installation.installation_id, &body.owner, &body.repo)
		.await
		.map_err(map_github_error)?;

	tracing::info!(
			full_name = %repo.full_name,
			private = repo.private,
			"proxy_github_repo_info: returning info"
	);

	Ok((
		StatusCode::OK,
		Json(GithubRepoInfoResponse {
			id: repo.id,
			full_name: repo.full_name,
			description: repo.description,
			private: repo.private,
			default_branch: repo.default_branch,
			language: repo.language,
			stargazers_count: repo.stargazers_count,
			html_url: repo.html_url,
		}),
	))
}

#[utoipa::path(
    post,
    path = "/proxy/github/file-contents",
    request_body = GithubFileContentsRequest,
    responses(
        (status = 200, description = "File contents", body = GithubFileContentsResponse),
        (status = 404, description = "File not found", body = ErrorResponse),
        (status = 500, description = "GitHub error", body = ErrorResponse)
    ),
    tag = "github"
)]
/// POST /proxy/github/file-contents - Get file contents.
#[axum::debug_handler]
async fn proxy_github_file_contents(
	State(state): State<AppState>,
	Json(body): Json<GithubFileContentsRequest>,
) -> Result<impl IntoResponse, ServerError> {
	let client = state.github_client.as_ref().ok_or_else(|| {
		tracing::error!("proxy_github_file_contents: GitHub App not configured");
		ServerError::ServiceUnavailable("GitHub App is not configured on the server".into())
	})?;

	let installation = state
		.repo
		.get_github_installation_for_repo(&body.owner, &body.repo)
		.await?
		.ok_or_else(|| {
			ServerError::NotFound(format!(
				"GitHub App not installed for {}/{}",
				body.owner, body.repo
			))
		})?;

	tracing::debug!(
			owner = %body.owner,
			repo = %body.repo,
			path = %body.path,
			git_ref = ?body.git_ref,
			installation_id = installation.installation_id,
			"proxy_github_file_contents: fetching"
	);

	let contents = client
		.get_file_contents(
			installation.installation_id,
			&body.owner,
			&body.repo,
			&body.path,
			body.git_ref.as_deref(),
		)
		.await
		.map_err(map_github_error)?;

	tracing::info!(
			path = %contents.path,
			size = contents.size,
			"proxy_github_file_contents: returning contents"
	);

	Ok((
		StatusCode::OK,
		Json(GithubFileContentsResponse {
			name: contents.name,
			path: contents.path,
			sha: contents.sha,
			size: contents.size,
			encoding: contents.encoding,
			content: contents.content,
		}),
	))
}

/// Map GitHub App errors to server errors.
fn map_github_error(err: GithubAppError) -> ServerError {
	match err {
		GithubAppError::Timeout => {
			tracing::warn!("GitHub request timed out");
			ServerError::UpstreamTimeout("GitHub API request timed out".into())
		}
		GithubAppError::RateLimited => {
			tracing::warn!("GitHub rate limit exceeded");
			ServerError::ServiceUnavailable("GitHub API rate limit exceeded; try again later".into())
		}
		GithubAppError::Unauthorized => {
			tracing::error!("GitHub unauthorized");
			ServerError::Internal("GitHub App authentication failed".into())
		}
		GithubAppError::Forbidden => {
			tracing::warn!("GitHub forbidden");
			ServerError::Forbidden("Insufficient permissions for this GitHub operation".into())
		}
		GithubAppError::InstallationNotFound { owner, repo } => {
			ServerError::NotFound(format!("GitHub App not installed for {}/{}", owner, repo))
		}
		GithubAppError::Network(e) => {
			tracing::error!(error = %e, "GitHub network error");
			ServerError::UpstreamError(format!("Failed to contact GitHub: {}", e))
		}
		GithubAppError::InvalidResponse(msg) => {
			tracing::error!(error = %msg, "Invalid GitHub response");
			ServerError::UpstreamError(format!("Invalid GitHub response: {}", msg))
		}
		GithubAppError::ApiError { status, message } => {
			tracing::warn!(status = status, message = %message, "GitHub API error");
			ServerError::UpstreamError(format!("GitHub error: {} - {}", status, message))
		}
		GithubAppError::Config(msg) => {
			tracing::error!(error = %msg, "GitHub config error");
			ServerError::Internal(format!("GitHub App configuration error: {}", msg))
		}
		GithubAppError::Jwt(msg) => {
			tracing::error!(error = %msg, "GitHub JWT error");
			ServerError::Internal(format!("GitHub App JWT error: {}", msg))
		}
		GithubAppError::InvalidWebhookSignature => {
			ServerError::Unauthorized("Invalid webhook signature".into())
		}
	}
}

#[utoipa::path(
    get,
    path = "/v1/debug/query-traces/{trace_id}",
    params(
        ("trace_id" = String, Path, description = "Trace ID")
    ),
    responses(
        (status = 200, description = "Trace timeline"),
        (status = 404, description = "Trace not found", body = ErrorResponse)
    ),
    tag = "debug"
)]
/// GET /v1/debug/query-traces/{trace_id} - Get a query trace by ID.
///
/// Returns the full trace timeline with all events and their durations.
#[axum::debug_handler]
async fn get_query_trace(
	State(state): State<AppState>,
	Path(trace_id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
	use crate::query_tracing::TraceTimeline;

	tracing::debug!(trace_id = %trace_id, "fetching query trace");

	let tracer = state
		.trace_store
		.get(&trace_id)
		.await
		.ok_or_else(|| ServerError::NotFound(format!("Trace not found: {}", trace_id)))?;

	let timeline = TraceTimeline::from_tracer(&tracer);

	tracing::info!(
			trace_id = %trace_id,
			query_id = %timeline.query_id,
			total_duration_ms = timeline.total_duration_ms,
			"returning query trace"
	);

	Ok(Json(timeline))
}

#[utoipa::path(
    get,
    path = "/v1/debug/query-traces",
    params(
        ("session_id" = Option<String>, Query, description = "Filter by session ID")
    ),
    responses(
        (status = 200, description = "List of traces")
    ),
    tag = "debug"
)]
/// GET /v1/debug/query-traces - List all trace IDs.
///
/// Optionally filter by session_id query parameter.
#[axum::debug_handler]
async fn list_query_traces(
	State(state): State<AppState>,
	Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, ServerError> {
	let session_id = params.get("session_id").cloned();

	tracing::debug!(session_id = ?session_id, "listing query traces");

	let traces = if let Some(session_id) = session_id {
		state.trace_store.get_session_traces(&session_id).await
	} else {
		let trace_ids = state.trace_store.list_trace_ids().await;
		// Convert IDs back to tracers (simple version)
		let mut results = Vec::new();
		for id in trace_ids {
			if let Some(tracer) = state.trace_store.get(&id).await {
				results.push(tracer);
			}
		}
		results
	};

	let response = serde_json::json!({
			"traces": traces.iter().map(|t| {
					serde_json::json!({
							"trace_id": t.trace_id.as_str(),
							"query_id": t.query_id,
							"session_id": t.session_id,
							"event_count": t.events.len(),
							"total_duration_ms": t.total_duration().as_millis() as u64,
							"has_error": t.has_error(),
					})
			}).collect::<Vec<_>>(),
			"count": traces.len(),
	});

	Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1/debug/query-traces/stats",
    responses(
        (status = 200, description = "Trace statistics")
    ),
    tag = "debug"
)]
/// GET /v1/debug/query-traces/stats - Get trace store statistics.
///
/// Returns aggregated statistics about all traces in the store.
#[axum::debug_handler]
async fn get_trace_stats(State(state): State<AppState>) -> Result<impl IntoResponse, ServerError> {
	use std::time::Duration;

	tracing::debug!("fetching trace store statistics");

	let stats = state.trace_store.get_stats().await;
	let slow_traces = state
		.trace_store
		.get_slow_traces(Duration::from_secs(5))
		.await;

	let response = serde_json::json!({
			"total_traces": stats.total_traces,
			"traces_with_errors": stats.traces_with_errors,
			"slow_traces": stats.slow_traces,
			"avg_events_per_trace": stats.avg_events_per_trace,
			"slow_trace_details": slow_traces,
	});

	tracing::info!(
		total_traces = stats.total_traces,
		error_traces = stats.traces_with_errors,
		slow_traces = stats.slow_traces,
		"returning trace statistics"
	);

	Ok(Json(response))
}

#[cfg(test)]
mod tests {
	use super::*;
	use axum::{
		body::Body,
		http::{Request, StatusCode},
	};
	use loom_thread::{
		AgentStateKind, AgentStateSnapshot, ConversationSnapshot, ThreadMetadata, ThreadVisibility,
	};
	use tempfile::tempdir;
	use tower::ServiceExt;

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
			response.status() == StatusCode::OK || response.status() == StatusCode::SERVICE_UNAVAILABLE
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
		use std::sync::Arc;

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
