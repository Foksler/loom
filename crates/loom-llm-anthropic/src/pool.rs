// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Anthropic OAuth account pool with automatic failover.
//!
//! This module provides pooling of multiple Claude Pro/Max OAuth subscriptions
//! with automatic failover when an account hits its usage quota.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use loom_core::{LlmClient, LlmError, LlmRequest, LlmResponse, LlmStream};
use loom_credentials::{CredentialStore, CredentialValue, FileCredentialStore};

use serde::Serialize;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::auth::{AnthropicAuth, OAuthClient, OAuthCredentials};
use crate::client::{is_permanent_auth_message, is_quota_message, AnthropicClient};
use crate::types::AnthropicConfig;

/// Runtime state for each account in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    /// Account is ready to use.
    Available,
    /// Account hit quota, cooling down until specified instant.
    CoolingDown { until: Instant },
    /// Account permanently disabled (e.g., invalid credentials).
    Disabled,
}

/// Account selection strategy.
#[derive(Debug, Clone, Copy, Default)]
pub enum AccountSelectionStrategy {
    /// Round-robin through available accounts.
    #[default]
    RoundRobin,
    /// Always try first available account.
    FirstAvailable,
}

/// Pool configuration.
#[derive(Debug, Clone)]
pub struct AnthropicPoolConfig {
    /// How long to cool down an account after quota exhaustion.
    pub cooldown: Duration,
    /// Account selection strategy.
    pub strategy: AccountSelectionStrategy,
}

impl Default for AnthropicPoolConfig {
    fn default() -> Self {
        Self {
            cooldown: Duration::from_secs(2 * 60 * 60), // 2 hours
            strategy: AccountSelectionStrategy::RoundRobin,
        }
    }
}

/// Health status for an account (serializable for API response).
#[derive(Debug, Clone, Serialize)]
pub struct AccountHealthInfo {
    pub id: String,
    pub status: AccountHealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_remaining_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Account health status for serialization.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountHealthStatus {
    Available,
    CoolingDown,
    Disabled,
}

/// Overall pool status for health reporting.
#[derive(Debug, Clone, Serialize)]
pub struct PoolStatus {
    pub accounts_total: usize,
    pub accounts_available: usize,
    pub accounts_cooling: usize,
    pub accounts_disabled: usize,
    pub accounts: Vec<AccountHealthInfo>,
}

/// Runtime tracking for an account.
#[derive(Debug)]
struct AccountRuntime {
    status: AccountStatus,
    last_error: Option<String>,
}

impl Default for AccountRuntime {
    fn default() -> Self {
        Self {
            status: AccountStatus::Available,
            last_error: None,
        }
    }
}

/// An account entry in the pool.
struct AccountEntry {
    id: String,
    client: AnthropicClient<FileCredentialStore>,
}

impl std::fmt::Debug for AccountEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountEntry")
            .field("id", &self.id)
            .finish()
    }
}

/// Mutable pool state.
#[derive(Debug)]
struct PoolState {
    runtimes: Vec<AccountRuntime>,
    next_index: usize,
}

/// Pool of Anthropic OAuth accounts with automatic failover.
///
/// Manages multiple Claude Pro/Max subscriptions and automatically
/// fails over to the next available account when one hits its quota.
pub struct AnthropicPool {
    accounts: Vec<AccountEntry>,
    state: Mutex<PoolState>,
    config: AnthropicPoolConfig,
}

impl std::fmt::Debug for AnthropicPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicPool")
            .field("accounts", &self.accounts)
            .field("config", &self.config)
            .finish()
    }
}

impl AnthropicPool {
    /// Create a new pool from OAuth credentials.
    ///
    /// Loads credentials from the specified file for each provider ID.
    /// Skips providers with missing or invalid credentials (logs a warning).
    /// Returns an error if no valid accounts could be loaded.
    pub async fn new(
        credential_file: impl AsRef<Path>,
        provider_ids: Vec<String>,
        model: Option<String>,
        config: AnthropicPoolConfig,
    ) -> Result<Self, LlmError> {
        let store = Arc::new(FileCredentialStore::new(credential_file.as_ref()));
        let model = model.unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

        let mut accounts = Vec::new();

        for provider_id in &provider_ids {
            match store.load(provider_id).await {
                Ok(Some(CredentialValue::OAuth {
                    refresh,
                    access,
                    expires,
                })) => {
                    let creds = OAuthCredentials::new(refresh, access, expires);
                    let oauth_client = OAuthClient::new(provider_id.clone(), creds, Arc::clone(&store));
                    let auth = AnthropicAuth::OAuth { client: oauth_client };
                    let config = AnthropicConfig::new_with_auth(auth).with_model(model.clone());

                    match AnthropicClient::new_with_store(config) {
                        Ok(client) => {
                            info!(provider = %provider_id, "Loaded OAuth account");
                            accounts.push(AccountEntry {
                                id: provider_id.clone(),
                                client,
                            });
                        }
                        Err(e) => {
                            warn!(provider = %provider_id, error = %e, "Failed to create client for provider");
                        }
                    }
                }
                Ok(Some(CredentialValue::ApiKey { .. })) => {
                    warn!(provider = %provider_id, "Expected OAuth credentials but found API key, skipping");
                }
                Ok(None) => {
                    warn!(provider = %provider_id, "No credentials found for provider, skipping");
                }
                Err(e) => {
                    warn!(provider = %provider_id, error = %e, "Failed to load credentials for provider, skipping");
                }
            }
        }

        if accounts.is_empty() {
            return Err(LlmError::Api(
                "No valid OAuth accounts could be loaded for the pool".to_string(),
            ));
        }

        let runtimes = accounts.iter().map(|_| AccountRuntime::default()).collect();
        let state = PoolState {
            runtimes,
            next_index: 0,
        };

        info!(
            accounts_loaded = accounts.len(),
            requested = provider_ids.len(),
            "AnthropicPool initialized"
        );

        Ok(Self {
            accounts,
            state: Mutex::new(state),
            config,
        })
    }

    /// Select an available account index.
    ///
    /// Refreshes cooling accounts whose cooldown has expired.
    /// Returns None if all accounts are exhausted.
    async fn select_account_index(&self) -> Option<usize> {
        let mut state = self.state.lock().await;
        let n = self.accounts.len();
        let now = Instant::now();

        // Refresh any expired cooldowns
        for runtime in &mut state.runtimes {
            if let AccountStatus::CoolingDown { until } = runtime.status {
                if now >= until {
                    debug!("Account cooldown expired, marking available");
                    runtime.status = AccountStatus::Available;
                    runtime.last_error = None;
                }
            }
        }

        match self.config.strategy {
            AccountSelectionStrategy::RoundRobin => {
                let start = state.next_index;
                for i in 0..n {
                    let idx = (start + i) % n;
                    if state.runtimes[idx].status == AccountStatus::Available {
                        state.next_index = (idx + 1) % n;
                        debug!(account_id = %self.accounts[idx].id, index = idx, "Selected account (round-robin)");
                        return Some(idx);
                    }
                }
                None
            }
            AccountSelectionStrategy::FirstAvailable => {
                for (idx, runtime) in state.runtimes.iter().enumerate() {
                    if runtime.status == AccountStatus::Available {
                        debug!(account_id = %self.accounts[idx].id, index = idx, "Selected account (first-available)");
                        return Some(idx);
                    }
                }
                None
            }
        }
    }

    /// Mark an account as cooling down due to quota exhaustion.
    async fn mark_cooling(&self, index: usize, error_msg: &str) {
        let mut state = self.state.lock().await;
        let until = Instant::now() + self.config.cooldown;
        state.runtimes[index].status = AccountStatus::CoolingDown { until };
        state.runtimes[index].last_error = Some(error_msg.to_string());
        info!(
            account_id = %self.accounts[index].id,
            cooldown_secs = self.config.cooldown.as_secs(),
            "Account marked as cooling down"
        );
    }

    /// Mark an account as permanently disabled.
    async fn mark_disabled(&self, index: usize, error_msg: &str) {
        let mut state = self.state.lock().await;
        state.runtimes[index].status = AccountStatus::Disabled;
        state.runtimes[index].last_error = Some(error_msg.to_string());
        error!(
            account_id = %self.accounts[index].id,
            error = %error_msg,
            "Account permanently disabled"
        );
    }

    /// Get current pool status for health reporting.
    pub async fn pool_status(&self) -> PoolStatus {
        let state = self.state.lock().await;
        let now = Instant::now();

        let mut accounts_available = 0;
        let mut accounts_cooling = 0;
        let mut accounts_disabled = 0;
        let mut accounts = Vec::with_capacity(self.accounts.len());

        for (entry, runtime) in self.accounts.iter().zip(state.runtimes.iter()) {
            let (status, cooldown_remaining_secs) = match runtime.status {
                AccountStatus::Available => {
                    accounts_available += 1;
                    (AccountHealthStatus::Available, None)
                }
                AccountStatus::CoolingDown { until } => {
                    accounts_cooling += 1;
                    let remaining = if until > now {
                        until.duration_since(now).as_secs()
                    } else {
                        0
                    };
                    (AccountHealthStatus::CoolingDown, Some(remaining))
                }
                AccountStatus::Disabled => {
                    accounts_disabled += 1;
                    (AccountHealthStatus::Disabled, None)
                }
            };

            accounts.push(AccountHealthInfo {
                id: entry.id.clone(),
                status,
                cooldown_remaining_secs,
                last_error: runtime.last_error.clone(),
            });
        }

        PoolStatus {
            accounts_total: self.accounts.len(),
            accounts_available,
            accounts_cooling,
            accounts_disabled,
            accounts,
        }
    }
}



#[async_trait]
impl LlmClient for AnthropicPool {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let index = self
            .select_account_index()
            .await
            .ok_or(LlmError::RateLimited { retry_after_secs: None })?;

        let account = &self.accounts[index];
        debug!(account_id = %account.id, "Attempting completion with account");

        match account.client.complete(request).await {
            Ok(response) => Ok(response),
            Err(e) => {
                let error_msg = e.to_string();
                if is_quota_message(&error_msg) {
                    self.mark_cooling(index, &error_msg).await;
                } else if is_permanent_auth_message(&error_msg) {
                    self.mark_disabled(index, &error_msg).await;
                }
                Err(e)
            }
        }
    }

    async fn complete_streaming(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        let index = self
            .select_account_index()
            .await
            .ok_or(LlmError::RateLimited { retry_after_secs: None })?;

        let account = &self.accounts[index];
        debug!(account_id = %account.id, "Attempting streaming completion with account");

        match account.client.complete_streaming(request).await {
            Ok(stream) => Ok(stream),
            Err(e) => {
                let error_msg = e.to_string();
                if is_quota_message(&error_msg) {
                    self.mark_cooling(index, &error_msg).await;
                } else if is_permanent_auth_message(&error_msg) {
                    self.mark_disabled(index, &error_msg).await;
                }
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pool_config() {
        let config = AnthropicPoolConfig::default();
        assert_eq!(config.cooldown, Duration::from_secs(2 * 60 * 60));
        assert!(matches!(config.strategy, AccountSelectionStrategy::RoundRobin));
    }

    #[test]
    fn test_account_status_equality() {
        assert_eq!(AccountStatus::Available, AccountStatus::Available);
        assert_eq!(AccountStatus::Disabled, AccountStatus::Disabled);
        assert_ne!(AccountStatus::Available, AccountStatus::Disabled);
    }

    #[test]
    fn test_is_quota_message() {
        assert!(is_quota_message("5-hour rolling window exceeded"));
        assert!(is_quota_message("You have hit the 5 hour usage limit"));
        assert!(is_quota_message("usage limit for your plan exceeded"));
        assert!(is_quota_message("subscription usage limit reached"));
        assert!(!is_quota_message("rate limit exceeded"));
        assert!(!is_quota_message("internal server error"));
    }

    #[test]
    fn test_is_permanent_auth_message() {
        assert!(is_permanent_auth_message("401 Unauthorized"));
        assert!(is_permanent_auth_message("403 Forbidden"));
        assert!(is_permanent_auth_message("unauthorized access"));
        assert!(is_permanent_auth_message("Invalid API key provided"));
        assert!(is_permanent_auth_message("Invalid authentication credentials"));
        assert!(!is_permanent_auth_message("rate limit exceeded"));
        assert!(!is_permanent_auth_message("internal server error"));
    }

    #[test]
    fn test_pool_status_serialization() {
        let status = PoolStatus {
            accounts_total: 3,
            accounts_available: 1,
            accounts_cooling: 1,
            accounts_disabled: 1,
            accounts: vec![
                AccountHealthInfo {
                    id: "account-1".to_string(),
                    status: AccountHealthStatus::Available,
                    cooldown_remaining_secs: None,
                    last_error: None,
                },
                AccountHealthInfo {
                    id: "account-2".to_string(),
                    status: AccountHealthStatus::CoolingDown,
                    cooldown_remaining_secs: Some(3600),
                    last_error: Some("Usage limit exceeded".to_string()),
                },
                AccountHealthInfo {
                    id: "account-3".to_string(),
                    status: AccountHealthStatus::Disabled,
                    cooldown_remaining_secs: None,
                    last_error: Some("Unauthorized".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"accounts_total\":3"));
        assert!(json.contains("\"status\":\"available\""));
        assert!(json.contains("\"status\":\"cooling_down\""));
        assert!(json.contains("\"status\":\"disabled\""));
    }

    #[test]
    fn test_pool_status_counts() {
        let status = PoolStatus {
            accounts_total: 5,
            accounts_available: 2,
            accounts_cooling: 2,
            accounts_disabled: 1,
            accounts: vec![
                AccountHealthInfo {
                    id: "acc-1".to_string(),
                    status: AccountHealthStatus::Available,
                    cooldown_remaining_secs: None,
                    last_error: None,
                },
                AccountHealthInfo {
                    id: "acc-2".to_string(),
                    status: AccountHealthStatus::Available,
                    cooldown_remaining_secs: None,
                    last_error: None,
                },
                AccountHealthInfo {
                    id: "acc-3".to_string(),
                    status: AccountHealthStatus::CoolingDown,
                    cooldown_remaining_secs: Some(1800),
                    last_error: Some("5-hour limit".to_string()),
                },
                AccountHealthInfo {
                    id: "acc-4".to_string(),
                    status: AccountHealthStatus::CoolingDown,
                    cooldown_remaining_secs: Some(3600),
                    last_error: Some("rolling window".to_string()),
                },
                AccountHealthInfo {
                    id: "acc-5".to_string(),
                    status: AccountHealthStatus::Disabled,
                    cooldown_remaining_secs: None,
                    last_error: Some("401 Unauthorized".to_string()),
                },
            ],
        };

        assert_eq!(status.accounts_total, 5);
        assert_eq!(status.accounts_available, 2);
        assert_eq!(status.accounts_cooling, 2);
        assert_eq!(status.accounts_disabled, 1);
        assert_eq!(status.accounts.len(), 5);
    }

    #[test]
    fn test_account_runtime_default() {
        let runtime = AccountRuntime::default();
        assert_eq!(runtime.status, AccountStatus::Available);
        assert!(runtime.last_error.is_none());
    }

    #[test]
    fn test_account_health_info_serialization_without_optional() {
        let info = AccountHealthInfo {
            id: "test-account".to_string(),
            status: AccountHealthStatus::Available,
            cooldown_remaining_secs: None,
            last_error: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":\"test-account\""));
        assert!(json.contains("\"status\":\"available\""));
        assert!(!json.contains("cooldown_remaining_secs"));
        assert!(!json.contains("last_error"));
    }

    #[test]
    fn test_selection_strategy_display() {
        let round_robin = AccountSelectionStrategy::RoundRobin;
        let first_available = AccountSelectionStrategy::FirstAvailable;

        assert!(matches!(round_robin, AccountSelectionStrategy::RoundRobin));
        assert!(matches!(first_available, AccountSelectionStrategy::FirstAvailable));

        let default_strategy = AccountSelectionStrategy::default();
        assert!(matches!(default_strategy, AccountSelectionStrategy::RoundRobin));
    }
}
