//! Integration layer for loom-web client requests.
//!
//! Provides HTTP handlers that bridge Leptos server functions to loom-server endpoints.
//! These handlers are called from loom-web's #[server] functions and execute
//! database operations via the ThreadRepository.

use crate::error::ServerError;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use loom_thread::{Thread, ThreadId, ThreadSummary};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};

/// Request to add a message to a thread.
/// 
/// This is sent from loom-web's `add_message()` server function.
#[derive(Debug, Deserialize)]
pub struct AddMessageRequest {
    /// Thread ID to add message to
    pub thread_id: String,
    /// Message content
    pub content: String,
    /// Message role: "user", "assistant", or "system"
    pub role: String,
}

/// Response containing a created message.
///
/// Sent back to loom-web client with message details.
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub content: String,
    pub role: String,
    pub created_at: String,
}

/// List threads (GET /api/web/threads)
///
/// Called by loom-web's `get_threads()` server function.
/// Returns paginated list of thread summaries.
#[instrument(skip_all)]
pub async fn get_threads_handler(
    State(state): State<crate::api::AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<ThreadSummary>>, ServerError> {
    info!(limit = params.limit, offset = params.offset, "Fetching threads");

    let threads = state
        .repo
        .list(params.workspace.as_deref(), params.limit, params.offset)
        .await?;

    debug!(count = threads.len(), "Retrieved threads from database");
    Ok(Json(threads))
}

/// Get single thread (GET /api/web/threads/:id)
///
/// Called by loom-web's `get_thread(id)` server function.
/// Returns complete Thread with all messages.
#[instrument(skip_all)]
pub async fn get_thread_handler(
    State(state): State<crate::api::AppState>,
    Path(id): Path<String>,
) -> Result<Json<Thread>, ServerError> {
    info!(thread_id = %id, "Fetching single thread");

    // Parse and validate thread ID
    let thread_id = ThreadId::parse(&id).map_err(|e| {
        error!(thread_id = %id, "Invalid thread ID: {}", e);
        ServerError::NotFound(format!("Invalid thread ID: {}", id))
    })?;

    let thread = state.repo.get(&thread_id).await?.ok_or_else(|| {
        error!(thread_id = %id, "Thread not found");
        ServerError::NotFound(format!("Thread not found: {}", id))
    })?;

    debug!(thread_id = %id, "Retrieved thread from database");
    Ok(Json(thread))
}

/// Create thread (PUT /api/web/threads/:id)
///
/// Called by loom-web's `create_thread(title)` server function.
/// Creates a new thread with the given title.
#[instrument(skip_all)]
pub async fn create_thread_handler(
    State(state): State<crate::api::AppState>,
    Path(id): Path<String>,
    Json(req): Json<CreateThreadRequest>,
) -> Result<(StatusCode, Json<Thread>), ServerError> {
    info!(thread_id = %id, title = %req.title, "Creating thread");

    // Validate inputs
    if id.is_empty() {
        error!("Cannot create thread: ID is empty");
        return Err(ServerError::BadRequest("Thread ID cannot be empty".into()));
    }

    if req.title.is_empty() {
        error!("Cannot create thread: title is empty");
        return Err(ServerError::BadRequest("Thread title cannot be empty".into()));
    }

    if req.title.len() > 500 {
        error!(len = req.title.len(), "Thread title exceeds maximum length");
        return Err(ServerError::BadRequest(
            "Thread title must be less than 500 characters".into(),
        ));
    }

    // Parse thread ID
    let thread_id = ThreadId::parse(&id).map_err(|e| {
        error!(thread_id = %id, "Invalid thread ID: {}", e);
        ServerError::NotFound(format!("Invalid thread ID: {}", id))
    })?;

    // Create thread with minimal data
    let mut thread = Thread::new();
    thread.id = thread_id.clone();
    thread.metadata.title = Some(req.title.clone());
    
    // Upsert to database
    let thread = state.repo.upsert(&thread, None).await?;

    debug!(thread_id = %id, "Successfully created thread");
    Ok((StatusCode::CREATED, Json(thread)))
}

/// Update thread (PUT /api/web/threads/:id)
///
/// Called by loom-web's `update_thread(id, title)` server function.
#[instrument(skip_all)]
pub async fn update_thread_handler(
    State(state): State<crate::api::AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateThreadRequest>,
) -> Result<Json<Thread>, ServerError> {
    info!(thread_id = %id, "Updating thread");

    // Validate ID
    if id.is_empty() {
        error!("Cannot update thread: ID is empty");
        return Err(ServerError::BadRequest("Thread ID cannot be empty".into()));
    }

    if req.title.is_empty() {
        error!(thread_id = %id, "Cannot update thread: title is empty");
        return Err(ServerError::BadRequest("Thread title cannot be empty".into()));
    }

    if req.title.len() > 500 {
        error!(
            thread_id = %id,
            len = req.title.len(),
            "New title exceeds maximum length"
        );
        return Err(ServerError::BadRequest(
            "Thread title must be less than 500 characters".into(),
        ));
    }

    // Parse thread ID
    let thread_id = ThreadId::parse(&id).map_err(|e| {
        error!(thread_id = %id, "Invalid thread ID: {}", e);
        ServerError::NotFound(format!("Invalid thread ID: {}", id))
    })?;

    // Fetch existing thread
    let mut thread = state.repo.get(&thread_id).await?.ok_or_else(|| {
        error!(thread_id = %id, "Thread not found");
        ServerError::NotFound(format!("Thread not found: {}", id))
    })?;

    // Update title
    thread.metadata.title = Some(req.title.clone());

    // Upsert to database
    let thread = state.repo.upsert(&thread, None).await?;

    info!(thread_id = %id, "Successfully updated thread");
    Ok(Json(thread))
}

/// Delete thread (DELETE /api/web/threads/:id)
///
/// Called by loom-web's `delete_thread(id)` server function.
/// Soft deletes the thread (sets deleted_at timestamp).
#[instrument(skip_all)]
pub async fn delete_thread_handler(
    State(state): State<crate::api::AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ServerError> {
    info!(thread_id = %id, "Deleting thread");

    if id.is_empty() {
        error!("Cannot delete thread: ID is empty");
        return Err(ServerError::BadRequest("Thread ID cannot be empty".into()));
    }

    // Parse thread ID
    let thread_id = ThreadId::parse(&id).map_err(|e| {
        error!(thread_id = %id, "Invalid thread ID: {}", e);
        ServerError::NotFound(format!("Invalid thread ID: {}", id))
    })?;

    // Check thread exists
    state.repo.get(&thread_id).await?.ok_or_else(|| {
        error!(thread_id = %id, "Thread not found");
        ServerError::NotFound(format!("Thread not found: {}", id))
    })?;

    // Delete thread (soft delete)
    state.repo.delete(&thread_id).await?;

    info!(thread_id = %id, "Successfully deleted thread");
    Ok(StatusCode::NO_CONTENT)
}

/// Search threads (GET /api/web/threads/search)
///
/// Called by loom-web's `search_threads(query)` server function.
/// Full-text search across thread titles and content.
#[instrument(skip_all)]
pub async fn search_threads_handler(
    State(state): State<crate::api::AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<ThreadSummary>>, ServerError> {
    info!(query = %params.q, "Searching threads");

    // Validate query
    if params.q.is_empty() {
        error!("Cannot search: query is empty");
        return Err(ServerError::BadRequest("Search query cannot be empty".into()));
    }

    if params.q.len() > 200 {
        error!(len = params.q.len(), "Search query exceeds maximum length");
        return Err(ServerError::BadRequest(
            "Search query must be less than 200 characters".into(),
        ));
    }

    let results = state
        .repo
        .search(&params.q, params.workspace.as_deref(), params.limit, params.offset)
        .await?;

    info!(query = %params.q, result_count = results.len(), "Search completed");
    Ok(Json(results.into_iter().map(|hit| hit.summary).collect()))
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    pub workspace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    pub workspace: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateThreadRequest {
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateThreadRequest {
    pub title: String,
}

fn default_limit() -> u32 {
    50
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_thread_request_validation() {
        let req = CreateThreadRequest {
            title: "Test Thread".to_string(),
        };
        assert!(!req.title.is_empty());
        assert!(req.title.len() < 500);
    }
}
