// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! WhatsApp configuration endpoints for organizations.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_api::whatsapp::{
	CreateWhatsAppConfigRequest, CreateWhatsAppGroupRequest, ListWhatsAppGroupsResponse,
	MoveConversationRequest, WhatsAppConfigResponse, WhatsAppErrorResponse, WhatsAppGroupResponse,
	WhatsAppSuccessResponse,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::types::{OrgId, OrgRole};
use tracing::info;
use uuid::Uuid;

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

/// Check if user is an admin of the organization.
async fn check_org_admin(
	org_id: Uuid,
	current_user: &loom_server_auth::middleware::CurrentUser,
	state: &AppState,
	locale: &str,
) -> Result<(), (StatusCode, Json<WhatsAppErrorResponse>)> {
	let org_id_typed = OrgId::new(org_id);

	let org = state
		.org_repo
		.get_org_by_id(&org_id_typed)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get organization");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
		})?;

	if org.is_none() {
		return Err((
			StatusCode::NOT_FOUND,
			Json(WhatsAppErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.org.not_found").to_string(),
			}),
		));
	}

	let membership = state
		.org_repo
		.get_membership(&org_id_typed, &current_user.user.id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to check org membership");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
		})?;

	let is_admin = match membership {
		Some(m) => m.role == OrgRole::Owner || m.role == OrgRole::Admin,
		None => false,
	};

	if !is_admin {
		return Err((
			StatusCode::FORBIDDEN,
			Json(WhatsAppErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.whatsapp.admin_required").to_string(),
			}),
		));
	}

	Ok(())
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/whatsapp/config",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "WhatsApp configuration", body = WhatsAppConfigResponse),
        (status = 401, description = "Not authenticated", body = WhatsAppErrorResponse),
        (status = 403, description = "Not authorized", body = WhatsAppErrorResponse),
        (status = 404, description = "Not found", body = WhatsAppErrorResponse)
    ),
    tag = "whatsapp"
)]
/// Get WhatsApp configuration for an organization.
#[tracing::instrument(skip(state), fields(org_id = %org_id))]
pub async fn get_whatsapp_config(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if let Err(e) = check_org_admin(org_id, &current_user, &state, locale).await {
		return e.into_response();
	}

	let whatsapp_repo = match state.whatsapp_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(WhatsAppErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	match whatsapp_repo.find_config_by_org_id(&org_id.to_string()).await {
		Ok(Some(config)) => {
			let webhook_url = format!("{}/api/whatsapp/webhook", state.base_url);
			(
				StatusCode::OK,
				Json(WhatsAppConfigResponse {
					id: config.id,
					phone_number_id: config.phone_number_id,
					enabled: config.enabled,
					webhook_url,
					created_at: config.created_at.to_rfc3339(),
					updated_at: config.updated_at.to_rfc3339(),
				}),
			)
				.into_response()
		}
		Ok(None) => (
			StatusCode::NOT_FOUND,
			Json(WhatsAppErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.whatsapp.not_configured").to_string(),
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to get WhatsApp config");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
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
    path = "/api/orgs/{org_id}/whatsapp/config",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = CreateWhatsAppConfigRequest,
    responses(
        (status = 201, description = "WhatsApp configuration created", body = WhatsAppConfigResponse),
        (status = 400, description = "Invalid request", body = WhatsAppErrorResponse),
        (status = 401, description = "Not authenticated", body = WhatsAppErrorResponse),
        (status = 403, description = "Not authorized", body = WhatsAppErrorResponse)
    ),
    tag = "whatsapp"
)]
/// Create or update WhatsApp configuration for an organization.
#[tracing::instrument(skip(state, payload), fields(org_id = %org_id))]
pub async fn create_or_update_whatsapp_config(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<Uuid>,
	Json(payload): Json<CreateWhatsAppConfigRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if let Err(e) = check_org_admin(org_id, &current_user, &state, locale).await {
		return e.into_response();
	}

	// Validate phone_number_id
	if payload.phone_number_id.is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(WhatsAppErrorResponse {
				error: "invalid_phone_number_id".to_string(),
				message: t(locale, "server.api.whatsapp.invalid_phone").to_string(),
			}),
		)
			.into_response();
	}

	let whatsapp_repo = match state.whatsapp_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(WhatsAppErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Hash the secrets before storage
	let app_secret_hash = match loom_server_whatsapp::WhatsAppService::hash_otp(&payload.app_secret)
	{
		Ok(h) => h,
		Err(e) => {
			tracing::error!(error = %e, "Failed to hash app secret");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let verify_token_hash =
		match loom_server_whatsapp::WhatsAppService::hash_otp(&payload.verify_token) {
			Ok(h) => h,
			Err(e) => {
				tracing::error!(error = %e, "Failed to hash verify token");
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(WhatsAppErrorResponse {
						error: "internal_error".to_string(),
						message: t(locale, "server.api.error.internal").to_string(),
					}),
				)
					.into_response();
			}
		};

	// TODO: Encrypt access token using secrets service
	// For now, we'll store it directly (should be encrypted in production)
	let access_token_encrypted = payload.access_token.clone();

	match whatsapp_repo
		.upsert_config(
			&org_id.to_string(),
			&payload.phone_number_id,
			&access_token_encrypted,
			&app_secret_hash,
			&verify_token_hash,
		)
		.await
	{
		Ok(config) => {
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::OrgUpdated)
					.actor(AuditUserId::new(current_user.user.id.into_inner()))
					.resource("whatsapp_config", config.id.clone())
					.details(serde_json::json!({
						"action": "whatsapp_config_created",
						"org_id": org_id.to_string(),
						"phone_number_id": &config.phone_number_id,
					}))
					.build(),
			);

			info!(
				org_id = %org_id,
				phone_number_id = %config.phone_number_id,
				"WhatsApp config created/updated"
			);

			let webhook_url = format!("{}/api/whatsapp/webhook", state.base_url);
			(
				StatusCode::CREATED,
				Json(WhatsAppConfigResponse {
					id: config.id,
					phone_number_id: config.phone_number_id,
					enabled: config.enabled,
					webhook_url,
					created_at: config.created_at.to_rfc3339(),
					updated_at: config.updated_at.to_rfc3339(),
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to create WhatsApp config");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
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
    path = "/api/orgs/{org_id}/whatsapp/config",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "WhatsApp configuration deleted", body = WhatsAppSuccessResponse),
        (status = 401, description = "Not authenticated", body = WhatsAppErrorResponse),
        (status = 403, description = "Not authorized", body = WhatsAppErrorResponse),
        (status = 404, description = "Not found", body = WhatsAppErrorResponse)
    ),
    tag = "whatsapp"
)]
/// Delete WhatsApp configuration for an organization.
#[tracing::instrument(skip(state), fields(org_id = %org_id))]
pub async fn delete_whatsapp_config(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if let Err(e) = check_org_admin(org_id, &current_user, &state, locale).await {
		return e.into_response();
	}

	let whatsapp_repo = match state.whatsapp_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(WhatsAppErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Check if config exists
	let config = match whatsapp_repo.find_config_by_org_id(&org_id.to_string()).await {
		Ok(Some(c)) => c,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(WhatsAppErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get WhatsApp config");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	match whatsapp_repo.delete_config(&org_id.to_string()).await {
		Ok(()) => {
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::OrgUpdated)
					.actor(AuditUserId::new(current_user.user.id.into_inner()))
					.resource("whatsapp_config", config.id)
					.details(serde_json::json!({
						"action": "whatsapp_config_deleted",
						"org_id": org_id.to_string(),
					}))
					.build(),
			);

			info!(org_id = %org_id, "WhatsApp config deleted");

			(
				StatusCode::OK,
				Json(WhatsAppSuccessResponse {
					message: "WhatsApp configuration deleted".to_string(),
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to delete WhatsApp config");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

// =========================================================================
// Group Management Endpoints
// =========================================================================

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/whatsapp/groups",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of WhatsApp groups", body = ListWhatsAppGroupsResponse),
        (status = 401, description = "Not authenticated", body = WhatsAppErrorResponse),
        (status = 403, description = "Not authorized", body = WhatsAppErrorResponse),
        (status = 404, description = "Config not found", body = WhatsAppErrorResponse)
    ),
    tag = "whatsapp"
)]
/// List WhatsApp groups for an organization.
#[tracing::instrument(skip(state), fields(org_id = %org_id))]
pub async fn list_whatsapp_groups(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if let Err(e) = check_org_admin(org_id, &current_user, &state, locale).await {
		return e.into_response();
	}

	let whatsapp_repo = match state.whatsapp_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(WhatsAppErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get the config first
	let config = match whatsapp_repo.find_config_by_org_id(&org_id.to_string()).await {
		Ok(Some(c)) => c,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(WhatsAppErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get WhatsApp config");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	match whatsapp_repo.list_groups(&config.id).await {
		Ok(groups) => {
			let groups: Vec<WhatsAppGroupResponse> = groups
				.into_iter()
				.map(|g| WhatsAppGroupResponse {
					id: g.id,
					name: g.name,
					description: g.description,
					color: g.color,
					is_default: g.is_default,
					created_at: g.created_at.to_rfc3339(),
					updated_at: g.updated_at.to_rfc3339(),
				})
				.collect();

			(StatusCode::OK, Json(ListWhatsAppGroupsResponse { groups })).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list WhatsApp groups");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
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
    path = "/api/orgs/{org_id}/whatsapp/groups",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = CreateWhatsAppGroupRequest,
    responses(
        (status = 201, description = "WhatsApp group created", body = WhatsAppGroupResponse),
        (status = 400, description = "Invalid request", body = WhatsAppErrorResponse),
        (status = 401, description = "Not authenticated", body = WhatsAppErrorResponse),
        (status = 403, description = "Not authorized", body = WhatsAppErrorResponse),
        (status = 404, description = "Config not found", body = WhatsAppErrorResponse)
    ),
    tag = "whatsapp"
)]
/// Create a WhatsApp group for an organization.
#[tracing::instrument(skip(state, payload), fields(org_id = %org_id))]
pub async fn create_whatsapp_group(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<Uuid>,
	Json(payload): Json<CreateWhatsAppGroupRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if let Err(e) = check_org_admin(org_id, &current_user, &state, locale).await {
		return e.into_response();
	}

	if payload.name.is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(WhatsAppErrorResponse {
				error: "invalid_name".to_string(),
				message: "Group name cannot be empty".to_string(),
			}),
		)
			.into_response();
	}

	let whatsapp_repo = match state.whatsapp_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(WhatsAppErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	// Get the config first
	let config = match whatsapp_repo.find_config_by_org_id(&org_id.to_string()).await {
		Ok(Some(c)) => c,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(WhatsAppErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get WhatsApp config");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	match whatsapp_repo
		.create_group(
			&config.id,
			&payload.name,
			payload.description.as_deref(),
			payload.color.as_deref(),
		)
		.await
	{
		Ok(group) => {
			info!(
				org_id = %org_id,
				group_id = %group.id,
				group_name = %group.name,
				"WhatsApp group created"
			);

			(
				StatusCode::CREATED,
				Json(WhatsAppGroupResponse {
					id: group.id,
					name: group.name,
					description: group.description,
					color: group.color,
					is_default: group.is_default,
					created_at: group.created_at.to_rfc3339(),
					updated_at: group.updated_at.to_rfc3339(),
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to create WhatsApp group");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
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
    path = "/api/orgs/{org_id}/whatsapp/groups/{group_id}",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID"),
        ("group_id" = String, Path, description = "Group ID")
    ),
    responses(
        (status = 200, description = "WhatsApp group deleted", body = WhatsAppSuccessResponse),
        (status = 401, description = "Not authenticated", body = WhatsAppErrorResponse),
        (status = 403, description = "Not authorized", body = WhatsAppErrorResponse),
        (status = 404, description = "Not found", body = WhatsAppErrorResponse)
    ),
    tag = "whatsapp"
)]
/// Delete a WhatsApp group.
#[tracing::instrument(skip(state), fields(org_id = %org_id, group_id = %group_id))]
pub async fn delete_whatsapp_group(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, group_id)): Path<(Uuid, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if let Err(e) = check_org_admin(org_id, &current_user, &state, locale).await {
		return e.into_response();
	}

	let whatsapp_repo = match state.whatsapp_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(WhatsAppErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	match whatsapp_repo.delete_group(&group_id).await {
		Ok(()) => {
			info!(org_id = %org_id, group_id = %group_id, "WhatsApp group deleted");

			(
				StatusCode::OK,
				Json(WhatsAppSuccessResponse {
					message: "Group deleted".to_string(),
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to delete WhatsApp group");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
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
    path = "/api/orgs/{org_id}/whatsapp/conversations/{conversation_id}/move",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID"),
        ("conversation_id" = String, Path, description = "Conversation ID")
    ),
    request_body = MoveConversationRequest,
    responses(
        (status = 200, description = "Conversation moved", body = WhatsAppSuccessResponse),
        (status = 401, description = "Not authenticated", body = WhatsAppErrorResponse),
        (status = 403, description = "Not authorized", body = WhatsAppErrorResponse),
        (status = 404, description = "Not found", body = WhatsAppErrorResponse)
    ),
    tag = "whatsapp"
)]
/// Move a WhatsApp conversation to a group.
#[tracing::instrument(skip(state, payload), fields(org_id = %org_id, conversation_id = %conversation_id))]
pub async fn move_whatsapp_conversation(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, conversation_id)): Path<(Uuid, String)>,
	Json(payload): Json<MoveConversationRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if let Err(e) = check_org_admin(org_id, &current_user, &state, locale).await {
		return e.into_response();
	}

	let whatsapp_repo = match state.whatsapp_repo.as_ref() {
		Some(repo) => repo,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(WhatsAppErrorResponse {
					error: "not_configured".to_string(),
					message: t(locale, "server.api.whatsapp.not_configured").to_string(),
				}),
			)
				.into_response();
		}
	};

	match whatsapp_repo
		.move_conversation_to_group(&conversation_id, payload.group_id.as_deref())
		.await
	{
		Ok(()) => {
			info!(
				org_id = %org_id,
				conversation_id = %conversation_id,
				group_id = ?payload.group_id,
				"WhatsApp conversation moved"
			);

			(
				StatusCode::OK,
				Json(WhatsAppSuccessResponse {
					message: "Conversation moved".to_string(),
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to move WhatsApp conversation");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WhatsAppErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}
