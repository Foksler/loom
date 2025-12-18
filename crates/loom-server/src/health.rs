//! Health check types and component checking logic.

use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{timeout, Instant};

use loom_github_app::{GithubAppClient, GithubAppError};

use crate::db::ThreadRepository;

/// Health status for components and overall system.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Server version and build information.
#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_timestamp: Option<&'static str>,
}

/// Database component health.
#[derive(Debug, Serialize)]
pub struct DatabaseHealth {
    pub status: HealthStatus,
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Binary directory component health.
#[derive(Debug, Serialize)]
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

/// Individual LLM provider health.
#[derive(Debug, Serialize)]
pub struct LlmProviderHealth {
    pub name: String,
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// LLM providers component health.
#[derive(Debug, Serialize)]
pub struct LlmProvidersHealth {
    pub status: HealthStatus,
    pub providers: Vec<LlmProviderHealth>,
}

/// Google CSE component health.
#[derive(Debug, Serialize)]
pub struct GoogleCseHealth {
    pub status: HealthStatus,
    pub latency_ms: u64,
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GitHub App component health.
#[derive(Debug, Serialize)]
pub struct GithubAppHealth {
    pub status: HealthStatus,
    pub latency_ms: u64,
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// All health check components.
#[derive(Debug, Serialize)]
pub struct HealthComponents {
    pub database: DatabaseHealth,
    pub bin_dir: BinDirHealth,
    pub llm_providers: LlmProvidersHealth,
    pub google_cse: GoogleCseHealth,
    pub github_app: GithubAppHealth,
}

/// Complete health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: String,
    pub duration_ms: u64,
    pub version: VersionInfo,
    pub components: HealthComponents,
}

/// Build information constant.
pub const VERSION_INFO: VersionInfo = VersionInfo {
    version: env!("CARGO_PKG_VERSION"),
    git_sha: option_env!("GIT_SHA"),
    build_timestamp: option_env!("BUILD_TIMESTAMP"),
};

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
pub fn check_llm_providers(llm_service: Option<&loom_llm_service::LlmService>) -> LlmProvidersHealth {
    match llm_service {
        Some(service) => {
            let mut providers = Vec::new();

            if service.has_anthropic() {
                providers.push(LlmProviderHealth {
                    name: "anthropic".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: None,
                    error: None,
                });
            }

            if service.has_openai() {
                providers.push(LlmProviderHealth {
                    name: "openai".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: None,
                    error: None,
                });
            }

            LlmProvidersHealth {
                status: HealthStatus::Healthy,
                providers,
            }
        }
        None => LlmProvidersHealth {
            status: HealthStatus::Degraded,
            providers: vec![LlmProviderHealth {
                name: "none".to_string(),
                status: HealthStatus::Degraded,
                latency_ms: None,
                error: Some("LLM service not configured".to_string()),
            }],
        },
    }
}

const CSE_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Check Google CSE health by verifying configuration and optionally testing connectivity.
pub async fn check_google_cse() -> GoogleCseHealth {
    use loom_google_cse::{CseClient, CseRequest};
    
    let start = Instant::now();

    // Check if CSE is configured
    let api_key = std::env::var("LOOM_SERVER_GOOGLE_CSE_API_KEY");
    let cx = std::env::var("LOOM_SERVER_GOOGLE_CSE_CX");

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
                        (true, HealthStatus::Unhealthy, Some("Invalid API key or CSE ID".to_string()))
                    } else if err_str.contains("Rate limit") {
                        (true, HealthStatus::Degraded, Some("Rate limited".to_string()))
                    } else {
                        (true, HealthStatus::Degraded, Some(err_str))
                    }
                }
                Err(_) => (true, HealthStatus::Degraded, Some("CSE health check timed out".to_string())),
            }
        }
        _ => {
            // Not configured - this is degraded, not unhealthy (CSE is optional)
            (false, HealthStatus::Degraded, Some("Google CSE not configured".to_string()))
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
        Some(client) => {
            match timeout(GITHUB_CHECK_TIMEOUT, client.list_installations()).await {
                Ok(Ok(_)) => (true, HealthStatus::Healthy, None),
                Ok(Err(e)) => {
                    let status = match &e {
                        GithubAppError::Unauthorized
                        | GithubAppError::Config(_)
                        | GithubAppError::Jwt(_) => HealthStatus::Unhealthy,
                        GithubAppError::Timeout
                        | GithubAppError::RateLimited
                        | GithubAppError::Network(_) => HealthStatus::Degraded,
                        GithubAppError::ApiError { status, .. } if *status >= 500 => {
                            HealthStatus::Degraded
                        }
                        _ => HealthStatus::Degraded,
                    };
                    (true, status, Some(e.to_string()))
                }
                Err(_) => (
                    true,
                    HealthStatus::Degraded,
                    Some("GitHub health check timed out".to_string()),
                ),
            }
        }
    };

    let latency_ms = start.elapsed().as_millis() as u64;

    GithubAppHealth {
        status,
        latency_ms,
        configured,
        error,
    }
}

/// Aggregate component statuses into overall status.
pub fn aggregate_status(components: &HealthComponents) -> HealthStatus {
    let statuses = [
        components.database.status,
        components.bin_dir.status,
        components.google_cse.status,
        components.github_app.status,
    ];

    if statuses.iter().any(|s| matches!(s, HealthStatus::Unhealthy)) {
        HealthStatus::Unhealthy
    } else if statuses.iter().any(|s| matches!(s, HealthStatus::Degraded)) {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    }
}
