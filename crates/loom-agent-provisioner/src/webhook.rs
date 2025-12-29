// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Webhook dispatch system for agent lifecycle events.

use crate::config::{WebhookConfig, WebhookEvent};
use crate::types::Agent;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;
use std::collections::HashMap;
use tracing::{debug, error, warn};

type HmacSha256 = Hmac<Sha256>;

/// Payload sent to webhook endpoints.
#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    /// Event type (e.g., "agent.created")
    pub event: String,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Agent data for single-agent events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<WebhookAgentPayload>,
    /// Agent data for multi-agent events (cleanup)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<Vec<WebhookAgentPayload>>,
    /// Count of affected agents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// Error reason for failed events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Agent data included in webhook payloads.
#[derive(Debug, Clone, Serialize)]
pub struct WebhookAgentPayload {
    /// Agent ID
    pub id: String,
    /// Container image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// User-defined metadata tags
    pub tags: HashMap<String, String>,
}

impl WebhookAgentPayload {
    fn from_agent(agent: &Agent) -> Self {
        Self {
            id: agent.id.to_string(),
            image: Some(agent.image.clone()),
            tags: agent.tags.clone(),
        }
    }
}

impl WebhookPayload {
    /// Create payload for agent.created event.
    pub fn agent_created(agent: &Agent) -> Self {
        Self {
            event: "agent.created".to_string(),
            timestamp: Utc::now(),
            agent: Some(WebhookAgentPayload::from_agent(agent)),
            agents: None,
            count: None,
            reason: None,
        }
    }

    /// Create payload for agent.deleted event.
    pub fn agent_deleted(agent: &Agent) -> Self {
        Self {
            event: "agent.deleted".to_string(),
            timestamp: Utc::now(),
            agent: Some(WebhookAgentPayload::from_agent(agent)),
            agents: None,
            count: None,
            reason: None,
        }
    }

    /// Create payload for agent.failed event.
    pub fn agent_failed(agent: &Agent, reason: &str) -> Self {
        Self {
            event: "agent.failed".to_string(),
            timestamp: Utc::now(),
            agent: Some(WebhookAgentPayload::from_agent(agent)),
            agents: None,
            count: None,
            reason: Some(reason.to_string()),
        }
    }

    /// Create payload for agents.cleanup event.
    pub fn agents_cleanup(agents: &[Agent]) -> Self {
        Self {
            event: "agents.cleanup".to_string(),
            timestamp: Utc::now(),
            agent: None,
            agents: Some(agents.iter().map(WebhookAgentPayload::from_agent).collect()),
            count: Some(agents.len() as u32),
            reason: None,
        }
    }
}

/// Compute HMAC-SHA256 signature for webhook payload.
fn compute_signature(secret: &str, body: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(body.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Dispatches webhook notifications for agent lifecycle events.
#[derive(Clone)]
pub struct WebhookDispatcher {
    webhooks: Vec<WebhookConfig>,
    http_client: reqwest::Client,
}

impl WebhookDispatcher {
    /// Create a new webhook dispatcher.
    pub fn new(webhooks: Vec<WebhookConfig>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            webhooks,
            http_client,
        }
    }

    /// Dispatch a webhook event to all matching endpoints.
    pub fn dispatch(&self, event: WebhookEvent, payload: WebhookPayload) {
        let matching_webhooks: Vec<_> = self
            .webhooks
            .iter()
            .filter(|w| w.events.contains(&event))
            .cloned()
            .collect();

        if matching_webhooks.is_empty() {
            debug!(?event, "No webhooks configured for event");
            return;
        }

        let body = match serde_json::to_string(&payload) {
            Ok(b) => b,
            Err(e) => {
                error!(?event, error = %e, "Failed to serialize webhook payload");
                return;
            }
        };

        for webhook in matching_webhooks {
            let client = self.http_client.clone();
            let body = body.clone();
            let url = webhook.url.clone();
            let secret = webhook.secret.clone();

            tokio::spawn(async move {
                let mut request = client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .body(body.clone());

                if let Some(ref secret) = secret {
                    let signature = compute_signature(secret, &body);
                    request = request.header("X-Webhook-Signature", signature);
                }

                match request.send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            debug!(url = %url, "Webhook delivered successfully");
                        } else {
                            warn!(
                                url = %url,
                                status = %response.status(),
                                "Webhook returned non-success status"
                            );
                        }
                    }
                    Err(e) => {
                        error!(url = %url, error = %e, "Failed to deliver webhook");
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_signature() {
        let secret = "test-secret";
        let body = r#"{"event":"agent.created"}"#;
        let sig = compute_signature(secret, body);
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn test_payload_serialization() {
        let payload = WebhookPayload {
            event: "agent.created".to_string(),
            timestamp: Utc::now(),
            agent: Some(WebhookAgentPayload {
                id: "test-id".to_string(),
                image: Some("python:3.12".to_string()),
                tags: HashMap::new(),
            }),
            agents: None,
            count: None,
            reason: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("agent.created"));
        assert!(json.contains("test-id"));
        assert!(!json.contains("agents")); // None fields should be skipped
    }
}
