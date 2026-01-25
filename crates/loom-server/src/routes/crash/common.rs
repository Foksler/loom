// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for crash analytics HTTP handlers.

use std::sync::Arc;

use axum::{http::StatusCode, Json};
use serde::Serialize;
use tracing::info;

use loom_crash_core::{CrashEvent, OrgId, ProjectId};
use loom_server_auth::types::OrgId as AuthOrgId;

use crate::api::AppState;
use crate::i18n::t;
use loom_server_crash::SymbolicationService;

/// Error response for crash endpoints.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CrashErrorResponse {
	pub error: String,
	pub message: String,
}

/// Verify that the current user is a member of the specified organization.
pub async fn verify_org_membership(
	state: &AppState,
	org_id: &OrgId,
	user_id: &loom_server_auth::types::UserId,
	locale: &str,
) -> Result<(), (StatusCode, Json<CrashErrorResponse>)> {
	let auth_org_id = AuthOrgId::from(org_id.0);

	match state.org_repo.get_membership(&auth_org_id, user_id).await {
		Ok(Some(_)) => Ok(()),
		Ok(None) => Err((
			StatusCode::FORBIDDEN,
			Json(CrashErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.not_a_member").to_string(),
			}),
		)),
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CrashErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			))
		}
	}
}

/// Symbolicate a crash event's stacktrace if source maps are available.
///
/// This function:
/// 1. Saves the original (minified) stacktrace as raw_stacktrace
/// 2. Attempts to symbolicate the stacktrace using uploaded source maps
/// 3. Updates the event's stacktrace with the symbolicated version
///
/// Symbolication is only attempted if a release is specified.
pub async fn symbolicate_event(state: &AppState, event: &mut CrashEvent, project_id: ProjectId) {
	// Skip if no release specified
	let (release, dist) = match (&event.release, &event.dist) {
		(Some(r), d) => (r.as_str(), d.as_deref()),
		(None, _) => return,
	};

	// Save the original (minified) stacktrace
	let raw_stacktrace = event.stacktrace.clone();

	// Create symbolication service and symbolicate
	let symbolication_service = SymbolicationService::new(Arc::clone(&state.crash_repo));
	match symbolication_service
		.symbolicate(
			&event.stacktrace,
			event.platform,
			project_id,
			Some(release),
			dist,
		)
		.await
	{
		Ok(symbolicated) => {
			// Only save raw_stacktrace if symbolication actually changed something
			if symbolicated.frames != raw_stacktrace.frames {
				event.raw_stacktrace = Some(raw_stacktrace);
				event.stacktrace = symbolicated;
				info!(
					project_id = %project_id,
					release = ?event.release,
					"Symbolicated crash stacktrace"
				);
			}
		}
		Err(e) => {
			tracing::warn!(error = %e, "Symbolication failed, using original stacktrace");
		}
	}
}

/// Helper to parse a project ID from a string path parameter.
pub fn parse_project_id(
	project_id_str: &str,
) -> Result<ProjectId, (StatusCode, Json<CrashErrorResponse>)> {
	project_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_project_id".to_string(),
				message: "Invalid project ID".to_string(),
			}),
		)
	})
}

/// Helper to create an internal error response.
pub fn internal_error(locale: &str) -> (StatusCode, Json<CrashErrorResponse>) {
	(
		StatusCode::INTERNAL_SERVER_ERROR,
		Json(CrashErrorResponse {
			error: "internal_error".to_string(),
			message: t(locale, "server.api.error.internal").to_string(),
		}),
	)
}

/// Helper to create a not found error response.
pub fn not_found(resource: &str) -> (StatusCode, Json<CrashErrorResponse>) {
	(
		StatusCode::NOT_FOUND,
		Json(CrashErrorResponse {
			error: format!("{}_not_found", resource),
			message: format!("{} not found", resource.replace('_', " ")),
		}),
	)
}
