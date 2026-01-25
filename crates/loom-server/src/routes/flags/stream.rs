// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! SSE streaming HTTP handlers for feature flags.
//!
//! Implements real-time flag update streaming via Server-Sent Events.

use std::convert::Infallible;

use axum::{
	extract::{Query, State},
	http::{HeaderMap, StatusCode},
	response::{
		sse::{Event, Sse},
		IntoResponse,
	},
	Json,
};
use futures::stream::Stream;
use loom_flags_core::{EnvironmentId, FlagState, FlagStreamEvent, KillSwitchState, SdkKey};
use loom_server_flags::FlagsRepository;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::{api::AppState, auth_middleware::RequireAuth};

// ============================================================================
// Types
// ============================================================================

/// Query parameters for the flag stream.
#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct StreamFlagsParams {
	/// Environment name to subscribe to (e.g., "prod", "staging").
	pub environment: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Authenticate an SDK key from the Authorization header.
///
/// Returns (org_id, environment_id, environment_name) on success.
pub async fn authenticate_sdk_key(
	state: &AppState,
	headers: &HeaderMap,
) -> Result<(loom_flags_core::OrgId, EnvironmentId, String), (StatusCode, String)> {
	// Extract bearer token
	let auth_header = headers
		.get("authorization")
		.and_then(|v| v.to_str().ok())
		.ok_or((
			StatusCode::UNAUTHORIZED,
			"Missing Authorization header".to_string(),
		))?;

	if !auth_header.to_lowercase().starts_with("bearer ") {
		return Err((
			StatusCode::UNAUTHORIZED,
			"Invalid Authorization header format".to_string(),
		));
	}

	let sdk_key_raw = &auth_header[7..]; // Skip "Bearer "

	// Parse SDK key to extract type and environment
	let (_key_type, env_name, _) = SdkKey::parse_key(sdk_key_raw).ok_or((
		StatusCode::UNAUTHORIZED,
		"Invalid SDK key format".to_string(),
	))?;

	// Find and verify the SDK key
	// This iterates through all SDK keys for environments with matching name
	// and verifies against the Argon2 hash. It's O(n) but acceptable for
	// connection establishment (one-time per SSE stream).
	let (sdk_key_record, environment) = state
		.flags_repo
		.find_sdk_key_by_verification(sdk_key_raw, &env_name)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to verify SDK key");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				"Database error".to_string(),
			)
		})?
		.ok_or((StatusCode::UNAUTHORIZED, "Invalid SDK key".to_string()))?;

	// Update last used timestamp (fire and forget)
	let flags_repo = state.flags_repo.clone();
	let key_id = sdk_key_record.id;
	tokio::spawn(async move {
		let _ = flags_repo.update_sdk_key_last_used(key_id).await;
	});

	Ok((environment.org_id, environment.id, env_name))
}

// ============================================================================
// SSE Streaming Routes
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/flags/stream",
    params(StreamFlagsParams),
    responses(
        (status = 200, description = "SSE stream of flag updates", content_type = "text/event-stream"),
        (status = 401, description = "Invalid or missing SDK key"),
        (status = 404, description = "Environment not found")
    ),
    tag = "flags",
    security(
        ("sdk_key" = [])
    )
)]
/// Stream real-time flag updates via Server-Sent Events.
///
/// This endpoint requires SDK key authentication via the Authorization header.
/// On connection, an `init` event is sent with the full state of all flags.
/// Subsequent events are sent when flags are updated, archived, or when kill
/// switches are activated/deactivated.
///
/// Event types:
/// - `init`: Full state of all flags on connect
/// - `flag.updated`: Flag or config changed
/// - `flag.archived`: Flag archived
/// - `flag.restored`: Flag restored from archive
/// - `killswitch.activated`: Kill switch activated
/// - `killswitch.deactivated`: Kill switch deactivated
/// - `heartbeat`: Keep-alive (every 30s)
#[tracing::instrument(skip(state, headers))]
pub async fn stream_flags(
	State(state): State<AppState>,
	headers: HeaderMap,
	Query(params): Query<StreamFlagsParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
	// Authenticate SDK key
	let (org_id, env_id, env_name) = authenticate_sdk_key(&state, &headers).await?;

	// Verify the environment matches the query param
	if env_name != params.environment {
		return Err((
			StatusCode::BAD_REQUEST,
			format!(
				"SDK key is for environment '{}', but '{}' was requested",
				env_name, params.environment
			),
		));
	}

	tracing::info!(
		org_id = %org_id,
		environment = %params.environment,
		"Client connected to flag stream"
	);

	// Build initial state
	let flags = state
		.flags_repo
		.list_flags(Some(org_id), false)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list flags for init");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				"Failed to load flags".to_string(),
			)
		})?;

	let kill_switches = state
		.flags_repo
		.list_active_kill_switches(Some(org_id))
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list kill switches for init");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				"Failed to load kill switches".to_string(),
			)
		})?;

	// Build FlagState for each flag
	let mut flag_states = Vec::with_capacity(flags.len());
	for flag in &flags {
		let config = state
			.flags_repo
			.get_flag_config(flag.id, env_id)
			.await
			.ok()
			.flatten();
		flag_states.push(FlagState::from_flag_and_config(flag, config.as_ref()));
	}

	// Build KillSwitchState for each active kill switch
	let kill_switch_states: Vec<KillSwitchState> = kill_switches.iter().map(|ks| ks.into()).collect();

	// Create init event
	let init_event = FlagStreamEvent::init(flag_states, kill_switch_states);

	// Subscribe to broadcast channel
	let receiver = state.flags_broadcaster.subscribe(org_id, env_id).await;
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
					tracing::warn!(error = %e, "Failed to serialize SSE event");
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

/// Get broadcaster statistics.
#[utoipa::path(
    get,
    path = "/api/flags/stream/stats",
    responses(
        (status = 200, description = "Broadcaster statistics"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized")
    ),
    tag = "flags"
)]
#[tracing::instrument(skip(state))]
pub async fn stream_stats(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
) -> impl IntoResponse {
	// Only allow system admins to view stats
	if !current_user.user.is_system_admin() {
		return (
			StatusCode::FORBIDDEN,
			Json(serde_json::json!({ "error": "Forbidden" })),
		)
			.into_response();
	}

	let stats = state.flags_broadcaster.stats().await;
	(StatusCode::OK, Json(stats)).into_response()
}
