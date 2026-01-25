// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Organization-scoped secret handlers.

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
	get_secrets_service, verify_org_admin, verify_org_membership, CreateSecretRequest,
	ListSecretsResponse, SecretErrorResponse, SecretMetadataResponse, SecretSuccessResponse,
	UpdateSecretRequest,
};
use crate::{
	api::AppState,
	api_response::id_parse_error,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	validation::parse_org_id as shared_parse_org_id,
};

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/secrets",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of secrets", body = ListSecretsResponse),
        (status = 401, description = "Not authenticated", body = SecretErrorResponse),
        (status = 403, description = "Not a member", body = SecretErrorResponse)
    ),
    tag = "secrets"
)]
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_org_secrets(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = match shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	if let Err(resp) = verify_org_membership(&state, &org_id, &current_user.user.id, locale).await {
		return resp.into_response();
	}

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	let filter = SecretFilter {
		org_id: Some(org_id),
		scope: Some(SecretScope::Org { org_id }),
		..Default::default()
	};

	match service.list_secrets(&filter).await {
		Ok(secrets) => {
			let responses: Vec<SecretMetadataResponse> = secrets
				.into_iter()
				.map(|s| SecretMetadataResponse {
					name: s.name,
					scope: "org".to_string(),
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
			tracing::error!(error = %e, "Failed to list org secrets");
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
    path = "/api/orgs/{org_id}/secrets",
    params(
        ("org_id" = String, Path, description = "Organization ID")
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
#[tracing::instrument(skip(state, payload), fields(%org_id))]
pub async fn create_org_secret(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateSecretRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = match shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	if let Err(resp) = verify_org_admin(&state, &org_id, &current_user.user.id, locale).await {
		return resp.into_response();
	}

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	let input = CreateSecretInput {
		org_id,
		scope: SecretScope::Org { org_id },
		repo_id: None,
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
				scope: "org".to_string(),
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
			tracing::error!(error = %e, "Failed to create org secret");
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
    path = "/api/orgs/{org_id}/secrets/{name}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("name" = String, Path, description = "Secret name")
    ),
    responses(
        (status = 200, description = "Secret metadata", body = SecretMetadataResponse),
        (status = 401, description = "Not authenticated", body = SecretErrorResponse),
        (status = 403, description = "Not a member", body = SecretErrorResponse),
        (status = 404, description = "Secret not found", body = SecretErrorResponse)
    ),
    tag = "secrets"
)]
#[tracing::instrument(skip(state), fields(%org_id, %name))]
pub async fn get_org_secret(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = match shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	if let Err(resp) = verify_org_membership(&state, &org_id, &current_user.user.id, locale).await {
		return resp.into_response();
	}

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	match service
		.get_secret_by_name(org_id, SecretScope::Org { org_id }, None, None, &name)
		.await
	{
		Ok(Some(meta)) => {
			let response = SecretMetadataResponse {
				name: meta.name,
				scope: "org".to_string(),
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
			tracing::error!(error = %e, "Failed to get org secret");
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
    path = "/api/orgs/{org_id}/secrets/{name}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
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
#[tracing::instrument(skip(state, payload), fields(%org_id, %name))]
pub async fn update_org_secret(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, name)): Path<(String, String)>,
	Json(payload): Json<UpdateSecretRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = match shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	if let Err(resp) = verify_org_admin(&state, &org_id, &current_user.user.id, locale).await {
		return resp.into_response();
	}

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	let meta = match service
		.get_secret_by_name(org_id, SecretScope::Org { org_id }, None, None, &name)
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
				scope: "org".to_string(),
				description: meta.description,
				current_version: new_version,
				created_at: chrono::DateTime::default(),
				updated_at: chrono::DateTime::default(),
			};
			(StatusCode::OK, Json(response)).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to update org secret");
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
    path = "/api/orgs/{org_id}/secrets/{name}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
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
#[tracing::instrument(skip(state), fields(%org_id, %name))]
pub async fn delete_org_secret(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, name)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = match shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id")) {
		Ok(id) => id,
		Err(e) => return id_parse_error::<SecretErrorResponse>(e).into_response(),
	};

	if let Err(resp) = verify_org_admin(&state, &org_id, &current_user.user.id, locale).await {
		return resp.into_response();
	}

	let service = match get_secrets_service(&state, locale).await {
		Ok(svc) => svc,
		Err(resp) => return resp.into_response(),
	};

	let meta = match service
		.get_secret_by_name(org_id, SecretScope::Org { org_id }, None, None, &name)
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
			tracing::error!(error = %e, "Failed to delete org secret");
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
