// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! DERP map handler.

use axum::{extract::State, response::IntoResponse, Json};
use tracing::instrument;

use super::common::wg_error_to_response;
use crate::{api::AppState, auth_middleware::RequireAuth, error::ServerError};

/// GET /api/wg/derp-map - Get the DERP relay map
#[utoipa::path(
    get,
    path = "/api/wg/derp-map",
    responses(
        (status = 200, description = "DERP map"),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = crate::error::ErrorResponse)
    ),
    tag = "wgtunnel",
    security(("api_key" = []))
)]
#[instrument(skip(state, _current_user))]
pub async fn get_derp_map(
	State(state): State<AppState>,
	RequireAuth(_current_user): RequireAuth,
) -> Result<impl IntoResponse, ServerError> {
	let services = state
		.wg_tunnel_services
		.as_ref()
		.ok_or_else(|| ServerError::ServiceUnavailable("WireGuard tunnel not enabled".to_string()))?;

	let derp_map = services
		.derp_service
		.get_derp_map()
		.await
		.map_err(wg_error_to_response)?;

	Ok(Json(derp_map))
}
