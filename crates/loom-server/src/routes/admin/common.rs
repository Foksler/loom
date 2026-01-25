// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for admin handlers.

pub use loom_server_api::admin::{
	AdminErrorResponse, AdminSuccessResponse, AdminUserResponse, AuditLogEntryResponse,
	DeleteUserResponse, ImpersonateRequest, ImpersonateResponse, ImpersonationState,
	ImpersonationUserInfo, ListAuditLogsParams, ListAuditLogsResponse, ListUsersParams,
	ListUsersResponse, UpdateRolesRequest,
};
use loom_server_auth::UserId;
use uuid::Uuid;

use crate::i18n::t;

pub fn parse_user_id(id: &str, locale: &str) -> Result<UserId, AdminErrorResponse> {
	Uuid::parse_str(id)
		.map(UserId::new)
		.map_err(|_| AdminErrorResponse {
			error: "bad_request".to_string(),
			message: t(locale, "server.api.user.invalid_id").to_string(),
		})
}
