// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git browser API HTTP handlers.

use axum::{
	extract::{Path, Query, State},
	response::IntoResponse,
	Json,
};

use crate::{api::AppState, auth_middleware::OptionalAuth, error::ServerError};

use super::super::git::{check_read_access, get_repo_path_by_id, resolve_repo};
use super::helpers::{
	format_git_time, get_content_type_for_path, is_repo_empty, parse_blame_output,
	parse_ref_and_path, repo_to_browser_response, resolve_optional_locale,
};
use super::types::{
	Branch, CommitInfo, CommitWithDiff, CompareResult, ListCommitsParams, ListCommitsResponse,
	TreeEntry,
};

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
	for reference in git_repo
		.references()
		.map_err(|e| ServerError::Internal(format!("Failed to list references: {}", e)))?
		.local_branches()
		.map_err(|e| ServerError::Internal(format!("Failed to list branches: {}", e)))?
	{
		let reference =
			reference.map_err(|e| ServerError::Internal(format!("Failed to read reference: {}", e)))?;

		let name = reference.name().shorten().to_string();

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

	if is_repo_empty(&git_repo) {
		return Ok(Json(Vec::<TreeEntry>::new()));
	}

	let (git_ref, tree_path) = parse_ref_and_path(&ref_and_path, &git_repo)?;

	let commit = match git_repo.rev_parse_single(git_ref.as_bytes()) {
		Ok(rev) => rev
			.object()
			.map_err(|e| ServerError::Internal(format!("Failed to get object: {}", e)))?
			.peel_to_commit()
			.map_err(|e| ServerError::Internal(format!("Failed to peel to commit: {}", e)))?,
		Err(_) => {
			return Ok(Json(Vec::<TreeEntry>::new()));
		}
	};

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
		let entry = entry_result
			.map_err(|e| ServerError::Internal(format!("Failed to read tree entry: {}", e)))?;

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
			entry.object().ok().map(|o| o.data.len() as u64)
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

	if object.kind != gix::object::Kind::Blob {
		return Err(ServerError::NotFound(format!("Not a file: {}", file_path)));
	}

	let content = String::from_utf8_lossy(&object.data).to_string();

	Ok(content)
}

#[tracing::instrument(skip(state))]
pub async fn get_raw(
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
		ServerError::Internal(
			loom_common_i18n::t(locale, "server.api.scm.browser.failed_to_open_repo").to_string(),
		)
	})?;

	let (git_ref, file_path) = parse_ref_and_path(&ref_and_path, &git_repo)?;

	if file_path.is_empty() {
		return Err(ServerError::BadRequest(
			loom_common_i18n::t(locale, "server.api.scm.browser.file_path_required").to_string(),
		));
	}

	let commit = git_repo
		.rev_parse_single(git_ref.as_bytes())
		.map_err(|_| {
			ServerError::NotFound(
				loom_common_i18n::t_fmt(
					locale,
					"server.api.scm.browser.ref_not_found",
					&[("ref", &git_ref)],
				)
				.to_string(),
			)
		})?
		.object()
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get object");
			ServerError::Internal(
				loom_common_i18n::t(locale, "server.api.scm.browser.failed_to_get_object").to_string(),
			)
		})?
		.peel_to_commit()
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get commit");
			ServerError::Internal(
				loom_common_i18n::t(locale, "server.api.scm.browser.failed_to_get_commit").to_string(),
			)
		})?;

	let tree = commit.tree().map_err(|e| {
		tracing::error!(error = %e, "Failed to get tree");
		ServerError::Internal(
			loom_common_i18n::t(locale, "server.api.scm.browser.failed_to_get_tree").to_string(),
		)
	})?;

	let entry = tree
		.lookup_entry_by_path(file_path.as_str())
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to lookup path");
			ServerError::Internal(
				loom_common_i18n::t(locale, "server.api.scm.browser.failed_to_lookup_path").to_string(),
			)
		})?
		.ok_or_else(|| {
			ServerError::NotFound(
				loom_common_i18n::t_fmt(
					locale,
					"server.api.scm.browser.path_not_found",
					&[("path", &file_path)],
				)
				.to_string(),
			)
		})?;

	let object = entry.object().map_err(|e| {
		tracing::error!(error = %e, "Failed to get object");
		ServerError::Internal(
			loom_common_i18n::t(locale, "server.api.scm.browser.failed_to_get_object").to_string(),
		)
	})?;

	if object.kind != gix::object::Kind::Blob {
		return Err(ServerError::NotFound(
			loom_common_i18n::t_fmt(
				locale,
				"server.api.scm.browser.not_a_file",
				&[("path", &file_path)],
			)
			.to_string(),
		));
	}

	let content_type = get_content_type_for_path(&file_path);
	let filename = file_path.rsplit('/').next().unwrap_or(&file_path);

	Ok((
		[
			(axum::http::header::CONTENT_TYPE, content_type),
			(
				axum::http::header::CONTENT_DISPOSITION,
				format!("inline; filename=\"{}\"", filename),
			),
		],
		object.data.to_vec(),
	))
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
		let commit_info =
			commit_info.map_err(|e| ServerError::Internal(format!("Failed to get commit: {}", e)))?;

		total += 1;

		if count < offset {
			count += 1;
			continue;
		}

		if commits.len() >= limit {
			continue;
		}

		let commit = commit_info
			.object()
			.map_err(|e| ServerError::Internal(format!("Failed to get commit object: {}", e)))?;

		let author = commit
			.author()
			.map_err(|e| ServerError::Internal(format!("Failed to get author: {}", e)))?;

		let parent_shas: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

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

	let diff_start = output_str
		.find("\n\n")
		.map(|i| i + 2)
		.unwrap_or(output_str.len());
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

	let (git_ref, file_path) = {
		let git_repo = gix::open(&repo_path)
			.map_err(|e| ServerError::Internal(format!("Failed to open repository: {}", e)))?;
		parse_ref_and_path(&ref_and_path, &git_repo)?
	};

	if file_path.is_empty() {
		return Err(ServerError::BadRequest("File path is required".to_string()));
	}

	let output = tokio::process::Command::new("git")
		.args(["blame", "--line-porcelain", &git_ref, "--", &file_path])
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

	let (base_ref, head_ref) = refs.split_once("...").ok_or_else(|| {
		ServerError::BadRequest("Invalid compare format. Use: base...head".to_string())
	})?;

	let diff_output = tokio::process::Command::new("git")
		.args(["diff", &format!("{}...{}", base_ref, head_ref)])
		.current_dir(&repo_path)
		.output()
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to run git diff: {}", e)))?;

	let diff = String::from_utf8_lossy(&diff_output.stdout).to_string();

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

	let ahead_behind_output = tokio::process::Command::new("git")
		.args([
			"rev-list",
			"--left-right",
			"--count",
			&format!("{}...{}", base_ref, head_ref),
		])
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
