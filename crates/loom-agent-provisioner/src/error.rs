// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Provisioner error types.

/// Errors that can occur during agent provisioning operations.
#[derive(Debug, thiserror::Error)]
pub enum ProvisionerError {
    /// Agent not found
    #[error("Agent not found: {id}")]
    AgentNotFound { id: String },

    /// Too many concurrent agents
    #[error("Too many agents: {current} running (max: {max})")]
    TooManyAgents { current: u32, max: u32 },

    /// Requested lifetime exceeds maximum
    #[error("Invalid lifetime: {requested} hours (max: {max} hours)")]
    InvalidLifetime { requested: u32, max: u32 },

    /// Agent failed to start
    #[error("Agent failed: {id} - {reason}")]
    AgentFailed { id: String, reason: String },

    /// Agent timed out waiting for ready state
    #[error("Agent timed out waiting for ready state: {id}")]
    AgentTimeout { id: String },

    /// Kubernetes error
    #[error(transparent)]
    K8sError(#[from] loom_k8s::K8sError),

    /// Namespace not found
    #[error("Namespace not found: {name}")]
    NamespaceNotFound { name: String },
}
