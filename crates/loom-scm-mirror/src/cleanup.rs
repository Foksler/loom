// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::{info, instrument, warn};

use crate::error::Result;
use crate::types::ExternalMirror;

#[async_trait]
pub trait ExternalMirrorStore: Send + Sync {
	async fn find_stale(&self, stale_threshold: DateTime<Utc>) -> Result<Vec<ExternalMirror>>;
	async fn delete(&self, id: uuid::Uuid) -> Result<()>;
	async fn update_last_accessed(&self, id: uuid::Uuid, at: DateTime<Utc>) -> Result<()>;
}

#[instrument(skip(store))]
pub async fn find_stale_mirrors(
	store: &impl ExternalMirrorStore,
	stale_after: Duration,
) -> Result<Vec<ExternalMirror>> {
	let threshold = Utc::now() - chrono::Duration::from_std(stale_after).unwrap_or_default();
	store.find_stale(threshold).await
}

#[instrument(skip(store), fields(mirror_id = %mirror.id, platform = ?mirror.platform))]
pub async fn delete_mirror(
	mirror: &ExternalMirror,
	repo_path: &Path,
	store: &impl ExternalMirrorStore,
) -> Result<()> {
	if repo_path.exists() {
		info!(path = ?repo_path, "Removing mirror repository from disk");
		if let Err(e) = std::fs::remove_dir_all(repo_path) {
			warn!(path = ?repo_path, error = %e, "Failed to remove repository directory");
		}
	}

	store.delete(mirror.id).await?;

	info!(
		platform = ?mirror.platform,
		owner = %mirror.external_owner,
		repo = %mirror.external_repo,
		"External mirror deleted"
	);

	Ok(())
}

pub async fn touch_mirror(store: &impl ExternalMirrorStore, id: uuid::Uuid) -> Result<()> {
	store.update_last_accessed(id, Utc::now()).await
}
