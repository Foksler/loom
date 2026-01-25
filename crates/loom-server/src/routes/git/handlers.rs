// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git HTTP Smart Protocol route handlers.

use axum::{
	body::Bytes,
	extract::{Path, Query, State},
	http::{header, HeaderMap, StatusCode},
	response::{IntoResponse, Response},
};
use loom_server_scm::{check_push_allowed, ProtectionStore, PushCheck};
use tracing::{info, instrument};

use crate::{api::AppState, auth_middleware::OptionalAuth, error::ServerError, i18n::t};

use super::{
	access::{check_read_access, check_user_is_repo_admin, check_write_access},
	common::{
		extract_basic_auth_user, extract_branch_name, get_repo_path, git_unauthorized_response,
		is_force_push, parse_push_commands, pkt_flush, pkt_line, resolve_repo, run_git_command,
		update_mirror_access_time, GitService, InfoRefsParams, ZERO_SHA,
	},
	mirror::{create_on_demand_mirror, is_mirror_path, parse_mirror_git_path, parse_mirror_path},
};

#[instrument(skip(state, headers), fields(owner = %owner, repo = %repo))]
pub async fn info_refs(
	Path((owner, repo)): Path<(String, String)>,
	Query(params): Query<InfoRefsParams>,
	OptionalAuth(auth): OptionalAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Response, ServerError> {
	let effective_user = match auth {
		Some(user) => Some(user),
		None => extract_basic_auth_user(&headers, &state).await,
	};

	let locale = effective_user
		.as_ref()
		.and_then(|u| u.user.locale.as_deref())
		.unwrap_or(&state.default_locale);

	let service = GitService::from_str(&params.service)
		.ok_or_else(|| ServerError::BadRequest(format!("Invalid service: {}", params.service)))?;

	let scm_repo = match resolve_repo(&owner, &repo, &state, locale).await {
		Ok(repo) => repo,
		Err(_) if is_mirror_path(&owner) => {
			let mirror_info = parse_mirror_path(&owner, &repo).ok_or_else(|| {
				ServerError::BadRequest(t(locale, "server.api.scm.mirror.invalid_path").to_string())
			})?;

			if service == GitService::ReceivePack {
				return Err(ServerError::Forbidden(
					t(locale, "server.api.scm.mirror.push_not_allowed").to_string(),
				));
			}

			info!(
				platform = ?mirror_info.platform,
				owner = %mirror_info.external_owner,
				repo = %mirror_info.external_repo,
				"On-demand mirror requested"
			);

			create_on_demand_mirror(&mirror_info, &state, locale).await?
		}
		Err(e) => return Err(e),
	};

	let repo_path = get_repo_path(&scm_repo);

	if !repo_path.exists() {
		return Err(ServerError::NotFound(
			t(locale, "server.api.scm.repo_not_found").to_string(),
		));
	}

	match service {
		GitService::UploadPack => {
			if let Err(e) = check_read_access(&scm_repo, effective_user.as_ref(), &state, locale).await {
				return match e {
					ServerError::Unauthorized(msg) => Ok(git_unauthorized_response(&msg)),
					other => Err(other),
				};
			}
		}
		GitService::ReceivePack => {
			if let Err(e) = check_write_access(&scm_repo, effective_user.as_ref(), &state, locale).await {
				return match e {
					ServerError::Unauthorized(msg) => Ok(git_unauthorized_response(&msg)),
					other => Err(other),
				};
			}
		}
	}

	update_mirror_access_time(&state, scm_repo.id).await;

	let git_output = run_git_command(&repo_path, service, &[], true).await?;

	let mut response_body = Vec::new();
	response_body.extend(pkt_line(&format!("# service={}\n", service.as_str())));
	response_body.extend(pkt_flush());
	response_body.extend(git_output);

	Ok(
		(
			StatusCode::OK,
			[
				(header::CONTENT_TYPE, service.content_type()),
				(header::CACHE_CONTROL, "no-cache"),
			],
			response_body,
		)
			.into_response(),
	)
}

#[instrument(skip(state, body, headers), fields(owner = %owner, repo = %repo))]
pub async fn upload_pack(
	Path((owner, repo)): Path<(String, String)>,
	OptionalAuth(auth): OptionalAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
	body: Bytes,
) -> Result<Response, ServerError> {
	let effective_user = match auth {
		Some(user) => Some(user),
		None => extract_basic_auth_user(&headers, &state).await,
	};

	let locale = effective_user
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

	let scm_repo = match resolve_repo(&owner, &repo, &state, locale).await {
		Ok(repo) => repo,
		Err(_) if is_mirror_path(&owner) => {
			let mirror_info = parse_mirror_path(&owner, &repo).ok_or_else(|| {
				ServerError::BadRequest(t(locale, "server.api.scm.mirror.invalid_path").to_string())
			})?;

			create_on_demand_mirror(&mirror_info, &state, locale).await?
		}
		Err(e) => return Err(e),
	};

	let repo_path = get_repo_path(&scm_repo);

	if !repo_path.exists() {
		return Err(ServerError::NotFound(
			t(locale, "server.api.scm.repo_not_found").to_string(),
		));
	}

	if let Err(e) = check_read_access(&scm_repo, effective_user.as_ref(), &state, locale).await {
		return match e {
			ServerError::Unauthorized(msg) => Ok(git_unauthorized_response(&msg)),
			other => Err(other),
		};
	}

	update_mirror_access_time(&state, scm_repo.id).await;

	let output = run_git_command(&repo_path, GitService::UploadPack, &body, false).await?;

	Ok(
		(
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
			.into_response(),
	)
}

#[instrument(skip(state, body, headers), fields(owner = %owner, repo = %repo))]
pub async fn receive_pack(
	Path((owner, repo)): Path<(String, String)>,
	OptionalAuth(auth): OptionalAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
	body: Bytes,
) -> Result<Response, ServerError> {
	let effective_user = match auth {
		Some(user) => Some(user),
		None => extract_basic_auth_user(&headers, &state).await,
	};

	let locale = effective_user
		.as_ref()
		.and_then(|u| u.user.locale.as_deref())
		.unwrap_or(&state.default_locale);

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
		return Err(ServerError::NotFound(
			t(locale, "server.api.scm.repo_not_found").to_string(),
		));
	}

	if let Err(e) = check_write_access(&scm_repo, effective_user.as_ref(), &state, locale).await {
		return match e {
			ServerError::Unauthorized(msg) => Ok(git_unauthorized_response(&msg)),
			other => Err(other),
		};
	}

	let user = match effective_user.as_ref() {
		Some(u) => u,
		None => {
			return Ok(git_unauthorized_response(&t(
				locale,
				"server.api.scm.git.auth_required",
			)))
		}
	};

	if let Some(protection_store) = state.scm_protection_store.as_ref() {
		let rules = protection_store
			.list_by_repo(scm_repo.id)
			.await
			.map_err(|e| {
				ServerError::Internal(format!(
					"{}: {e}",
					t(locale, "server.api.scm.protection.failed_to_load")
				))
			})?;

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

	Ok(
		(
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
			.into_response(),
	)
}

pub async fn git_wildcard_handler(
	axum::extract::Path(path): axum::extract::Path<String>,
	_method: axum::http::Method,
	query: Option<Query<InfoRefsParams>>,
	auth: OptionalAuth,
	state: State<AppState>,
	headers: HeaderMap,
	body: Bytes,
) -> Result<Response, ServerError> {
	let (owner, repo) = parse_mirror_git_path(&path)
		.ok_or_else(|| ServerError::BadRequest("Invalid git path".to_string()))?;

	if path.ends_with("/info/refs") {
		let query =
			query.ok_or_else(|| ServerError::BadRequest("Missing service parameter".to_string()))?;
		info_refs(Path((owner, repo)), query, auth, state, headers).await
	} else if path.ends_with("/git-upload-pack") {
		upload_pack(Path((owner, repo)), auth, state, headers, body).await
	} else if path.ends_with("/git-receive-pack") {
		receive_pack(Path((owner, repo)), auth, state, headers, body).await
	} else {
		Err(ServerError::NotFound("Unknown git endpoint".to_string()))
	}
}
