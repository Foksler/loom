// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! WireGuard session management handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_wgtunnel::{SessionListItem, SessionResponse};
use tracing::{info, instrument};
use uuid::Uuid;

use super::common::wg_error_to_response;
use crate::{api::AppState, auth_middleware::RequireAuth, error::ServerError};

/// POST /api/wg/sessions - Create a tunnel session
#[utoipa::path(
    post,
    path = "/api/wg/sessions",
    request_body = loom_server_wgtunnel::CreateSessionApiRequest,
    responses(
        (status = 201, description = "Session created", body = SessionResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Device or weaver not found", body = crate::error::ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = crate::error::ErrorResponse)
    ),
    tag = "wgtunnel",
    security(("api_key" = []))
)]
#[instrument(skip(state, current_user, request), fields(user_id = %current_user.user.id))]
pub async fn create_session(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Json(request): Json<loom_server_wgtunnel::CreateSessionApiRequest>,
) -> Result<impl IntoResponse, ServerError> {
	let services = state
		.wg_tunnel_services
		.as_ref()
		.ok_or_else(|| ServerError::ServiceUnavailable("WireGuard tunnel not enabled".to_string()))?;

	let weaver_id: loom_server_weaver::WeaverId = request
		.weaver_id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid weaver ID: {}", request.weaver_id)))?;

	let provisioner = state.provisioner.as_ref().ok_or_else(|| {
		ServerError::ServiceUnavailable("Weaver provisioner not configured".to_string())
	})?;
	let weaver = provisioner.get_weaver(&weaver_id).await?;
	if weaver.owner_user_id != current_user.user.id.to_string() {
		return Err(ServerError::Forbidden(
			"You do not have access to this weaver".to_string(),
		));
	}

	let weaver_id_uuid: Uuid = request
		.weaver_id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid weaver ID: {}", request.weaver_id)))?;

	let user_id: Uuid = current_user.user.id.into_inner();
	let devices = services
		.device_service
		.list(user_id)
		.await
		.map_err(wg_error_to_response)?;

	let device = devices.first().ok_or_else(|| {
		ServerError::BadRequest("No registered device. Register a device first.".to_string())
	})?;

	let response = services
		.session_service
		.create(device.id, weaver_id_uuid)
		.await
		.map_err(wg_error_to_response)?;

	info!(session_id = %response.session_id, "Session created");

	Ok((
		StatusCode::CREATED,
		Json(SessionResponse {
			session_id: response.session_id.to_string(),
			client_ip: response.client_ip.to_string(),
			weaver_ip: response.weaver_ip.to_string(),
			weaver_public_key: response.weaver_public_key,
			derp_map: serde_json::to_value(&response.derp_map).unwrap_or_default(),
		}),
	))
}

/// GET /api/wg/sessions - List active sessions
#[utoipa::path(
    get,
    path = "/api/wg/sessions",
    responses(
        (status = 200, description = "List of sessions", body = Vec<SessionListItem>),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = crate::error::ErrorResponse)
    ),
    tag = "wgtunnel",
    security(("api_key" = []))
)]
#[instrument(skip(state, current_user), fields(user_id = %current_user.user.id))]
pub async fn list_sessions(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
) -> Result<impl IntoResponse, ServerError> {
	let services = state
		.wg_tunnel_services
		.as_ref()
		.ok_or_else(|| ServerError::ServiceUnavailable("WireGuard tunnel not enabled".to_string()))?;

	let user_id: Uuid = current_user.user.id.into_inner();
	let devices = services
		.device_service
		.list(user_id)
		.await
		.map_err(wg_error_to_response)?;

	let mut all_sessions = Vec::new();
	for device in devices {
		let sessions = services
			.session_service
			.list_for_device(device.id)
			.await
			.map_err(wg_error_to_response)?;

		for session in sessions {
			all_sessions.push(SessionListItem {
				session_id: session.id.to_string(),
				device_id: session.device_id.to_string(),
				weaver_id: session.weaver_id.to_string(),
				client_ip: session.client_ip.to_string(),
				created_at: session.created_at.to_rfc3339(),
				last_handshake_at: session.last_handshake_at.map(|t| t.to_rfc3339()),
			});
		}
	}

	Ok(Json(all_sessions))
}

/// DELETE /api/wg/sessions/{id} - Terminate a session
#[utoipa::path(
    delete,
    path = "/api/wg/sessions/{id}",
    params(
        ("id" = String, Path, description = "Session ID")
    ),
    responses(
        (status = 204, description = "Session terminated"),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Session not found", body = crate::error::ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = crate::error::ErrorResponse)
    ),
    tag = "wgtunnel",
    security(("api_key" = []))
)]
#[instrument(skip(state, current_user), fields(user_id = %current_user.user.id, session_id = %id))]
pub async fn terminate_session(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
	let services = state
		.wg_tunnel_services
		.as_ref()
		.ok_or_else(|| ServerError::ServiceUnavailable("WireGuard tunnel not enabled".to_string()))?;

	let session_id: Uuid = id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid session ID: {id}")))?;

	let session = services
		.session_service
		.get(session_id)
		.await
		.map_err(wg_error_to_response)?
		.ok_or_else(|| ServerError::NotFound("Session not found".to_string()))?;

	let user_id: Uuid = current_user.user.id.into_inner();
	let device = services
		.device_service
		.get(session.device_id)
		.await
		.map_err(wg_error_to_response)?
		.ok_or_else(|| ServerError::NotFound("Device not found".to_string()))?;

	if device.user_id != user_id && !current_user.user.is_system_admin() {
		return Err(ServerError::Forbidden(
			"Not authorized to terminate this session".to_string(),
		));
	}

	services
		.session_service
		.terminate(session_id)
		.await
		.map_err(wg_error_to_response)?;

	info!(session_id = %id, "Session terminated");

	Ok(StatusCode::NO_CONTENT)
}
