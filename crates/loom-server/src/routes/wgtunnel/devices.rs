// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! WireGuard device management handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_wgtunnel::{DeviceResponse, RegisterDeviceRequest};
use tracing::{info, instrument};
use uuid::Uuid;

use super::common::{parse_public_key, wg_error_to_response};
use crate::{api::AppState, auth_middleware::RequireAuth, error::ServerError};

/// POST /api/wg/devices - Register a new WireGuard device
#[utoipa::path(
    post,
    path = "/api/wg/devices",
    request_body = RegisterDeviceRequest,
    responses(
        (status = 201, description = "Device registered", body = DeviceResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = crate::error::ErrorResponse)
    ),
    tag = "wgtunnel",
    security(("api_key" = []))
)]
#[instrument(skip(state, current_user, request), fields(user_id = %current_user.user.id))]
pub async fn register_device(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Json(request): Json<RegisterDeviceRequest>,
) -> Result<impl IntoResponse, ServerError> {
	let services = state
		.wg_tunnel_services
		.as_ref()
		.ok_or_else(|| ServerError::ServiceUnavailable("WireGuard tunnel not enabled".to_string()))?;

	let public_key = parse_public_key(&request.public_key)?;
	let user_id: Uuid = current_user.user.id.into_inner();

	let device = services
		.device_service
		.register(user_id, public_key, request.name.clone())
		.await
		.map_err(wg_error_to_response)?;

	info!(device_id = %device.id, "Device registered");

	Ok((
		StatusCode::CREATED,
		Json(DeviceResponse {
			id: device.id.to_string(),
			public_key: device.public_key_base64(),
			name: device.name,
			created_at: device.created_at.to_rfc3339(),
			last_seen_at: device.last_seen_at.map(|t| t.to_rfc3339()),
		}),
	))
}

/// GET /api/wg/devices - List user's WireGuard devices
#[utoipa::path(
    get,
    path = "/api/wg/devices",
    responses(
        (status = 200, description = "List of devices", body = Vec<DeviceResponse>),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = crate::error::ErrorResponse)
    ),
    tag = "wgtunnel",
    security(("api_key" = []))
)]
#[instrument(skip(state, current_user), fields(user_id = %current_user.user.id))]
pub async fn list_devices(
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

	let response: Vec<DeviceResponse> = devices
		.into_iter()
		.map(|d| DeviceResponse {
			id: d.id.to_string(),
			public_key: d.public_key_base64(),
			name: d.name,
			created_at: d.created_at.to_rfc3339(),
			last_seen_at: d.last_seen_at.map(|t| t.to_rfc3339()),
		})
		.collect();

	Ok(Json(response))
}

/// DELETE /api/wg/devices/{id} - Revoke a device
#[utoipa::path(
    delete,
    path = "/api/wg/devices/{id}",
    params(
        ("id" = String, Path, description = "Device ID")
    ),
    responses(
        (status = 204, description = "Device revoked"),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Device not found", body = crate::error::ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = crate::error::ErrorResponse)
    ),
    tag = "wgtunnel",
    security(("api_key" = []))
)]
#[instrument(skip(state, current_user), fields(user_id = %current_user.user.id, device_id = %id))]
pub async fn revoke_device(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
	let services = state
		.wg_tunnel_services
		.as_ref()
		.ok_or_else(|| ServerError::ServiceUnavailable("WireGuard tunnel not enabled".to_string()))?;

	let device_id: Uuid = id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid device ID: {id}")))?;
	let user_id: Uuid = current_user.user.id.into_inner();

	services
		.device_service
		.revoke(device_id, user_id)
		.await
		.map_err(wg_error_to_response)?;

	info!(device_id = %id, "Device revoked");

	Ok(StatusCode::NO_CONTENT)
}
