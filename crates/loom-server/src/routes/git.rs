// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git HTTP Smart Protocol endpoints for clone/fetch/push operations.
//!
//! Implements the Git smart HTTP protocol as specified in:
//! https://git-scm.com/docs/http-protocol
//!
//! # Endpoints
//!
//! - `GET /git/{owner}/{repo}.git/info/refs` - Advertise refs for clone/fetch/push
//! - `POST /git/{owner}/{repo}.git/git-upload-pack` - Serve clone/fetch
//! - `POST /git/{owner}/{repo}.git/git-receive-pack` - Receive push
//!
//! # Authentication
//!
//! - Public repos: anonymous clone allowed
//! - Private repos: authentication required for all operations
//! - Push: always requires authentication

use axum::{
	body::Bytes,
	extract::{Path, Query, State},
	http::{header, HeaderMap, StatusCode},
	response::{IntoResponse, Response},
	routing::{get, post},
};
use loom_auth::middleware::CurrentUser;
use loom_auth::types::{OrgId, OrgRole};
use loom_scm::{
	check_push_allowed, OwnerType, ProtectionStore, PushCheck, RepoStore, Repository, Visibility,
};
use serde::Deserialize;
use std::{path::PathBuf, process::Stdio};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::instrument;

use crate::{api::AppState, auth_middleware::OptionalAuth, error::ServerError, i18n::t};

#[derive(Debug, Deserialize)]
pub struct InfoRefsParams {
	service: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitService {
	UploadPack,
	ReceivePack,
}

impl GitService {
	fn from_str(s: &str) -> Option<Self> {
		match s {
			"git-upload-pack" => Some(GitService::UploadPack),
			"git-receive-pack" => Some(GitService::ReceivePack),
			_ => None,
		}
	}

	fn as_str(&self) -> &'static str {
		match self {
			GitService::UploadPack => "git-upload-pack",
			GitService::ReceivePack => "git-receive-pack",
		}
	}

	fn content_type(&self) -> &'static str {
		match self {
			GitService::UploadPack => "application/x-git-upload-pack-advertisement",
			GitService::ReceivePack => "application/x-git-receive-pack-advertisement",
		}
	}

	fn result_content_type(&self) -> &'static str {
		match self {
			GitService::UploadPack => "application/x-git-upload-pack-result",
			GitService::ReceivePack => "application/x-git-receive-pack-result",
		}
	}
}

fn get_repos_base_dir() -> PathBuf {
	std::env::var("LOOM_DATA_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|_| PathBuf::from("/var/lib/loom"))
		.join("repos")
}

fn get_repo_path(repo: &Repository) -> PathBuf {
	let id_str = repo.id.to_string();
	let shard = &id_str[..2];
	get_repos_base_dir()
		.join(shard)
		.join(&id_str)
		.join("git")
}

async fn resolve_repo(
	owner: &str,
	repo_name: &str,
	state: &AppState,
	locale: &str,
) -> Result<Repository, ServerError> {
	let repo_name = repo_name.strip_suffix(".git").unwrap_or(repo_name);

	let scm_store = state
		.scm_repo_store
		.as_ref()
		.ok_or_else(|| ServerError::Internal(t(locale, "server.api.scm.not_configured").to_string()))?;

	if let Some(org) = state.org_repo.get_org_by_slug(owner).await? {
		if let Some(scm_repo) = scm_store
			.get_by_owner_and_name(loom_scm::OwnerType::Org, org.id.into(), repo_name)
			.await
			.map_err(|e| ServerError::Internal(e.to_string()))?
		{
			return Ok(scm_repo);
		}
	}

	Err(ServerError::NotFound(t(locale, "server.api.scm.repo_not_found").to_string()))
}

fn check_read_access(repo: &Repository, user: Option<&CurrentUser>, locale: &str) -> Result<(), ServerError> {
	match repo.visibility {
		Visibility::Public => Ok(()),
		Visibility::Private => {
			if user.is_some() {
				Ok(())
			} else {
				Err(ServerError::Unauthorized(
					t(locale, "server.api.scm.git.auth_required_private").to_string(),
				))
			}
		}
	}
}

fn check_write_access(user: Option<&CurrentUser>, locale: &str) -> Result<(), ServerError> {
	if user.is_some() {
		Ok(())
	} else {
		Err(ServerError::Unauthorized(
			t(locale, "server.api.scm.git.auth_required_push").to_string(),
		))
	}
}

const ZERO_SHA: &str = "0000000000000000000000000000000000000000";

#[derive(Debug)]
struct PushCommand {
	old_sha: String,
	new_sha: String,
	ref_name: String,
}

fn parse_push_commands(body: &[u8]) -> Vec<PushCommand> {
	let mut commands = Vec::new();
	let mut pos = 0;

	while pos + 4 <= body.len() {
		let len_str = std::str::from_utf8(&body[pos..pos + 4]).unwrap_or("0000");
		let len = usize::from_str_radix(len_str, 16).unwrap_or(0);

		if len == 0 {
			break;
		}

		if pos + len > body.len() {
			break;
		}

		let line_end = pos + len;
		let line = &body[pos + 4..line_end];
		pos = line_end;

		if let Ok(line_str) = std::str::from_utf8(line) {
			let line_str = line_str.trim_end_matches('\n');
			let parts: Vec<&str> = line_str.split(' ').collect();
			if parts.len() >= 3 {
				let old_sha = parts[0].to_string();
				let new_sha = parts[1].to_string();
				let ref_with_caps = parts[2..].join(" ");
				let ref_name = ref_with_caps
					.split('\0')
					.next()
					.unwrap_or(&ref_with_caps)
					.to_string();

				commands.push(PushCommand {
					old_sha,
					new_sha,
					ref_name,
				});
			}
		}
	}

	commands
}

fn extract_branch_name(ref_name: &str) -> Option<&str> {
	ref_name.strip_prefix("refs/heads/")
}

async fn check_user_is_repo_admin(
	repo: &Repository,
	user: &CurrentUser,
	state: &AppState,
) -> bool {
	match repo.owner_type {
		OwnerType::User => repo.owner_id == user.user.id.into_inner(),
		OwnerType::Org => {
			let org_id = OrgId::new(repo.owner_id);
			match state.org_repo.get_membership(&org_id, &user.user.id).await {
				Ok(Some(m)) => m.role == OrgRole::Owner || m.role == OrgRole::Admin,
				_ => false,
			}
		}
	}
}

async fn is_force_push(
	repo_path: &std::path::Path,
	old_sha: &str,
	new_sha: &str,
) -> bool {
	if old_sha == ZERO_SHA || new_sha == ZERO_SHA {
		return false;
	}

	let output = Command::new("git")
		.arg("merge-base")
		.arg("--is-ancestor")
		.arg(old_sha)
		.arg(new_sha)
		.current_dir(repo_path)
		.output()
		.await;

	match output {
		Ok(o) => !o.status.success(),
		Err(_) => false,
	}
}

fn pkt_line(data: &str) -> Vec<u8> {
	let len = data.len() + 4;
	format!("{len:04x}{data}").into_bytes()
}

fn pkt_flush() -> Vec<u8> {
	b"0000".to_vec()
}

async fn run_git_command(
	repo_path: &std::path::Path,
	service: GitService,
	input: &[u8],
	advertise: bool,
) -> Result<Vec<u8>, ServerError> {
	let mut cmd = Command::new("git");
	cmd.arg(service.as_str());

	if advertise {
		cmd.arg("--advertise-refs");
	}

	cmd.arg("--stateless-rpc");
	cmd.arg(repo_path);
	cmd.stdin(Stdio::piped());
	cmd.stdout(Stdio::piped());
	cmd.stderr(Stdio::piped());

	let mut child = cmd
		.spawn()
		.map_err(|e| ServerError::Internal(format!("Failed to spawn git: {e}")))?;

	if let Some(mut stdin) = child.stdin.take() {
		stdin
			.write_all(input)
			.await
			.map_err(|e| ServerError::Internal(format!("Failed to write to git stdin: {e}")))?;
	}

	let output = child
		.wait_with_output()
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to wait for git: {e}")))?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		tracing::error!(stderr = %stderr, "git command failed");
		return Err(ServerError::Internal(format!(
			"Git command failed: {stderr}"
		)));
	}

	Ok(output.stdout)
}

#[instrument(skip(state), fields(owner = %owner, repo = %repo))]
pub async fn info_refs(
	Path((owner, repo)): Path<(String, String)>,
	Query(params): Query<InfoRefsParams>,
	OptionalAuth(auth): OptionalAuth,
	State(state): State<AppState>,
) -> Result<Response, ServerError> {
	let locale = auth
		.as_ref()
		.and_then(|u| u.user.locale.as_deref())
		.unwrap_or(&state.default_locale);

	let service = GitService::from_str(&params.service).ok_or_else(|| {
		ServerError::BadRequest(format!("Invalid service: {}", params.service))
	})?;

	let scm_repo = resolve_repo(&owner, &repo, &state, locale).await?;
	let repo_path = get_repo_path(&scm_repo);

	if !repo_path.exists() {
		return Err(ServerError::NotFound(t(locale, "server.api.scm.repo_not_found").to_string()));
	}

	match service {
		GitService::UploadPack => {
			check_read_access(&scm_repo, auth.as_ref(), locale)?;
		}
		GitService::ReceivePack => {
			check_write_access(auth.as_ref(), locale)?;
		}
	}

	let git_output = run_git_command(&repo_path, service, &[], true).await?;

	let mut response_body = Vec::new();
	response_body.extend(pkt_line(&format!("# service={}\n", service.as_str())));
	response_body.extend(pkt_flush());
	response_body.extend(git_output);

	Ok((
		StatusCode::OK,
		[
			(header::CONTENT_TYPE, service.content_type()),
			(header::CACHE_CONTROL, "no-cache"),
		],
		response_body,
	)
		.into_response())
}

#[instrument(skip(state, body), fields(owner = %owner, repo = %repo))]
pub async fn upload_pack(
	Path((owner, repo)): Path<(String, String)>,
	OptionalAuth(auth): OptionalAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
	body: Bytes,
) -> Result<Response, ServerError> {
	let locale = auth
		.as_ref()
		.and_then(|u| u.user.locale.as_deref())
		.unwrap_or(&state.default_locale);

	let content_type = headers
		.get(header::CONTENT_TYPE)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");

	if content_type != "application/x-git-upload-pack-request" {
		return Err(ServerError::BadRequest(format!(
			"Invalid content type: {content_type}"
		)));
	}

	let scm_repo = resolve_repo(&owner, &repo, &state, locale).await?;
	let repo_path = get_repo_path(&scm_repo);

	if !repo_path.exists() {
		return Err(ServerError::NotFound(t(locale, "server.api.scm.repo_not_found").to_string()));
	}

	check_read_access(&scm_repo, auth.as_ref(), locale)?;

	let output = run_git_command(&repo_path, GitService::UploadPack, &body, false).await?;

	Ok((
		StatusCode::OK,
		[
			(
				header::CONTENT_TYPE,
				GitService::UploadPack.result_content_type(),
			),
			(header::CACHE_CONTROL, "no-cache"),
		],
		output,
	)
		.into_response())
}

#[instrument(skip(state, body), fields(owner = %owner, repo = %repo))]
pub async fn receive_pack(
	Path((owner, repo)): Path<(String, String)>,
	OptionalAuth(auth): OptionalAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
	body: Bytes,
) -> Result<Response, ServerError> {
	let locale = auth
		.as_ref()
		.and_then(|u| u.user.locale.as_deref())
		.unwrap_or(&state.default_locale);

	check_write_access(auth.as_ref(), locale)?;

	let content_type = headers
		.get(header::CONTENT_TYPE)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");

	if content_type != "application/x-git-receive-pack-request" {
		return Err(ServerError::BadRequest(format!(
			"Invalid content type: {content_type}"
		)));
	}

	let scm_repo = resolve_repo(&owner, &repo, &state, locale).await?;
	let repo_path = get_repo_path(&scm_repo);

	if !repo_path.exists() {
		return Err(ServerError::NotFound(t(locale, "server.api.scm.repo_not_found").to_string()));
	}

	let user = auth
		.as_ref()
		.ok_or_else(|| ServerError::Unauthorized(t(locale, "server.api.scm.git.auth_required").to_string()))?;

	if let Some(protection_store) = state.scm_protection_store.as_ref() {
		let rules = protection_store
			.list_by_repo(scm_repo.id)
			.await
			.map_err(|e| ServerError::Internal(format!("{}: {e}", t(locale, "server.api.scm.protection.failed_to_load"))))?;

		if !rules.is_empty() {
			let user_is_admin = check_user_is_repo_admin(&scm_repo, user, &state).await;
			let commands = parse_push_commands(&body);

			for cmd in commands {
				if let Some(branch) = extract_branch_name(&cmd.ref_name) {
					let is_deletion = cmd.new_sha == ZERO_SHA;
					let force_push = is_force_push(&repo_path, &cmd.old_sha, &cmd.new_sha).await;

					let check = PushCheck {
						branch: branch.to_string(),
						is_force_push: force_push,
						is_deletion,
						user_is_admin,
					};

					if let Err(violation) = check_push_allowed(&rules, &check) {
						tracing::warn!(
							repo_id = %scm_repo.id,
							branch = %branch,
							user_id = %user.user.id,
							violation = %violation,
							"Push blocked by branch protection"
						);
						return Err(ServerError::Forbidden(violation.to_string()));
					}
				}
			}
		}
	}

	let output = run_git_command(&repo_path, GitService::ReceivePack, &body, false).await?;

	Ok((
		StatusCode::OK,
		[
			(
				header::CONTENT_TYPE,
				GitService::ReceivePack.result_content_type(),
			),
			(header::CACHE_CONTROL, "no-cache"),
		],
		output,
	)
		.into_response())
}

pub fn router() -> crate::OptionalAuthRouter {
	crate::OptionalAuthRouter::new()
		.route("/git/{owner}/{repo}/info/refs", get(info_refs))
		.route("/git/{owner}/{repo}/git-upload-pack", post(upload_pack))
		.route("/git/{owner}/{repo}/git-receive-pack", post(receive_pack))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_pkt_line() {
		let line = pkt_line("# service=git-upload-pack\n");
		assert_eq!(line, b"001e# service=git-upload-pack\n");
	}

	#[test]
	fn test_pkt_flush() {
		assert_eq!(pkt_flush(), b"0000");
	}

	#[test]
	fn test_git_service_from_str() {
		assert_eq!(
			GitService::from_str("git-upload-pack"),
			Some(GitService::UploadPack)
		);
		assert_eq!(
			GitService::from_str("git-receive-pack"),
			Some(GitService::ReceivePack)
		);
		assert_eq!(GitService::from_str("invalid"), None);
	}

	#[test]
	fn test_git_service_content_types() {
		assert_eq!(
			GitService::UploadPack.content_type(),
			"application/x-git-upload-pack-advertisement"
		);
		assert_eq!(
			GitService::ReceivePack.content_type(),
			"application/x-git-receive-pack-advertisement"
		);
		assert_eq!(
			GitService::UploadPack.result_content_type(),
			"application/x-git-upload-pack-result"
		);
		assert_eq!(
			GitService::ReceivePack.result_content_type(),
			"application/x-git-receive-pack-result"
		);
	}

	#[test]
	fn test_get_repos_base_dir() {
		let path = get_repos_base_dir();
		assert!(path.to_string_lossy().contains("repos"));
	}

	#[test]
	fn test_parse_push_commands_empty() {
		let commands = parse_push_commands(b"");
		assert!(commands.is_empty());
	}

	#[test]
	fn test_parse_push_commands_flush() {
		let commands = parse_push_commands(b"0000");
		assert!(commands.is_empty());
	}

	#[test]
	fn test_parse_push_commands_single() {
		let old = "0000000000000000000000000000000000000000";
		let new = "1234567890abcdef1234567890abcdef12345678";
		let refname = "refs/heads/main";
		let line = format!("{} {} {}\n", old, new, refname);
		let pkt = format!("{:04x}{}", line.len() + 4, line);
		let data = format!("{}0000", pkt);

		let commands = parse_push_commands(data.as_bytes());
		assert_eq!(commands.len(), 1);
		assert_eq!(commands[0].old_sha, old);
		assert_eq!(commands[0].new_sha, new);
		assert_eq!(commands[0].ref_name, refname);
	}

	#[test]
	fn test_extract_branch_name() {
		assert_eq!(extract_branch_name("refs/heads/main"), Some("main"));
		assert_eq!(extract_branch_name("refs/heads/feature/test"), Some("feature/test"));
		assert_eq!(extract_branch_name("refs/tags/v1.0"), None);
		assert_eq!(extract_branch_name("main"), None);
	}

	#[test]
	fn test_zero_sha_constant() {
		assert_eq!(ZERO_SHA.len(), 40);
		assert!(ZERO_SHA.chars().all(|c| c == '0'));
	}
}
