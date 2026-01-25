// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Helper functions for Git browser API.

use chrono::DateTime;
use loom_server_auth::middleware::CurrentUser;
use loom_server_scm::Repository;
use uuid::Uuid;

use crate::error::ServerError;

use super::types::{BlameLine, BrowserRepoResponse};

pub fn resolve_optional_locale<'a>(user: Option<&'a CurrentUser>, default: &'a str) -> &'a str {
	if let Some(u) = user {
		loom_common_i18n::resolve_locale(u.user.locale.as_deref(), default)
	} else {
		default
	}
}

pub fn build_clone_url(base_url: &str, owner_id: &Uuid, repo_name: &str) -> String {
	format!(
		"{}/git/{}/{}.git",
		base_url.trim_end_matches('/'),
		owner_id,
		repo_name
	)
}

pub fn repo_to_browser_response(repo: &Repository, base_url: &str) -> BrowserRepoResponse {
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

pub fn is_repo_empty(repo: &gix::Repository) -> bool {
	match repo.head() {
		Ok(head) => head.into_peeled_id().is_err(),
		Err(_) => true,
	}
}

pub fn parse_ref_and_path(
	combined: &str,
	repo: &gix::Repository,
) -> Result<(String, String), ServerError> {
	let parts: Vec<&str> = combined.splitn(2, '/').collect();

	let first_segment = parts.first().copied().unwrap_or(combined);

	if repo.rev_parse_single(first_segment.as_bytes()).is_ok() {
		let path = parts.get(1).copied().unwrap_or("").to_string();
		return Ok((first_segment.to_string(), path));
	}

	if repo.rev_parse_single(combined.as_bytes()).is_ok() {
		return Ok((combined.to_string(), String::new()));
	}

	Ok((
		first_segment.to_string(),
		parts.get(1).copied().unwrap_or("").to_string(),
	))
}

pub fn format_git_time(time: gix::date::Time) -> String {
	let secs = time.seconds;
	let dt =
		DateTime::from_timestamp(secs, 0).unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
	dt.to_rfc3339()
}

pub fn parse_blame_output(output: &str) -> Result<Vec<BlameLine>, ServerError> {
	let mut lines = Vec::new();
	let mut current_sha = String::new();
	let mut current_author = String::new();
	let mut current_email = String::new();
	let mut current_time = String::new();
	let mut line_number = 0usize;

	for line in output.lines() {
		if let Some(stripped) = line.strip_prefix('\t') {
			lines.push(BlameLine {
				line_number,
				commit_sha: current_sha.clone(),
				author_name: current_author.clone(),
				author_email: current_email.clone(),
				author_date: current_time.clone(),
				content: stripped.to_string(),
			});
		} else if line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
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

pub fn get_content_type_for_path(path: &str) -> String {
	let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
	match ext.as_str() {
		// Text
		"txt" => "text/plain; charset=utf-8",
		"md" | "markdown" => "text/markdown; charset=utf-8",
		"html" | "htm" => "text/html; charset=utf-8",
		"css" => "text/css; charset=utf-8",
		"js" | "mjs" => "text/javascript; charset=utf-8",
		"json" => "application/json; charset=utf-8",
		"xml" => "application/xml; charset=utf-8",
		"yaml" | "yml" => "text/yaml; charset=utf-8",
		"toml" => "text/toml; charset=utf-8",
		"csv" => "text/csv; charset=utf-8",
		// Code
		"rs" => "text/x-rust; charset=utf-8",
		"py" => "text/x-python; charset=utf-8",
		"rb" => "text/x-ruby; charset=utf-8",
		"go" => "text/x-go; charset=utf-8",
		"java" => "text/x-java; charset=utf-8",
		"c" | "h" => "text/x-c; charset=utf-8",
		"cpp" | "hpp" | "cc" | "cxx" => "text/x-c++; charset=utf-8",
		"ts" | "tsx" => "text/typescript; charset=utf-8",
		"jsx" => "text/jsx; charset=utf-8",
		"sh" | "bash" | "zsh" => "text/x-shellscript; charset=utf-8",
		"sql" => "text/x-sql; charset=utf-8",
		"nix" => "text/x-nix; charset=utf-8",
		"svelte" => "text/x-svelte; charset=utf-8",
		"vue" => "text/x-vue; charset=utf-8",
		// Images
		"png" => "image/png",
		"jpg" | "jpeg" => "image/jpeg",
		"gif" => "image/gif",
		"svg" => "image/svg+xml",
		"webp" => "image/webp",
		"ico" => "image/x-icon",
		// Other
		"pdf" => "application/pdf",
		"zip" => "application/zip",
		"gz" | "gzip" => "application/gzip",
		"tar" => "application/x-tar",
		"wasm" => "application/wasm",
		_ => "application/octet-stream",
	}
	.to_string()
}
