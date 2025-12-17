//! Server error types and HTTP response conversions.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Server error types for thread operations.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Database operation failed.
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),

    /// Thread not found.
    #[error("Thread not found: {0}")]
    NotFound(String),

    /// Version conflict during update.
    #[error("Version conflict: expected {expected}, got {actual}")]
    Conflict { expected: u64, actual: u64 },

    /// Invalid request payload.
    #[error("Invalid request: {0}")]
    BadRequest(String),

    /// Internal server error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_version: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_version: Option<u64>,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, error_response) = match &self {
            ServerError::Db(e) => {
                tracing::error!(error = %e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error: "database_error".to_string(),
                        message: "A database error occurred".to_string(),
                        server_version: None,
                        client_version: None,
                    },
                )
            }
            ServerError::NotFound(id) => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: "not_found".to_string(),
                    message: format!("Thread not found: {}", id),
                    server_version: None,
                    client_version: None,
                },
            ),
            ServerError::Conflict { expected, actual } => (
                StatusCode::CONFLICT,
                ErrorResponse {
                    error: "conflict".to_string(),
                    message: format!(
                        "Version conflict: expected {}, got {}",
                        expected, actual
                    ),
                    server_version: Some(*expected),
                    client_version: Some(*actual),
                },
            ),
            ServerError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: "bad_request".to_string(),
                    message: msg.clone(),
                    server_version: None,
                    client_version: None,
                },
            ),
            ServerError::Internal(msg) => {
                tracing::error!(error = %msg, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error: "internal_error".to_string(),
                        message: "An internal error occurred".to_string(),
                        server_version: None,
                        client_version: None,
                    },
                )
            }
            ServerError::Serialization(e) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: "serialization_error".to_string(),
                    message: format!("Invalid JSON: {}", e),
                    server_version: None,
                    client_version: None,
                },
            ),
        };

        (status, Json(error_response)).into_response()
    }
}
