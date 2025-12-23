// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use std::path::Path;

use async_trait::async_trait;

use crate::error::AutoCommitError;

/// Git diff information.
#[derive(Clone, Debug, Default)]
pub struct GitDiff {
	/// The raw diff content.
	pub content: String,
	/// List of files changed in the diff.
	pub files_changed: Vec<String>,
}

impl GitDiff {
	pub fn is_empty(&self) -> bool {
		self.content.is_empty()
	}
}

/// Trait for git operations required by auto-commit.
#[async_trait]
pub trait GitClient: Send + Sync {
	/// Check if the given path is inside a git repository.
	async fn is_repository(&self, path: &Path) -> bool;

	/// Get the diff of all uncommitted changes (staged and unstaged).
	async fn diff_all(&self, path: &Path) -> Result<GitDiff, AutoCommitError>;

	/// Stage all changes in the repository.
	async fn stage_all(&self, path: &Path) -> Result<(), AutoCommitError>;

	/// Create a commit with the given message.
	/// Returns the commit hash.
	async fn commit(&self, path: &Path, message: &str) -> Result<String, AutoCommitError>;
}
