// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Repository CRUD handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::types::{OrgId, OrgRole, UserId};
use loom_server_scm::{validate_repo_name, GitRepository, OwnerType, RepoStore, Repository};
use uuid::Uuid;

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{
	build_clone_url, check_repo_admin_access, generate_initial_readme, get_repo_disk_path,
	repo_to_response, CreateRepoRequest, RepoErrorResponse, RepoResponse, UpdateRepoRequest,
};

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
			owner_name = current_user
				.user
				.username
				.clone()
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

	let git_repo = match GitRepository::init_bare(&git_path) {
		Ok(repo) => repo,
		Err(e) => {
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
	};

	let _ = git_repo.set_default_branch(&created_repo.default_branch);

	// Create initial commit with README.md
	let readme_content = generate_initial_readme(&created_repo.name);
	if let Err(e) = git_repo.create_initial_commit(
		&created_repo.default_branch,
		"README.md",
		readme_content.as_bytes(),
		"Initial commit\n\nCreated repository with README.md",
		&current_user.user.display_name,
		current_user
			.user
			.primary_email
			.as_deref()
			.unwrap_or("noreply@loom.dev"),
	) {
		tracing::warn!(error = %e, "Failed to create initial commit (repo still usable)");
	}

	tracing::info!(
		repo_id = %created_repo.id,
		name = %created_repo.name,
		owner_type = ?created_repo.owner_type,
		created_by = %current_user.user.id,
		"Repository created"
	);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::RepoCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("repo", created_repo.id.to_string())
			.details(serde_json::json!({
				"name": created_repo.name,
				"owner_type": format!("{:?}", created_repo.owner_type),
				"owner_id": created_repo.owner_id.to_string(),
				"visibility": format!("{:?}", created_repo.visibility),
			}))
			.build(),
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
	use loom_server_scm::Visibility;

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
					state
						.org_repo
						.get_membership(&org_id, &current_user.user.id)
						.await,
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
			match state
				.user_repo
				.get_user_by_id(&UserId::new(repo.owner_id))
				.await
			{
				Ok(Some(u)) => u.username.unwrap_or(u.display_name),
				_ => "unknown".to_string(),
			}
		}
		OwnerType::Org => {
			match state
				.org_repo
				.get_org_by_id(&OrgId::new(repo.owner_id))
				.await
			{
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

	if let Err(resp) = check_repo_admin_access(&current_user, &repo, &state, locale).await {
		return resp.into_response();
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
			match state
				.user_repo
				.get_user_by_id(&UserId::new(updated_repo.owner_id))
				.await
			{
				Ok(Some(u)) => u.username.unwrap_or(u.display_name),
				_ => "unknown".to_string(),
			}
		}
		OwnerType::Org => {
			match state
				.org_repo
				.get_org_by_id(&OrgId::new(updated_repo.owner_id))
				.await
			{
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

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::OrgUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("repo", updated_repo.id.to_string())
			.details(serde_json::json!({
				"name": updated_repo.name,
				"visibility": format!("{:?}", updated_repo.visibility),
				"default_branch": updated_repo.default_branch,
			}))
			.build(),
	);

	let clone_url = build_clone_url(&state.base_url, &owner_name, &updated_repo.name);
	let _ = locale;

	(
		StatusCode::OK,
		Json(repo_to_response(updated_repo, clone_url)),
	)
		.into_response()
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

	if let Err(resp) = check_repo_admin_access(&current_user, &repo, &state, locale).await {
		return resp.into_response();
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

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::RepoDeleted)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("repo", id.to_string())
			.details(serde_json::json!({
				"name": repo.name,
				"owner_type": format!("{:?}", repo.owner_type),
				"owner_id": repo.owner_id.to_string(),
			}))
			.build(),
	);

	StatusCode::NO_CONTENT.into_response()
}
