// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use crate::context::{CancellationToken, JobContext};
use crate::error::{JobError, Result};
use crate::health::{HealthState, JobHealthStatus, JobsHealthStatus, LastRunInfo};
use crate::job::Job;
use crate::repository::JobRepository;
use crate::types::{JobDefinition, JobRun, JobStatus, JobType, TriggerSource};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tracing::{info, instrument, warn};

const BASE_RETRY_DELAY_SECS: u64 = 1;
const MAX_RETRY_DELAY_SECS: u64 = 60;
const RETRY_FACTOR: f64 = 2.0;
const MAX_RETRIES: u32 = 3;

struct RegisteredJob {
    job: Arc<dyn Job>,
    job_type: JobType,
    cancellation_token: CancellationToken,
}

pub struct JobScheduler {
    jobs: HashMap<String, RegisteredJob>,
    repository: Arc<JobRepository>,
    shutdown_tx: broadcast::Sender<()>,
    handles: Mutex<Vec<JoinHandle<()>>>,
}

impl JobScheduler {
    pub fn new(repository: Arc<JobRepository>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            jobs: HashMap::new(),
            repository,
            shutdown_tx,
            handles: Mutex::new(Vec::new()),
        }
    }

    pub fn register_periodic(&mut self, job: Arc<dyn Job>, interval: Duration) {
        let id = job.id().to_string();
        self.jobs.insert(
            id,
            RegisteredJob {
                job,
                job_type: JobType::Periodic { interval },
                cancellation_token: CancellationToken::new(),
            },
        );
    }

    pub fn register_one_shot(&mut self, job: Arc<dyn Job>) {
        let id = job.id().to_string();
        self.jobs.insert(
            id,
            RegisteredJob {
                job,
                job_type: JobType::OneShot,
                cancellation_token: CancellationToken::new(),
            },
        );
    }

    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        let mut handles = self.handles.lock().await;

        for (job_id, registered) in &self.jobs {
            let def = JobDefinition {
                id: job_id.clone(),
                name: registered.job.name().to_string(),
                description: registered.job.description().to_string(),
                job_type: match &registered.job_type {
                    JobType::Periodic { .. } => "periodic".to_string(),
                    JobType::OneShot => "one_shot".to_string(),
                },
                interval_secs: match &registered.job_type {
                    JobType::Periodic { interval } => Some(interval.as_secs() as i64),
                    JobType::OneShot => None,
                },
                enabled: true,
            };
            self.repository.upsert_definition(&def).await?;

            if let JobType::Periodic { interval } = registered.job_type {
                let job = Arc::clone(&registered.job);
                let repository = Arc::clone(&self.repository);
                let mut shutdown_rx = self.shutdown_tx.subscribe();
                let cancellation_token = registered.cancellation_token.clone();
                let job_id = job_id.clone();

                let handle = tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(interval) => {
                                if cancellation_token.is_cancelled() {
                                    continue;
                                }
                                let _ = run_job_with_retry(
                                    &job,
                                    &repository,
                                    TriggerSource::Schedule,
                                    &cancellation_token,
                                ).await;
                            }
                            _ = shutdown_rx.recv() => {
                                info!(job_id = %job_id, "Shutting down periodic job");
                                break;
                            }
                        }
                    }
                });

                handles.push(handle);
            }
        }

        info!(job_count = handles.len(), "Job scheduler started");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn trigger_job(&self, job_id: &str, triggered_by: TriggerSource) -> Result<String> {
        let registered = self
            .jobs
            .get(job_id)
            .ok_or_else(|| JobError::NotFound(job_id.to_string()))?;

        run_job_with_retry(
            &registered.job,
            &self.repository,
            triggered_by,
            &registered.cancellation_token,
        )
        .await
    }

    #[instrument(skip(self))]
    pub async fn cancel_job(&self, job_id: &str) -> Result<()> {
        let registered = self
            .jobs
            .get(job_id)
            .ok_or_else(|| JobError::NotFound(job_id.to_string()))?;

        registered.cancellation_token.cancel();
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());

        let mut handles = self.handles.lock().await;
        for handle in handles.drain(..) {
            let _ = handle.await;
        }

        info!("Job scheduler shut down");
    }

    pub fn job_ids(&self) -> Vec<String> {
        self.jobs.keys().cloned().collect()
    }

    #[instrument(skip(self))]
    pub async fn job_status(&self, job_id: &str) -> Option<JobHealthStatus> {
        let registered = self.jobs.get(job_id)?;

        let last_run = self.repository.get_last_run(job_id).await.ok().flatten();
        let consecutive_failures = self
            .repository
            .count_consecutive_failures(job_id)
            .await
            .unwrap_or(0);

        let status = determine_health_state(&last_run, consecutive_failures);

        Some(JobHealthStatus {
            job_id: job_id.to_string(),
            name: registered.job.name().to_string(),
            status,
            last_run: last_run.map(|r| LastRunInfo {
                run_id: r.id,
                status: r.status,
                started_at: r.started_at,
                duration_ms: r.duration_ms,
                error: r.error_message,
            }),
            consecutive_failures,
        })
    }

    #[instrument(skip(self))]
    pub async fn health_status(&self) -> JobsHealthStatus {
        let mut jobs = Vec::new();
        let mut worst_state = HealthState::Healthy;

        for job_id in self.jobs.keys() {
            if let Some(status) = self.job_status(job_id).await {
                if status.status == HealthState::Unhealthy {
                    worst_state = HealthState::Unhealthy;
                } else if status.status == HealthState::Degraded
                    && worst_state != HealthState::Unhealthy
                {
                    worst_state = HealthState::Degraded;
                }
                jobs.push(status);
            }
        }

        JobsHealthStatus {
            status: worst_state,
            jobs,
        }
    }
}

fn determine_health_state(last_run: &Option<JobRun>, consecutive_failures: u32) -> HealthState {
    match last_run {
        None => HealthState::Healthy,
        Some(run) => match run.status {
            JobStatus::Succeeded => HealthState::Healthy,
            JobStatus::Running => HealthState::Healthy,
            JobStatus::Cancelled => HealthState::Healthy,
            JobStatus::Failed => {
                if consecutive_failures >= 3 {
                    HealthState::Unhealthy
                } else if consecutive_failures >= 1 {
                    HealthState::Degraded
                } else {
                    HealthState::Healthy
                }
            }
        },
    }
}

async fn run_job_with_retry(
    job: &Arc<dyn Job>,
    repository: &Arc<JobRepository>,
    triggered_by: TriggerSource,
    cancellation_token: &CancellationToken,
) -> Result<String> {
    let mut retry_count = 0u32;
    let run_id = uuid::Uuid::new_v4().to_string();

    loop {
        let ctx = JobContext {
            run_id: run_id.clone(),
            triggered_by: if retry_count > 0 {
                TriggerSource::Retry
            } else {
                triggered_by
            },
            cancellation_token: cancellation_token.clone(),
        };

        let run = JobRun {
            id: run_id.clone(),
            job_id: job.id().to_string(),
            status: JobStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            error_message: None,
            retry_count,
            triggered_by: ctx.triggered_by,
            metadata: None,
        };

        if retry_count == 0 {
            repository.record_run_start(&run).await?;
        }

        match job.run(&ctx).await {
            Ok(output) => {
                repository
                    .record_run_complete(&run_id, JobStatus::Succeeded, None, output.metadata)
                    .await?;
                info!(job_id = %job.id(), run_id = %run_id, "Job completed successfully");
                return Ok(run_id);
            }
            Err(JobError::Cancelled) => {
                repository
                    .record_run_complete(&run_id, JobStatus::Cancelled, None, None)
                    .await?;
                info!(job_id = %job.id(), run_id = %run_id, "Job cancelled");
                return Err(JobError::Cancelled);
            }
            Err(JobError::Failed { message, retryable }) => {
                if retryable && retry_count < MAX_RETRIES {
                    retry_count += 1;
                    let delay_secs = calculate_backoff_delay(retry_count);
                    warn!(
                        job_id = %job.id(),
                        run_id = %run_id,
                        retry_count,
                        delay_secs,
                        error = %message,
                        "Job failed, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                    continue;
                }

                repository
                    .record_run_complete(&run_id, JobStatus::Failed, Some(message.clone()), None)
                    .await?;
                warn!(job_id = %job.id(), run_id = %run_id, error = %message, "Job failed");
                return Err(JobError::Failed { message, retryable });
            }
            Err(e) => {
                let message = e.to_string();
                repository
                    .record_run_complete(&run_id, JobStatus::Failed, Some(message.clone()), None)
                    .await?;
                warn!(job_id = %job.id(), run_id = %run_id, error = %message, "Job failed with error");
                return Err(e);
            }
        }
    }
}

fn calculate_backoff_delay(retry_count: u32) -> u64 {
    let delay = BASE_RETRY_DELAY_SECS as f64 * RETRY_FACTOR.powi(retry_count as i32 - 1);
    (delay as u64).min(MAX_RETRY_DELAY_SECS)
}
