// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash SSE stream HTTP handler.
//!
//! Implements the Server-Sent Events endpoint for real-time crash updates.

use std::convert::Infallible;

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::sse::{Event, Sse},
	Json,
};
use futures::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tracing::{info, instrument};

use loom_server_crash::{CrashRepository, CrashStreamEvent};

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::resolve_user_locale;

use super::common::{internal_error, not_found, parse_project_id, verify_org_membership, CrashErrorResponse};

// ============================================================================
// SSE Stream Endpoint
// ============================================================================

/// GET /api/crash/projects/{project_id}/stream - SSE stream for crash events
///
/// Streams real-time updates for crash events including:
/// - `init`: Initial state with issue count on connect
/// - `crash.new`: New crash event received
/// - `issue.regressed`: Resolved issue regressed
/// - `issue.resolved`: Issue was resolved
/// - `issue.assigned`: Issue was assigned
/// - `heartbeat`: Keep-alive (every 30s)
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/stream",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	responses(
		(status = 200, description = "SSE stream connection established"),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
		(status = 404, description = "Project not found"),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn stream_crash(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
) -> Result<
	Sse<impl Stream<Item = Result<Event, Infallible>>>,
	(StatusCode, Json<CrashErrorResponse>),
> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;

	// Get project to verify it exists and get org_id
	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			internal_error(&locale)
		})?
		.ok_or_else(|| not_found("project"))?;

	// Verify org membership
	verify_org_membership(&state, &project.org_id, &current_user.user.id, &locale).await?;

	info!(
		project_id = %project_id,
		user_id = %current_user.user.id,
		"Client connected to crash stream"
	);

	// Get issue count for init event
	let issues = state
		.crash_repo
		.list_issues(project_id, 1)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get issue count for init");
			internal_error(&locale)
		})?;

	// Get the actual count - we need a count method, but for now we'll use a rough estimate
	let issue_count = state
		.crash_repo
		.get_issue_count(project_id)
		.await
		.unwrap_or(issues.len() as u64);

	// Create init event
	let init_event = CrashStreamEvent::init(project_id, issue_count);

	// Subscribe to broadcast channel
	let receiver = state.crash_broadcaster.subscribe(project_id).await;
	let broadcast_stream = BroadcastStream::new(receiver);

	// Create a stream that first yields the init event, then yields broadcast events
	let init_stream = futures::stream::once(async move {
		let json = serde_json::to_string(&init_event).unwrap_or_else(|_| "{}".to_string());
		Ok::<_, Infallible>(Event::default().event("init").data(json))
	});

	let updates_stream = broadcast_stream.filter_map(|result| match result {
		Ok(event) => {
			let event_type = event.event_type();
			match serde_json::to_string(&event) {
				Ok(json) => Some(Ok::<_, Infallible>(
					Event::default().event(event_type).data(json),
				)),
				Err(e) => {
					tracing::warn!(error = %e, "Failed to serialize crash SSE event");
					None
				}
			}
		}
		Err(e) => {
			tracing::debug!(error = %e, "Broadcast stream error (client may have disconnected)");
			None
		}
	});

	let combined_stream = init_stream.chain(updates_stream);

	Ok(
		Sse::new(combined_stream).keep_alive(
			axum::response::sse::KeepAlive::new()
				.interval(std::time::Duration::from_secs(30))
				.text("heartbeat"),
		),
	)
}
