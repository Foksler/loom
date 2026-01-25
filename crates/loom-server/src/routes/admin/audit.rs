// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Admin audit log HTTP handlers.

use axum::{
	extract::{Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
};

use super::common::{
	AdminErrorResponse, AuditLogEntryResponse, ListAuditLogsParams, ListAuditLogsResponse,
};

/// Query audit logs.
///
/// # Authorization
///
/// Requires `system_admin` or `auditor` role. Returns 403 Forbidden otherwise.
///
/// # Request
///
/// Query parameters:
/// - `event_type` (optional): Filter by event type
/// - `actor_id` (optional): Filter by actor user ID
/// - `resource_type` (optional): Filter by resource type
/// - `resource_id` (optional): Filter by resource ID
/// - `from` (optional): Start of time range (ISO 8601)
/// - `to` (optional): End of time range (ISO 8601)
/// - `limit` (optional): Maximum logs to return (default: 50)
/// - `offset` (optional): Pagination offset (default: 0)
///
/// # Response
///
/// Returns [`ListAuditLogsResponse`] with paginated audit log entries.
///
/// # Errors
///
/// - `401 Unauthorized`: Missing or invalid authentication
/// - `403 Forbidden`: Caller lacks required role
/// - `500 Internal Server Error`: Database error
///
/// # Security
///
/// - Audit logs contain sensitive operational data
/// - Access is restricted to admins and auditors only
#[utoipa::path(
    get,
    path = "/api/admin/audit-logs",
    params(ListAuditLogsParams),
    responses(
        (status = 200, description = "List of audit logs", body = ListAuditLogsResponse),
        (status = 401, description = "Not authenticated", body = AdminErrorResponse),
        (status = 403, description = "Not authorized (system_admin or auditor required)", body = AdminErrorResponse)
    ),
    tag = "admin"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		event_type = ?params.event_type,
		filter_actor_id = ?params.actor_id,
		resource_type = ?params.resource_type,
		limit = params.limit,
		offset = params.offset
	)
)]
pub async fn list_audit_logs(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Query(params): Query<ListAuditLogsParams>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin && !current_user.user.is_auditor {
		tracing::warn!(
			actor_id = %current_user.user.id,
			"Unauthorized audit log access attempt"
		);
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.error.forbidden").to_string(),
			}),
		)
			.into_response();
	}

	let (logs, total) = match state
		.audit_repo
		.query_logs(
			params.event_type.as_deref(),
			params.actor_id.as_deref(),
			params.resource_type.as_deref(),
			params.resource_id.as_deref(),
			params.from,
			params.to,
			Some(params.limit.into()),
			Some(params.offset.into()),
		)
		.await
	{
		Ok(result) => result,
		Err(e) => {
			tracing::error!(error = %e, "Failed to query audit logs");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AdminErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let logs: Vec<AuditLogEntryResponse> = logs
		.into_iter()
		.map(|l| AuditLogEntryResponse {
			id: l.id.to_string(),
			timestamp: l.timestamp,
			event_type: l.event_type.to_string(),
			actor_user_id: l.actor_user_id.map(|id| id.to_string()),
			impersonating_user_id: l.impersonating_user_id.map(|id| id.to_string()),
			resource_type: l.resource_type,
			resource_id: l.resource_id,
			action: l.action,
			ip_address: l.ip_address,
			user_agent: l.user_agent,
			details: if l.details == serde_json::Value::Null {
				None
			} else {
				Some(l.details)
			},
		})
		.collect();

	tracing::info!(
		actor_id = %current_user.user.id,
		log_count = logs.len(),
		total = total,
		"Audit logs queried"
	);

	(
		StatusCode::OK,
		Json(ListAuditLogsResponse {
			logs,
			total,
			limit: params.limit,
			offset: params.offset,
		}),
	)
		.into_response()
}
