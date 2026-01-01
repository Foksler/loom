// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use std::path::Path;
use std::process::Command;

use tracing::{debug, error, info, instrument};

use crate::error::{MirrorError, Result};
use crate::types::Platform;

pub fn get_clone_url(platform: Platform, owner: &str, repo: &str) -> String {
	match platform {
		Platform::GitHub => format!("https://github.com/{}/{}.git", owner, repo),
		Platform::GitLab => format!("https://gitlab.com/{}/{}.git", owner, repo),
	}
}

#[instrument(fields(platform = ?platform, owner = %owner, repo = %repo))]
pub async fn pull_mirror(
	platform: Platform,
	owner: &str,
	repo: &str,
	target_path: &Path,
) -> Result<()> {
	let clone_url = get_clone_url(platform, owner, repo);

	if target_path.exists() {
		fetch_updates(target_path, &clone_url).await
	} else {
		clone_bare(target_path, &clone_url).await
	}
}

async fn clone_bare(target_path: &Path, clone_url: &str) -> Result<()> {
	info!(url = %clone_url, path = ?target_path, "Cloning bare repository");

	if let Some(parent) = target_path.parent() {
		std::fs::create_dir_all(parent)?;
	}

	let output = Command::new("git")
		.args(["clone", "--bare", "--mirror", clone_url])
		.arg(target_path)
		.output()?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		error!(error = %stderr, "Git clone failed");
		return Err(MirrorError::GitError(stderr.to_string()));
	}

	debug!("Clone completed successfully");
	Ok(())
}

async fn fetch_updates(target_path: &Path, clone_url: &str) -> Result<()> {
	info!(url = %clone_url, path = ?target_path, "Fetching updates");

	let output = Command::new("git")
		.args(["remote", "set-url", "origin", clone_url])
		.current_dir(target_path)
		.output()?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		return Err(MirrorError::GitError(format!(
			"Failed to set remote URL: {}",
			stderr
		)));
	}

	let output = Command::new("git")
		.args(["fetch", "--prune", "origin", "+refs/*:refs/*"])
		.current_dir(target_path)
		.output()?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		error!(error = %stderr, "Git fetch failed");
		return Err(MirrorError::GitError(stderr.to_string()));
	}

	debug!("Fetch completed successfully");
	Ok(())
}

pub async fn check_repo_exists(platform: Platform, owner: &str, repo: &str) -> Result<bool> {
	let url = match platform {
		Platform::GitHub => format!("https://api.github.com/repos/{}/{}", owner, repo),
		Platform::GitLab => format!(
			"https://gitlab.com/api/v4/projects/{}%2F{}",
			owner, repo
		),
	};

	let client = loom_http::new_client();
	let response = client.get(&url).send().await?;

	Ok(response.status().is_success())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_clone_url_github() {
		let url = get_clone_url(Platform::GitHub, "torvalds", "linux");
		assert_eq!(url, "https://github.com/torvalds/linux.git");
	}

	#[test]
	fn test_get_clone_url_gitlab() {
		let url = get_clone_url(Platform::GitLab, "gitlab-org", "gitlab");
		assert_eq!(url, "https://gitlab.com/gitlab-org/gitlab.git");
	}
}
