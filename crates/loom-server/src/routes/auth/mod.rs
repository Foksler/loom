// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Authentication HTTP handlers.
//!
//! Implements auth endpoints per the auth-abac-system.md specification:
//! - OAuth provider listing
//! - Current user retrieval
//! - Logout
//! - Magic link authentication
//! - Device code flow for CLI/VS Code
//!
//! # Security Considerations
//!
//! - Tokens in URLs/headers should be scanned for accidental logging
//! - Email addresses are PII and should be handled carefully

pub mod basic;
pub mod common;
pub mod device_code;
pub mod magic_link;
pub mod oauth;

// Re-export all handlers and their utoipa path types
pub use basic::{
	__path_get_current_user, __path_get_providers, __path_get_ws_token, __path_logout,
	get_current_user, get_providers, get_ws_token, logout,
};
pub use device_code::{
	__path_device_complete, __path_device_page, __path_device_poll, __path_device_start,
	device_complete, device_page, device_poll, device_start,
};
pub use magic_link::{
	__path_request_magic_link, __path_verify_magic_link, request_magic_link, verify_magic_link,
};
pub use oauth::{
	__path_callback_github, __path_callback_google, __path_callback_okta, __path_login_github,
	__path_login_google, __path_login_okta, callback_github, callback_google, callback_okta,
	complete_oauth_login, login_github, login_google, login_okta,
};

// Re-export API types for external use
pub use common::{
	AuthErrorResponse, AuthProvidersResponse, AuthSuccessResponse, CurrentUserResponse,
	DeviceCodeCompleteRequest, DeviceCodeCompleteResponse, DeviceCodePollRequest,
	DeviceCodePollResponse, DeviceCodeStartResponse, MagicLinkRequest, MagicLinkVerifyQuery,
	OAuthCallbackQuery, OAuthLoginQuery, OAuthRedirectResponse, WsTokenResponse,
};
