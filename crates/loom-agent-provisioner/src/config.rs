// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Agent provisioner configuration.

use loom_secret::Secret;
use serde::{Deserialize, Serialize};

/// Configuration for the agent provisioner.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Kubernetes namespace for agent pods
    pub namespace: String,
    /// API key for agent endpoint authentication
    pub api_key: Secret<String>,
    /// Cleanup task interval in seconds
    pub cleanup_interval_secs: u64,
    /// Default agent TTL in hours
    pub default_ttl_hours: u32,
    /// Maximum agent TTL in hours
    pub max_ttl_hours: u32,
    /// Maximum concurrent running agents
    pub max_concurrent: u32,
    /// Timeout waiting for agent ready state in seconds
    pub ready_timeout_secs: u64,
    /// Webhook configurations
    pub webhooks: Vec<WebhookConfig>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            namespace: "loom-agents".to_string(),
            api_key: Secret::new(String::new()),
            cleanup_interval_secs: 1800, // 30 minutes
            default_ttl_hours: 4,
            max_ttl_hours: 48,
            max_concurrent: 64,
            ready_timeout_secs: 60,
            webhooks: Vec::new(),
        }
    }
}

/// Configuration for a webhook endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL
    pub url: String,
    /// Events to trigger this webhook
    pub events: Vec<WebhookEvent>,
    /// HMAC secret for signing payloads
    pub secret: Option<String>,
}

/// Events that can trigger webhooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    /// Agent successfully created
    #[serde(rename = "agent.created")]
    AgentCreated,
    /// Agent deleted (manual or cleanup)
    #[serde(rename = "agent.deleted")]
    AgentDeleted,
    /// Agent entered failed state
    #[serde(rename = "agent.failed")]
    AgentFailed,
    /// Cleanup task completed
    #[serde(rename = "agents.cleanup")]
    AgentsCleanup,
}
