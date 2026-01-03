// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git browser API endpoints for the web UI.
//!
//! These endpoints support browsing repository contents via the web UI:
//! - Get repository by owner/name
//! - List branches
//! - Browse tree (directories)
//! - View blob (files)
//! - List commits
//! - View single commit with diff
//! - Blame view
//! - Compare refs

use axum::{
	extract::{Path, Query, State},
	response::IntoResponse,
	routing::get,
	Json,
};
use chrono::{DateTime, Utc};
use loom_server_auth::middleware::CurrentUser;
use loom_server_scm::Repository;
use serde::{Deserialize, Serialize};

use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
	api::AppState,
	auth_middleware::OptionalAuth,
	error::ServerError,
};

use super::git::{check_read_access, get_repo_path_by_id, resolve_repo};

fn resolve_optional_locale<'a>(user: Option<&'a CurrentUser>, default: &'a str) -> &'a str {
	if let Some(u) = user {
		loom_common_i18n::resolve_locale(u.user.locale.as_deref(), default)
	} else {
		default
	}
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BrowserRepoResponse {
	pub id: Uuid,
	pub owner_type: String,
	pub owner_id: Uuid,
	pub name: String,
	pub visibility: String,
	pub default_branch: String,
	pub clone_url: String,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TreeEntry {
	pub name: String,
	pub path: String,
	pub kind: String,
	pub sha: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub size: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Branch {
	pub name: String,
	pub sha: String,
	pub is_default: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommitInfo {
	pub sha: String,
	pub message: String,
	pub author_name: String,
	pub author_email: String,
	pub author_date: String,
	pub parent_shas: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommitWithDiff {
	#[serde(flatten)]
	pub commit: CommitInfo,
	pub diff: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListCommitsResponse {
	pub commits: Vec<CommitInfo>,
	pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BlameLine {
	pub line_number: usize,
	pub commit_sha: String,
	pub author_name: String,
	pub author_email: String,
	pub author_date: String,
	pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompareResult {
	pub base_ref: String,
	pub head_ref: String,
	pub commits: Vec<CommitInfo>,
	pub diff: String,
	pub ahead_by: usize,
	pub behind_by: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListCommitsParams {
	pub limit: Option<usize>,
	pub offset: Option<usize>,
	pub path: Option<String>,
}

fn build_clone_url(base_url: &str, owner_id: &Uuid, repo_name: &str) -> String {
	format!(
		"{}/git/{}/{}.git",
		base_url.trim_end_matches('/'),
		owner_id,
		repo_name
	)
}

fn repo_to_browser_response(repo: &Repository, base_url: &str) -> BrowserRepoResponse {
	BrowserRepoResponse {
		id: repo.id,
		owner_type: repo.owner_type.as_str().to_string(),
		owner_id: repo.owner_id,
		name: repo.name.clone(),
		visibility: repo.visibility.as_str().to_string(),
		default_branch: repo.default_branch.clone(),
		clone_url: build_clone_url(base_url, &repo.owner_id, &repo.name),
		created_at: repo.created_at,
		updated_at: repo.updated_at,
	}
}

#[tracing::instrument(skip(state))]
pub async fn get_repo_by_owner_name(
	OptionalAuth(user): OptionalAuth,
	State(state): State<AppState>,
	Path((owner, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, ServerError> {
	let locale = resolve_optional_locale(user.as_ref(), &state.default_locale);

	let repo = resolve_repo(&owner, &name, &state, locale).await?;
	check_read_access(&repo, user.as_ref(), &state, locale).await?;

	Ok(Json(repo_to_browser_response(&repo, &state.base_url)))
}

#[tracing::instrument(skip(state))]
pub async fn list_branches(
	OptionalAuth(user): OptionalAuth,
	State(state): State<AppState>,
	Path((owner, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, ServerError> {
	let locale = resolve_optional_locale(user.as_ref(), &state.default_locale);

	let repo = resolve_repo(&owner, &name, &state, locale).await?;
	check_read_access(&repo, user.as_ref(), &state, locale).await?;

	let repo_path = get_repo_path_by_id(repo.id);

	let git_repo = gix::open(&repo_path).map_err(|e| {
		tracing::error!(error = %e, "Failed to open git repository");
		ServerError::Internal("Failed to open repository".to_string())
	})?;

	let mut branches = Vec::new();
	for reference in git_repo.references().map_err(|e| {
		ServerError::Internal(format!("Failed to list references: {}", e))
	})?.local_branches().map_err(|e| {
		ServerError::Internal(format!("Failed to list branches: {}", e))
	})? {
		let reference = reference.map_err(|e| {
			ServerError::Internal(format!("Failed to read reference: {}", e))
		})?;

		let name = reference
			.name()
			.shorten()
			.to_string();

		let sha = reference
			.into_fully_peeled_id()
			.map_err(|e| ServerError::Internal(format!("Failed to peel reference: {}", e)))?
			.to_string();

		branches.push(Branch {
			is_default: name == repo.default_branch,
			name,
			sha,
		});
	}

	branches.sort_by(|a, b| {
		if a.is_default {
			std::cmp::Ordering::Less
		} else if b.is_default {
			std::cmp::Ordering::Greater
		} else {
			a.name.cmp(&b.name)
		}
	});

	Ok(Json(branches))
}

#[tracing::instrument(skip(state))]
pub async fn get_tree(
	OptionalAuth(user): OptionalAuth,
	State(state): State<AppState>,
	Path((owner, name, ref_and_path)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ServerError> {
	let locale = resolve_optional_locale(user.as_ref(), &state.default_locale);

	let repo = resolve_repo(&owner, &name, &state, locale).await?;
	check_read_access(&repo, user.as_ref(), &state, locale).await?;

	let repo_path = get_repo_path_by_id(repo.id);

	let git_repo = gix::open(&repo_path).map_err(|e| {
		tracing::error!(error = %e, "Failed to open git repository");
		ServerError::Internal("Failed to open repository".to_string())
	})?;

	// Parse ref and path from the combined path segment
	// Format: {ref} or {ref}/{path...}
	let (git_ref, tree_path) = parse_ref_and_path(&ref_and_path, &git_repo)?;

	let commit = git_repo
		.rev_parse_single(git_ref.as_bytes())
		.map_err(|e| ServerError::NotFound(format!("Ref not found: {}", e)))?
		.object()
		.map_err(|e| ServerError::Internal(format!("Failed to get object: {}", e)))?
		.peel_to_commit()
		.map_err(|e| ServerError::Internal(format!("Failed to peel to commit: {}", e)))?;

	let tree = commit
		.tree()
		.map_err(|e| ServerError::Internal(format!("Failed to get tree: {}", e)))?;

	let target_tree = if tree_path.is_empty() {
		tree
	} else {
		let entry = tree
			.lookup_entry_by_path(tree_path.as_str())
			.map_err(|e| ServerError::Internal(format!("Failed to lookup path: {}", e)))?
			.ok_or_else(|| ServerError::NotFound(format!("Path not found: {}", tree_path)))?;

		entry
			.object()
			.map_err(|e| ServerError::Internal(format!("Failed to get object: {}", e)))?
			.peel_to_tree()
			.map_err(|_| ServerError::NotFound(format!("Not a directory: {}", tree_path)))?
	};

	let mut entries = Vec::new();
	for entry_result in target_tree.iter() {
		let entry = entry_result.map_err(|e| {
			ServerError::Internal(format!("Failed to read tree entry: {}", e))
		})?;

		let entry_name = entry.filename().to_string();
		let entry_path = if tree_path.is_empty() {
			entry_name.clone()
		} else {
			format!("{}/{}", tree_path, entry_name)
		};

		let kind = match entry.mode().kind() {
			gix::object::tree::EntryKind::Blob | gix::object::tree::EntryKind::BlobExecutable => "file",
			gix::object::tree::EntryKind::Tree => "directory",
			gix::object::tree::EntryKind::Link => "symlink",
			gix::object::tree::EntryKind::Commit => "submodule",
		};

		let size = if kind == "file" {
			entry
				.object()
				.ok()
				.map(|o| o.data.len() as u64)
		} else {
			None
		};

		entries.push(TreeEntry {
			name: entry_name,
			path: entry_path,
			kind: kind.to_string(),
			sha: entry.oid().to_string(),
			size,
		});
	}

	// Sort: directories first, then alphabetically
	entries.sort_by(|a, b| {
		let a_is_dir = a.kind == "directory";
		let b_is_dir = b.kind == "directory";
		if a_is_dir && !b_is_dir {
			std::cmp::Ordering::Less
		} else if !a_is_dir && b_is_dir {
			std::cmp::Ordering::Greater
		} else {
			a.name.to_lowercase().cmp(&b.name.to_lowercase())
		}
	});

	Ok(Json(entries))
}

#[tracing::instrument(skip(state))]
pub async fn get_blob(
	OptionalAuth(user): OptionalAuth,
	State(state): State<AppState>,
	Path((owner, name, ref_and_path)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ServerError> {
	let locale = resolve_optional_locale(user.as_ref(), &state.default_locale);

	let repo = resolve_repo(&owner, &name, &state, locale).await?;
	check_read_access(&repo, user.as_ref(), &state, locale).await?;

	let repo_path = get_repo_path_by_id(repo.id);

	let git_repo = gix::open(&repo_path).map_err(|e| {
		tracing::error!(error = %e, "Failed to open git repository");
		ServerError::Internal("Failed to open repository".to_string())
	})?;

	// Parse ref and path - for blob, path is required
	let (git_ref, file_path) = parse_ref_and_path(&ref_and_path, &git_repo)?;

	if file_path.is_empty() {
		return Err(ServerError::BadRequest("File path is required".to_string()));
	}

	let commit = git_repo
		.rev_parse_single(git_ref.as_bytes())
		.map_err(|e| ServerError::NotFound(format!("Ref not found: {}", e)))?
		.object()
		.map_err(|e| ServerError::Internal(format!("Failed to get object: {}", e)))?
		.peel_to_commit()
		.map_err(|e| ServerError::Internal(format!("Failed to peel to commit: {}", e)))?;

	let tree = commit
		.tree()
		.map_err(|e| ServerError::Internal(format!("Failed to get tree: {}", e)))?;

	let entry = tree
		.lookup_entry_by_path(file_path.as_str())
		.map_err(|e| ServerError::Internal(format!("Failed to lookup path: {}", e)))?
		.ok_or_else(|| ServerError::NotFound(format!("Path not found: {}", file_path)))?;

	let object = entry
		.object()
		.map_err(|e| ServerError::Internal(format!("Failed to get object: {}", e)))?;

	// Check that this is a blob (file) not a tree (directory)
	if object.kind != gix::object::Kind::Blob {
		return Err(ServerError::NotFound(format!("Not a file: {}", file_path)));
	}

	let content = String::from_utf8_lossy(&object.data).to_string();

	Ok(content)
}

#[tracing::instrument(skip(state))]
pub async fn list_commits(
	OptionalAuth(user): OptionalAuth,
	State(state): State<AppState>,
	Path((owner, name, git_ref)): Path<(String, String, String)>,
	Query(params): Query<ListCommitsParams>,
) -> Result<impl IntoResponse, ServerError> {
	let locale = resolve_optional_locale(user.as_ref(), &state.default_locale);

	let repo = resolve_repo(&owner, &name, &state, locale).await?;
	check_read_access(&repo, user.as_ref(), &state, locale).await?;

	let repo_path = get_repo_path_by_id(repo.id);

	let git_repo = gix::open(&repo_path).map_err(|e| {
		tracing::error!(error = %e, "Failed to open git repository");
		ServerError::Internal("Failed to open repository".to_string())
	})?;

	let commit_id = git_repo
		.rev_parse_single(git_ref.as_bytes())
		.map_err(|e| ServerError::NotFound(format!("Ref not found: {}", e)))?;

	let limit = params.limit.unwrap_or(50).min(100);
	let offset = params.offset.unwrap_or(0);

	let mut commits = Vec::new();
	let mut count = 0;
	let mut total = 0;

	let walk = commit_id
		.ancestors()
		.all()
		.map_err(|e| ServerError::Internal(format!("Failed to walk commits: {}", e)))?;

	for commit_info in walk {
		let commit_info = commit_info.map_err(|e| {
			ServerError::Internal(format!("Failed to get commit: {}", e))
		})?;

		total += 1;

		if count < offset {
			count += 1;
			continue;
		}

		if commits.len() >= limit {
			continue; // Keep counting total
		}

		let commit = commit_info
			.object()
			.map_err(|e| ServerError::Internal(format!("Failed to get commit object: {}", e)))?;

		let author = commit.author().map_err(|e| {
			ServerError::Internal(format!("Failed to get author: {}", e))
		})?;

		let parent_shas: Vec<String> = commit
			.parent_ids()
			.map(|id| id.to_string())
			.collect();

		commits.push(CommitInfo {
			sha: commit.id.to_string(),
			message: commit.message_raw_sloppy().to_string(),
			author_name: author.name.to_string(),
			author_email: author.email.to_string(),
			author_date: format_git_time(author.time),
			parent_shas,
		});

		count += 1;
	}

	Ok(Json(ListCommitsResponse { commits, total }))
}

#[tracing::instrument(skip(state))]
pub async fn get_commit(
	OptionalAuth(user): OptionalAuth,
	State(state): State<AppState>,
	Path((owner, name, sha)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ServerError> {
	let locale = resolve_optional_locale(user.as_ref(), &state.default_locale);

	let repo = resolve_repo(&owner, &name, &state, locale).await?;
	check_read_access(&repo, user.as_ref(), &state, locale).await?;

	let repo_path = get_repo_path_by_id(repo.id);

	// Use git CLI for diff generation (gix diff support is limited)
	let output = tokio::process::Command::new("git")
		.args(["show", "--format=format:%H%n%s%n%an%n%ae%n%aI%n%P", &sha])
		.current_dir(&repo_path)
		.output()
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to run git: {}", e)))?;

	if !output.status.success() {
		return Err(ServerError::NotFound(format!("Commit not found: {}", sha)));
	}

	let output_str = String::from_utf8_lossy(&output.stdout);
	let mut lines = output_str.lines();

	let sha = lines.next().unwrap_or("").to_string();
	let message = lines.next().unwrap_or("").to_string();
	let author_name = lines.next().unwrap_or("").to_string();
	let author_email = lines.next().unwrap_or("").to_string();
	let author_date = lines.next().unwrap_or("").to_string();
	let parents = lines.next().unwrap_or("");
	let parent_shas: Vec<String> = parents.split_whitespace().map(|s| s.to_string()).collect();

	// Get the diff part (after the empty line following headers)
	let diff_start = output_str.find("\n\n").map(|i| i + 2).unwrap_or(output_str.len());
	let diff = output_str[diff_start..].to_string();

	Ok(Json(CommitWithDiff {
		commit: CommitInfo {
			sha,
			message,
			author_name,
			author_email,
			author_date,
			parent_shas,
		},
		diff,
	}))
}

#[tracing::instrument(skip(state))]
pub async fn get_blame(
	OptionalAuth(user): OptionalAuth,
	State(state): State<AppState>,
	Path((owner, name, ref_and_path)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ServerError> {
	let locale = resolve_optional_locale(user.as_ref(), &state.default_locale);

	let repo = resolve_repo(&owner, &name, &state, locale).await?;
	check_read_access(&repo, user.as_ref(), &state, locale).await?;

	let repo_path = get_repo_path_by_id(repo.id);

	// Parse ref and path using a simpler approach (since we use git CLI anyway)
	let (git_ref, file_path) = {
		let git_repo = gix::open(&repo_path).map_err(|e| {
			ServerError::Internal(format!("Failed to open repository: {}", e))
		})?;
		parse_ref_and_path(&ref_and_path, &git_repo)?
	}; // git_repo is dropped here before the await

	if file_path.is_empty() {
		return Err(ServerError::BadRequest("File path is required".to_string()));
	}

	// Use git CLI for blame (gix blame support is limited)
	let output = tokio::process::Command::new("git")
		.args([
			"blame",
			"--line-porcelain",
			&git_ref,
			"--",
			&file_path,
		])
		.current_dir(&repo_path)
		.output()
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to run git blame: {}", e)))?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		return Err(ServerError::NotFound(format!("Blame failed: {}", stderr)));
	}

	let output_str = String::from_utf8_lossy(&output.stdout);
	let blame_lines = parse_blame_output(&output_str)?;

	Ok(Json(blame_lines))
}

#[tracing::instrument(skip(state))]
pub async fn compare_refs(
	OptionalAuth(user): OptionalAuth,
	State(state): State<AppState>,
	Path((owner, name, refs)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, ServerError> {
	let locale = resolve_optional_locale(user.as_ref(), &state.default_locale);

	let repo = resolve_repo(&owner, &name, &state, locale).await?;
	check_read_access(&repo, user.as_ref(), &state, locale).await?;

	let repo_path = get_repo_path_by_id(repo.id);

	// Parse "base...head" format
	let (base_ref, head_ref) = refs
		.split_once("...")
		.ok_or_else(|| ServerError::BadRequest("Invalid compare format. Use: base...head".to_string()))?;

	// Get diff using git CLI
	let diff_output = tokio::process::Command::new("git")
		.args(["diff", &format!("{}...{}", base_ref, head_ref)])
		.current_dir(&repo_path)
		.output()
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to run git diff: {}", e)))?;

	let diff = String::from_utf8_lossy(&diff_output.stdout).to_string();

	// Get commits between refs
	let log_output = tokio::process::Command::new("git")
		.args([
			"log",
			"--format=%H|%s|%an|%ae|%aI|%P",
			&format!("{}..{}", base_ref, head_ref),
		])
		.current_dir(&repo_path)
		.output()
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to run git log: {}", e)))?;

	let log_str = String::from_utf8_lossy(&log_output.stdout);
	let commits: Vec<CommitInfo> = log_str
		.lines()
		.filter(|line| !line.is_empty())
		.map(|line| {
			let parts: Vec<&str> = line.splitn(6, '|').collect();
			CommitInfo {
				sha: parts.first().unwrap_or(&"").to_string(),
				message: parts.get(1).unwrap_or(&"").to_string(),
				author_name: parts.get(2).unwrap_or(&"").to_string(),
				author_email: parts.get(3).unwrap_or(&"").to_string(),
				author_date: parts.get(4).unwrap_or(&"").to_string(),
				parent_shas: parts
					.get(5)
					.unwrap_or(&"")
					.split_whitespace()
					.map(|s| s.to_string())
					.collect(),
			}
		})
		.collect();

	// Count ahead/behind
	let ahead_behind_output = tokio::process::Command::new("git")
		.args(["rev-list", "--left-right", "--count", &format!("{}...{}", base_ref, head_ref)])
		.current_dir(&repo_path)
		.output()
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to count: {}", e)))?;

	let count_str = String::from_utf8_lossy(&ahead_behind_output.stdout);
	let counts: Vec<&str> = count_str.trim().split('\t').collect();
	let behind_by: usize = counts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
	let ahead_by: usize = counts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

	Ok(Json(CompareResult {
		base_ref: base_ref.to_string(),
		head_ref: head_ref.to_string(),
		commits,
		diff,
		ahead_by,
		behind_by,
	}))
}

fn parse_ref_and_path(combined: &str, repo: &gix::Repository) -> Result<(String, String), ServerError> {
	// Try to find a valid ref by progressively taking more path segments
	let parts: Vec<&str> = combined.splitn(2, '/').collect();

	// First, try the first segment as the ref
	let first_segment = parts.first().copied().unwrap_or(combined);

	if repo.rev_parse_single(first_segment.as_bytes()).is_ok() {
		let path = parts.get(1).copied().unwrap_or("").to_string();
		return Ok((first_segment.to_string(), path));
	}

	// If that fails, try the whole thing as a ref (for refs with slashes)
	if repo.rev_parse_single(combined.as_bytes()).is_ok() {
		return Ok((combined.to_string(), String::new()));
	}

	// Default to treating first segment as ref
	Ok((first_segment.to_string(), parts.get(1).copied().unwrap_or("").to_string()))
}

fn format_git_time(time: gix::date::Time) -> String {
	let secs = time.seconds;
	let dt = DateTime::from_timestamp(secs, 0)
		.unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
	dt.to_rfc3339()
}

fn parse_blame_output(output: &str) -> Result<Vec<BlameLine>, ServerError> {
	let mut lines = Vec::new();
	let mut current_sha = String::new();
	let mut current_author = String::new();
	let mut current_email = String::new();
	let mut current_time = String::new();
	let mut line_number = 0usize;

	for line in output.lines() {
		if line.starts_with('\t') {
			lines.push(BlameLine {
				line_number,
				commit_sha: current_sha.clone(),
				author_name: current_author.clone(),
				author_email: current_email.clone(),
				author_date: current_time.clone(),
				content: line[1..].to_string(),
			});
		} else if line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
			// This is a commit line: sha orig_line final_line [num_lines]
			{
				let parts: Vec<&str> = line.split_whitespace().collect();
				if parts.len() >= 3 {
					current_sha = parts[0].to_string();
					line_number = parts[2].parse().unwrap_or(0);
				}
			}
		}

		if let Some(author) = line.strip_prefix("author ") {
			current_author = author.to_string();
		} else if let Some(email) = line.strip_prefix("author-mail ") {
			current_email = email.trim_matches(|c| c == '<' || c == '>').to_string();
		} else if let Some(time) = line.strip_prefix("author-time ") {
			if let Ok(secs) = time.parse::<i64>() {
				current_time = format_git_time(gix::date::Time::new(secs, 0));
			}
		}
	}

	Ok(lines)
}

pub fn router() -> crate::OptionalAuthRouter {
	crate::OptionalAuthRouter::new()
		.route("/api/repos/{owner}/{name}", get(get_repo_by_owner_name))
		.route("/api/repos/{owner}/{name}/branches", get(list_branches))
		.route("/api/repos/{owner}/{name}/tree/{*ref_and_path}", get(get_tree))
		.route("/api/repos/{owner}/{name}/blob/{*ref_and_path}", get(get_blob))
		.route("/api/repos/{owner}/{name}/commits/{git_ref}", get(list_commits))
		.route("/api/repos/{owner}/{name}/commit/{sha}", get(get_commit))
		.route("/api/repos/{owner}/{name}/blame/{*ref_and_path}", get(get_blame))
		.route("/api/repos/{owner}/{name}/compare/{*refs}", get(compare_refs))
}
