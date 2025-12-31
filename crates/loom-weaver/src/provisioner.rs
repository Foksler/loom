// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Core provisioner implementation for weaver lifecycle management.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use k8s_openapi::api::core::v1::Capabilities;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use loom_k8s::{
    AttachedProcess, Container, EnvVar, K8sClient, LocalObjectReference, LogOptions, LogStream,
    Pod, PodSpec, ResourceRequirements, SecurityContext,
};

use crate::config::WeaverConfig;
use crate::error::ProvisionerError;
use crate::types::{Weaver, WeaverId, WeaverStatus, CleanupResult, CreateWeaverRequest, LogStreamOptions};

const MANAGED_LABEL: &str = "loom.dev/managed";
const WEAVER_ID_LABEL: &str = "loom.dev/weaver-id";
const LABEL_OWNER_USER_ID: &str = "loom.dev/owner-user-id";
const TAGS_ANNOTATION: &str = "loom.dev/tags";
const LIFETIME_ANNOTATION: &str = "loom.dev/lifetime-hours";
const CONTAINER_NAME: &str = "weaver";
const DEFAULT_MEMORY_LIMIT: &str = "16Gi";
const POLL_INTERVAL_MS: u64 = 500;

/// The main provisioner for managing weaver lifecycle.
pub struct Provisioner {
    client: Arc<dyn K8sClient>,
    config: WeaverConfig,
}

impl Provisioner {
    /// Create a new provisioner with the given K8s client and configuration.
    pub fn new(client: Arc<dyn K8sClient>, config: WeaverConfig) -> Self {
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

    /// Create a new weaver based on the provided request.
    pub async fn create_weaver(&self, req: CreateWeaverRequest) -> Result<Weaver, ProvisionerError> {
        let lifetime_hours = self.validate_lifetime(req.lifetime_hours)?;

        let active_count = self.count_active_weavers().await?;
        if active_count >= self.config.max_concurrent {
            return Err(ProvisionerError::TooManyWeavers {
                current: active_count,
                max: self.config.max_concurrent,
            });
        }

        let id = WeaverId::new();
        let pod = build_pod_spec(&id, &req, &self.config, lifetime_hours);
        let pod_name = id.as_k8s_name();

        tracing::info!(weaver_id = %id, pod_name = %pod_name, image = %req.image, "Creating weaver pod");

        self.client.create_pod(&self.config.namespace, pod).await?;

        let status = self
            .poll_until_ready(&pod_name, Duration::from_secs(self.config.ready_timeout_secs))
            .await?;

        let created_at = Utc::now();
        Ok(Weaver {
            id,
            pod_name,
            status,
            image: req.image,
            tags: req.tags,
            created_at,
            lifetime_hours,
            age_hours: 0.0,
            owner_user_id: req.owner_user_id.unwrap_or_default(),
        })
    }

    /// Count the number of active (Pending or Running) weavers.
    pub async fn count_active_weavers(&self) -> Result<u32, ProvisionerError> {
        let pods = self
            .client
            .list_pods(&self.config.namespace, &format!("{MANAGED_LABEL}=true"))
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
    ) -> Result<WeaverStatus, ProvisionerError> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);

        loop {
            if start.elapsed() > timeout {
                return Err(ProvisionerError::WeaverTimeout {
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
                "Running" => return Ok(WeaverStatus::Running),
                "Succeeded" => return Ok(WeaverStatus::Succeeded),
                "Failed" => {
                    let reason = pod
                        .status
                        .as_ref()
                        .and_then(|s| s.message.clone())
                        .unwrap_or_else(|| "Unknown failure".to_string());
                    return Err(ProvisionerError::WeaverFailed {
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

    /// List all weavers, optionally filtered by tags.
    ///
    /// If `tag_filter` is provided, only weavers matching all specified tags are returned.
    pub async fn list_weavers(
        &self,
        tag_filter: Option<HashMap<String, String>>,
    ) -> Result<Vec<Weaver>, ProvisionerError> {
        let pods = self
            .client
            .list_pods(&self.config.namespace, &format!("{MANAGED_LABEL}=true"))
            .await?;

        let mut weavers = Vec::new();
        for pod in &pods {
            match pod_to_weaver(pod) {
                Ok(weaver) => weavers.push(weaver),
                Err(e) => {
                    tracing::warn!("Failed to parse pod as weaver: {}", e);
                }
            }
        }

        if let Some(filter) = tag_filter {
            weavers.retain(|weaver| {
                filter
                    .iter()
                    .all(|(k, v)| weaver.tags.get(k).map(|av| av == v).unwrap_or(false))
            });
        }

        Ok(weavers)
    }

    /// List weavers owned by a specific user.
    pub async fn list_weavers_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<Weaver>, ProvisionerError> {
        let all = self.list_weavers(None).await?;
        Ok(all.into_iter().filter(|w| w.owner_user_id == user_id).collect())
    }

    /// Get a specific weaver by ID.
    pub async fn get_weaver(&self, id: &WeaverId) -> Result<Weaver, ProvisionerError> {
        let pod_name = id.as_k8s_name();
        match self.client.get_pod(&pod_name, &self.config.namespace).await {
            Ok(pod) => pod_to_weaver(&pod),
            Err(loom_k8s::K8sError::PodNotFound { .. }) => Err(ProvisionerError::WeaverNotFound {
                id: id.to_string(),
            }),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a weaver by ID with a 5-second grace period.
    pub async fn delete_weaver(&self, id: &WeaverId) -> Result<(), ProvisionerError> {
        let pod_name = id.as_k8s_name();
        match self
            .client
            .delete_pod(&pod_name, &self.config.namespace, 5)
            .await
        {
            Ok(()) => Ok(()),
            Err(loom_k8s::K8sError::PodNotFound { .. }) => Err(ProvisionerError::WeaverNotFound {
                id: id.to_string(),
            }),
            Err(e) => Err(e.into()),
        }
    }

    /// Stream logs from a weaver's container.
    pub async fn stream_logs(
        &self,
        id: &WeaverId,
        opts: LogStreamOptions,
    ) -> Result<LogStream, ProvisionerError> {
        self.get_weaver(id).await?;

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

    /// Attach to a weaver's container for interactive I/O.
    ///
    /// Returns an `AttachedProcess` with stdin/stdout streams for bidirectional
    /// communication with the running container.
    pub async fn attach_weaver(&self, id: &WeaverId) -> Result<AttachedProcess, ProvisionerError> {
        let weaver = self.get_weaver(id).await?;

        if weaver.status != WeaverStatus::Running {
            return Err(ProvisionerError::WeaverNotRunning {
                id: id.to_string(),
                status: format!("{:?}", weaver.status),
            });
        }

        let pod_name = id.as_k8s_name();
        self.client
            .exec_attach(&pod_name, &self.config.namespace, CONTAINER_NAME)
            .await
            .map_err(Into::into)
    }

    /// Find all weavers that have exceeded their lifetime.
    pub async fn find_expired_weavers(&self) -> Result<Vec<Weaver>, ProvisionerError> {
        let weavers = self.list_weavers(None).await?;
        let expired = weavers
            .into_iter()
            .filter(|weaver| weaver.age_hours >= weaver.lifetime_hours as f64)
            .collect();
        Ok(expired)
    }

    /// Clean up all expired weavers.
    pub async fn cleanup_expired_weavers(&self) -> Result<CleanupResult, ProvisionerError> {
        let expired = self.find_expired_weavers().await?;
        let mut deleted = Vec::new();

        for weaver in expired {
            match self.delete_weaver(&weaver.id).await {
                Ok(()) => {
                    tracing::info!(weaver_id = %weaver.id, age_hours = weaver.age_hours, "Deleted expired weaver");
                    deleted.push(weaver.id);
                }
                Err(ProvisionerError::WeaverNotFound { .. }) => {
                    tracing::debug!(weaver_id = %weaver.id, "Weaver already deleted");
                }
                Err(e) => {
                    tracing::error!(weaver_id = %weaver.id, error = %e, "Failed to delete expired weaver");
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

/// Build a Kubernetes Pod spec for a weaver.
fn build_pod_spec(
    id: &WeaverId,
    req: &CreateWeaverRequest,
    config: &WeaverConfig,
    lifetime_hours: u32,
) -> Pod {
    let pod_name = id.as_k8s_name();

    let mut labels = BTreeMap::new();
    labels.insert(MANAGED_LABEL.to_string(), "true".to_string());
    labels.insert(WEAVER_ID_LABEL.to_string(), id.to_string());
    labels.insert(
        LABEL_OWNER_USER_ID.to_string(),
        req.owner_user_id.clone().unwrap_or_default(),
    );

    let mut annotations = BTreeMap::new();
    if !req.tags.is_empty() {
        if let Ok(tags_json) = serde_json::to_string(&req.tags) {
            annotations.insert(TAGS_ANNOTATION.to_string(), tags_json);
        }
    }
    annotations.insert(LIFETIME_ANNOTATION.to_string(), lifetime_hours.to_string());

    let mut env_vars: Vec<EnvVar> = req
        .env
        .iter()
        .map(|(k, v)| EnvVar {
            name: k.clone(),
            value: Some(v.clone()),
            value_from: None,
        })
        .collect();

    if let Some(repo) = &req.repo {
        env_vars.push(EnvVar {
            name: "LOOM_REPO".to_string(),
            value: Some(repo.clone()),
            value_from: None,
        });
    }
    if let Some(branch) = &req.branch {
        env_vars.push(EnvVar {
            name: "LOOM_BRANCH".to_string(),
            value: Some(branch.clone()),
            value_from: None,
        });
    }

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
        read_only_root_filesystem: Some(false),
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
        // Enable TTY and stdin for interactive REPL sessions
        // Required for tmux to work inside the container
        tty: Some(true),
        stdin: Some(true),
        ..Default::default()
    };

    let image_pull_secrets = if config.image_pull_secrets.is_empty() {
        None
    } else {
        Some(
            config
                .image_pull_secrets
                .iter()
                .map(|name| LocalObjectReference {
                    name: name.clone(),
                })
                .collect(),
        )
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
            image_pull_secrets,
            ..Default::default()
        }),
        status: None,
    }
}

/// Convert a Kubernetes Pod to a Weaver.
fn pod_to_weaver(pod: &Pod) -> Result<Weaver, ProvisionerError> {
    let metadata = pod.metadata.clone();
    let pod_name = metadata.name.clone().unwrap_or_default();

    let labels = metadata.labels.unwrap_or_default();
    let annotations = metadata.annotations.unwrap_or_default();

    let id_str = labels
        .get(WEAVER_ID_LABEL)
        .ok_or_else(|| ProvisionerError::WeaverFailed {
            id: pod_name.clone(),
            reason: format!("Missing {WEAVER_ID_LABEL} label"),
        })?;

    let id = id_str.parse::<WeaverId>().map_err(|_| ProvisionerError::WeaverFailed {
        id: pod_name.clone(),
        reason: format!("Invalid weaver ID: {id_str}"),
    })?;

    let tags: HashMap<String, String> = annotations
        .get(TAGS_ANNOTATION)
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let lifetime_hours: u32 = annotations
        .get(LIFETIME_ANNOTATION)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let owner_user_id = labels
        .get(LABEL_OWNER_USER_ID)
        .cloned()
        .unwrap_or_default();

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

    Ok(Weaver {
        id,
        pod_name,
        status,
        image,
        tags,
        created_at,
        lifetime_hours,
        age_hours,
        owner_user_id,
    })
}

/// Map Kubernetes Pod phase to WeaverStatus.
fn map_pod_phase(pod: &Pod) -> WeaverStatus {
    // Check if pod is being deleted (has deletionTimestamp)
    if pod.metadata.deletion_timestamp.is_some() {
        return WeaverStatus::Terminating;
    }

    let phase = pod
        .status
        .as_ref()
        .and_then(|s| s.phase.as_deref())
        .unwrap_or("Unknown");

    match phase {
        "Pending" => WeaverStatus::Pending,
        "Running" => WeaverStatus::Running,
        "Succeeded" => WeaverStatus::Succeeded,
        "Failed" => WeaverStatus::Failed,
        _ => WeaverStatus::Pending,
    }
}

/// Calculate the age of a weaver in hours from its creation timestamp.
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
        let id = WeaverId::new();
        let req = CreateWeaverRequest {
            image: "python:3.12".to_string(),
            env: HashMap::new(),
            resources: Default::default(),
            tags: HashMap::new(),
            lifetime_hours: None,
            command: None,
            args: None,
            workdir: None,
            repo: None,
            branch: None,
            owner_user_id: None,
        };
        let config = WeaverConfig::default();

        let pod = build_pod_spec(&id, &req, &config, 4);

        assert_eq!(pod.metadata.name, Some(id.as_k8s_name()));
        assert_eq!(pod.metadata.namespace, Some("loom-weavers".to_string()));

        let labels = pod.metadata.labels.unwrap();
        assert_eq!(labels.get(MANAGED_LABEL), Some(&"true".to_string()));
        assert_eq!(labels.get(WEAVER_ID_LABEL), Some(&id.to_string()));

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
        assert_eq!(security.read_only_root_filesystem, Some(false));

        let caps = security.capabilities.as_ref().unwrap();
        assert_eq!(caps.drop, Some(vec!["ALL".to_string()]));
    }

    #[test]
    fn test_build_pod_spec_with_env_and_resources() {
        let id = WeaverId::new();
        let mut env = HashMap::new();
        env.insert("TASK_ID".to_string(), "abc123".to_string());
        env.insert("API_URL".to_string(), "https://api.example.com".to_string());

        let req = CreateWeaverRequest {
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
            repo: None,
            branch: None,
            owner_user_id: None,
        };
        let config = WeaverConfig::default();

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
        let id = WeaverId::new();
        let mut tags = HashMap::new();
        tags.insert("project".to_string(), "ai-worker".to_string());
        tags.insert("env".to_string(), "prod".to_string());

        let req = CreateWeaverRequest {
            image: "test:latest".to_string(),
            env: HashMap::new(),
            resources: Default::default(),
            tags,
            lifetime_hours: None,
            command: None,
            args: None,
            workdir: None,
            repo: None,
            branch: None,
            owner_user_id: None,
        };
        let config = WeaverConfig::default();

        let pod = build_pod_spec(&id, &req, &config, 4);
        let annotations = pod.metadata.annotations.unwrap();

        let tags_json = annotations.get(TAGS_ANNOTATION).unwrap();
        let parsed: HashMap<String, String> = serde_json::from_str(tags_json).unwrap();
        assert_eq!(parsed.get("project"), Some(&"ai-worker".to_string()));
        assert_eq!(parsed.get("env"), Some(&"prod".to_string()));
    }

    #[test]
    fn test_map_pod_phase_running() {
        let pod = Pod {
            metadata: ObjectMeta::default(),
            spec: None,
            status: Some(loom_k8s::PodStatus {
                phase: Some("Running".to_string()),
                ..Default::default()
            }),
        };
        assert_eq!(map_pod_phase(&pod), WeaverStatus::Running);
    }

    #[test]
    fn test_map_pod_phase_pending() {
        let pod = Pod {
            metadata: ObjectMeta::default(),
            spec: None,
            status: Some(loom_k8s::PodStatus {
                phase: Some("Pending".to_string()),
                ..Default::default()
            }),
        };
        assert_eq!(map_pod_phase(&pod), WeaverStatus::Pending);
    }

    #[test]
    fn test_map_pod_phase_succeeded() {
        let pod = Pod {
            metadata: ObjectMeta::default(),
            spec: None,
            status: Some(loom_k8s::PodStatus {
                phase: Some("Succeeded".to_string()),
                ..Default::default()
            }),
        };
        assert_eq!(map_pod_phase(&pod), WeaverStatus::Succeeded);
    }

    #[test]
    fn test_map_pod_phase_failed() {
        let pod = Pod {
            metadata: ObjectMeta::default(),
            spec: None,
            status: Some(loom_k8s::PodStatus {
                phase: Some("Failed".to_string()),
                ..Default::default()
            }),
        };
        assert_eq!(map_pod_phase(&pod), WeaverStatus::Failed);
    }

    #[test]
    fn test_map_pod_phase_terminating() {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
        let pod = Pod {
            metadata: ObjectMeta {
                deletion_timestamp: Some(Time(Utc::now())),
                ..Default::default()
            },
            spec: None,
            status: Some(loom_k8s::PodStatus {
                phase: Some("Running".to_string()),
                ..Default::default()
            }),
        };
        // Even though phase is Running, deletionTimestamp means Terminating
        assert_eq!(map_pod_phase(&pod), WeaverStatus::Terminating);
    }
}
