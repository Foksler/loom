// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! SSE streaming endpoint for cron monitor events.

use std::convert::Infallible;

use axum::{
	extract::{Query, State},
	http::StatusCode,
	response::sse::{Event, Sse},
	Json,
};
use futures::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tracing::{info, instrument};

use loom_crons_core::{CronStreamEvent, MonitorState, OrgId};
use loom_server_crons::CronsRepository;

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::{resolve_user_locale, t};

use super::common::{verify_org_membership, CronsErrorResponse, StreamCronsParams};

/// GET /api/crons/stream - SSE stream for all monitors in an organization
///
/// Streams real-time updates for cron monitor events including:
/// - `init`: Full state of all monitors on connect
/// - `checkin.started`: Job started (in_progress)
/// - `checkin.ok`: Job completed successfully
/// - `checkin.error`: Job failed
/// - `monitor.missed`: Expected check-in didn't arrive
/// - `monitor.timeout`: Job exceeded max runtime
/// - `monitor.healthy`: Monitor recovered from failure
/// - `heartbeat`: Keep-alive (every 30s)
#[utoipa::path(
	get,
	path = "/api/crons/stream",
	params(
		("org_id" = OrgId, Query, description = "Organization ID"),
	),
	responses(
		(status = 200, description = "SSE stream connection established"),
		(status = 401, description = "Not authenticated"),
		(status = 403, description = "Not a member of the organization"),
	),
	tag = "crons"
)]
#[instrument(skip(state, current_user))]
pub async fn stream_crons(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Query(params): Query<StreamCronsParams>,
) -> Result<
	Sse<impl Stream<Item = Result<Event, Infallible>>>,
	(StatusCode, Json<CronsErrorResponse>),
> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	// Verify org membership
	verify_org_membership(&state, &params.org_id, &current_user.user.id, &locale).await?;

	info!(
		org_id = %params.org_id,
		user_id = %current_user.user.id,
		"Client connected to crons stream"
	);

	// Build initial state - list all monitors for this org
	let monitors = state
		.crons_repo
		.list_monitors(params.org_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list monitors for init");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(CronsErrorResponse {
					error: "internal_error".to_string(),
					message: t(&locale, "server.api.error.internal").to_string(),
				}),
			)
		})?;

	// Convert to MonitorState for SSE init
	let monitor_states: Vec<MonitorState> = monitors
		.into_iter()
		.map(|m| MonitorState {
			id: m.id,
			slug: m.slug,
			name: m.name,
			status: m.status,
			health: m.health,
			last_checkin_status: m.last_checkin_status,
			last_checkin_at: m.last_checkin_at,
			next_expected_at: m.next_expected_at,
			consecutive_failures: m.consecutive_failures,
		})
		.collect();

	// Create init event
	let init_event = CronStreamEvent::init(monitor_states);

	// Subscribe to broadcast channel
	let receiver = state.crons_broadcaster.subscribe(params.org_id).await;
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
					tracing::warn!(error = %e, "Failed to serialize crons SSE event");
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
