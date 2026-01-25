// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for Git HTTP Smart Protocol.

use axum::{
	http::{header, HeaderMap, StatusCode},
	response::{IntoResponse, Response},
};
use base64::Engine;
use loom_server_auth::middleware::{identify_bearer_token, BearerTokenType, CurrentUser};
use loom_server_scm::{RepoStore, Repository};
use loom_server_scm_mirror::ExternalMirrorStore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use std::process::Stdio;

use crate::{api::AppState, error::ServerError, i18n::t};

pub const ZERO_SHA: &str = "0000000000000000000000000000000000000000";

#[derive(Debug, Deserialize)]
pub struct InfoRefsParams {
	pub service: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitService {
	UploadPack,
	ReceivePack,
}

impl GitService {
	pub fn from_str(s: &str) -> Option<Self> {
		match s {
			"git-upload-pack" => Some(GitService::UploadPack),
			"git-receive-pack" => Some(GitService::ReceivePack),
			_ => None,
		}
	}

	pub fn as_str(&self) -> &'static str {
		match self {
			GitService::UploadPack => "git-upload-pack",
			GitService::ReceivePack => "git-receive-pack",
		}
	}

	pub fn as_git_subcommand(&self) -> &'static str {
		match self {
			GitService::UploadPack => "upload-pack",
			GitService::ReceivePack => "receive-pack",
		}
	}

	pub fn content_type(&self) -> &'static str {
		match self {
			GitService::UploadPack => "application/x-git-upload-pack-advertisement",
			GitService::ReceivePack => "application/x-git-receive-pack-advertisement",
		}
	}

	pub fn result_content_type(&self) -> &'static str {
		match self {
			GitService::UploadPack => "application/x-git-upload-pack-result",
			GitService::ReceivePack => "application/x-git-receive-pack-result",
		}
	}
}

#[derive(Debug)]
pub struct PushCommand {
	pub old_sha: String,
	pub new_sha: String,
	pub ref_name: String,
}

pub fn git_unauthorized_response(message: &str) -> Response {
	(
		StatusCode::UNAUTHORIZED,
		[(header::WWW_AUTHENTICATE, "Basic realm=\"git\"")],
		axum::Json(serde_json::json!({
			"error": "unauthorized",
			"message": message
		})),
	)
		.into_response()
}

pub fn get_repos_base_dir() -> PathBuf {
	std::env::var("LOOM_SERVER_DATA_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|_| PathBuf::from("/var/lib/loom"))
		.join("repos")
}

pub fn get_repo_path(repo: &Repository) -> PathBuf {
	let id_str = repo.id.to_string();
	let shard = &id_str[..2];
	get_repos_base_dir().join(shard).join(&id_str).join("git")
}

pub fn get_repo_path_by_id(repo_id: uuid::Uuid) -> PathBuf {
	let id_str = repo_id.to_string();
	let shard = &id_str[..2];
	get_repos_base_dir().join(shard).join(&id_str).join("git")
}

pub fn pkt_line(data: &str) -> Vec<u8> {
	let len = data.len() + 4;
	format!("{len:04x}{data}").into_bytes()
}

pub fn pkt_flush() -> Vec<u8> {
	b"0000".to_vec()
}

pub fn parse_push_commands(body: &[u8]) -> Vec<PushCommand> {
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

pub fn extract_branch_name(ref_name: &str) -> Option<&str> {
	ref_name.strip_prefix("refs/heads/")
}

pub async fn extract_basic_auth_user(headers: &HeaderMap, state: &AppState) -> Option<CurrentUser> {
	let auth_header = headers.get(header::AUTHORIZATION)?.to_str().ok()?;

	if !auth_header.starts_with("Basic ") {
		return None;
	}

	let encoded = auth_header.strip_prefix("Basic ")?;
	let decoded = base64::engine::general_purpose::STANDARD
		.decode(encoded)
		.ok()?;
	let credentials = String::from_utf8(decoded).ok()?;

	let (_, password) = credentials.split_once(':')?;

	match identify_bearer_token(password) {
		BearerTokenType::AccessToken => {
			let mut hasher = Sha256::new();
			hasher.update(password.as_bytes());
			let token_hash = hex::encode(hasher.finalize());

			let (_token_id, user_id) = state
				.session_repo
				.get_access_token_by_hash(&token_hash)
				.await
				.ok()??;

			let user = state.user_repo.get_user_by_id(&user_id).await.ok()??;
			Some(CurrentUser::from_access_token(user))
		}
		BearerTokenType::ApiKey => {
			let mut hasher = Sha256::new();
			hasher.update(password.as_bytes());
			let key_hash = hex::encode(hasher.finalize());

			let api_key = state
				.api_key_repo
				.get_api_key_by_hash(&key_hash)
				.await
				.ok()??;

			if api_key.revoked_at.is_some() {
				return None;
			}

			let user = state
				.user_repo
				.get_user_by_id(&api_key.created_by)
				.await
				.ok()??;
			Some(CurrentUser::from_api_key(user, api_key.id.into()))
		}
		BearerTokenType::Unknown => None,
		BearerTokenType::WsToken => None,
	}
}

pub async fn update_mirror_access_time(state: &AppState, repo_id: uuid::Uuid) {
	if let Some(store) = &state.external_mirror_store {
		if let Ok(Some(mirror)) = store.get_by_repo_id(repo_id).await {
			if let Err(e) = loom_server_scm_mirror::touch_mirror(store.as_ref(), mirror.id).await {
				tracing::warn!(
					mirror_id = %mirror.id,
					error = %e,
					"Failed to update external mirror access time"
				);
			}
		}
	}
}

pub async fn is_force_push(repo_path: &std::path::Path, old_sha: &str, new_sha: &str) -> bool {
	if old_sha == ZERO_SHA || new_sha == ZERO_SHA {
		return false;
	}

	let path = repo_path.to_path_buf();
	let old = old_sha.to_string();
	let new = new_sha.to_string();

	let result = tokio::task::spawn_blocking(move || {
		let repo = match gix::open(&path) {
			Ok(r) => r,
			Err(_) => return false,
		};

		let old_oid = match gix::ObjectId::from_hex(old.as_bytes()) {
			Ok(oid) => oid,
			Err(_) => return false,
		};

		let new_oid = match gix::ObjectId::from_hex(new.as_bytes()) {
			Ok(oid) => oid,
			Err(_) => return false,
		};

		match repo.merge_base(old_oid, new_oid) {
			Ok(base) => base.detach() != old_oid,
			Err(_) => true,
		}
	})
	.await;

	result.unwrap_or(false)
}

// NOTE: run_git_command uses git subprocess because gitoxide doesn't yet support
// server-side git protocol (upload-pack/receive-pack). The client-side protocol is
// supported but serving git HTTP requires additional server-side implementation.
// See: https://github.com/GitoxideLabs/gitoxide/discussions/362
// Track progress at: https://github.com/GitoxideLabs/gitoxide/issues/307
pub async fn run_git_command(
	repo_path: &std::path::Path,
	service: GitService,
	input: &[u8],
	advertise: bool,
) -> Result<Vec<u8>, ServerError> {
	let mut cmd = Command::new("git");
	cmd.arg(service.as_git_subcommand());

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

pub async fn resolve_repo(
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

	// Try parsing owner as UUID (owner_id) first
	if let Ok(owner_id) = uuid::Uuid::parse_str(owner) {
		// Try as user owner
		if let Some(scm_repo) = scm_store
			.get_by_owner_and_name(loom_server_scm::OwnerType::User, owner_id, repo_name)
			.await
			.map_err(|e| ServerError::Internal(e.to_string()))?
		{
			return Ok(scm_repo);
		}
		// Try as org owner
		if let Some(scm_repo) = scm_store
			.get_by_owner_and_name(loom_server_scm::OwnerType::Org, owner_id, repo_name)
			.await
			.map_err(|e| ServerError::Internal(e.to_string()))?
		{
			return Ok(scm_repo);
		}
	}

	// Try looking up by org slug
	if let Some(org) = state.org_repo.get_org_by_slug(owner).await? {
		if let Some(scm_repo) = scm_store
			.get_by_owner_and_name(loom_server_scm::OwnerType::Org, org.id.into(), repo_name)
			.await
			.map_err(|e| ServerError::Internal(e.to_string()))?
		{
			return Ok(scm_repo);
		}
	}

	// Try looking up by username
	if let Ok(Some(user)) = state.user_repo.get_user_by_username(owner).await {
		if let Some(scm_repo) = scm_store
			.get_by_owner_and_name(
				loom_server_scm::OwnerType::User,
				user.id.into_inner(),
				repo_name,
			)
			.await
			.map_err(|e| ServerError::Internal(e.to_string()))?
		{
			return Ok(scm_repo);
		}
	}

	Err(ServerError::NotFound(
		t(locale, "server.api.scm.repo_not_found").to_string(),
	))
}
