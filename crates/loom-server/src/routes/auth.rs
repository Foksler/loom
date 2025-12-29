// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Authentication stub HTTP handlers.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use utoipa::ToSchema;

/// Response for authentication stub endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthStubResponse {
	pub status: String,
	pub message: String,
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    responses(
        (status = 501, description = "Not implemented", body = AuthStubResponse)
    ),
    tag = "auth"
)]
/// POST /api/auth/login - Stub login endpoint.
pub async fn login_stub() -> impl IntoResponse {
	(
		StatusCode::NOT_IMPLEMENTED,
		Json(AuthStubResponse {
			status: "not_implemented".to_string(),
			message: "Authentication is not implemented yet.".to_string(),
		}),
	)
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 501, description = "Not implemented", body = AuthStubResponse)
    ),
    tag = "auth"
)]
/// POST /api/auth/logout - Stub logout endpoint.
pub async fn logout_stub() -> impl IntoResponse {
	(
		StatusCode::NOT_IMPLEMENTED,
		Json(AuthStubResponse {
			status: "not_implemented".to_string(),
			message: "Logout is not implemented yet.".to_string(),
		}),
	)
}
