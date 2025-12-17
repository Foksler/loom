//! Error types for Google Custom Search Engine client.

use thiserror::Error;

/// Errors that can occur when interacting with the Google CSE API.
#[derive(Debug, Error)]
pub enum CseError {
    /// Network-level error during HTTP communication.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Request timed out.
    #[error("Request timed out")]
    Timeout,

    /// Rate limit exceeded.
    #[error("Rate limit exceeded")]
    RateLimited,

    /// Invalid API key or CSE ID.
    #[error("Invalid API key or CSE ID")]
    Unauthorized,

    /// Invalid or unparseable response from Google.
    #[error("Invalid response from Google: {0}")]
    InvalidResponse(String),

    /// Google API returned an error status.
    #[error("Google API error: {status} - {message}")]
    ApiError {
        status: u16,
        message: String,
    },
}
