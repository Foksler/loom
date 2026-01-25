// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Git bare repository management for clips.
//!
//! Each clip is stored as a bare git repository, enabling:
//! - Version history for clip content
//! - Git-based clone/push operations
//! - Efficient storage and deduplication

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tracing::{info, instrument, warn};

use crate::error::{ClipsError, Result};
use crate::types::{ClipFile, ClipId};

/// Git store for clip repositories.
pub struct ClipsGitStore {
	/// Base directory for clip repositories.
	base_dir: PathBuf,
}

impl ClipsGitStore {
	/// Create a new clips git store.
	pub fn new(base_dir: PathBuf) -> Self {
		Self { base_dir }
	}

	/// Get the path to a clip's bare repository.
	pub fn get_clip_path(&self, clip_id: ClipId) -> PathBuf {
		let id_str = clip_id.0.to_string();
		let prefix = &id_str[..2];
		self.base_dir.join("clips").join(prefix).join(format!("{}.git", id_str))
	}

	/// Initialize a new bare repository for a clip.
	#[instrument(skip(self), fields(clip_id = %clip_id))]
	pub async fn init_repo(&self, clip_id: ClipId) -> Result<PathBuf> {
		let repo_path = self.get_clip_path(clip_id);

		if repo_path.exists() {
			warn!(%clip_id, "Clip repository already exists");
			return Ok(repo_path);
		}

		if let Some(parent) = repo_path.parent() {
			fs::create_dir_all(parent).await?;
		}

		let output = Command::new("git")
			.args(["init", "--bare"])
			.arg(&repo_path)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			return Err(ClipsError::Git(format!("git init failed: {}", stderr)));
		}

		info!(%clip_id, path = ?repo_path, "Initialized clip repository");
		Ok(repo_path)
	}

	/// Delete a clip's repository.
	#[instrument(skip(self), fields(clip_id = %clip_id))]
	pub async fn delete_repo(&self, clip_id: ClipId) -> Result<()> {
		let repo_path = self.get_clip_path(clip_id);

		if repo_path.exists() {
			fs::remove_dir_all(&repo_path).await?;
			info!(%clip_id, "Deleted clip repository");
		}

		Ok(())
	}

	/// Check if a clip repository exists.
	pub async fn repo_exists(&self, clip_id: ClipId) -> bool {
		self.get_clip_path(clip_id).exists()
	}

	/// List files in a clip at a given ref (default: HEAD).
	#[instrument(skip(self), fields(clip_id = %clip_id))]
	pub async fn list_files(&self, clip_id: ClipId, git_ref: Option<&str>) -> Result<Vec<String>> {
		let repo_path = self.get_clip_path(clip_id);
		let tree_ref = git_ref.unwrap_or("HEAD");

		let output = Command::new("git")
			.args(["--git-dir", repo_path.to_str().unwrap_or_default()])
			.args(["ls-tree", "-r", "--name-only", tree_ref])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			if stderr.contains("Not a valid object name") {
				return Ok(vec![]);
			}
			return Err(ClipsError::Git(format!("git ls-tree failed: {}", stderr)));
		}

		let stdout = String::from_utf8_lossy(&output.stdout);
		Ok(stdout.lines().map(|s| s.to_string()).collect())
	}

	/// Read a file from a clip, with secret redaction.
	#[instrument(skip(self), fields(clip_id = %clip_id, path = %path))]
	pub async fn read_file_redacted(
		&self,
		clip_id: ClipId,
		path: &str,
		git_ref: Option<&str>,
	) -> Result<ClipFile> {
		let repo_path = self.get_clip_path(clip_id);
		let tree_ref = git_ref.unwrap_or("HEAD");
		let object_path = format!("{}:{}", tree_ref, path);

		let output = Command::new("git")
			.args(["--git-dir", repo_path.to_str().unwrap_or_default()])
			.args(["show", &object_path])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			return Err(ClipsError::Git(format!("git show failed: {}", stderr)));
		}

		let raw_content = String::from_utf8_lossy(&output.stdout).to_string();
		let size_bytes = raw_content.len() as u64;

		// Apply secret redaction
		let redacted = loom_redact::redact(&raw_content);
		let is_redacted = matches!(redacted, std::borrow::Cow::Owned(_));
		let content = redacted.into_owned();

		// Detect language from file extension
		let language = detect_language(path);

		Ok(ClipFile {
			path: path.to_string(),
			content,
			size_bytes,
			is_redacted,
			language,
		})
	}

	/// Read a file from a clip without redaction (for trusted contexts).
	#[instrument(skip(self), fields(clip_id = %clip_id, path = %path))]
	pub async fn read_file_raw(
		&self,
		clip_id: ClipId,
		path: &str,
		git_ref: Option<&str>,
	) -> Result<Vec<u8>> {
		let repo_path = self.get_clip_path(clip_id);
		let tree_ref = git_ref.unwrap_or("HEAD");
		let object_path = format!("{}:{}", tree_ref, path);

		let output = Command::new("git")
			.args(["--git-dir", repo_path.to_str().unwrap_or_default()])
			.args(["show", &object_path])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			return Err(ClipsError::Git(format!("git show failed: {}", stderr)));
		}

		Ok(output.stdout)
	}

	/// Get the total size of a clip.
	#[instrument(skip(self), fields(clip_id = %clip_id))]
	pub async fn get_clip_size(&self, clip_id: ClipId) -> Result<u64> {
		let files = self.list_files(clip_id, None).await?;
		let mut total_size = 0u64;

		for file in files {
			if let Ok(file_data) = self.read_file_raw(clip_id, &file, None).await {
				total_size += file_data.len() as u64;
			}
		}

		Ok(total_size)
	}

	/// Commit files to a clip repository.
	/// Creates an initial commit with the given files.
	#[instrument(skip(self, files), fields(clip_id = %clip_id, file_count = files.len()))]
	pub async fn commit_files(
		&self,
		clip_id: ClipId,
		files: &[(String, String)], // (path, content)
		author_name: &str,
		author_email: &str,
		message: &str,
	) -> Result<String> {
		let repo_path = self.get_clip_path(clip_id);

		// Create a temporary directory for the work tree
		let temp_dir = tempfile::tempdir()?;
		let work_tree = temp_dir.path();

		// Write files to the work tree
		for (path, content) in files {
			let file_path = work_tree.join(path);
			if let Some(parent) = file_path.parent() {
				fs::create_dir_all(parent).await?;
			}
			fs::write(&file_path, content).await?;
		}

		// Add all files
		let output = Command::new("git")
			.args(["--git-dir", repo_path.to_str().unwrap_or_default()])
			.args(["--work-tree", work_tree.to_str().unwrap_or_default()])
			.args(["add", "-A"])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			return Err(ClipsError::Git(format!("git add failed: {}", stderr)));
		}

		// Create commit
		let output = Command::new("git")
			.args(["--git-dir", repo_path.to_str().unwrap_or_default()])
			.args(["--work-tree", work_tree.to_str().unwrap_or_default()])
			.args(["commit", "-m", message])
			.args(["--author", &format!("{} <{}>", author_name, author_email)])
			.env("GIT_COMMITTER_NAME", author_name)
			.env("GIT_COMMITTER_EMAIL", author_email)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			return Err(ClipsError::Git(format!("git commit failed: {}", stderr)));
		}

		// Get the commit hash
		let output = Command::new("git")
			.args(["--git-dir", repo_path.to_str().unwrap_or_default()])
			.args(["rev-parse", "HEAD"])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			return Err(ClipsError::Git(format!("git rev-parse failed: {}", stderr)));
		}

		let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
		info!(%clip_id, %commit_hash, "Committed files to clip");

		Ok(commit_hash)
	}

	/// Clone a clip repository to create a fork.
	#[instrument(skip(self), fields(source_clip_id = %source_clip_id, target_clip_id = %target_clip_id))]
	pub async fn clone_repo(&self, source_clip_id: ClipId, target_clip_id: ClipId) -> Result<()> {
		let source_path = self.get_clip_path(source_clip_id);
		let target_path = self.get_clip_path(target_clip_id);

		if let Some(parent) = target_path.parent() {
			fs::create_dir_all(parent).await?;
		}

		let output = Command::new("git")
			.args(["clone", "--bare"])
			.arg(&source_path)
			.arg(&target_path)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			return Err(ClipsError::Git(format!("git clone failed: {}", stderr)));
		}

		info!(%source_clip_id, %target_clip_id, "Cloned clip repository");
		Ok(())
	}

	/// Get the latest commit hash for a clip.
	#[instrument(skip(self), fields(clip_id = %clip_id))]
	pub async fn get_head_commit(&self, clip_id: ClipId) -> Result<Option<String>> {
		let repo_path = self.get_clip_path(clip_id);

		let output = Command::new("git")
			.args(["--git-dir", repo_path.to_str().unwrap_or_default()])
			.args(["rev-parse", "HEAD"])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			return Ok(None);
		}

		let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
		if hash.is_empty() {
			Ok(None)
		} else {
			Ok(Some(hash))
		}
	}

	/// Get the default branch name for a clip.
	#[instrument(skip(self), fields(clip_id = %clip_id))]
	pub async fn get_default_branch(&self, clip_id: ClipId) -> Result<Option<String>> {
		let repo_path = self.get_clip_path(clip_id);

		let output = Command::new("git")
			.args(["--git-dir", repo_path.to_str().unwrap_or_default()])
			.args(["symbolic-ref", "--short", "HEAD"])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			return Ok(None);
		}

		let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
		if branch.is_empty() {
			Ok(None)
		} else {
			Ok(Some(branch))
		}
	}

	/// List commits (revisions) for a clip.
	#[instrument(skip(self), fields(clip_id = %clip_id))]
	pub async fn list_commits(
		&self,
		clip_id: ClipId,
		limit: u32,
	) -> Result<Vec<crate::types::ClipRevision>> {
		let repo_path = self.get_clip_path(clip_id);

		// Use git log with a specific format to parse commits
		let output = Command::new("git")
			.args(["--git-dir", repo_path.to_str().unwrap_or_default()])
			.args([
				"log",
				&format!("-{}", limit),
				"--format=%H%x00%an%x00%ae%x00%aI%x00%s",
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			if stderr.contains("does not have any commits") {
				return Ok(vec![]);
			}
			return Err(ClipsError::Git(format!("git log failed: {}", stderr)));
		}

		let stdout = String::from_utf8_lossy(&output.stdout);
		let mut revisions = Vec::new();

		for line in stdout.lines() {
			let parts: Vec<&str> = line.split('\0').collect();
			if parts.len() == 5 {
				revisions.push(crate::types::ClipRevision {
					sha: parts[0].to_string(),
					author_name: parts[1].to_string(),
					author_email: parts[2].to_string(),
					timestamp: parts[3].to_string(),
					message: parts[4].to_string(),
				});
			}
		}

		Ok(revisions)
	}
}

/// Detect language from file extension.
fn detect_language(path: &str) -> Option<String> {
	let ext = Path::new(path).extension()?.to_str()?;
	match ext.to_lowercase().as_str() {
		"rs" => Some("rust"),
		"js" | "jsx" | "mjs" => Some("javascript"),
		"ts" | "tsx" | "mts" => Some("typescript"),
		"py" => Some("python"),
		"go" => Some("go"),
		"java" => Some("java"),
		"c" | "h" => Some("c"),
		"cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
		"cs" => Some("csharp"),
		"rb" => Some("ruby"),
		"php" => Some("php"),
		"swift" => Some("swift"),
		"kt" | "kts" => Some("kotlin"),
		"scala" => Some("scala"),
		"sh" | "bash" => Some("shell"),
		"sql" => Some("sql"),
		"html" | "htm" => Some("html"),
		"css" => Some("css"),
		"scss" | "sass" => Some("scss"),
		"less" => Some("less"),
		"json" => Some("json"),
		"yaml" | "yml" => Some("yaml"),
		"toml" => Some("toml"),
		"xml" => Some("xml"),
		"md" | "markdown" => Some("markdown"),
		"dockerfile" => Some("dockerfile"),
		"nix" => Some("nix"),
		"svelte" => Some("svelte"),
		"vue" => Some("vue"),
		"elm" => Some("elm"),
		"ex" | "exs" => Some("elixir"),
		"erl" | "hrl" => Some("erlang"),
		"hs" | "lhs" => Some("haskell"),
		"clj" | "cljs" | "cljc" => Some("clojure"),
		"ml" | "mli" => Some("ocaml"),
		"fs" | "fsi" | "fsx" => Some("fsharp"),
		"r" => Some("r"),
		"lua" => Some("lua"),
		"pl" | "pm" => Some("perl"),
		"zig" => Some("zig"),
		"v" => Some("v"),
		"nim" => Some("nim"),
		"cr" => Some("crystal"),
		"dart" => Some("dart"),
		"groovy" | "gvy" => Some("groovy"),
		"ps1" | "psm1" => Some("powershell"),
		"bat" | "cmd" => Some("batch"),
		"asm" | "s" => Some("assembly"),
		"proto" => Some("protobuf"),
		"graphql" | "gql" => Some("graphql"),
		"tf" | "tfvars" => Some("hcl"),
		_ => None,
	}
	.map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_detect_language() {
		assert_eq!(detect_language("main.rs"), Some("rust".to_string()));
		assert_eq!(detect_language("index.ts"), Some("typescript".to_string()));
		assert_eq!(detect_language("app.py"), Some("python".to_string()));
		assert_eq!(detect_language("unknown.xyz"), None);
	}

	#[test]
	fn test_clip_path_format() {
		let store = ClipsGitStore::new(PathBuf::from("/data"));
		let clip_id = ClipId(uuid::Uuid::parse_str("12345678-1234-1234-1234-123456789012").unwrap());
		let path = store.get_clip_path(clip_id);

		assert!(path.to_str().unwrap().contains("12"));
		assert!(path.to_str().unwrap().ends_with("12345678-1234-1234-1234-123456789012.git"));
	}
}
