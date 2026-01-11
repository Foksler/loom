// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Analytics API key authentication middleware.
//!
//! This module provides types and utilities for authenticating analytics API requests.
//! The actual authentication is performed in loom-server where the full request context
//! is available.

use axum::{
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};

use loom_analytics_core::{AnalyticsApiKeyId, AnalyticsKeyType, OrgId};
use loom_server_api::analytics::AnalyticsErrorResponse;

pub const WRITE_KEY_PREFIX: &str = "loom_analytics_write_";
pub const READ_WRITE_KEY_PREFIX: &str = "loom_analytics_rw_";

#[derive(Debug, Clone)]
pub struct AnalyticsApiKeyContext {
	pub api_key_id: AnalyticsApiKeyId,
	pub org_id: OrgId,
	pub key_type: AnalyticsKeyType,
}

pub enum AnalyticsApiKeyError {
	MissingAuthorization,
	InvalidFormat,
	InvalidKey,
	RevokedKey,
}

impl IntoResponse for AnalyticsApiKeyError {
	fn into_response(self) -> Response {
		let (status, error, message) = match self {
			Self::MissingAuthorization => (
				StatusCode::UNAUTHORIZED,
				"missing_authorization",
				"Authorization header is required",
			),
			Self::InvalidFormat => (
				StatusCode::UNAUTHORIZED,
				"invalid_format",
				"Invalid authorization header format",
			),
			Self::InvalidKey => (
				StatusCode::UNAUTHORIZED,
				"invalid_key",
				"Invalid API key",
			),
			Self::RevokedKey => (
				StatusCode::UNAUTHORIZED,
				"revoked_key",
				"API key has been revoked",
			),
		};

		(
			status,
			Json(AnalyticsErrorResponse {
				error: error.to_string(),
				message: message.to_string(),
			}),
		)
			.into_response()
	}
}

pub fn parse_key_type(key: &str) -> Option<AnalyticsKeyType> {
	if key.starts_with(WRITE_KEY_PREFIX) {
		Some(AnalyticsKeyType::Write)
	} else if key.starts_with(READ_WRITE_KEY_PREFIX) {
		Some(AnalyticsKeyType::ReadWrite)
	} else {
		None
	}
}

pub fn extract_bearer_token(auth_header: &str) -> Option<&str> {
	if auth_header.starts_with("Bearer ") {
		Some(&auth_header[7..])
	} else {
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_key_type_works() {
		assert_eq!(
			parse_key_type("loom_analytics_write_abc123"),
			Some(AnalyticsKeyType::Write)
		);
		assert_eq!(
			parse_key_type("loom_analytics_rw_abc123"),
			Some(AnalyticsKeyType::ReadWrite)
		);
		assert_eq!(parse_key_type("invalid_key"), None);
		assert_eq!(parse_key_type(""), None);
	}

	#[test]
	fn parse_key_type_is_prefix_sensitive() {
		assert_eq!(
			parse_key_type("loom_analytics_write_"),
			Some(AnalyticsKeyType::Write)
		);
		assert_eq!(
			parse_key_type("loom_analytics_rw_"),
			Some(AnalyticsKeyType::ReadWrite)
		);
		assert_eq!(parse_key_type("loom_analytics_"), None);
	}
}
