//! HTTP API routes and handlers for thread operations.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use loom_thread::{Thread, ThreadId, ThreadSummary};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{db::ThreadRepository, error::ServerError};

/// Application state shared across handlers.
pub type AppState = Arc<ThreadRepository>;

/// Create the API router with all routes.
pub fn create_router(repo: Arc<ThreadRepository>) -> Router {
    Router::new()
        .route("/v1/threads/{id}", put(upsert_thread))
        .route("/v1/threads/{id}", get(get_thread))
        .route("/v1/threads/{id}", delete(delete_thread))
        .route("/v1/threads", get(list_threads))
        .route("/health", get(health_check))
        .route("/v1/auth/login", post(login_stub))
        .route("/v1/auth/logout", post(logout_stub))
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

/// Response for list endpoint.
#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub threads: Vec<ThreadSummary>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Response for authentication stub endpoints.
#[derive(Debug, Serialize)]
pub struct AuthStubResponse {
    pub status: String,
    pub message: String,
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

/// GET /health - Health check endpoint.
async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use loom_thread::{AgentStateKind, AgentStateSnapshot, ConversationSnapshot, ThreadMetadata};
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
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        let (app, _dir) = create_test_app().await;

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
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
}
