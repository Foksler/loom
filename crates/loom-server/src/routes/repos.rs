// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Repository management HTTP handlers.
//!
//! Implements repository endpoints per the scm-system.md specification:
//! - Create repository
//! - Get repository by ID
//! - Update repository
//! - Soft delete repository
//! - List user's repositories
//! - List organization's repositories

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_auth::types::{OrgId, OrgRole, UserId};
use loom_server_scm::{validate_repo_name, GitRepository, OwnerType, RepoRole, RepoStore, RepoTeamAccessStore, Repository, Visibility};
use std::path::PathBuf;
use uuid::Uuid;

pub use loom_server_api::repos::*;

use crate::{api::AppState, auth_middleware::RequireAuth, i18n::{resolve_user_locale, t}};

fn get_repos_base_dir() -> PathBuf {
	std::env::var("LOOM_DATA_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|_| PathBuf::from("/var/lib/loom"))
		.join("repos")
}

fn get_repo_disk_path(repo_id: Uuid) -> PathBuf {
	let id_str = repo_id.to_string();
	let shard = &id_str[..2];
	get_repos_base_dir().join(shard).join(&id_str).join("git")
}

fn build_clone_url(base_url: &str, owner_name: &str, repo_name: &str) -> String {
	format!("{}/git/{}/{}.git", base_url.trim_end_matches('/'), owner_name, repo_name)
}

fn repo_to_response(repo: Repository, clone_url: String) -> RepoResponse {
	RepoResponse {
		id: repo.id,
		owner_type: repo.owner_type.into(),
		owner_id: repo.owner_id,
		name: repo.name,
		visibility: repo.visibility.into(),
		default_branch: repo.default_branch,
		clone_url,
		created_at: repo.created_at,
		updated_at: repo.updated_at,
	}
}

#[utoipa::path(
    post,
    path = "/api/v1/repos",
    request_body = CreateRepoRequest,
    responses(
        (status = 201, description = "Repository created", body = RepoResponse),
        (status = 400, description = "Invalid request", body = RepoErrorResponse),
        (status = 401, description = "Not authenticated", body = RepoErrorResponse),
        (status = 403, description = "Not authorized", body = RepoErrorResponse),
        (status = 409, description = "Repository already exists", body = RepoErrorResponse)
    ),
    tag = "repos"
)]
#[tracing::instrument(skip(state, payload))]
pub async fn create_repo(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Json(payload): Json<CreateRepoRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	if let Err(e) = validate_repo_name(&payload.name) {
		return (
			StatusCode::BAD_REQUEST,
			Json(RepoErrorResponse {
				error: "invalid_name".to_string(),
				message: e.to_string(),
			}),
		)
			.into_response();
	}

	let owner_type: OwnerType = payload.owner_type.clone().into();
	let owner_name: String;

	match owner_type {
		OwnerType::User => {
			if payload.owner_id != current_user.user.id.into_inner() {
				return (
					StatusCode::FORBIDDEN,
					Json(RepoErrorResponse {
						error: "forbidden".to_string(),
						message: t(locale, "server.api.scm.cannot_create_for_other_user").to_string(),
					}),
				)
					.into_response();
			}
			owner_name = current_user.user.username.clone()
				.unwrap_or_else(|| current_user.user.display_name.clone());
		}
		OwnerType::Org => {
			let org_id = OrgId::new(payload.owner_id);
			let membership = match state
				.org_repo
				.get_membership(&org_id, &current_user.user.id)
				.await
			{
				Ok(Some(m)) => m,
				Ok(None) => {
					return (
						StatusCode::FORBIDDEN,
						Json(RepoErrorResponse {
							error: "forbidden".to_string(),
							message: t(locale, "server.api.scm.not_org_member").to_string(),
						}),
					)
						.into_response();
				}
				Err(e) => {
					tracing::error!(error = %e, "Failed to check org membership");
					return (
						StatusCode::INTERNAL_SERVER_ERROR,
						Json(RepoErrorResponse {
							error: "internal_error".to_string(),
							message: t(locale, "server.api.scm.internal_error").to_string(),
						}),
					)
						.into_response();
				}
			};

			if membership.role != OrgRole::Owner && membership.role != OrgRole::Admin {
				return (
					StatusCode::FORBIDDEN,
					Json(RepoErrorResponse {
						error: "forbidden".to_string(),
						message: t(locale, "server.api.scm.org_admin_required").to_string(),
					}),
				)
					.into_response();
			}

			let org = match state.org_repo.get_org_by_id(&org_id).await {
				Ok(Some(o)) => o,
				Ok(None) => {
					return (
						StatusCode::NOT_FOUND,
						Json(RepoErrorResponse {
							error: "not_found".to_string(),
							message: t(locale, "server.api.scm.org_not_found").to_string(),
						}),
					)
						.into_response();
				}
				Err(e) => {
					tracing::error!(error = %e, "Failed to get organization");
					return (
						StatusCode::INTERNAL_SERVER_ERROR,
						Json(RepoErrorResponse {
							error: "internal_error".to_string(),
							message: t(locale, "server.api.scm.internal_error").to_string(),
						}),
					)
						.into_response();
				}
			};
			owner_name = org.slug;
		}
	}

	let repo = Repository::new(
		owner_type,
		payload.owner_id,
		payload.name.clone(),
		payload.visibility.into(),
	);

	let created_repo = match scm_store.create(&repo).await {
		Ok(r) => r,
		Err(loom_server_scm::ScmError::AlreadyExists) => {
			return (
				StatusCode::CONFLICT,
				Json(RepoErrorResponse {
					error: "already_exists".to_string(),
					message: t(locale, "server.api.scm.repo_already_exists").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to create repository");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.failed_to_create_repo").to_string(),
				}),
			)
				.into_response();
		}
	};

	let git_path = get_repo_disk_path(created_repo.id);
	if let Err(e) = std::fs::create_dir_all(git_path.parent().unwrap()) {
		tracing::error!(error = %e, "Failed to create repo directory");
		let _ = scm_store.hard_delete(created_repo.id).await;
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(RepoErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.scm.failed_to_init_repo_disk").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = GitRepository::init_bare(&git_path) {
		tracing::error!(error = %e, "Failed to init bare git repo");
		let _ = scm_store.hard_delete(created_repo.id).await;
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(RepoErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.scm.failed_to_init_git_repo").to_string(),
			}),
		)
			.into_response();
	}

	if let Ok(git_repo) = GitRepository::open(&git_path) {
		let _ = git_repo.set_default_branch(&created_repo.default_branch);
	}

	tracing::info!(
		repo_id = %created_repo.id,
		name = %created_repo.name,
		owner_type = ?created_repo.owner_type,
		created_by = %current_user.user.id,
		"Repository created"
	);

	let clone_url = build_clone_url(&state.base_url, &owner_name, &created_repo.name);
	let _ = locale;

	(
		StatusCode::CREATED,
		Json(repo_to_response(created_repo, clone_url)),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/repos/{id}",
    params(
        ("id" = Uuid, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Repository details", body = RepoResponse),
        (status = 401, description = "Not authenticated", body = RepoErrorResponse),
        (status = 403, description = "Not authorized", body = RepoErrorResponse),
        (status = 404, description = "Repository not found", body = RepoErrorResponse)
    ),
    tag = "repos"
)]
#[tracing::instrument(skip(state), fields(%id))]
pub async fn get_repo(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let repo = match scm_store.get_by_id(id).await {
		Ok(Some(r)) => r,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(RepoErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.scm.repo_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get repository");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response();
		}
	};

	if repo.visibility == Visibility::Private {
		let has_access = match repo.owner_type {
			OwnerType::User => repo.owner_id == current_user.user.id.into_inner(),
			OwnerType::Org => {
				let org_id = OrgId::new(repo.owner_id);
				matches!(
					state.org_repo.get_membership(&org_id, &current_user.user.id).await,
					Ok(Some(_))
				)
			}
		};

		if !has_access {
			return (
				StatusCode::FORBIDDEN,
				Json(RepoErrorResponse {
					error: "forbidden".to_string(),
					message: t(locale, "server.api.scm.access_denied").to_string(),
				}),
			)
				.into_response();
		}
	}

	let owner_name = match repo.owner_type {
		OwnerType::User => {
			match state.user_repo.get_user_by_id(&UserId::new(repo.owner_id)).await {
				Ok(Some(u)) => u.username.unwrap_or(u.display_name),
				_ => "unknown".to_string(),
			}
		}
		OwnerType::Org => {
			match state.org_repo.get_org_by_id(&OrgId::new(repo.owner_id)).await {
				Ok(Some(o)) => o.slug,
				_ => "unknown".to_string(),
			}
		}
	};

	let clone_url = build_clone_url(&state.base_url, &owner_name, &repo.name);
	let _ = locale;

	(StatusCode::OK, Json(repo_to_response(repo, clone_url))).into_response()
}

#[utoipa::path(
    patch,
    path = "/api/v1/repos/{id}",
    params(
        ("id" = Uuid, Path, description = "Repository ID")
    ),
    request_body = UpdateRepoRequest,
    responses(
        (status = 200, description = "Repository updated", body = RepoResponse),
        (status = 400, description = "Invalid request", body = RepoErrorResponse),
        (status = 401, description = "Not authenticated", body = RepoErrorResponse),
        (status = 403, description = "Not authorized", body = RepoErrorResponse),
        (status = 404, description = "Repository not found", body = RepoErrorResponse)
    ),
    tag = "repos"
)]
#[tracing::instrument(skip(state, payload), fields(%id))]
pub async fn update_repo(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	Json(payload): Json<UpdateRepoRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let mut repo = match scm_store.get_by_id(id).await {
		Ok(Some(r)) => r,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(RepoErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.scm.repo_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get repository");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response();
		}
	};

	let is_admin = match repo.owner_type {
		OwnerType::User => repo.owner_id == current_user.user.id.into_inner(),
		OwnerType::Org => {
			let org_id = OrgId::new(repo.owner_id);
			match state.org_repo.get_membership(&org_id, &current_user.user.id).await {
				Ok(Some(m)) => m.role == OrgRole::Owner || m.role == OrgRole::Admin,
				_ => false,
			}
		}
	};

	if !is_admin {
		return (
			StatusCode::FORBIDDEN,
			Json(RepoErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.scm.admin_required").to_string(),
			}),
		)
			.into_response();
	}

	if let Some(name) = payload.name {
		if name.is_empty() || name.len() > 100 {
			return (
				StatusCode::BAD_REQUEST,
				Json(RepoErrorResponse {
					error: "invalid_name".to_string(),
					message: t(locale, "server.api.scm.invalid_repo_name").to_string(),
				}),
			)
				.into_response();
		}
		repo.name = name;
	}

	if let Some(visibility) = payload.visibility {
		repo.visibility = visibility.into();
	}

	if let Some(default_branch) = payload.default_branch {
		if default_branch.is_empty() {
			return (
				StatusCode::BAD_REQUEST,
				Json(RepoErrorResponse {
					error: "invalid_branch".to_string(),
					message: t(locale, "server.api.scm.invalid_default_branch").to_string(),
				}),
			)
				.into_response();
		}
		repo.default_branch = default_branch.clone();

		let git_path = get_repo_disk_path(repo.id);
		if let Ok(git_repo) = GitRepository::open(&git_path) {
			let _ = git_repo.set_default_branch(&default_branch);
		}
	}

	let updated_repo = match scm_store.update(&repo).await {
		Ok(r) => r,
		Err(e) => {
			tracing::error!(error = %e, "Failed to update repository");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.failed_to_update_repo").to_string(),
				}),
			)
				.into_response();
		}
	};

	let owner_name = match updated_repo.owner_type {
		OwnerType::User => {
			match state.user_repo.get_user_by_id(&UserId::new(updated_repo.owner_id)).await {
				Ok(Some(u)) => u.username.unwrap_or(u.display_name),
				_ => "unknown".to_string(),
			}
		}
		OwnerType::Org => {
			match state.org_repo.get_org_by_id(&OrgId::new(updated_repo.owner_id)).await {
				Ok(Some(o)) => o.slug,
				_ => "unknown".to_string(),
			}
		}
	};

	tracing::info!(
		repo_id = %updated_repo.id,
		updated_by = %current_user.user.id,
		"Repository updated"
	);

	let clone_url = build_clone_url(&state.base_url, &owner_name, &updated_repo.name);
	let _ = locale;

	(StatusCode::OK, Json(repo_to_response(updated_repo, clone_url))).into_response()
}

#[utoipa::path(
    delete,
    path = "/api/v1/repos/{id}",
    params(
        ("id" = Uuid, Path, description = "Repository ID")
    ),
    responses(
        (status = 204, description = "Repository deleted"),
        (status = 401, description = "Not authenticated", body = RepoErrorResponse),
        (status = 403, description = "Not authorized", body = RepoErrorResponse),
        (status = 404, description = "Repository not found", body = RepoErrorResponse)
    ),
    tag = "repos"
)]
#[tracing::instrument(skip(state), fields(%id))]
pub async fn delete_repo(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let repo = match scm_store.get_by_id(id).await {
		Ok(Some(r)) => r,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(RepoErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.scm.repo_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get repository");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response();
		}
	};

	let is_admin = match repo.owner_type {
		OwnerType::User => repo.owner_id == current_user.user.id.into_inner(),
		OwnerType::Org => {
			let org_id = OrgId::new(repo.owner_id);
			match state.org_repo.get_membership(&org_id, &current_user.user.id).await {
				Ok(Some(m)) => m.role == OrgRole::Owner || m.role == OrgRole::Admin,
				_ => false,
			}
		}
	};

	if !is_admin {
		return (
			StatusCode::FORBIDDEN,
			Json(RepoErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.scm.admin_required").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = scm_store.soft_delete(id).await {
		tracing::error!(error = %e, "Failed to delete repository");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(RepoErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.scm.failed_to_delete_repo").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(
		repo_id = %id,
		deleted_by = %current_user.user.id,
		"Repository soft deleted"
	);

	StatusCode::NO_CONTENT.into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/repos",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "List of user's repositories", body = ListReposResponse),
        (status = 401, description = "Not authenticated", body = RepoErrorResponse),
        (status = 404, description = "User not found", body = RepoErrorResponse)
    ),
    tag = "repos"
)]
#[tracing::instrument(skip(state), fields(user_id = %id))]
pub async fn list_user_repos(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let target_user = match state.user_repo.get_user_by_id(&UserId::new(id)).await {
		Ok(Some(u)) => u,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(RepoErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.scm.user_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get user");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response();
		}
	};

	let repos = match scm_store.list_by_owner(OwnerType::User, id).await {
		Ok(r) => r,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list repositories");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.failed_to_list_repos").to_string(),
				}),
			)
				.into_response();
		}
	};

	let is_owner = id == current_user.user.id.into_inner();
	let owner_name = target_user.username.as_ref().unwrap_or(&target_user.display_name);
	let visible_repos: Vec<_> = repos
		.into_iter()
		.filter(|r| r.visibility == Visibility::Public || is_owner)
		.map(|r| {
			let clone_url = build_clone_url(&state.base_url, owner_name, &r.name);
			repo_to_response(r, clone_url)
		})
		.collect();

	(StatusCode::OK, Json(ListReposResponse { repos: visible_repos })).into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/orgs/{id}/repos",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of organization's repositories", body = ListReposResponse),
        (status = 401, description = "Not authenticated", body = RepoErrorResponse),
        (status = 404, description = "Organization not found", body = RepoErrorResponse)
    ),
    tag = "repos"
)]
#[tracing::instrument(skip(state), fields(org_id = %id))]
pub async fn list_org_repos(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let org_id = OrgId::new(id);
	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(o)) => o,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(RepoErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.scm.org_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get organization");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response();
		}
	};

	let is_member = matches!(
		state.org_repo.get_membership(&org_id, &current_user.user.id).await,
		Ok(Some(_))
	);

	let repos = match scm_store.list_by_owner(OwnerType::Org, id).await {
		Ok(r) => r,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list repositories");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.failed_to_list_repos").to_string(),
				}),
			)
				.into_response();
		}
	};

	let visible_repos: Vec<_> = repos
		.into_iter()
		.filter(|r| r.visibility == Visibility::Public || is_member)
		.map(|r| {
			let clone_url = build_clone_url(&state.base_url, &org.slug, &r.name);
			repo_to_response(r, clone_url)
		})
		.collect();

	(StatusCode::OK, Json(ListReposResponse { repos: visible_repos })).into_response()
}

async fn check_repo_admin_access(
	current_user: &loom_server_auth::middleware::CurrentUser,
	repo: &Repository,
	state: &AppState,
	locale: &str,
) -> Result<(), (StatusCode, Json<RepoErrorResponse>)> {
	let is_admin = match repo.owner_type {
		OwnerType::User => repo.owner_id == current_user.user.id.into_inner(),
		OwnerType::Org => {
			let org_id = OrgId::new(repo.owner_id);
			match state.org_repo.get_membership(&org_id, &current_user.user.id).await {
				Ok(Some(m)) => m.role == OrgRole::Owner || m.role == OrgRole::Admin,
				_ => false,
			}
		}
	};

	if !is_admin {
		if let Some(store) = &state.scm_team_access_store {
			if let Ok(Some(role)) = store.get_user_role_via_teams(current_user.user.id.into_inner(), repo.id).await {
				if role == RepoRole::Admin {
					return Ok(());
				}
			}
		}
		return Err((
			StatusCode::FORBIDDEN,
			Json(RepoErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.scm.admin_required").to_string(),
			}),
		));
	}

	Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/repos/{id}/teams",
    params(
        ("id" = Uuid, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "List of teams with access", body = ListRepoTeamAccessResponse),
        (status = 401, description = "Not authenticated", body = RepoErrorResponse),
        (status = 403, description = "Not authorized", body = RepoErrorResponse),
        (status = 404, description = "Repository not found", body = RepoErrorResponse)
    ),
    tag = "repos"
)]
#[tracing::instrument(skip(state), fields(repo_id = %id))]
pub async fn list_repo_team_access(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let team_access_store = match state.scm_team_access_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let repo = match scm_store.get_by_id(id).await {
		Ok(Some(r)) => r,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(RepoErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.scm.repo_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get repository");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response();
		}
	};

	if let Err(resp) = check_repo_admin_access(&current_user, &repo, &state, locale).await {
		return resp.into_response();
	}

	match team_access_store.list_repo_team_access(id).await {
		Ok(access_list) => {
			let teams = access_list
				.into_iter()
				.map(|a| RepoTeamAccessResponse {
					team_id: a.team_id,
					role: a.role.into(),
				})
				.collect();
			(StatusCode::OK, Json(ListRepoTeamAccessResponse { teams })).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list repo team access");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
    post,
    path = "/api/v1/repos/{id}/teams",
    params(
        ("id" = Uuid, Path, description = "Repository ID")
    ),
    request_body = GrantTeamAccessRequest,
    responses(
        (status = 200, description = "Team access granted", body = RepoSuccessResponse),
        (status = 401, description = "Not authenticated", body = RepoErrorResponse),
        (status = 403, description = "Not authorized", body = RepoErrorResponse),
        (status = 404, description = "Repository not found", body = RepoErrorResponse)
    ),
    tag = "repos"
)]
#[tracing::instrument(skip(state, payload), fields(repo_id = %id))]
pub async fn grant_repo_team_access(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	Json(payload): Json<GrantTeamAccessRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let team_access_store = match state.scm_team_access_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let repo = match scm_store.get_by_id(id).await {
		Ok(Some(r)) => r,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(RepoErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.scm.repo_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get repository");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response();
		}
	};

	if let Err(resp) = check_repo_admin_access(&current_user, &repo, &state, locale).await {
		return resp.into_response();
	}

	let role: RepoRole = payload.role.into();
	match team_access_store.grant_team_access(id, payload.team_id, role).await {
		Ok(()) => {
			tracing::info!(
				repo_id = %id,
				team_id = %payload.team_id,
				role = %role.as_str(),
				granted_by = %current_user.user.id,
				"Team access granted to repository"
			);
			(
				StatusCode::OK,
				Json(RepoSuccessResponse {
					message: "Team access granted".to_string(),
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to grant team access");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
    delete,
    path = "/api/v1/repos/{id}/teams/{tid}",
    params(
        ("id" = Uuid, Path, description = "Repository ID"),
        ("tid" = Uuid, Path, description = "Team ID")
    ),
    responses(
        (status = 204, description = "Team access revoked"),
        (status = 401, description = "Not authenticated", body = RepoErrorResponse),
        (status = 403, description = "Not authorized", body = RepoErrorResponse),
        (status = 404, description = "Repository or team access not found", body = RepoErrorResponse)
    ),
    tag = "repos"
)]
#[tracing::instrument(skip(state), fields(repo_id = %id, team_id = %tid))]
pub async fn revoke_repo_team_access(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((id, tid)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let team_access_store = match state.scm_team_access_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	let repo = match scm_store.get_by_id(id).await {
		Ok(Some(r)) => r,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(RepoErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.scm.repo_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get repository");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response();
		}
	};

	if let Err(resp) = check_repo_admin_access(&current_user, &repo, &state, locale).await {
		return resp.into_response();
	}

	match team_access_store.revoke_team_access(id, tid).await {
		Ok(()) => {
			tracing::info!(
				repo_id = %id,
				team_id = %tid,
				revoked_by = %current_user.user.id,
				"Team access revoked from repository"
			);
			StatusCode::NO_CONTENT.into_response()
		}
		Err(loom_server_scm::ScmError::NotFound) => (
			StatusCode::NOT_FOUND,
			Json(RepoErrorResponse {
				error: "not_found".to_string(),
				message: "Team access not found".to_string(),
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to revoke team access");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(RepoErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.scm.internal_error").to_string(),
				}),
			)
				.into_response()
		}
	}
}
