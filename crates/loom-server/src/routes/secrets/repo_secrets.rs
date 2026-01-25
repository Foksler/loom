// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Repository-scoped secret handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_common_secret::SecretString;
use loom_server_secrets::store::SecretFilter;
use loom_server_secrets::{CreateSecretInput, SecretScope};

use super::common::{
	get_secrets_service, verify_repo_access, CreateSecretRequest, ListSecretsResponse,
	SecretErrorResponse, SecretMetadataResponse, SecretSuccessResponse, UpdateSecretRequest,
};
use crate::{
	api::AppState,
	api_response::id_parse_error,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	validation::parse_uuid,
};

#[utoipa::path(
    get,
    path = "/api/repos/{repo_id}/secrets",
    params(
        ("repo_id" = String, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "List of secrets", body = ListSecretsResponse),
        (status = 401, description = "Not authenticated", body = SecretErrorResponse),
        (status = 403, description = "Not authorized", body = SecretErrorResponse),
        (status = 404, description = "Repository not found", body = SecretErrorResponse)
    ),
    tag = "secrets"
)]
#[tracing::instrument(skip(state), fields(%repo_id))]
pub async fn list_repo_secrets(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(repo_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let repo_id = match parse_uuid(&repo_id, &t(locale, "server.api.repo.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	let org_id = match verify_repo_access(&state, repo_id, &current_user.user.id, locale).await {
		Ok(org_id) => org_id,
		Err(resp) => return resp.into_response(),
	};

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	let filter = SecretFilter {
		org_id: Some(org_id),
		repo_id: Some(repo_id),
		..Default::default()
	};

	match service.list_secrets(&filter).await {
		Ok(secrets) => {
			let responses: Vec<SecretMetadataResponse> = secrets
				.into_iter()
				.map(|s| SecretMetadataResponse {
					name: s.name,
					scope: "repo".to_string(),
					description: s.description,
					current_version: s.current_version,
					created_at: chrono::DateTime::default(),
					updated_at: chrono::DateTime::default(),
				})
				.collect();
			(
				StatusCode::OK,
				Json(ListSecretsResponse { secrets: responses }),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list repo secrets");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
    post,
    path = "/api/repos/{repo_id}/secrets",
    params(
        ("repo_id" = String, Path, description = "Repository ID")
    ),
    request_body = CreateSecretRequest,
    responses(
        (status = 201, description = "Secret created", body = SecretMetadataResponse),
        (status = 400, description = "Invalid request", body = SecretErrorResponse),
        (status = 401, description = "Not authenticated", body = SecretErrorResponse),
        (status = 403, description = "Not authorized", body = SecretErrorResponse),
        (status = 409, description = "Secret already exists", body = SecretErrorResponse)
    ),
    tag = "secrets"
)]
#[tracing::instrument(skip(state, payload), fields(%repo_id))]
pub async fn create_repo_secret(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(repo_id): Path<String>,
	Json(payload): Json<CreateSecretRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let repo_id = match parse_uuid(&repo_id, &t(locale, "server.api.repo.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	let org_id = match verify_repo_access(&state, repo_id, &current_user.user.id, locale).await {
		Ok(org_id) => org_id,
		Err(resp) => return resp.into_response(),
	};

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	let input = CreateSecretInput {
		org_id,
		scope: SecretScope::Repo {
			org_id,
			repo_id: repo_id.to_string(),
		},
		repo_id: Some(repo_id),
		weaver_id: None,
		name: payload.name,
		value: SecretString::new(payload.value),
		description: payload.description,
		created_by: current_user.user.id,
	};

	match service.create_secret(input).await {
		Ok(meta) => {
			let response = SecretMetadataResponse {
				name: meta.name,
				scope: "repo".to_string(),
				description: meta.description,
				current_version: meta.current_version,
				created_at: chrono::DateTime::default(),
				updated_at: chrono::DateTime::default(),
			};
			(StatusCode::CREATED, Json(response)).into_response()
		}
		Err(loom_server_secrets::SecretsError::SecretAlreadyExists(_)) => (
			StatusCode::CONFLICT,
			Json(SecretErrorResponse {
				error: "already_exists".to_string(),
				message: t(locale, "server.api.secrets.already_exists").to_string(),
			}),
		)
			.into_response(),
		Err(loom_server_secrets::SecretsError::InvalidSecretName(msg)) => (
			StatusCode::BAD_REQUEST,
			Json(SecretErrorResponse {
				error: "invalid_name".to_string(),
				message: msg,
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to create repo secret");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
    get,
    path = "/api/repos/{repo_id}/secrets/{name}",
    params(
        ("repo_id" = String, Path, description = "Repository ID"),
        ("name" = String, Path, description = "Secret name")
    ),
    responses(
        (status = 200, description = "Secret metadata", body = SecretMetadataResponse),
        (status = 401, description = "Not authenticated", body = SecretErrorResponse),
        (status = 403, description = "Not authorized", body = SecretErrorResponse),
        (status = 404, description = "Secret not found", body = SecretErrorResponse)
    ),
    tag = "secrets"
)]
#[tracing::instrument(skip(state), fields(%repo_id, %name))]
pub async fn get_repo_secret(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((repo_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let repo_id = match parse_uuid(&repo_id, &t(locale, "server.api.repo.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	let org_id = match verify_repo_access(&state, repo_id, &current_user.user.id, locale).await {
		Ok(org_id) => org_id,
		Err(resp) => return resp.into_response(),
	};

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	let scope = SecretScope::Repo {
		org_id,
		repo_id: repo_id.to_string(),
	};

	match service
		.get_secret_by_name(org_id, scope, Some(repo_id), None, &name)
		.await
	{
		Ok(Some(meta)) => {
			let response = SecretMetadataResponse {
				name: meta.name,
				scope: "repo".to_string(),
				description: meta.description,
				current_version: meta.current_version,
				created_at: chrono::DateTime::default(),
				updated_at: chrono::DateTime::default(),
			};
			(StatusCode::OK, Json(response)).into_response()
		}
		Ok(None) => (
			StatusCode::NOT_FOUND,
			Json(SecretErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.secrets.not_found").to_string(),
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get repo secret");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
    put,
    path = "/api/repos/{repo_id}/secrets/{name}",
    params(
        ("repo_id" = String, Path, description = "Repository ID"),
        ("name" = String, Path, description = "Secret name")
    ),
    request_body = UpdateSecretRequest,
    responses(
        (status = 200, description = "Secret updated", body = SecretMetadataResponse),
        (status = 401, description = "Not authenticated", body = SecretErrorResponse),
        (status = 403, description = "Not authorized", body = SecretErrorResponse),
        (status = 404, description = "Secret not found", body = SecretErrorResponse)
    ),
    tag = "secrets"
)]
#[tracing::instrument(skip(state, payload), fields(%repo_id, %name))]
pub async fn update_repo_secret(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((repo_id, name)): Path<(String, String)>,
	Json(payload): Json<UpdateSecretRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let repo_id = match parse_uuid(&repo_id, &t(locale, "server.api.repo.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	let org_id = match verify_repo_access(&state, repo_id, &current_user.user.id, locale).await {
		Ok(org_id) => org_id,
		Err(resp) => return resp.into_response(),
	};

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	let scope = SecretScope::Repo {
		org_id,
		repo_id: repo_id.to_string(),
	};

	let meta = match service
		.get_secret_by_name(org_id, scope, Some(repo_id), None, &name)
		.await
	{
		Ok(Some(m)) => m,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(SecretErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.secrets.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get secret for update");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	match service
		.rotate_secret(
			meta.id,
			SecretString::new(payload.value),
			current_user.user.id,
		)
		.await
	{
		Ok(new_version) => {
			let response = SecretMetadataResponse {
				name: meta.name,
				scope: "repo".to_string(),
				description: meta.description,
				current_version: new_version,
				created_at: chrono::DateTime::default(),
				updated_at: chrono::DateTime::default(),
			};
			(StatusCode::OK, Json(response)).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to update repo secret");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
    delete,
    path = "/api/repos/{repo_id}/secrets/{name}",
    params(
        ("repo_id" = String, Path, description = "Repository ID"),
        ("name" = String, Path, description = "Secret name")
    ),
    responses(
        (status = 200, description = "Secret deleted", body = SecretSuccessResponse),
        (status = 401, description = "Not authenticated", body = SecretErrorResponse),
        (status = 403, description = "Not authorized", body = SecretErrorResponse),
        (status = 404, description = "Secret not found", body = SecretErrorResponse)
    ),
    tag = "secrets"
)]
#[tracing::instrument(skip(state), fields(%repo_id, %name))]
pub async fn delete_repo_secret(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((repo_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let repo_id = match parse_uuid(&repo_id, &t(locale, "server.api.repo.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	let org_id = match verify_repo_access(&state, repo_id, &current_user.user.id, locale).await {
		Ok(org_id) => org_id,
		Err(resp) => return resp.into_response(),
	};

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	let scope = SecretScope::Repo {
		org_id,
		repo_id: repo_id.to_string(),
	};

	let meta = match service
		.get_secret_by_name(org_id, scope, Some(repo_id), None, &name)
		.await
	{
		Ok(Some(m)) => m,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(SecretErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.secrets.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get secret for delete");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	match service.delete_secret(meta.id, current_user.user.id).await {
		Ok(()) => (
			StatusCode::OK,
			Json(SecretSuccessResponse {
				message: t(locale, "server.api.secrets.deleted").to_string(),
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to delete repo secret");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}
