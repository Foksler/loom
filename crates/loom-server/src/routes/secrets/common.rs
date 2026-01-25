// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for secrets handlers.

use axum::{http::StatusCode, Json};
use loom_server_auth::types::{OrgId, OrgRole, UserId};
use loom_server_scm::RepoStore;
use loom_server_secrets::{SecretsService, SoftwareKeyBackend, SqliteSecretStore};
use std::sync::Arc;
use uuid::Uuid;

pub use loom_server_api::secrets::*;

use crate::{api::AppState, i18n::t, impl_api_error_response};

impl_api_error_response!(SecretErrorResponse);

pub async fn get_secrets_service(
	state: &AppState,
	locale: &str,
) -> Result<
	Arc<SecretsService<SoftwareKeyBackend, SqliteSecretStore>>,
	(StatusCode, Json<SecretErrorResponse>),
> {
	match state.secrets_service.as_ref() {
		Some(svc) => Ok(svc.clone()),
		None => Err((
			StatusCode::SERVICE_UNAVAILABLE,
			Json(SecretErrorResponse {
				error: "not_configured".to_string(),
				message: t(locale, "server.api.secrets.not_configured").to_string(),
			}),
		)),
	}
}

pub async fn verify_org_membership(
	state: &AppState,
	org_id: &OrgId,
	user_id: &UserId,
	locale: &str,
) -> Result<(), (StatusCode, Json<SecretErrorResponse>)> {
	match state.org_repo.get_membership(org_id, user_id).await {
		Ok(Some(_)) => Ok(()),
		Ok(None) => Err((
			StatusCode::FORBIDDEN,
			Json(SecretErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.not_a_member").to_string(),
			}),
		)),
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			))
		}
	}
}

pub async fn verify_org_admin(
	state: &AppState,
	org_id: &OrgId,
	user_id: &UserId,
	locale: &str,
) -> Result<(), (StatusCode, Json<SecretErrorResponse>)> {
	match state.org_repo.get_membership(org_id, user_id).await {
		Ok(Some(m)) if m.role == OrgRole::Owner || m.role == OrgRole::Admin => Ok(()),
		Ok(Some(_)) => Err((
			StatusCode::FORBIDDEN,
			Json(SecretErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.admin_required").to_string(),
			}),
		)),
		Ok(None) => Err((
			StatusCode::FORBIDDEN,
			Json(SecretErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.not_a_member").to_string(),
			}),
		)),
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			))
		}
	}
}

pub async fn verify_repo_access(
	state: &AppState,
	repo_id: Uuid,
	user_id: &UserId,
	locale: &str,
) -> Result<OrgId, (StatusCode, Json<SecretErrorResponse>)> {
	let scm_store = match state.scm_repo_store.as_ref() {
		Some(store) => store,
		None => {
			return Err((
				StatusCode::SERVICE_UNAVAILABLE,
				Json(SecretErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.scm.not_configured").to_string(),
				}),
			));
		}
	};

	let repo = match scm_store.get_by_id(repo_id).await {
		Ok(Some(r)) => r,
		Ok(None) => {
			return Err((
				StatusCode::NOT_FOUND,
				Json(SecretErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.repo.not_found").to_string(),
				}),
			));
		}
		Err(e) => {
			tracing::error!(error = %e, %repo_id, "Failed to get repo");
			return Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			));
		}
	};

	let org_id = OrgId::new(repo.owner_id);

	match state.org_repo.get_membership(&org_id, user_id).await {
		Ok(Some(m)) if m.role == OrgRole::Owner || m.role == OrgRole::Admin => Ok(org_id),
		Ok(Some(_)) => Err((
			StatusCode::FORBIDDEN,
			Json(SecretErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.admin_required").to_string(),
			}),
		)),
		Ok(None) => Err((
			StatusCode::FORBIDDEN,
			Json(SecretErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.not_a_member").to_string(),
			}),
		)),
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(SecretErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			))
		}
	}
}
