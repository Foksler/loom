// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

// NOTE: Push operations use git subprocess because gitoxide doesn't yet support push.
// See: https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md
// Track progress at: https://github.com/GitoxideLabs/gitoxide/issues/307

use std::path::Path;
use std::process::Command;

use loom_credentials::{CredentialStore, CredentialValue};
use tracing::{debug, error, info, instrument};

use crate::error::{MirrorError, Result};
use crate::types::{MirrorBranchRule, PushMirror};

#[instrument(skip(credentials), fields(mirror_id = %mirror.id, repo_id = %mirror.repo_id))]
pub async fn push_mirror(
	repo_path: &Path,
	mirror: &PushMirror,
	credentials: &impl CredentialStore,
	branch_rules: &[MirrorBranchRule],
) -> Result<()> {
	if !mirror.enabled {
		debug!("Mirror is disabled, skipping");
		return Ok(());
	}

	let creds = credentials
		.load(&mirror.credential_key)
		.await
		.map_err(|e| MirrorError::CredentialNotFound(e.to_string()))?
		.ok_or_else(|| MirrorError::CredentialNotFound(mirror.credential_key.clone()))?;

	let remote_url = build_authenticated_url(&mirror.remote_url, &creds)?;

	let active_rules: Vec<_> = branch_rules.iter().filter(|r| r.enabled).collect();

	if active_rules.is_empty() {
		push_all_refs(repo_path, &remote_url).await?;
	} else {
		push_matching_refs(repo_path, &remote_url, &active_rules).await?;
	}

	info!(mirror_id = %mirror.id, "Push mirror completed");
	Ok(())
}

fn build_authenticated_url(remote_url: &str, creds: &CredentialValue) -> Result<String> {
	let (username, password) = match creds {
		CredentialValue::ApiKey { key } => ("git", key.expose().to_string()),
		CredentialValue::OAuth { access, .. } => ("oauth2", access.expose().to_string()),
	};

	let url = if remote_url.starts_with("https://") {
		let without_scheme = remote_url.strip_prefix("https://").unwrap();
		format!("https://{}:{}@{}", username, password, without_scheme)
	} else if remote_url.starts_with("http://") {
		let without_scheme = remote_url.strip_prefix("http://").unwrap();
		format!("http://{}:{}@{}", username, password, without_scheme)
	} else {
		return Err(MirrorError::InvalidUrl(format!(
			"unsupported URL scheme: {}",
			remote_url
		)));
	};

	Ok(url)
}

async fn push_all_refs(repo_path: &Path, remote_url: &str) -> Result<()> {
	debug!("Pushing all refs with --mirror");

	let output = Command::new("git")
		.args(["push", "--mirror", remote_url])
		.current_dir(repo_path)
		.output()?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		error!(error = %stderr, "Git push --mirror failed");
		return Err(MirrorError::GitError(stderr.to_string()));
	}

	Ok(())
}

async fn push_matching_refs(
	repo_path: &Path,
	remote_url: &str,
	rules: &[&MirrorBranchRule],
) -> Result<()> {
	let branches = list_branches(repo_path)?;

	let matching_branches: Vec<_> = branches
		.iter()
		.filter(|branch| rules.iter().any(|rule| matches_pattern(branch, &rule.pattern)))
		.collect();

	if matching_branches.is_empty() {
		debug!("No branches match the mirror rules");
		return Ok(());
	}

	for branch in matching_branches {
		debug!(branch = %branch, "Pushing branch");
		let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);

		let output = Command::new("git")
			.args(["push", remote_url, &refspec])
			.current_dir(repo_path)
			.output()?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			error!(branch = %branch, error = %stderr, "Git push failed");
			return Err(MirrorError::GitError(stderr.to_string()));
		}
	}

	Ok(())
}

fn list_branches(repo_path: &Path) -> Result<Vec<String>> {
	let repo = gix::open(repo_path)
		.map_err(|e| MirrorError::GitError(format!("Failed to open repo: {}", e)))?;

	let refs = repo
		.references()
		.map_err(|e| MirrorError::GitError(format!("Failed to get refs: {}", e)))?;

	let branches = refs
		.prefixed("refs/heads/")
		.map_err(|e| MirrorError::GitError(format!("Failed to list branches: {}", e)))?;

	let mut result = Vec::new();
	for r in branches {
		if let Ok(reference) = r {
			let name = reference.name().shorten().to_string();
			result.push(name);
		}
	}

	Ok(result)
}

fn matches_pattern(branch: &str, pattern: &str) -> bool {
	if pattern == "*" {
		return true;
	}

	if let Some(prefix) = pattern.strip_suffix("/*") {
		return branch.starts_with(&format!("{}/", prefix));
	}

	branch == pattern
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_matches_pattern_exact() {
		assert!(matches_pattern("cannon", "cannon"));
		assert!(!matches_pattern("main", "cannon"));
	}

	#[test]
	fn test_matches_pattern_wildcard() {
		assert!(matches_pattern("release/v1.0", "release/*"));
		assert!(matches_pattern("release/v2.0-beta", "release/*"));
		assert!(!matches_pattern("feature/foo", "release/*"));
	}

	#[test]
	fn test_matches_pattern_all() {
		assert!(matches_pattern("any-branch", "*"));
		assert!(matches_pattern("feature/foo", "*"));
	}
}
