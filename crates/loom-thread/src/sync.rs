use async_trait::async_trait;
use reqwest::StatusCode;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::error::{ThreadStoreError, ThreadSyncError};
use crate::model::{Thread, ThreadId, ThreadSummary};
use crate::store::{LocalThreadStore, ThreadStore};

pub struct ThreadSyncClient {
    base_url: Url,
    http: reqwest::Client,
    retry_config: loom_http_retry::RetryConfig,
}

impl ThreadSyncClient {
    pub fn new(base_url: Url, http: reqwest::Client) -> Self {
        Self {
            base_url,
            http,
            retry_config: loom_http_retry::RetryConfig::default(),
        }
    }

    pub fn with_retry_config(mut self, config: loom_http_retry::RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    fn threads_url(&self) -> Result<Url, ThreadSyncError> {
        self.base_url
            .join("threads")
            .map_err(|e| ThreadSyncError::InvalidUrl(e.to_string()))
    }

    fn thread_url(&self, id: &ThreadId) -> Result<Url, ThreadSyncError> {
        self.base_url
            .join(&format!("threads/{}", id))
            .map_err(|e| ThreadSyncError::InvalidUrl(e.to_string()))
    }

    pub async fn upsert_thread(&self, thread: &Thread) -> Result<(), ThreadSyncError> {
        let url = self.thread_url(&thread.id)?;

        debug!(
            thread_id = %thread.id,
            version = thread.version,
            url = %url,
            "upserting thread to server"
        );

        let response = loom_http_retry::retry(&self.retry_config, || async {
            self.http
                .put(url.clone())
                .json(thread)
                .send()
                .await
                .map_err(ThreadSyncError::from)
        })
        .await?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED | StatusCode::NO_CONTENT => {
                info!(
                    thread_id = %thread.id,
                    version = thread.version,
                    "thread synced to server"
                );
                Ok(())
            }
            StatusCode::CONFLICT => {
                warn!(
                    thread_id = %thread.id,
                    version = thread.version,
                    "thread sync conflict"
                );
                Err(ThreadSyncError::Conflict)
            }
            status if status.is_server_error() => {
                let message = response.text().await.unwrap_or_default();
                Err(ThreadSyncError::Server { status, message })
            }
            status => Err(ThreadSyncError::UnexpectedStatus(status)),
        }
    }

    pub async fn get_thread(&self, id: &ThreadId) -> Result<Option<Thread>, ThreadSyncError> {
        let url = self.thread_url(id)?;

        debug!(thread_id = %id, url = %url, "fetching thread from server");

        let response = loom_http_retry::retry(&self.retry_config, || async {
            self.http
                .get(url.clone())
                .send()
                .await
                .map_err(ThreadSyncError::from)
        })
        .await?;

        match response.status() {
            StatusCode::OK => {
                let thread: Thread = response.json().await.map_err(ThreadSyncError::Network)?;
                debug!(
                    thread_id = %id,
                    version = thread.version,
                    "fetched thread from server"
                );
                Ok(Some(thread))
            }
            StatusCode::NOT_FOUND => {
                debug!(thread_id = %id, "thread not found on server");
                Ok(None)
            }
            status if status.is_server_error() => {
                let message = response.text().await.unwrap_or_default();
                Err(ThreadSyncError::Server { status, message })
            }
            status => Err(ThreadSyncError::UnexpectedStatus(status)),
        }
    }

    pub async fn list_threads(&self, limit: u32) -> Result<Vec<ThreadSummary>, ThreadSyncError> {
        let mut url = self.threads_url()?;
        url.query_pairs_mut()
            .append_pair("limit", &limit.to_string());

        debug!(url = %url, limit = limit, "listing threads from server");

        let response = loom_http_retry::retry(&self.retry_config, || async {
            self.http
                .get(url.clone())
                .send()
                .await
                .map_err(ThreadSyncError::from)
        })
        .await?;

        match response.status() {
            StatusCode::OK => {
                let summaries: Vec<ThreadSummary> =
                    response.json().await.map_err(ThreadSyncError::Network)?;
                debug!(count = summaries.len(), "listed threads from server");
                Ok(summaries)
            }
            status if status.is_server_error() => {
                let message = response.text().await.unwrap_or_default();
                Err(ThreadSyncError::Server { status, message })
            }
            status => Err(ThreadSyncError::UnexpectedStatus(status)),
        }
    }

    pub async fn delete_thread(&self, id: &ThreadId) -> Result<(), ThreadSyncError> {
        let url = self.thread_url(id)?;

        debug!(thread_id = %id, url = %url, "deleting thread from server");

        let response = loom_http_retry::retry(&self.retry_config, || async {
            self.http
                .delete(url.clone())
                .send()
                .await
                .map_err(ThreadSyncError::from)
        })
        .await?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => {
                info!(thread_id = %id, "deleted thread from server");
                Ok(())
            }
            StatusCode::NOT_FOUND => {
                debug!(thread_id = %id, "thread not found on server for deletion");
                Ok(())
            }
            status if status.is_server_error() => {
                let message = response.text().await.unwrap_or_default();
                Err(ThreadSyncError::Server { status, message })
            }
            status => Err(ThreadSyncError::UnexpectedStatus(status)),
        }
    }
}

pub struct SyncingThreadStore {
    local: LocalThreadStore,
    sync_client: Option<ThreadSyncClient>,
}

impl SyncingThreadStore {
    pub fn new(local: LocalThreadStore, sync_client: Option<ThreadSyncClient>) -> Self {
        Self { local, sync_client }
    }

    pub fn local_only(local: LocalThreadStore) -> Self {
        Self::new(local, None)
    }

    pub fn with_sync(local: LocalThreadStore, sync_client: ThreadSyncClient) -> Self {
        Self::new(local, Some(sync_client))
    }
}

#[async_trait]
impl ThreadStore for SyncingThreadStore {
    async fn load(&self, id: &ThreadId) -> Result<Option<Thread>, ThreadStoreError> {
        self.local.load(id).await
    }

    async fn save(&self, thread: &Thread) -> Result<(), ThreadStoreError> {
        self.local.save(thread).await?;

        if let Some(sync_client) = &self.sync_client {
            let thread_clone = thread.clone();
            let sync_client_base_url = sync_client.base_url.clone();
            let http_clone = sync_client.http.clone();
            let retry_config = sync_client.retry_config.clone();

            tokio::spawn(async move {
                let client = ThreadSyncClient {
                    base_url: sync_client_base_url,
                    http: http_clone,
                    retry_config,
                };

                match client.upsert_thread(&thread_clone).await {
                    Ok(()) => {
                        debug!(
                            thread_id = %thread_clone.id,
                            "background sync completed"
                        );
                    }
                    Err(e) => {
                        error!(
                            thread_id = %thread_clone.id,
                            error = %e,
                            "background sync failed"
                        );
                    }
                }
            });
        }

        Ok(())
    }

    async fn list(&self, limit: u32) -> Result<Vec<ThreadSummary>, ThreadStoreError> {
        self.local.list(limit).await
    }

    async fn delete(&self, id: &ThreadId) -> Result<(), ThreadStoreError> {
        self.local.delete(id).await?;

        if let Some(sync_client) = &self.sync_client {
            let id_clone = id.clone();
            let sync_client_base_url = sync_client.base_url.clone();
            let http_clone = sync_client.http.clone();
            let retry_config = sync_client.retry_config.clone();

            tokio::spawn(async move {
                let client = ThreadSyncClient {
                    base_url: sync_client_base_url,
                    http: http_clone,
                    retry_config,
                };

                match client.delete_thread(&id_clone).await {
                    Ok(()) => {
                        debug!(
                            thread_id = %id_clone,
                            "background delete sync completed"
                        );
                    }
                    Err(e) => {
                        error!(
                            thread_id = %id_clone,
                            error = %e,
                            "background delete sync failed"
                        );
                    }
                }
            });
        }

        Ok(())
    }
}
