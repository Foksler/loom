// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::error::Result;
use crate::pull::check_repo_exists;
use crate::types::ExternalMirror;

#[async_trait]
pub trait ExternalMirrorStore: Send + Sync {
	async fn get_by_id(&self, id: Uuid) -> Result<Option<ExternalMirror>>;
	async fn get_by_repo_id(&self, repo_id: Uuid) -> Result<Option<ExternalMirror>>;
	async fn find_stale(&self, stale_threshold: DateTime<Utc>) -> Result<Vec<ExternalMirror>>;
	async fn delete(&self, id: Uuid) -> Result<()>;
	async fn update_last_accessed(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;
	async fn update_last_synced(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupDecision {
	Deleted,
	Kept,
	RemoteGone,
	Error,
}

#[derive(Debug)]
pub struct CleanupResult {
	pub mirror_id: Uuid,
	pub decision: CleanupDecision,
	pub reason: String,
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
pub async fn cleanup_mirror_with_check(
	mirror: &ExternalMirror,
	repo_path: &Path,
	store: &impl ExternalMirrorStore,
	delete_if_stale: bool,
) -> CleanupResult {
	let remote_exists = match check_repo_exists(
		mirror.platform,
		&mirror.external_owner,
		&mirror.external_repo,
	)
	.await
	{
		Ok(exists) => exists,
		Err(e) => {
			warn!(
				mirror_id = %mirror.id,
				error = %e,
				"Failed to check if remote exists, keeping mirror"
			);
			return CleanupResult {
				mirror_id: mirror.id,
				decision: CleanupDecision::Error,
				reason: format!("Failed to check remote: {}", e),
			};
		}
	};

	if !remote_exists {
		info!(
			mirror_id = %mirror.id,
			platform = ?mirror.platform,
			owner = %mirror.external_owner,
			repo = %mirror.external_repo,
			"Remote repository no longer exists (404), deleting mirror"
		);

		if let Err(e) = delete_mirror(mirror, repo_path, store).await {
			return CleanupResult {
				mirror_id: mirror.id,
				decision: CleanupDecision::Error,
				reason: format!("Failed to delete mirror: {}", e),
			};
		}

		return CleanupResult {
			mirror_id: mirror.id,
			decision: CleanupDecision::RemoteGone,
			reason: "Remote repository deleted (404)".to_string(),
		};
	}

	if delete_if_stale {
		info!(
			mirror_id = %mirror.id,
			platform = ?mirror.platform,
			owner = %mirror.external_owner,
			repo = %mirror.external_repo,
			"Deleting stale mirror (remote still exists but configured to delete)"
		);

		if let Err(e) = delete_mirror(mirror, repo_path, store).await {
			return CleanupResult {
				mirror_id: mirror.id,
				decision: CleanupDecision::Error,
				reason: format!("Failed to delete mirror: {}", e),
			};
		}

		return CleanupResult {
			mirror_id: mirror.id,
			decision: CleanupDecision::Deleted,
			reason: "Stale mirror deleted (configured to delete even if remote exists)".to_string(),
		};
	}

	info!(
		mirror_id = %mirror.id,
		platform = ?mirror.platform,
		owner = %mirror.external_owner,
		repo = %mirror.external_repo,
		"Keeping stale mirror (remote still exists)"
	);

	CleanupResult {
		mirror_id: mirror.id,
		decision: CleanupDecision::Kept,
		reason: "Remote still exists, keeping mirror".to_string(),
	}
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

pub async fn touch_mirror(store: &impl ExternalMirrorStore, id: Uuid) -> Result<()> {
	store.update_last_accessed(id, Utc::now()).await
}

pub async fn run_cleanup_job(
	store: &impl ExternalMirrorStore,
	stale_after: Duration,
	mirrors_base_path: &Path,
	delete_if_stale: bool,
) -> Vec<CleanupResult> {
	let stale_mirrors = match find_stale_mirrors(store, stale_after).await {
		Ok(mirrors) => mirrors,
		Err(e) => {
			warn!(error = %e, "Failed to find stale mirrors");
			return vec![];
		}
	};

	info!(count = stale_mirrors.len(), "Found stale mirrors for cleanup");

	let mut results = Vec::new();
	for mirror in &stale_mirrors {
		let repo_path = mirrors_base_path
			.join(mirror.platform.as_str())
			.join(&mirror.external_owner)
			.join(&mirror.external_repo);

		let result = cleanup_mirror_with_check(mirror, &repo_path, store, delete_if_stale).await;

		info!(
			mirror_id = %result.mirror_id,
			decision = ?result.decision,
			reason = %result.reason,
			"Cleanup decision made"
		);

		results.push(result);
	}

	results
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_cleanup_decision_variants() {
		assert_eq!(CleanupDecision::Deleted, CleanupDecision::Deleted);
		assert_eq!(CleanupDecision::Kept, CleanupDecision::Kept);
		assert_eq!(CleanupDecision::RemoteGone, CleanupDecision::RemoteGone);
		assert_eq!(CleanupDecision::Error, CleanupDecision::Error);
		assert_ne!(CleanupDecision::Deleted, CleanupDecision::Kept);
	}

	#[test]
	fn test_cleanup_result_structure() {
		let result = CleanupResult {
			mirror_id: uuid::Uuid::new_v4(),
			decision: CleanupDecision::Kept,
			reason: "Remote still exists".to_string(),
		};

		assert_eq!(result.decision, CleanupDecision::Kept);
		assert!(result.reason.contains("exists"));
	}
}
