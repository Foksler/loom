// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Agent provisioning types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an agent, using UUID7 (time-ordered).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(uuid7::Uuid);

impl AgentId {
    /// Create a new agent ID with UUID7.
    pub fn new() -> Self {
        Self(uuid7::uuid7())
    }

    /// Get the Kubernetes-compatible name for this agent.
    pub fn as_k8s_name(&self) -> String {
        format!("agent-{}", self.0)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for AgentId {
    type Err = uuid7::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = s.parse::<uuid7::Uuid>()?;
        Ok(Self(uuid))
    }
}

/// Status of an agent, mapped from Kubernetes Pod phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Pod created, containers starting
    Pending,
    /// Containers running
    Running,
    /// Completed successfully (exit 0)
    Succeeded,
    /// Container failed (non-zero exit)
    Failed,
}

/// An agent instance with its current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique agent identifier
    pub id: AgentId,
    /// Kubernetes Pod name
    pub pod_name: String,
    /// Current agent status
    pub status: AgentStatus,
    /// Container image
    pub image: String,
    /// User-defined metadata tags
    pub tags: HashMap<String, String>,
    /// When the agent was created
    pub created_at: DateTime<Utc>,
    /// Configured lifetime in hours
    pub lifetime_hours: u32,
    /// Current age in hours
    pub age_hours: f64,
}

/// Resource limits for an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    /// Memory limit (e.g., "8Gi")
    pub memory_limit: Option<String>,
    /// CPU limit (e.g., "4")
    pub cpu_limit: Option<String>,
}

/// Request to create a new agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    /// Container image to run
    pub image: String,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Resource limits
    #[serde(default)]
    pub resources: ResourceSpec,
    /// User-defined metadata tags
    #[serde(default)]
    pub tags: HashMap<String, String>,
    /// TTL override in hours
    pub lifetime_hours: Option<u32>,
    /// Override container ENTRYPOINT
    pub command: Option<Vec<String>>,
    /// Override container CMD
    pub args: Option<Vec<String>>,
    /// Override container WORKDIR
    pub workdir: Option<String>,
}

/// Options for streaming agent logs.
#[derive(Debug, Clone)]
pub struct LogStreamOptions {
    /// Number of lines to tail from the end of the log (default: 256)
    pub tail: u32,
    /// Whether to include timestamps in log output (default: true)
    pub timestamps: bool,
}

impl Default for LogStreamOptions {
    fn default() -> Self {
        Self {
            tail: 256,
            timestamps: true,
        }
    }
}

/// Result of a cleanup operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    /// IDs of agents that were deleted
    pub deleted: Vec<AgentId>,
    /// Number of agents deleted
    pub count: u32,
}
