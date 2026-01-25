// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git mirror handling for on-demand external mirrors.

use loom_server_scm::{OwnerType, RepoStore, Repository, Visibility};
use loom_server_scm_mirror::{CreateExternalMirror, ExternalMirrorStore, Platform};
use tracing::{info, warn};

use crate::{api::AppState, error::ServerError, i18n::t};

use super::common::get_repo_path_by_id;

#[derive(Debug, Clone)]
pub struct MirrorInfo {
	pub platform: Platform,
	pub external_owner: String,
	pub external_repo: String,
}

pub fn parse_mirror_path(owner: &str, repo_name: &str) -> Option<MirrorInfo> {
	let repo_name = repo_name.strip_suffix(".git").unwrap_or(repo_name);

	let parts: Vec<&str> = owner.split('/').collect();
	if parts.len() != 3 {
		return None;
	}

	if parts[0] != "mirrors" {
		return None;
	}

	let platform = Platform::parse(parts[1])?;
	let external_owner = parts[2].to_string();
	let external_repo = repo_name.to_string();

	Some(MirrorInfo {
		platform,
		external_owner,
		external_repo,
	})
}

pub fn is_mirror_path(owner: &str) -> bool {
	owner.starts_with("mirrors/")
}

pub fn parse_mirror_git_path(path: &str) -> Option<(String, String)> {
	let path = path.strip_prefix('/').unwrap_or(path);

	let suffix_patterns = ["/info/refs", "/git-upload-pack", "/git-receive-pack"];
	for suffix in suffix_patterns {
		if let Some(repo_path) = path.strip_suffix(suffix) {
			if let Some(last_slash) = repo_path.rfind('/') {
				let platform_and_owner = &repo_path[..last_slash];
				let repo = &repo_path[last_slash + 1..];
				if !platform_and_owner.is_empty() && !repo.is_empty() {
					let owner = format!("mirrors/{}", platform_and_owner);
					return Some((owner, repo.to_string()));
				}
			}
		}
	}
	None
}

pub async fn create_on_demand_mirror(
	mirror_info: &MirrorInfo,
	state: &AppState,
	locale: &str,
) -> Result<Repository, ServerError> {
	let external_mirror_store = state
		.external_mirror_store
		.as_ref()
		.ok_or_else(|| ServerError::Internal(t(locale, "server.api.scm.not_configured").to_string()))?;

	let scm_store = state
		.scm_repo_store
		.as_ref()
		.ok_or_else(|| ServerError::Internal(t(locale, "server.api.scm.not_configured").to_string()))?;

	if let Ok(Some(existing)) = external_mirror_store
		.get_by_external(
			mirror_info.platform,
			&mirror_info.external_owner,
			&mirror_info.external_repo,
		)
		.await
	{
		if let Ok(Some(repo)) = scm_store.get_by_id(existing.repo_id).await {
			return Ok(repo);
		}
	}

	info!(
		platform = ?mirror_info.platform,
		owner = %mirror_info.external_owner,
		repo = %mirror_info.external_repo,
		"Checking if remote repository exists"
	);

	if !loom_server_scm_mirror::check_repo_exists(
		mirror_info.platform,
		&mirror_info.external_owner,
		&mirror_info.external_repo,
	)
	.await
	.map_err(|e| ServerError::Internal(format!("Failed to check remote: {e}")))?
	{
		return Err(ServerError::NotFound(
			t(locale, "server.api.scm.mirror.remote_not_found").to_string(),
		));
	}

	let mirrors_org = state
		.org_repo
		.get_org_by_slug("mirrors")
		.await?
		.ok_or_else(|| {
			ServerError::Internal(t(locale, "server.api.scm.mirror.mirrors_org_not_found").to_string())
		})?;

	let repo_name = format!(
		"{}-{}-{}",
		mirror_info.platform.as_str(),
		mirror_info.external_owner,
		mirror_info.external_repo
	);

	let repo = Repository::new(
		OwnerType::Org,
		mirrors_org.id.into_inner(),
		repo_name,
		Visibility::Public,
	);

	let repo = scm_store
		.create(&repo)
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to create repo: {e}")))?;

	info!(
		repo_id = %repo.id,
		platform = ?mirror_info.platform,
		owner = %mirror_info.external_owner,
		repo_name = %mirror_info.external_repo,
		"Created repository for on-demand mirror"
	);

	let create_mirror = CreateExternalMirror {
		platform: mirror_info.platform,
		external_owner: mirror_info.external_owner.clone(),
		external_repo: mirror_info.external_repo.clone(),
		repo_id: repo.id,
	};

	external_mirror_store
		.create(&create_mirror)
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to create mirror entry: {e}")))?;

	info!(
		repo_id = %repo.id,
		platform = ?mirror_info.platform,
		owner = %mirror_info.external_owner,
		repo_name = %mirror_info.external_repo,
		"Created external mirror entry"
	);

	let repo_path = get_repo_path_by_id(repo.id);

	info!(
		repo_id = %repo.id,
		path = ?repo_path,
		platform = ?mirror_info.platform,
		owner = %mirror_info.external_owner,
		repo_name = %mirror_info.external_repo,
		"Starting on-demand mirror clone"
	);

	loom_server_scm_mirror::pull_mirror(
		mirror_info.platform,
		&mirror_info.external_owner,
		&mirror_info.external_repo,
		&repo_path,
	)
	.await
	.map_err(|e| {
		warn!(
			repo_id = %repo.id,
			error = %e,
			"On-demand mirror clone failed"
		);
		ServerError::Internal(t(locale, "server.api.scm.mirror.clone_failed").to_string())
	})?;

	if let Some(store) = &state.external_mirror_store {
		if let Ok(Some(mirror)) = store.get_by_repo_id(repo.id).await {
			let _ = store
				.update_last_synced(mirror.id, chrono::Utc::now())
				.await;
		}
	}

	info!(
		repo_id = %repo.id,
		platform = ?mirror_info.platform,
		owner = %mirror_info.external_owner,
		repo_name = %mirror_info.external_repo,
		"On-demand mirror clone completed successfully"
	);

	Ok(repo)
}
