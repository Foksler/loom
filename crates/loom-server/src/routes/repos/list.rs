// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Repository list handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_auth::types::{OrgId, UserId};
use loom_server_scm::{OwnerType, RepoStore, Visibility};
use uuid::Uuid;

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{build_clone_url, repo_to_response, ListReposResponse, RepoErrorResponse};

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
	let owner_name = target_user
		.username
		.as_ref()
		.unwrap_or(&target_user.display_name);
	let visible_repos: Vec<_> = repos
		.into_iter()
		.filter(|r| r.visibility == Visibility::Public || is_owner)
		.map(|r| {
			let clone_url = build_clone_url(&state.base_url, owner_name, &r.name);
			repo_to_response(r, clone_url)
		})
		.collect();

	(
		StatusCode::OK,
		Json(ListReposResponse {
			repos: visible_repos,
		}),
	)
		.into_response()
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
		state
			.org_repo
			.get_membership(&org_id, &current_user.user.id)
			.await,
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

	(
		StatusCode::OK,
		Json(ListReposResponse {
			repos: visible_repos,
		}),
	)
		.into_response()
}
