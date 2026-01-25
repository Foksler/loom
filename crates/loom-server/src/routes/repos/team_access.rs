// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Repository team access handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_scm::{RepoRole, RepoStore, RepoTeamAccessStore};
use uuid::Uuid;

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{
	check_repo_admin_access, GrantTeamAccessRequest, ListRepoTeamAccessResponse, RepoErrorResponse,
	RepoSuccessResponse, RepoTeamAccessResponse,
};

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
	match team_access_store
		.grant_team_access(id, payload.team_id, role)
		.await
	{
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
