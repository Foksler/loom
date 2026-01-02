// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Health check types and component checking logic.

use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{timeout, Instant};
use utoipa::ToSchema;

use loom_server_weaver::Provisioner;
use loom_server_github_app::{GithubAppClient, GithubAppError};
use loom_server_jobs::JobScheduler;
use loom_server_llm_service::LlmService;
use loom_server_smtp::SmtpClient;

use crate::db::ThreadRepository;

/// Health status for components and overall system.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
	Healthy,
	Degraded,
	Unhealthy,
	Unknown,
}

/// Database component health.
#[derive(Debug, Serialize, ToSchema)]
pub struct DatabaseHealth {
	pub status: HealthStatus,
	pub latency_ms: u64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

/// Binary directory component health.
#[derive(Debug, Serialize, ToSchema)]
pub struct BinDirHealth {
	pub status: HealthStatus,
	pub latency_ms: u64,
	pub path: String,
	pub exists: bool,
	pub is_dir: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub file_count: Option<usize>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

/// Individual account health in the pool
#[derive(Debug, Serialize, ToSchema)]
pub struct AnthropicAccountHealth {
	pub id: String,
	pub status: AnthropicAccountStatus,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cooldown_remaining_secs: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnthropicAccountStatus {
	Available,
	CoolingDown,
	Disabled,
}

/// Pool status for health reporting
#[derive(Debug, Serialize, ToSchema)]
pub struct AnthropicPoolHealth {
	pub accounts_total: usize,
	pub accounts_available: usize,
	pub accounts_cooling: usize,
	pub accounts_disabled: usize,
	pub accounts: Vec<AnthropicAccountHealth>,
}

/// Individual LLM provider health.
#[derive(Debug, Serialize, ToSchema)]
pub struct LlmProviderHealth {
	pub name: String,
	pub status: HealthStatus,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mode: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pool: Option<AnthropicPoolHealth>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub latency_ms: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

/// LLM providers component health.
#[derive(Debug, Serialize, ToSchema)]
pub struct LlmProvidersHealth {
	pub status: HealthStatus,
	pub providers: Vec<LlmProviderHealth>,
}

/// Google CSE component health.
#[derive(Debug, Serialize, ToSchema)]
pub struct GoogleCseHealth {
	pub status: HealthStatus,
	pub latency_ms: u64,
	pub configured: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

/// GitHub App component health.
#[derive(Debug, Serialize, ToSchema)]
pub struct GithubAppHealth {
	pub status: HealthStatus,
	pub latency_ms: u64,
	pub configured: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

/// Kubernetes component health.
#[derive(Debug, Serialize, ToSchema)]
pub struct KubernetesHealth {
	pub status: HealthStatus,
	pub latency_ms: u64,
	pub namespace: String,
	pub reachable: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

/// SMTP component health.
#[derive(Debug, Serialize, ToSchema)]
pub struct SmtpHealth {
	pub status: HealthStatus,
	pub latency_ms: u64,
	pub configured: bool,
	pub healthy: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

/// GeoIP component health.
#[derive(Debug, Serialize, ToSchema)]
pub struct GeoIpHealth {
	pub status: HealthStatus,
	pub latency_ms: u64,
	pub configured: bool,
	pub healthy: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub database_path: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub database_type: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

/// Jobs scheduler component health.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobsHealth {
	pub status: HealthStatus,
	pub jobs_total: usize,
	pub jobs_healthy: usize,
	pub jobs_failing: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failing_jobs: Option<Vec<String>>,
}

/// All health check components.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthComponents {
	pub database: DatabaseHealth,
	pub bin_dir: BinDirHealth,
	pub llm_providers: LlmProvidersHealth,
	pub google_cse: GoogleCseHealth,
	pub github_app: GithubAppHealth,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kubernetes: Option<KubernetesHealth>,
	pub smtp: SmtpHealth,
	pub geoip: GeoIpHealth,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub jobs: Option<JobsHealth>,
}

/// Complete health check response.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
	pub status: HealthStatus,
	pub timestamp: String,
	pub duration_ms: u64,
	pub version: loom_common_version::HealthVersionInfo,
	pub components: HealthComponents,
}

const DB_CHECK_TIMEOUT: Duration = Duration::from_millis(500);

/// Check database health.
pub async fn check_database(repo: &ThreadRepository) -> DatabaseHealth {
	let start = Instant::now();

	let result = timeout(DB_CHECK_TIMEOUT, repo.health_check()).await;
	let latency_ms = start.elapsed().as_millis() as u64;

	match result {
		Ok(Ok(())) => DatabaseHealth {
			status: HealthStatus::Healthy,
			latency_ms,
			error: None,
		},
		Ok(Err(e)) => DatabaseHealth {
			status: HealthStatus::Unhealthy,
			latency_ms,
			error: Some(e.to_string()),
		},
		Err(_) => DatabaseHealth {
			status: HealthStatus::Unhealthy,
			latency_ms,
			error: Some("database health check timed out".to_string()),
		},
	}
}

/// Check binary directory health.
pub fn check_bin_dir() -> BinDirHealth {
	let start = Instant::now();

	let bin_dir = std::env::var("LOOM_SERVER_BIN_DIR").unwrap_or_else(|_| "./bin".to_string());
	let path = Path::new(&bin_dir);

	let (exists, is_dir, file_count, status, error) = if !path.exists() {
		(
			false,
			false,
			None,
			HealthStatus::Degraded,
			Some("binary directory does not exist".to_string()),
		)
	} else if !path.is_dir() {
		(
			true,
			false,
			None,
			HealthStatus::Degraded,
			Some("binary path is not a directory".to_string()),
		)
	} else {
		match std::fs::read_dir(path) {
			Ok(entries) => {
				let count = entries
					.filter_map(Result::ok)
					.filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
					.count();
				if count == 0 {
					(
						true,
						true,
						Some(0),
						HealthStatus::Degraded,
						Some("binary directory is empty".to_string()),
					)
				} else {
					(true, true, Some(count), HealthStatus::Healthy, None)
				}
			}
			Err(e) => (
				true,
				true,
				None,
				HealthStatus::Degraded,
				Some(format!("failed to read binary directory: {e}")),
			),
		}
	};

	let latency_ms = start.elapsed().as_millis() as u64;

	BinDirHealth {
		status,
		latency_ms,
		path: bin_dir,
		exists,
		is_dir,
		file_count,
		error,
	}
}

/// Check LLM provider health by verifying if the service is configured.
pub async fn check_llm_providers(llm_service: Option<&LlmService>) -> LlmProvidersHealth {
	match llm_service {
		Some(service) => {
			let mut providers = Vec::new();
			let mut overall_status = HealthStatus::Healthy;

			if service.has_anthropic() {
				let anthropic_health = service.anthropic_health().await;
				let (status, mode, pool) = match anthropic_health {
					Some(loom_server_llm_service::AnthropicHealthInfo::ApiKey { .. }) => {
						(HealthStatus::Healthy, Some("api_key".to_string()), None)
					}
					Some(loom_server_llm_service::AnthropicHealthInfo::Pool(pool_status)) => {
						let status = if pool_status.accounts_available == pool_status.accounts_total {
							HealthStatus::Healthy
						} else if pool_status.accounts_available > 0 {
							HealthStatus::Degraded
						} else {
							HealthStatus::Unhealthy
						};

						let pool_health = AnthropicPoolHealth {
							accounts_total: pool_status.accounts_total,
							accounts_available: pool_status.accounts_available,
							accounts_cooling: pool_status.accounts_cooling,
							accounts_disabled: pool_status.accounts_disabled,
							accounts: pool_status
								.accounts
								.into_iter()
								.map(|a| AnthropicAccountHealth {
									id: a.id,
									status: match a.status {
										loom_server_llm_service::AccountHealthStatus::Available => {
											AnthropicAccountStatus::Available
										}
										loom_server_llm_service::AccountHealthStatus::CoolingDown => {
											AnthropicAccountStatus::CoolingDown
										}
										loom_server_llm_service::AccountHealthStatus::Disabled => {
											AnthropicAccountStatus::Disabled
										}
									},
									cooldown_remaining_secs: a.cooldown_remaining_secs,
									last_error: a.last_error,
								})
								.collect(),
						};

						(status, Some("oauth_pool".to_string()), Some(pool_health))
					}
					None => (HealthStatus::Healthy, None, None),
				};

				if status == HealthStatus::Degraded && overall_status == HealthStatus::Healthy {
					overall_status = HealthStatus::Degraded;
				} else if status == HealthStatus::Unhealthy {
					overall_status = HealthStatus::Unhealthy;
				}

				providers.push(LlmProviderHealth {
					name: "anthropic".to_string(),
					status,
					mode,
					pool,
					latency_ms: None,
					error: None,
				});
			}

			if service.has_openai() {
				providers.push(LlmProviderHealth {
					name: "openai".to_string(),
					status: HealthStatus::Healthy,
					mode: None,
					pool: None,
					latency_ms: None,
					error: None,
				});
			}

			LlmProvidersHealth {
				status: overall_status,
				providers,
			}
		}
		None => LlmProvidersHealth {
			status: HealthStatus::Degraded,
			providers: vec![LlmProviderHealth {
				name: "none".to_string(),
				status: HealthStatus::Degraded,
				mode: None,
				pool: None,
				latency_ms: None,
				error: Some("LLM service not configured".to_string()),
			}],
		},
	}
}

const CSE_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Check Google CSE health by verifying configuration and optionally testing
/// connectivity.
pub async fn check_google_cse() -> GoogleCseHealth {
	use loom_google_cse::{CseClient, CseRequest};

	let start = Instant::now();

	// Check if CSE is configured
	let api_key = std::env::var("LOOM_SERVER_GOOGLE_CSE_API_KEY");
	let cx = std::env::var("LOOM_SERVER_GOOGLE_CSE_SEARCH_ENGINE_ID");

	let (configured, status, error) = match (api_key, cx) {
		(Ok(key), Ok(cx_val)) if !key.is_empty() && !cx_val.is_empty() => {
			// CSE is configured, try a simple search to verify connectivity
			let client = CseClient::new(key, cx_val);
			let request = CseRequest::new("test", 1);

			match timeout(CSE_CHECK_TIMEOUT, client.search(request)).await {
				Ok(Ok(_)) => (true, HealthStatus::Healthy, None),
				Ok(Err(e)) => {
					// Check if it's an auth error vs network error
					let err_str = e.to_string();
					if err_str.contains("Unauthorized") || err_str.contains("Invalid API key") {
						(
							true,
							HealthStatus::Unhealthy,
							Some("Invalid API key or CSE ID".to_string()),
						)
					} else if err_str.contains("Rate limit") {
						(
							true,
							HealthStatus::Degraded,
							Some("Rate limited".to_string()),
						)
					} else {
						(true, HealthStatus::Degraded, Some(err_str))
					}
				}
				Err(_) => (
					true,
					HealthStatus::Degraded,
					Some("CSE health check timed out".to_string()),
				),
			}
		}
		_ => {
			// Not configured - this is degraded, not unhealthy (CSE is optional)
			(
				false,
				HealthStatus::Degraded,
				Some("Google CSE not configured".to_string()),
			)
		}
	};

	let latency_ms = start.elapsed().as_millis() as u64;

	GoogleCseHealth {
		status,
		latency_ms,
		configured,
		error,
	}
}

const GITHUB_CHECK_TIMEOUT: Duration = Duration::from_secs(3);

/// Check GitHub App health by validating JWT generation and API connectivity.
pub async fn check_github_app(client: Option<Arc<GithubAppClient>>) -> GithubAppHealth {
	let start = Instant::now();

	let (configured, status, error) = match client {
		None => (
			false,
			HealthStatus::Degraded,
			Some("GitHub App not configured".to_string()),
		),
		Some(client) => match timeout(GITHUB_CHECK_TIMEOUT, client.list_installations()).await {
			Ok(Ok(_)) => (true, HealthStatus::Healthy, None),
			Ok(Err(e)) => {
				let status = match &e {
					GithubAppError::Unauthorized | GithubAppError::Config(_) | GithubAppError::Jwt(_) => {
						HealthStatus::Unhealthy
					}
					GithubAppError::Timeout | GithubAppError::RateLimited | GithubAppError::Network(_) => {
						HealthStatus::Degraded
					}
					GithubAppError::ApiError { status, .. } if *status >= 500 => HealthStatus::Degraded,
					_ => HealthStatus::Degraded,
				};
				(true, status, Some(e.to_string()))
			}
			Err(_) => (
				true,
				HealthStatus::Degraded,
				Some("GitHub health check timed out".to_string()),
			),
		},
	};

	let latency_ms = start.elapsed().as_millis() as u64;

	GithubAppHealth {
		status,
		latency_ms,
		configured,
		error,
	}
}

const K8S_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Check Kubernetes connectivity by listing pods in the namespace.
pub async fn check_kubernetes(provisioner: Option<&Arc<Provisioner>>) -> Option<KubernetesHealth> {
	let provisioner = provisioner?;

	let start = Instant::now();
	let namespace = provisioner.namespace().to_string();

	let result = timeout(K8S_CHECK_TIMEOUT, provisioner.count_active_weavers()).await;
	let latency_ms = start.elapsed().as_millis() as u64;

	let (status, reachable, error) = match result {
		Ok(Ok(_)) => (HealthStatus::Healthy, true, None),
		Ok(Err(e)) => (HealthStatus::Unhealthy, false, Some(e.to_string())),
		Err(_) => (
			HealthStatus::Unhealthy,
			false,
			Some("Kubernetes health check timed out".to_string()),
		),
	};

	Some(KubernetesHealth {
		status,
		latency_ms,
		namespace,
		reachable,
		error,
	})
}

const SMTP_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Check SMTP server health by testing connectivity.
pub async fn check_smtp(client: Option<&Arc<SmtpClient>>) -> SmtpHealth {
	let start = Instant::now();

	let (configured, healthy, status, error) = match client {
		None => (
			false,
			false,
			HealthStatus::Degraded,
			Some("SMTP not configured".to_string()),
		),
		Some(client) => match timeout(SMTP_CHECK_TIMEOUT, client.check_health()).await {
			Ok(Ok(())) => (true, true, HealthStatus::Healthy, None),
			Ok(Err(e)) => (true, false, HealthStatus::Unhealthy, Some(e.to_string())),
			Err(_) => (
				true,
				false,
				HealthStatus::Unhealthy,
				Some("SMTP health check timed out".to_string()),
			),
		},
	};

	let latency_ms = start.elapsed().as_millis() as u64;

	SmtpHealth {
		status,
		latency_ms,
		configured,
		healthy,
		error,
	}
}

/// Check GeoIP service health by validating database accessibility.
pub fn check_geoip(service: Option<&Arc<loom_server_geoip::GeoIpService>>) -> GeoIpHealth {
	use tokio::time::Instant;

	let start = Instant::now();

	let (configured, healthy, status, database_path, database_type, error) = match service {
		None => (
			false,
			false,
			HealthStatus::Degraded,
			None,
			None,
			Some("GeoIP not configured".to_string()),
		),
		Some(svc) => {
			let path = svc.database_path().to_string();
			if svc.is_healthy() {
				let metadata = svc.database_metadata();
				(
					true,
					true,
					HealthStatus::Healthy,
					Some(path),
					Some(metadata.database_type),
					None,
				)
			} else {
				(
					true,
					false,
					HealthStatus::Unhealthy,
					Some(path),
					None,
					Some("GeoIP database lookup failed".to_string()),
				)
			}
		}
	};

	let latency_ms = start.elapsed().as_millis() as u64;

	GeoIpHealth {
		status,
		latency_ms,
		configured,
		healthy,
		database_path,
		database_type,
		error,
	}
}

/// Check jobs scheduler health.
pub async fn check_jobs(scheduler: Option<&Arc<JobScheduler>>) -> Option<JobsHealth> {
	let scheduler = scheduler?;

	let health = scheduler.health_status().await;

	let jobs_failing: Vec<String> = health
		.jobs
		.iter()
		.filter(|j| matches!(j.status, loom_server_jobs::HealthState::Unhealthy))
		.map(|j| j.job_id.clone())
		.collect();

	let status = match health.status {
		loom_server_jobs::HealthState::Healthy => HealthStatus::Healthy,
		loom_server_jobs::HealthState::Degraded => HealthStatus::Degraded,
		loom_server_jobs::HealthState::Unhealthy => HealthStatus::Unhealthy,
	};

	Some(JobsHealth {
		status,
		jobs_total: health.jobs.len(),
		jobs_healthy: health
			.jobs
			.iter()
			.filter(|j| matches!(j.status, loom_server_jobs::HealthState::Healthy))
			.count(),
		jobs_failing: jobs_failing.len(),
		failing_jobs: if jobs_failing.is_empty() {
			None
		} else {
			Some(jobs_failing)
		},
	})
}

/// Aggregate component statuses into overall status.
pub fn aggregate_status(components: &HealthComponents) -> HealthStatus {
	let mut statuses = vec![
		components.database.status,
		components.bin_dir.status,
		components.google_cse.status,
		components.github_app.status,
		components.smtp.status,
		components.geoip.status,
	];

	if let Some(ref k8s) = components.kubernetes {
		statuses.push(k8s.status);
	}

	if let Some(ref jobs) = components.jobs {
		statuses.push(jobs.status);
	}

	if statuses
		.iter()
		.any(|s| matches!(s, HealthStatus::Unhealthy))
	{
		HealthStatus::Unhealthy
	} else if statuses.iter().any(|s| matches!(s, HealthStatus::Degraded)) {
		HealthStatus::Degraded
	} else {
		HealthStatus::Healthy
	}
}
