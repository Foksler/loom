// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git repository access control.

use loom_server_auth::middleware::CurrentUser;
use loom_server_auth::types::{OrgId, OrgRole};
use loom_server_scm::{OwnerType, RepoRole, RepoTeamAccessStore, Repository, Visibility};

use crate::{api::AppState, error::ServerError, i18n::t};

fn higher_role(a: Option<RepoRole>, b: Option<RepoRole>) -> Option<RepoRole> {
	match (a, b) {
		(Some(x), Some(y)) => {
			if x.has_permission_of(&y) {
				Some(x)
			} else {
				Some(y)
			}
		}
		(Some(x), None) => Some(x),
		(None, Some(y)) => Some(y),
		(None, None) => None,
	}
}

async fn get_direct_role(
	user: &CurrentUser,
	repo: &Repository,
	state: &AppState,
) -> Option<RepoRole> {
	match repo.owner_type {
		OwnerType::User => {
			if repo.owner_id == user.user.id.into_inner() {
				Some(RepoRole::Admin)
			} else {
				None
			}
		}
		OwnerType::Org => {
			let org_id = OrgId::new(repo.owner_id);
			match state.org_repo.get_membership(&org_id, &user.user.id).await {
				Ok(Some(m)) => {
					if m.role == OrgRole::Owner || m.role == OrgRole::Admin {
						Some(RepoRole::Admin)
					} else {
						Some(RepoRole::Read)
					}
				}
				_ => None,
			}
		}
	}
}

pub async fn get_user_repo_role(
	user: &CurrentUser,
	repo: &Repository,
	state: &AppState,
) -> Option<RepoRole> {
	let direct_role = get_direct_role(user, repo, state).await;

	let team_role = match &state.scm_team_access_store {
		Some(store) => store
			.get_user_role_via_teams(user.user.id.into_inner(), repo.id)
			.await
			.ok()
			.flatten(),
		None => None,
	};

	higher_role(direct_role, team_role)
}

pub async fn check_read_access(
	repo: &Repository,
	user: Option<&CurrentUser>,
	state: &AppState,
	locale: &str,
) -> Result<(), ServerError> {
	if repo.visibility == Visibility::Public {
		return Ok(());
	}

	let user = user.ok_or_else(|| {
		ServerError::Unauthorized(t(locale, "server.api.scm.git.auth_required_private").to_string())
	})?;

	let role = get_user_repo_role(user, repo, state).await;

	if role
		.map(|r| r.has_permission_of(&RepoRole::Read))
		.unwrap_or(false)
	{
		Ok(())
	} else {
		Err(ServerError::Forbidden(
			t(locale, "server.api.scm.git.access_denied").to_string(),
		))
	}
}

pub async fn check_write_access(
	repo: &Repository,
	user: Option<&CurrentUser>,
	state: &AppState,
	locale: &str,
) -> Result<(), ServerError> {
	let user = user.ok_or_else(|| {
		ServerError::Unauthorized(t(locale, "server.api.scm.git.auth_required_push").to_string())
	})?;

	let role = get_user_repo_role(user, repo, state).await;

	if role
		.map(|r| r.has_permission_of(&RepoRole::Write))
		.unwrap_or(false)
	{
		Ok(())
	} else {
		Err(ServerError::Forbidden(
			t(locale, "server.api.scm.git.write_access_denied").to_string(),
		))
	}
}

pub async fn check_user_is_repo_admin(repo: &Repository, user: &CurrentUser, state: &AppState) -> bool {
	let role = get_user_repo_role(user, repo, state).await;
	role.map(|r| r == RepoRole::Admin).unwrap_or(false)
}
