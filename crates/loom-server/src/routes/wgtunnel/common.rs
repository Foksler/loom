// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared helpers for WireGuard tunnel handlers.

use axum::{http::HeaderMap, http::StatusCode, Json};
use base64::prelude::*;
use loom_server_wgtunnel::WgError;

use crate::error::ServerError;

use super::super::weaver_auth::ErrorResponse;

pub fn wg_error_to_response(e: WgError) -> ServerError {
	match e {
		WgError::DeviceNotFound => ServerError::NotFound("Device not found".to_string()),
		WgError::WeaverNotFound => ServerError::NotFound("Weaver not found".to_string()),
		WgError::SessionNotFound => ServerError::NotFound("Session not found".to_string()),
		WgError::DeviceAlreadyExists => {
			ServerError::BadRequest("Device already registered".to_string())
		}
		WgError::DeviceRevoked => ServerError::BadRequest("Device has been revoked".to_string()),
		WgError::WeaverAlreadyRegistered => {
			ServerError::BadRequest("Weaver already registered".to_string())
		}
		WgError::SessionAlreadyExists => ServerError::BadRequest("Session already exists".to_string()),
		WgError::InvalidPublicKey(msg) => ServerError::BadRequest(format!("Invalid public key: {msg}")),
		WgError::IpAllocation(msg) => ServerError::Internal(format!("IP allocation failed: {msg}")),
		WgError::Unauthorized(msg) => ServerError::Unauthorized(msg),
		WgError::Database(e) => ServerError::Db(e),
		WgError::Config(msg) | WgError::DerpMap(msg) | WgError::Internal(msg) => {
			ServerError::Internal(msg)
		}
	}
}

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, (StatusCode, Json<ErrorResponse>)> {
	let auth_header = headers.get("authorization").ok_or_else(|| {
		(
			StatusCode::UNAUTHORIZED,
			Json(ErrorResponse {
				error: "missing_token".to_string(),
				message: "Authorization header required".to_string(),
			}),
		)
	})?;

	let auth_str = auth_header.to_str().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(ErrorResponse {
				error: "invalid_header".to_string(),
				message: "Invalid authorization header encoding".to_string(),
			}),
		)
	})?;

	auth_str.strip_prefix("Bearer ").ok_or_else(|| {
		(
			StatusCode::BAD_REQUEST,
			Json(ErrorResponse {
				error: "invalid_token_format".to_string(),
				message: "Authorization header must be 'Bearer <token>'".to_string(),
			}),
		)
	})
}

pub fn parse_public_key(key_b64: &str) -> Result<[u8; 32], ServerError> {
	let bytes = BASE64_STANDARD
		.decode(key_b64)
		.or_else(|_| BASE64_STANDARD_NO_PAD.decode(key_b64))
		.map_err(|_| ServerError::BadRequest("Invalid base64 public key".to_string()))?;

	bytes
		.try_into()
		.map_err(|_| ServerError::BadRequest("Public key must be exactly 32 bytes".to_string()))
}
