// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Route-specific types for clips handlers.

use axum::{http::StatusCode, Json};
use serde::Serialize;

use crate::i18n::t;

/// Error response for clip endpoints.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ClipsErrorResponse {
	pub error: String,
	pub message: String,
}

/// Helper to create an internal error response.
pub fn internal_error(locale: &str) -> (StatusCode, Json<ClipsErrorResponse>) {
	(
		StatusCode::INTERNAL_SERVER_ERROR,
		Json(ClipsErrorResponse {
			error: "internal_error".to_string(),
			message: t(locale, "server.api.error.internal").to_string(),
		}),
	)
}

/// Helper to create a not found error response.
pub fn not_found(resource: &str) -> (StatusCode, Json<ClipsErrorResponse>) {
	(
		StatusCode::NOT_FOUND,
		Json(ClipsErrorResponse {
			error: format!("{}_not_found", resource),
			message: format!("{} not found", resource.replace('_', " ")),
		}),
	)
}

/// Helper to create a forbidden error response.
pub fn forbidden(locale: &str) -> (StatusCode, Json<ClipsErrorResponse>) {
	(
		StatusCode::FORBIDDEN,
		Json(ClipsErrorResponse {
			error: "forbidden".to_string(),
			message: t(locale, "server.api.error.forbidden").to_string(),
		}),
	)
}

/// Helper to create a conflict error response.
pub fn conflict(message: &str) -> (StatusCode, Json<ClipsErrorResponse>) {
	(
		StatusCode::CONFLICT,
		Json(ClipsErrorResponse {
			error: "conflict".to_string(),
			message: message.to_string(),
		}),
	)
}

/// Helper to create a bad request error response.
pub fn bad_request(message: &str) -> (StatusCode, Json<ClipsErrorResponse>) {
	(
		StatusCode::BAD_REQUEST,
		Json(ClipsErrorResponse {
			error: "bad_request".to_string(),
			message: message.to_string(),
		}),
	)
}
