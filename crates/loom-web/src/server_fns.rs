//! Server-side functions for loom-web.
//!
//! These `#[server]` functions bridge the Leptos client-server divide.
//! They execute on the server and communicate with loom-server backend.

use leptos::prelude::server;
use leptos::prelude::ServerFnError;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};

/// A thread summary (from loom-server)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_activity_at: String,
    pub provider: Option<String>,
    pub model: Option<String>,
}

/// A complete thread (from loom-server)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_activity_at: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub conversation: ConversationSnapshot,
}

/// Conversation snapshot
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConversationSnapshot {
    pub messages: Vec<MessageSnapshot>,
}

/// Message in a conversation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageSnapshot {
    pub role: String,
    pub content: String,
}

/// Get the loom-server base URL from environment
fn get_loom_server_url() -> Result<String, ServerFnError> {
    let url = std::env::var("LOOM_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    debug!(url = %url, "Using loom-server URL");
    Ok(url)
}

/// Fetch all threads from the database
///
/// Returns a list of ThreadSummary objects containing basic information about each thread.
/// This is used to populate the thread list sidebar.
///
/// # Returns
/// * `Result<Vec<ThreadSummary>, ServerFnError>` - List of thread summaries or error
///
/// # Errors
/// * `ServerFnError` - Network or database errors
#[server(GetThreads, "/api")]
#[instrument(skip_all)]
pub async fn get_threads() -> Result<Vec<ThreadSummary>, ServerFnError> {
    info!("Fetching all threads from loom-server");

    let server_url = get_loom_server_url()?;
    let url = format!("{}/api/web/threads?limit=50&offset=0", server_url);

    debug!(url = %url, "Making HTTP GET request");

    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to fetch threads");
            ServerFnError::new(format!("Failed to fetch threads: {}", e))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        error!(status = %status, "Server returned error");
        return Err(ServerFnError::new(format!(
            "Server error: {}",
            status
        )));
    }

    let threads: Vec<ThreadSummary> = response.json().await.map_err(|e| {
        error!(error = %e, "Failed to parse response");
        ServerFnError::new(format!("Failed to parse response: {}", e))
    })?;

    info!(count = threads.len(), "Successfully fetched threads");
    Ok(threads)
}

/// Fetch a single thread with all its messages
///
/// Returns the complete Thread object including all messages and metadata.
/// Used when loading the thread detail view.
///
/// # Arguments
/// * `id` - The unique thread identifier
///
/// # Returns
/// * `Result<Thread, ServerFnError>` - Complete thread with messages or error
///
/// # Errors
/// * `ServerFnError::NotFound` - Thread not found in database
/// * `ServerFnError` - Network or database errors
#[server(GetThread, "/api")]
#[instrument(skip_all)]
pub async fn get_thread(id: String) -> Result<Thread, ServerFnError> {
    info!(thread_id = %id, "Fetching thread with messages");

    // Validate thread ID format
    if id.is_empty() {
        error!("Invalid thread ID: empty string");
        return Err(ServerFnError::new("Invalid thread ID"));
    }

    let server_url = get_loom_server_url()?;
    let url = format!("{}/api/web/threads/{}", server_url, urlencoding::encode(&id));

    debug!(url = %url, "Making HTTP GET request");

    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to fetch thread");
            ServerFnError::new(format!("Failed to fetch thread: {}", e))
        })?;

    if response.status() == 404 {
        error!(thread_id = %id, "Thread not found");
        return Err(ServerFnError::new("Thread not found"));
    }

    if !response.status().is_success() {
        let status = response.status();
        error!(status = %status, "Server returned error");
        return Err(ServerFnError::new(format!(
            "Server error: {}",
            status
        )));
    }

    let thread: Thread = response.json().await.map_err(|e| {
        error!(error = %e, "Failed to parse response");
        ServerFnError::new(format!("Failed to parse response: {}", e))
    })?;

    info!(thread_id = %id, "Successfully fetched thread");
    Ok(thread)
}

/// Create a new thread
///
/// Inserts a new thread into the database and returns the created thread object.
/// The thread is created with an empty message list.
///
/// # Arguments
/// * `title` - The thread title/name
///
/// # Returns
/// * `Result<Thread, ServerFnError>` - Newly created thread or error
///
/// # Errors
/// * `ServerFnError` - Database or network errors
#[server(CreateThread, "/api")]
#[instrument(skip_all)]
pub async fn create_thread(title: String) -> Result<Thread, ServerFnError> {
    info!(title = %title, "Creating new thread");

    // Validate input
    if title.is_empty() {
        error!("Cannot create thread with empty title");
        return Err(ServerFnError::new("Thread title cannot be empty"));
    }

    if title.len() > 500 {
        error!(len = title.len(), "Thread title exceeds maximum length");
        return Err(ServerFnError::new(
            "Thread title must be less than 500 characters",
        ));
    }

    // Generate new thread ID
    let thread_id = format!(
        "T-{}",
        uuid::Uuid::new_v4()
            .to_string()
            .replace("-", "")
            .to_uppercase()
    );

    let server_url = get_loom_server_url()?;
    let url = format!("{}/api/web/threads/{}", server_url, &thread_id);

    let body = serde_json::json!({
        "title": title
    });

    debug!(thread_id = %thread_id, url = %url, "Making HTTP PUT request");

    let response = reqwest::Client::new()
        .put(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to create thread");
            ServerFnError::new(format!("Failed to create thread: {}", e))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        error!(status = %status, "Server returned error");
        return Err(ServerFnError::new(format!(
            "Server error: {}",
            status
        )));
    }

    let thread: Thread = response.json().await.map_err(|e| {
        error!(error = %e, "Failed to parse response");
        ServerFnError::new(format!("Failed to parse response: {}", e))
    })?;

    info!(thread_id = %thread_id, "Successfully created thread");
    Ok(thread)
}

/// Update a thread's title
///
/// Updates the title of an existing thread in the database.
/// Updates the `updated_at` timestamp to the current time.
///
/// # Arguments
/// * `id` - The thread ID to update
/// * `title` - The new title for the thread
///
/// # Returns
/// * `Result<Thread, ServerFnError>` - Updated thread or error
///
/// # Errors
/// * `ServerFnError::NotFound` - Thread not found
/// * `ServerFnError` - Database or network errors
#[server(UpdateThread, "/api")]
#[instrument(skip_all)]
pub async fn update_thread(id: String, title: String) -> Result<Thread, ServerFnError> {
    info!(thread_id = %id, "Updating thread title");

    // Validate inputs
    if id.is_empty() {
        error!("Cannot update thread: ID is empty");
        return Err(ServerFnError::new("Thread ID cannot be empty"));
    }

    if title.is_empty() {
        error!(thread_id = %id, "Cannot update thread: title is empty");
        return Err(ServerFnError::new("Thread title cannot be empty"));
    }

    if title.len() > 500 {
        error!(
            thread_id = %id,
            len = title.len(),
            "New title exceeds maximum length"
        );
        return Err(ServerFnError::new(
            "Thread title must be less than 500 characters",
        ));
    }

    let server_url = get_loom_server_url()?;
    let url = format!("{}/api/web/threads/{}", server_url, urlencoding::encode(&id));

    let body = serde_json::json!({
        "title": title
    });

    debug!(thread_id = %id, url = %url, "Making HTTP POST request");

    let response = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to update thread");
            ServerFnError::new(format!("Failed to update thread: {}", e))
        })?;

    if response.status() == 404 {
        error!(thread_id = %id, "Thread not found");
        return Err(ServerFnError::new("Thread not found"));
    }

    if !response.status().is_success() {
        let status = response.status();
        error!(status = %status, "Server returned error");
        return Err(ServerFnError::new(format!(
            "Server error: {}",
            status
        )));
    }

    let thread: Thread = response.json().await.map_err(|e| {
        error!(error = %e, "Failed to parse response");
        ServerFnError::new(format!("Failed to parse response: {}", e))
    })?;

    info!(thread_id = %id, "Successfully updated thread title");
    Ok(thread)
}

/// Delete a thread
///
/// Removes a thread and all associated messages from the database.
/// This operation is permanent.
///
/// # Arguments
/// * `id` - The thread ID to delete
///
/// # Returns
/// * `Result<(), ServerFnError>` - Success or error
///
/// # Errors
/// * `ServerFnError::NotFound` - Thread not found
/// * `ServerFnError` - Database or network errors
#[server(DeleteThread, "/api")]
#[instrument(skip_all)]
pub async fn delete_thread(id: String) -> Result<(), ServerFnError> {
    info!(thread_id = %id, "Deleting thread");

    // Validate input
    if id.is_empty() {
        error!("Cannot delete thread: ID is empty");
        return Err(ServerFnError::new("Thread ID cannot be empty"));
    }

    let server_url = get_loom_server_url()?;
    let url = format!("{}/api/web/threads/{}", server_url, urlencoding::encode(&id));

    debug!(thread_id = %id, url = %url, "Making HTTP DELETE request");

    let response = reqwest::Client::new()
        .delete(&url)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to delete thread");
            ServerFnError::new(format!("Failed to delete thread: {}", e))
        })?;

    if response.status() == 404 {
        error!(thread_id = %id, "Thread not found");
        return Err(ServerFnError::new("Thread not found"));
    }

    if !response.status().is_success() {
        let status = response.status();
        error!(status = %status, "Server returned error");
        return Err(ServerFnError::new(format!(
            "Server error: {}",
            status
        )));
    }

    info!(thread_id = %id, "Successfully deleted thread");
    Ok(())
}

/// Full-text search threads
///
/// Searches thread titles and message content using full-text search.
/// Returns matching thread summaries in relevance order.
///
/// # Arguments
/// * `query` - The search query string
///
/// # Returns
/// * `Result<Vec<ThreadSummary>, ServerFnError>` - Matching threads or error
///
/// # Errors
/// * `ServerFnError` - Database or network errors
#[server(SearchThreads, "/api")]
#[instrument(skip_all)]
pub async fn search_threads(query: String) -> Result<Vec<ThreadSummary>, ServerFnError> {
    info!(query = %query, "Searching threads");

    // Validate input
    if query.is_empty() {
        error!("Cannot search: query is empty");
        return Err(ServerFnError::new("Search query cannot be empty"));
    }

    if query.len() > 200 {
        error!(len = query.len(), "Search query exceeds maximum length");
        return Err(ServerFnError::new(
            "Search query must be less than 200 characters",
        ));
    }

    let server_url = get_loom_server_url()?;
    let url = format!(
        "{}/api/web/threads/search?q={}",
        server_url,
        urlencoding::encode(&query)
    );

    debug!(query = %query, url = %url, "Making HTTP GET request");

    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to search threads");
            ServerFnError::new(format!("Failed to search threads: {}", e))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        error!(status = %status, "Server returned error");
        return Err(ServerFnError::new(format!(
            "Server error: {}",
            status
        )));
    }

    let threads: Vec<ThreadSummary> = response.json().await.map_err(|e| {
        error!(error = %e, "Failed to parse response");
        ServerFnError::new(format!("Failed to parse response: {}", e))
    })?;

    info!(query = %query, result_count = threads.len(), "Search completed");
    Ok(threads)
}
