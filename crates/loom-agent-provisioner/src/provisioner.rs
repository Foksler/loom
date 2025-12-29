// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Core provisioner implementation for agent lifecycle management.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use k8s_openapi::api::core::v1::Capabilities;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use loom_k8s::{
    Container, EnvVar, K8sClient, LogOptions, LogStream, Pod, PodSpec, ResourceRequirements,
    SecurityContext,
};

use crate::config::AgentConfig;
use crate::error::ProvisionerError;
use crate::types::{Agent, AgentId, AgentStatus, CleanupResult, CreateAgentRequest, LogStreamOptions};

const MANAGED_LABEL: &str = "loom.dev/managed";
const AGENT_ID_LABEL: &str = "loom.dev/agent-id";
const TAGS_ANNOTATION: &str = "loom.dev/tags";
const LIFETIME_ANNOTATION: &str = "loom.dev/lifetime-hours";
const CONTAINER_NAME: &str = "agent";
const DEFAULT_MEMORY_LIMIT: &str = "16Gi";
const POLL_INTERVAL_MS: u64 = 500;

/// The main provisioner for managing agent lifecycle.
pub struct Provisioner {
    client: Arc<dyn K8sClient>,
    config: AgentConfig,
}

impl Provisioner {
    /// Create a new provisioner with the given K8s client and configuration.
    pub fn new(client: Arc<dyn K8sClient>, config: AgentConfig) -> Self {
        Self { client, config }
    }

    /// Get the namespace this provisioner operates in.
    pub fn namespace(&self) -> &str {
        &self.config.namespace
    }

    /// Validate that the configured namespace exists in the cluster.
    ///
    /// This should be called on startup to fail fast if the namespace
    /// is not properly configured.
    pub async fn validate_namespace(&self) -> Result<(), ProvisionerError> {
        match self.client.get_namespace(&self.config.namespace).await {
            Ok(_) => {
                tracing::info!(namespace = %self.config.namespace, "Validated namespace exists");
                Ok(())
            }
            Err(loom_k8s::K8sError::NamespaceNotFound { .. }) => {
                Err(ProvisionerError::NamespaceNotFound {
                    name: self.config.namespace.clone(),
                })
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Create a new agent based on the provided request.
    pub async fn create_agent(&self, req: CreateAgentRequest) -> Result<Agent, ProvisionerError> {
        let lifetime_hours = self.validate_lifetime(req.lifetime_hours)?;

        let active_count = self.count_active_agents().await?;
        if active_count >= self.config.max_concurrent {
            return Err(ProvisionerError::TooManyAgents {
                current: active_count,
                max: self.config.max_concurrent,
            });
        }

        let id = AgentId::new();
        let pod = build_pod_spec(&id, &req, &self.config, lifetime_hours);
        let pod_name = id.as_k8s_name();

        tracing::info!(agent_id = %id, pod_name = %pod_name, image = %req.image, "Creating agent pod");

        self.client.create_pod(&self.config.namespace, pod).await?;

        let status = self
            .poll_until_ready(&pod_name, Duration::from_secs(self.config.ready_timeout_secs))
            .await?;

        let created_at = Utc::now();
        Ok(Agent {
            id,
            pod_name,
            status,
            image: req.image,
            tags: req.tags,
            created_at,
            lifetime_hours,
            age_hours: 0.0,
        })
    }

    /// Count the number of active (Pending or Running) agents.
    pub async fn count_active_agents(&self) -> Result<u32, ProvisionerError> {
        let pods = self
            .client
            .list_pods(&self.config.namespace, &format!("{}=true", MANAGED_LABEL))
            .await?;

        let count = pods
            .iter()
            .filter(|pod| {
                let phase = pod
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.as_deref())
                    .unwrap_or("Unknown");
                matches!(phase, "Pending" | "Running")
            })
            .count() as u32;

        Ok(count)
    }

    fn validate_lifetime(&self, requested: Option<u32>) -> Result<u32, ProvisionerError> {
        match requested {
            Some(hours) if hours > self.config.max_ttl_hours => {
                Err(ProvisionerError::InvalidLifetime {
                    requested: hours,
                    max: self.config.max_ttl_hours,
                })
            }
            Some(hours) => Ok(hours),
            None => Ok(self.config.default_ttl_hours),
        }
    }

    async fn poll_until_ready(
        &self,
        pod_name: &str,
        timeout: Duration,
    ) -> Result<AgentStatus, ProvisionerError> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);

        loop {
            if start.elapsed() > timeout {
                return Err(ProvisionerError::AgentTimeout {
                    id: pod_name.to_string(),
                });
            }

            let pod = self.client.get_pod(pod_name, &self.config.namespace).await?;

            let phase = pod
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("Unknown");

            match phase {
                "Running" => return Ok(AgentStatus::Running),
                "Succeeded" => return Ok(AgentStatus::Succeeded),
                "Failed" => {
                    let reason = pod
                        .status
                        .as_ref()
                        .and_then(|s| s.message.clone())
                        .unwrap_or_else(|| "Unknown failure".to_string());
                    return Err(ProvisionerError::AgentFailed {
                        id: pod_name.to_string(),
                        reason,
                    });
                }
                "Pending" => {
                    tokio::time::sleep(poll_interval).await;
                }
                _ => {
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }

    /// List all agents, optionally filtered by tags.
    ///
    /// If `tag_filter` is provided, only agents matching all specified tags are returned.
    pub async fn list_agents(
        &self,
        tag_filter: Option<HashMap<String, String>>,
    ) -> Result<Vec<Agent>, ProvisionerError> {
        let pods = self
            .client
            .list_pods(&self.config.namespace, &format!("{}=true", MANAGED_LABEL))
            .await?;

        let mut agents = Vec::new();
        for pod in &pods {
            match pod_to_agent(pod) {
                Ok(agent) => agents.push(agent),
                Err(e) => {
                    tracing::warn!("Failed to parse pod as agent: {}", e);
                }
            }
        }

        if let Some(filter) = tag_filter {
            agents.retain(|agent| {
                filter
                    .iter()
                    .all(|(k, v)| agent.tags.get(k).map(|av| av == v).unwrap_or(false))
            });
        }

        Ok(agents)
    }

    /// Get a specific agent by ID.
    pub async fn get_agent(&self, id: &AgentId) -> Result<Agent, ProvisionerError> {
        let pod_name = id.as_k8s_name();
        match self.client.get_pod(&pod_name, &self.config.namespace).await {
            Ok(pod) => pod_to_agent(&pod),
            Err(loom_k8s::K8sError::PodNotFound { .. }) => Err(ProvisionerError::AgentNotFound {
                id: id.to_string(),
            }),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete an agent by ID with a 5-second grace period.
    pub async fn delete_agent(&self, id: &AgentId) -> Result<(), ProvisionerError> {
        let pod_name = id.as_k8s_name();
        match self
            .client
            .delete_pod(&pod_name, &self.config.namespace, 5)
            .await
        {
            Ok(()) => Ok(()),
            Err(loom_k8s::K8sError::PodNotFound { .. }) => Err(ProvisionerError::AgentNotFound {
                id: id.to_string(),
            }),
            Err(e) => Err(e.into()),
        }
    }

    /// Stream logs from an agent's container.
    pub async fn stream_logs(
        &self,
        id: &AgentId,
        opts: LogStreamOptions,
    ) -> Result<LogStream, ProvisionerError> {
        self.get_agent(id).await?;

        let pod_name = id.as_k8s_name();
        let log_opts = LogOptions {
            tail: opts.tail,
            timestamps: opts.timestamps,
        };

        self.client
            .stream_logs(&pod_name, &self.config.namespace, CONTAINER_NAME, log_opts)
            .await
            .map_err(Into::into)
    }

    /// Find all agents that have exceeded their lifetime.
    pub async fn find_expired_agents(&self) -> Result<Vec<Agent>, ProvisionerError> {
        let agents = self.list_agents(None).await?;
        let expired = agents
            .into_iter()
            .filter(|agent| agent.age_hours >= agent.lifetime_hours as f64)
            .collect();
        Ok(expired)
    }

    /// Clean up all expired agents.
    pub async fn cleanup_expired_agents(&self) -> Result<CleanupResult, ProvisionerError> {
        let expired = self.find_expired_agents().await?;
        let mut deleted = Vec::new();

        for agent in expired {
            match self.delete_agent(&agent.id).await {
                Ok(()) => {
                    tracing::info!(agent_id = %agent.id, age_hours = agent.age_hours, "Deleted expired agent");
                    deleted.push(agent.id);
                }
                Err(ProvisionerError::AgentNotFound { .. }) => {
                    tracing::debug!(agent_id = %agent.id, "Agent already deleted");
                }
                Err(e) => {
                    tracing::error!(agent_id = %agent.id, error = %e, "Failed to delete expired agent");
                }
            }
        }

        let count = deleted.len() as u32;
        Ok(CleanupResult { deleted, count })
    }

    /// Get the configured cleanup interval in seconds.
    pub fn cleanup_interval_secs(&self) -> u64 {
        self.config.cleanup_interval_secs
    }
}

/// Build a Kubernetes Pod spec for an agent.
fn build_pod_spec(
    id: &AgentId,
    req: &CreateAgentRequest,
    config: &AgentConfig,
    lifetime_hours: u32,
) -> Pod {
    let pod_name = id.as_k8s_name();

    let mut labels = BTreeMap::new();
    labels.insert(MANAGED_LABEL.to_string(), "true".to_string());
    labels.insert(AGENT_ID_LABEL.to_string(), id.to_string());

    let mut annotations = BTreeMap::new();
    if !req.tags.is_empty() {
        if let Ok(tags_json) = serde_json::to_string(&req.tags) {
            annotations.insert(TAGS_ANNOTATION.to_string(), tags_json);
        }
    }
    annotations.insert(LIFETIME_ANNOTATION.to_string(), lifetime_hours.to_string());

    let env_vars: Vec<EnvVar> = req
        .env
        .iter()
        .map(|(k, v)| EnvVar {
            name: k.clone(),
            value: Some(v.clone()),
            value_from: None,
        })
        .collect();

    let mut limits = BTreeMap::new();
    limits.insert(
        "memory".to_string(),
        Quantity(
            req.resources
                .memory_limit
                .clone()
                .unwrap_or_else(|| DEFAULT_MEMORY_LIMIT.to_string()),
        ),
    );
    if let Some(cpu) = &req.resources.cpu_limit {
        limits.insert("cpu".to_string(), Quantity(cpu.clone()));
    }

    let resources = ResourceRequirements {
        limits: Some(limits),
        requests: None,
        claims: None,
    };

    let security_context = SecurityContext {
        run_as_non_root: Some(true),
        run_as_user: Some(1000),
        run_as_group: Some(1000),
        allow_privilege_escalation: Some(false),
        read_only_root_filesystem: Some(true),
        capabilities: Some(Capabilities {
            drop: Some(vec!["ALL".to_string()]),
            add: None,
        }),
        ..Default::default()
    };

    let container = Container {
        name: CONTAINER_NAME.to_string(),
        image: Some(req.image.clone()),
        env: if env_vars.is_empty() {
            None
        } else {
            Some(env_vars)
        },
        command: req.command.clone(),
        args: req.args.clone(),
        working_dir: req.workdir.clone(),
        resources: Some(resources),
        security_context: Some(security_context),
        ..Default::default()
    };

    Pod {
        metadata: ObjectMeta {
            name: Some(pod_name),
            namespace: Some(config.namespace.clone()),
            labels: Some(labels),
            annotations: Some(annotations),
            ..Default::default()
        },
        spec: Some(PodSpec {
            containers: vec![container],
            restart_policy: Some("Never".to_string()),
            ..Default::default()
        }),
        status: None,
    }
}

/// Convert a Kubernetes Pod to an Agent.
fn pod_to_agent(pod: &Pod) -> Result<Agent, ProvisionerError> {
    let metadata = pod.metadata.clone();
    let pod_name = metadata.name.clone().unwrap_or_default();

    let labels = metadata.labels.unwrap_or_default();
    let annotations = metadata.annotations.unwrap_or_default();

    let id_str = labels
        .get(AGENT_ID_LABEL)
        .ok_or_else(|| ProvisionerError::AgentFailed {
            id: pod_name.clone(),
            reason: format!("Missing {} label", AGENT_ID_LABEL),
        })?;

    let id = id_str.parse::<AgentId>().map_err(|_| ProvisionerError::AgentFailed {
        id: pod_name.clone(),
        reason: format!("Invalid agent ID: {}", id_str),
    })?;

    let tags: HashMap<String, String> = annotations
        .get(TAGS_ANNOTATION)
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let lifetime_hours: u32 = annotations
        .get(LIFETIME_ANNOTATION)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let status = map_pod_phase(pod);

    let created_at = metadata
        .creation_timestamp
        .map(|ts| ts.0)
        .unwrap_or_else(Utc::now);

    let age_hours = calculate_age_hours(created_at);

    let image = pod
        .spec
        .as_ref()
        .and_then(|spec| spec.containers.first())
        .map(|c| c.image.clone().unwrap_or_default())
        .unwrap_or_default();

    Ok(Agent {
        id,
        pod_name,
        status,
        image,
        tags,
        created_at,
        lifetime_hours,
        age_hours,
    })
}

/// Map Kubernetes Pod phase to AgentStatus.
fn map_pod_phase(pod: &Pod) -> AgentStatus {
    let phase = pod
        .status
        .as_ref()
        .and_then(|s| s.phase.as_deref())
        .unwrap_or("Unknown");

    match phase {
        "Pending" => AgentStatus::Pending,
        "Running" => AgentStatus::Running,
        "Succeeded" => AgentStatus::Succeeded,
        "Failed" => AgentStatus::Failed,
        _ => AgentStatus::Pending,
    }
}

/// Calculate the age of an agent in hours from its creation timestamp.
fn calculate_age_hours(created_at: DateTime<Utc>) -> f64 {
    let now = Utc::now();
    let duration = now.signed_duration_since(created_at);
    duration.num_seconds() as f64 / 3600.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_build_pod_spec_basic() {
        let id = AgentId::new();
        let req = CreateAgentRequest {
            image: "python:3.12".to_string(),
            env: HashMap::new(),
            resources: Default::default(),
            tags: HashMap::new(),
            lifetime_hours: None,
            command: None,
            args: None,
            workdir: None,
        };
        let config = AgentConfig::default();

        let pod = build_pod_spec(&id, &req, &config, 4);

        assert_eq!(pod.metadata.name, Some(id.as_k8s_name()));
        assert_eq!(pod.metadata.namespace, Some("loom-agents".to_string()));

        let labels = pod.metadata.labels.unwrap();
        assert_eq!(labels.get(MANAGED_LABEL), Some(&"true".to_string()));
        assert_eq!(labels.get(AGENT_ID_LABEL), Some(&id.to_string()));

        let annotations = pod.metadata.annotations.unwrap();
        assert_eq!(annotations.get(LIFETIME_ANNOTATION), Some(&"4".to_string()));

        let spec = pod.spec.unwrap();
        assert_eq!(spec.restart_policy, Some("Never".to_string()));
        assert_eq!(spec.containers.len(), 1);

        let container = &spec.containers[0];
        assert_eq!(container.name, CONTAINER_NAME);
        assert_eq!(container.image, Some("python:3.12".to_string()));

        let security = container.security_context.as_ref().unwrap();
        assert_eq!(security.run_as_non_root, Some(true));
        assert_eq!(security.run_as_user, Some(1000));
        assert_eq!(security.run_as_group, Some(1000));
        assert_eq!(security.allow_privilege_escalation, Some(false));
        assert_eq!(security.read_only_root_filesystem, Some(true));

        let caps = security.capabilities.as_ref().unwrap();
        assert_eq!(caps.drop, Some(vec!["ALL".to_string()]));
    }

    #[test]
    fn test_build_pod_spec_with_env_and_resources() {
        let id = AgentId::new();
        let mut env = HashMap::new();
        env.insert("TASK_ID".to_string(), "abc123".to_string());
        env.insert("API_URL".to_string(), "https://api.example.com".to_string());

        let req = CreateAgentRequest {
            image: "worker:latest".to_string(),
            env,
            resources: crate::types::ResourceSpec {
                memory_limit: Some("8Gi".to_string()),
                cpu_limit: Some("4".to_string()),
            },
            tags: HashMap::new(),
            lifetime_hours: Some(8),
            command: Some(vec!["/bin/sh".to_string(), "-c".to_string()]),
            args: Some(vec!["python worker.py".to_string()]),
            workdir: Some("/app".to_string()),
        };
        let config = AgentConfig::default();

        let pod = build_pod_spec(&id, &req, &config, 8);
        let spec = pod.spec.unwrap();
        let container = &spec.containers[0];

        assert_eq!(
            container.command,
            Some(vec!["/bin/sh".to_string(), "-c".to_string()])
        );
        assert_eq!(
            container.args,
            Some(vec!["python worker.py".to_string()])
        );
        assert_eq!(container.working_dir, Some("/app".to_string()));

        let env_vars = container.env.as_ref().unwrap();
        assert_eq!(env_vars.len(), 2);

        let resources = container.resources.as_ref().unwrap();
        let limits = resources.limits.as_ref().unwrap();
        assert_eq!(limits.get("memory"), Some(&Quantity("8Gi".to_string())));
        assert_eq!(limits.get("cpu"), Some(&Quantity("4".to_string())));
    }

    #[test]
    fn test_build_pod_spec_with_tags() {
        let id = AgentId::new();
        let mut tags = HashMap::new();
        tags.insert("project".to_string(), "ai-worker".to_string());
        tags.insert("env".to_string(), "prod".to_string());

        let req = CreateAgentRequest {
            image: "test:latest".to_string(),
            env: HashMap::new(),
            resources: Default::default(),
            tags,
            lifetime_hours: None,
            command: None,
            args: None,
            workdir: None,
        };
        let config = AgentConfig::default();

        let pod = build_pod_spec(&id, &req, &config, 4);
        let annotations = pod.metadata.annotations.unwrap();

        let tags_json = annotations.get(TAGS_ANNOTATION).unwrap();
        let parsed: HashMap<String, String> = serde_json::from_str(tags_json).unwrap();
        assert_eq!(parsed.get("project"), Some(&"ai-worker".to_string()));
        assert_eq!(parsed.get("env"), Some(&"prod".to_string()));
    }
}
