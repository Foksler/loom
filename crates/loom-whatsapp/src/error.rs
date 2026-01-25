// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Error types for WhatsApp integration.

use thiserror::Error;

/// Errors that can occur when interacting with WhatsApp Cloud API.
#[derive(Debug, Error)]
pub enum WhatsAppError {
    /// Network error during HTTP request.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Request timed out.
    #[error("Request timed out")]
    Timeout,

    /// Authentication failed.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Rate limited by WhatsApp API.
    #[error("Rate limited")]
    RateLimited,

    /// WhatsApp API returned an error.
    #[error("WhatsApp API error: {status} - {message}")]
    ApiError {
        status: u16,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Invalid webhook signature.
    #[error("Invalid webhook signature")]
    InvalidWebhookSignature,

    /// Invalid challenge verification.
    #[error("Invalid challenge verification")]
    InvalidChallenge,

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// 24-hour session expired.
    #[error("Session expired - customer must message first")]
    SessionExpired,

    /// Phone verification failed.
    #[error("Phone verification failed: {0}")]
    VerificationFailed(String),

    /// JSON parsing error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Hex decoding error.
    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),

    /// HMAC error.
    #[error("HMAC error: {0}")]
    Hmac(String),

    /// Phone number not found or not linked.
    #[error("Phone number not linked")]
    PhoneNotLinked,

    /// OTP expired.
    #[error("Verification code expired")]
    OtpExpired,

    /// OTP invalid.
    #[error("Invalid verification code")]
    OtpInvalid,

    /// Too many OTP attempts.
    #[error("Too many failed attempts")]
    OtpTooManyAttempts,

    /// Phone already linked to another account.
    #[error("Phone number already linked to another account")]
    PhoneAlreadyLinked,
}

impl WhatsAppError {
    /// Create an API error from status and message.
    pub fn api_error(status: u16, message: impl Into<String>) -> Self {
        Self::ApiError {
            status,
            message: message.into(),
            source: None,
        }
    }
}

impl loom_common_http::RetryableError for WhatsAppError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_) => true,
            Self::Timeout => true,
            Self::RateLimited => true,
            Self::ApiError { status, .. } => *status >= 500,
            _ => false,
        }
    }
}

/// Result type for WhatsApp operations.
pub type Result<T> = std::result::Result<T, WhatsAppError>;
