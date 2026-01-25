// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for auth routes.

use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Re-export API types
pub use loom_server_api::auth::{
	AuthErrorResponse, AuthProvidersResponse, AuthSuccessResponse, CurrentUserResponse,
	DeviceCodeCompleteRequest, DeviceCodeCompleteResponse, DeviceCodePollRequest,
	DeviceCodePollResponse, DeviceCodeStartResponse, MagicLinkRequest, WsTokenResponse,
};

/// Response for OAuth login initiation.
#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthRedirectResponse {
	pub redirect_url: String,
}

/// Query parameters for OAuth callback.
#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthCallbackQuery {
	pub code: Option<String>,
	pub state: Option<String>,
	pub error: Option<String>,
}

/// Query parameters for magic link verification.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MagicLinkVerifyQuery {
	pub token: String,
}

/// Query parameters for OAuth login initiation.
#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthLoginQuery {
	/// Optional redirect URL after successful authentication
	pub redirect: Option<String>,
}

/// Resolve locale from HTTP headers.
pub fn resolve_locale_from_headers<'a>(headers: &HeaderMap, default_locale: &'a str) -> &'a str {
	if let Some(accept_lang) = headers.get("Accept-Language").and_then(|v| v.to_str().ok()) {
		for part in accept_lang.split(',') {
			let lang = part.split(';').next().unwrap_or("").trim();
			let lang_base = lang.split('-').next().unwrap_or(lang);
			if loom_common_i18n::is_supported(lang_base) {
				return match lang_base {
					"en" => "en",
					"es" => "es",
					"ar" => "ar",
					_ => default_locale,
				};
			}
		}
	}
	default_locale
}
