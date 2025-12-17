//! HTTP API routes and handlers for thread operations.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use tower_http::services::ServeDir;
use loom_thread::{Thread, ThreadId, ThreadSummary};
use loom_google_cse::{CseClient, CseError, CseRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{db::ThreadRepository, error::ServerError, health::{self, HealthComponents, HealthResponse, HealthStatus}};

/// Application state shared across handlers.
pub type AppState = Arc<ThreadRepository>;

/// Create the API router with all routes.
pub fn create_router(repo: Arc<ThreadRepository>) -> Router {
    let bin_dir = std::env::var("LOOM_SERVER_BIN_DIR").unwrap_or_else(|_| "./bin".to_string());

    Router::new()
        .route("/v1/threads/search", get(search_threads))
        .route("/v1/threads/{id}", put(upsert_thread))
        .route("/v1/threads/{id}", get(get_thread))
        .route("/v1/threads/{id}", delete(delete_thread))
        .route("/v1/threads/{id}/visibility", post(update_thread_visibility))
        .route("/v1/threads", get(list_threads))
        .route("/v1/auth/login", post(login_stub))
        .route("/v1/auth/logout", post(logout_stub))
        .route("/health", get(health_check))
        .route("/proxy/cse", post(proxy_cse))
        .nest_service("/bin", ServeDir::new(bin_dir))
        .with_state(repo)
}

/// Query parameters for listing threads.
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchResponseHit>,
    pub limit: u32,
    pub offset: u32,
}

/// Single search hit in the response
#[derive(Debug, Serialize)]
pub struct SearchResponseHit {
    #[serde(flatten)]
    pub summary: ThreadSummary,
    pub score: f64,
}

/// Request body for updating thread visibility.
#[derive(Debug, Deserialize)]
pub struct UpdateVisibilityRequest {
    pub visibility: loom_thread::ThreadVisibility,
}

/// Response for list endpoint.
#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub threads: Vec<ThreadSummary>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

/// Response for authentication stub endpoints.
#[derive(Debug, Serialize)]
pub struct AuthStubResponse {
    pub status: String,
    pub message: String,
}

/// Request body for CSE proxy endpoint.
#[derive(Debug, Deserialize)]
pub struct CseProxyRequest {
    pub query: String,
    pub max_results: Option<u32>,
}

/// Response for CSE proxy endpoint.
#[derive(Debug, Serialize)]
pub struct CseProxyResponse {
    pub query: String,
    pub results: Vec<CseProxyResultItem>,
}

/// Single result item in CSE proxy response.
#[derive(Debug, Serialize)]
pub struct CseProxyResultItem {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub display_link: Option<String>,
    pub rank: u32,
}

/// PUT /v1/threads/{id} - Create or update a thread.
///
/// Supports optimistic concurrency via If-Match header.
#[axum::debug_handler]
async fn upsert_thread(
    State(repo): State<AppState>,
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

    let stored = repo.upsert(&thread, expected_version).await?;

    tracing::info!(
        thread_id = %id,
        version = stored.version,
        "thread upserted"
    );

    Ok((StatusCode::OK, Json(stored)))
}

/// GET /v1/threads/{id} - Get a thread by ID.
#[axum::debug_handler]
async fn get_thread(
    State(repo): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let thread_id = ThreadId::from_string(id.clone());

    tracing::debug!(thread_id = %id, "getting thread");

    let thread = repo
        .get(&thread_id)
        .await?
        .ok_or_else(|| ServerError::NotFound(id.clone()))?;

    Ok(Json(thread))
}

/// GET /v1/threads - List threads.
#[axum::debug_handler]
async fn list_threads(
    State(repo): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<impl IntoResponse, ServerError> {
    tracing::debug!(
        workspace = ?params.workspace,
        limit = params.limit,
        offset = params.offset,
        "listing threads"
    );

    let threads = repo
        .list(params.workspace.as_deref(), params.limit, params.offset)
        .await?;

    let total = repo.count(params.workspace.as_deref()).await?;

    let response = ListResponse {
        threads,
        total,
        limit: params.limit,
        offset: params.offset,
    };

    Ok(Json(response))
}

/// DELETE /v1/threads/{id} - Soft-delete a thread.
#[axum::debug_handler]
async fn delete_thread(
    State(repo): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
    let thread_id = ThreadId::from_string(id.clone());

    tracing::debug!(thread_id = %id, "deleting thread");

    let deleted = repo.delete(&thread_id).await?;

    if deleted {
        tracing::info!(thread_id = %id, "thread deleted");
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ServerError::NotFound(id))
    }
}

/// POST /v1/threads/{id}/visibility - Update thread visibility.
///
/// Allows changing the visibility of a thread without syncing the full thread content.
/// Supports optimistic concurrency via If-Match header.
#[axum::debug_handler]
async fn update_thread_visibility(
    State(repo): State<AppState>,
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

    let mut thread = repo
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

    let stored = repo.upsert(&thread, None).await?;

    tracing::info!(
        thread_id = %id,
        version = stored.version,
        visibility = ?stored.visibility,
        "thread visibility updated"
    );

    Ok((StatusCode::OK, Json(stored)))
}

/// GET /v1/threads/search - Search threads.
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
        .search(query, params.workspace.as_deref(), params.limit, params.offset)
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

/// GET /health - Comprehensive health check endpoint.
async fn health_check(State(repo): State<AppState>) -> impl IntoResponse {
    use tokio::time::Instant;
    
    let overall_start = Instant::now();

    // Run checks in parallel
    let (database, bin_dir, google_cse) = tokio::join!(
        health::check_database(&repo),
        async { health::check_bin_dir() },
        health::check_google_cse()
    );
    
    let llm_providers = health::check_llm_providers();

    let components = HealthComponents {
        database,
        bin_dir,
        llm_providers,
        google_cse,
    };

    let status = health::aggregate_status(&components);
    let duration_ms = overall_start.elapsed().as_millis() as u64;

    let response = HealthResponse {
        status,
        timestamp: chrono::Utc::now().to_rfc3339(),
        duration_ms,
        version: health::VERSION_INFO,
        components,
    };

    let http_status = match status {
        HealthStatus::Healthy | HealthStatus::Degraded => StatusCode::OK,
        HealthStatus::Unhealthy | HealthStatus::Unknown => StatusCode::SERVICE_UNAVAILABLE,
    };

    (http_status, Json(response))
}

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

/// POST /proxy/cse - Proxy requests to Google Custom Search Engine.
#[axum::debug_handler]
async fn proxy_cse(
    State(repo): State<AppState>,
    Json(body): Json<CseProxyRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let query = body.query.trim().to_string();
    let max_results = body.max_results.unwrap_or(5).min(10);

    if query.is_empty() {
        tracing::warn!("proxy_cse: empty query");
        return Err(ServerError::BadRequest("query must not be empty".into()));
    }

    // Try cache first
    if let Some(cached) = repo.get_cse_cache(&query, max_results).await? {
        tracing::info!(
            query = %query,
            max_results = max_results,
            results_count = cached.results.len(),
            "proxy_cse: returning cached response"
        );

        let response = CseProxyResponse {
            query: cached.query,
            results: cached.results.into_iter().map(|item| CseProxyResultItem {
                title: item.title,
                url: item.url,
                snippet: item.snippet,
                display_link: item.display_link,
                rank: item.rank,
            }).collect(),
        };

        return Ok((StatusCode::OK, Json(response)));
    }

    tracing::debug!(
        query = %query,
        max_results = max_results,
        "proxy_cse: cache miss, calling Google CSE"
    );

    // Load secrets from environment
    let api_key = std::env::var("LOOM_SERVER_GOOGLE_CSE_API_KEY").map_err(|_| {
        tracing::error!("proxy_cse: LOOM_SERVER_GOOGLE_CSE_API_KEY not configured");
        ServerError::Internal("Google CSE is not configured on the server".to_string())
    })?;

    let cx = std::env::var("LOOM_SERVER_GOOGLE_CSE_CX").map_err(|_| {
        tracing::error!("proxy_cse: LOOM_SERVER_GOOGLE_CSE_CX not configured");
        ServerError::Internal("Google CSE is not configured on the server".to_string())
    })?;

    tracing::debug!(
        query = %query,
        max_results = max_results,
        "proxy_cse: performing Google CSE request"
    );

    let client = CseClient::new(api_key, cx);
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

    // Store in cache (best-effort, don't fail request if cache write fails)
    if let Err(e) = repo.put_cse_cache(&cse_response, max_results).await {
        tracing::warn!(error = %e, "proxy_cse: failed to write to cache");
    }

    tracing::info!(
        query = %query,
        results_count = cse_response.results.len(),
        "proxy_cse: returning results"
    );

    let response = CseProxyResponse {
        query: cse_response.query,
        results: cse_response.results.into_iter().map(|item| CseProxyResultItem {
            title: item.title,
            url: item.url,
            snippet: item.snippet,
            display_link: item.display_link,
            rank: item.rank,
        }).collect(),
    };

    Ok((StatusCode::OK, Json(response)))
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
        (create_router(repo), dir)
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
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should be OK (healthy or degraded, depending on bin dir)
        assert!(response.status() == StatusCode::OK || response.status() == StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_health_check_response_structure() {
        let (app, _dir) = create_test_app().await;

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
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
}
