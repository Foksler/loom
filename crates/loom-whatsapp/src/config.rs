// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Configuration for WhatsApp integration.

use chrono::{DateTime, Utc};
use loom_common_secret::SecretString;
use serde::{Deserialize, Serialize};

/// WhatsApp configuration for an organization.
#[derive(Debug, Clone)]
pub struct WhatsAppConfig {
    /// Unique identifier.
    pub id: String,

    /// Organization ID this config belongs to.
    pub org_id: String,

    /// WhatsApp Phone Number ID from Meta Business.
    pub phone_number_id: String,

    /// Access token for WhatsApp Cloud API (encrypted at rest).
    pub access_token: SecretString,

    /// App secret hash for webhook signature verification.
    pub app_secret_hash: String,

    /// Verify token hash for webhook challenge verification.
    pub verify_token_hash: String,

    /// Whether this configuration is enabled.
    pub enabled: bool,

    /// When this config was created.
    pub created_at: DateTime<Utc>,

    /// When this config was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Request to create or update WhatsApp configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WhatsAppConfigRequest {
    /// WhatsApp Phone Number ID.
    pub phone_number_id: String,

    /// Access token (will be encrypted).
    pub access_token: String,

    /// App secret (will be hashed).
    pub app_secret: String,

    /// Verify token for webhook challenge (will be hashed).
    pub verify_token: String,
}

/// Response for WhatsApp configuration status (without secrets).
#[derive(Debug, Clone, Serialize)]
pub struct WhatsAppConfigResponse {
    pub id: String,
    pub org_id: String,
    pub phone_number_id: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&WhatsAppConfig> for WhatsAppConfigResponse {
    fn from(config: &WhatsAppConfig) -> Self {
        Self {
            id: config.id.clone(),
            org_id: config.org_id.clone(),
            phone_number_id: config.phone_number_id.clone(),
            enabled: config.enabled,
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

/// Client configuration for WhatsApp API.
#[derive(Debug, Clone)]
pub struct WhatsAppClientConfig {
    /// Base URL for WhatsApp Cloud API.
    pub base_url: String,

    /// Request timeout in seconds.
    pub timeout_secs: u64,

    /// Phone Number ID for sending messages.
    pub phone_number_id: String,

    /// Access token for authentication.
    pub access_token: SecretString,
}

impl Default for WhatsAppClientConfig {
    fn default() -> Self {
        Self {
            base_url: "https://graph.facebook.com/v21.0".to_string(),
            timeout_secs: 30,
            phone_number_id: String::new(),
            access_token: SecretString::new(String::new()),
        }
    }
}

impl WhatsAppClientConfig {
    /// Create config from environment variables.
    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("LOOM_SERVER_WHATSAPP_BASE_URL")
                .unwrap_or_else(|_| "https://graph.facebook.com/v21.0".to_string()),
            timeout_secs: std::env::var("LOOM_SERVER_WHATSAPP_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            phone_number_id: String::new(),
            access_token: SecretString::new(String::new()),
        }
    }

    /// Set phone number ID.
    pub fn with_phone_number_id(mut self, phone_number_id: impl Into<String>) -> Self {
        self.phone_number_id = phone_number_id.into();
        self
    }

    /// Set access token.
    pub fn with_access_token(mut self, access_token: SecretString) -> Self {
        self.access_token = access_token;
        self
    }
}
