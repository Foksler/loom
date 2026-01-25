// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Internal weaver WireGuard handlers (SVID auth).

use std::convert::Infallible;

use axum::{
	extract::{Path, State},
	http::{HeaderMap, StatusCode},
	response::{sse::Event, IntoResponse, Sse},
	Json,
};
use futures::stream::Stream;
use loom_server_wgtunnel::{
	PeerEvent, RegisterWeaverRequest, RegisterWeaverResponse, WeaverResponse, WgError,
};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use super::common::{extract_bearer_token, parse_public_key};
use crate::api::AppState;

use super::super::weaver_auth::ErrorResponse;

/// POST /internal/wg/weavers - Register a weaver's WireGuard key
#[utoipa::path(
    post,
    path = "/internal/wg/weavers",
    request_body = RegisterWeaverRequest,
    responses(
        (status = 201, description = "Weaver registered", body = RegisterWeaverResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Invalid SVID", body = ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = ErrorResponse)
    ),
    tag = "wgtunnel-internal"
)]
#[instrument(skip(state, headers, request), fields(weaver_id = %request.weaver_id))]
pub async fn register_weaver(
	State(state): State<AppState>,
	headers: HeaderMap,
	Json(request): Json<RegisterWeaverRequest>,
) -> impl IntoResponse {
	let svid_issuer = match state.svid_issuer.as_ref() {
		Some(issuer) => issuer,
		None => {
			warn!("SVID issuer not configured");
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(ErrorResponse {
					error: "service_unavailable".to_string(),
					message: "SVID validation not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let services = match state.wg_tunnel_services.as_ref() {
		Some(s) => s,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(ErrorResponse {
					error: "service_unavailable".to_string(),
					message: "WireGuard tunnel not enabled".to_string(),
				}),
			)
				.into_response();
		}
	};

	let token = match extract_bearer_token(&headers) {
		Ok(t) => t,
		Err(resp) => return resp.into_response(),
	};

	let claims = match svid_issuer.verify_svid(token).await {
		Ok(c) => c,
		Err(e) => {
			warn!(error = %e, "SVID validation failed");
			return (
				StatusCode::UNAUTHORIZED,
				Json(ErrorResponse {
					error: "svid_validation_failed".to_string(),
					message: "SVID validation failed".to_string(),
				}),
			)
				.into_response();
		}
	};

	if claims.weaver_id != request.weaver_id {
		return (
			StatusCode::FORBIDDEN,
			Json(ErrorResponse {
				error: "weaver_id_mismatch".to_string(),
				message: "SVID weaver_id does not match request".to_string(),
			}),
		)
			.into_response();
	}

	let weaver_id: Uuid = match request.weaver_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ErrorResponse {
					error: "invalid_weaver_id".to_string(),
					message: "Invalid weaver ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	let public_key = match parse_public_key(&request.public_key) {
		Ok(k) => k,
		Err(e) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ErrorResponse {
					error: "invalid_public_key".to_string(),
					message: e.to_string(),
				}),
			)
				.into_response();
		}
	};

	let weaver = match services
		.weaver_service
		.register(weaver_id, public_key, request.derp_home_region)
		.await
	{
		Ok(w) => w,
		Err(e) => {
			let (status, error, message) = match &e {
				WgError::WeaverAlreadyRegistered => {
					(StatusCode::CONFLICT, "already_registered", e.to_string())
				}
				WgError::IpAllocation(msg) => (
					StatusCode::INTERNAL_SERVER_ERROR,
					"ip_allocation_failed",
					msg.clone(),
				),
				_ => (
					StatusCode::INTERNAL_SERVER_ERROR,
					"internal_error",
					e.to_string(),
				),
			};
			return (
				status,
				Json(ErrorResponse {
					error: error.to_string(),
					message,
				}),
			)
				.into_response();
		}
	};

	info!(weaver_id = %weaver.weaver_id, assigned_ip = %weaver.assigned_ip, "Weaver registered for WG tunnel");

	let base_url = &state.base_url;

	(
		StatusCode::CREATED,
		Json(RegisterWeaverResponse {
			assigned_ip: weaver.assigned_ip.to_string(),
			derp_map_url: format!("{base_url}/api/wg/derp-map"),
			peers_stream_url: format!("{base_url}/internal/wg/weavers/{}/peers", weaver.weaver_id),
		}),
	)
		.into_response()
}

/// DELETE /internal/wg/weavers/{id} - Unregister a weaver
#[utoipa::path(
    delete,
    path = "/internal/wg/weavers/{id}",
    params(
        ("id" = String, Path, description = "Weaver ID")
    ),
    responses(
        (status = 204, description = "Weaver unregistered"),
        (status = 401, description = "Invalid SVID", body = ErrorResponse),
        (status = 404, description = "Weaver not found", body = ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = ErrorResponse)
    ),
    tag = "wgtunnel-internal"
)]
#[instrument(skip(state, headers), fields(weaver_id = %id))]
pub async fn unregister_weaver(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(id): Path<String>,
) -> impl IntoResponse {
	let svid_issuer = match state.svid_issuer.as_ref() {
		Some(issuer) => issuer,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(ErrorResponse {
					error: "service_unavailable".to_string(),
					message: "SVID validation not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let services = match state.wg_tunnel_services.as_ref() {
		Some(s) => s,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(ErrorResponse {
					error: "service_unavailable".to_string(),
					message: "WireGuard tunnel not enabled".to_string(),
				}),
			)
				.into_response();
		}
	};

	let token = match extract_bearer_token(&headers) {
		Ok(t) => t,
		Err(resp) => return resp.into_response(),
	};

	let claims = match svid_issuer.verify_svid(token).await {
		Ok(c) => c,
		Err(e) => {
			warn!(error = %e, "SVID validation failed");
			return (
				StatusCode::UNAUTHORIZED,
				Json(ErrorResponse {
					error: "svid_validation_failed".to_string(),
					message: "SVID validation failed".to_string(),
				}),
			)
				.into_response();
		}
	};

	if claims.weaver_id != id {
		return (
			StatusCode::FORBIDDEN,
			Json(ErrorResponse {
				error: "weaver_id_mismatch".to_string(),
				message: "SVID weaver_id does not match request".to_string(),
			}),
		)
			.into_response();
	}

	let weaver_id: Uuid = match id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ErrorResponse {
					error: "invalid_weaver_id".to_string(),
					message: "Invalid weaver ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	services.peer_notifier.unregister(weaver_id).await;

	match services.weaver_service.unregister(weaver_id).await {
		Ok(()) => {
			info!(weaver_id = %id, "Weaver unregistered from WG tunnel");
			StatusCode::NO_CONTENT.into_response()
		}
		Err(WgError::WeaverNotFound) => (
			StatusCode::NOT_FOUND,
			Json(ErrorResponse {
				error: "not_found".to_string(),
				message: "Weaver not registered".to_string(),
			}),
		)
			.into_response(),
		Err(e) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ErrorResponse {
				error: "internal_error".to_string(),
				message: e.to_string(),
			}),
		)
			.into_response(),
	}
}

/// GET /internal/wg/weavers/{id}/peers - SSE stream of peer updates
#[utoipa::path(
    get,
    path = "/internal/wg/weavers/{id}/peers",
    params(
        ("id" = String, Path, description = "Weaver ID")
    ),
    responses(
        (status = 200, description = "SSE stream of peer events"),
        (status = 401, description = "Invalid SVID", body = ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = ErrorResponse)
    ),
    tag = "wgtunnel-internal"
)]
#[instrument(skip(state, headers), fields(weaver_id = %id))]
pub async fn stream_peers(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(id): Path<String>,
) -> impl IntoResponse {
	let svid_issuer = match state.svid_issuer.as_ref() {
		Some(issuer) => issuer,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(ErrorResponse {
					error: "service_unavailable".to_string(),
					message: "SVID validation not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let services = match state.wg_tunnel_services.as_ref() {
		Some(s) => s.clone(),
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(ErrorResponse {
					error: "service_unavailable".to_string(),
					message: "WireGuard tunnel not enabled".to_string(),
				}),
			)
				.into_response();
		}
	};

	let token = match extract_bearer_token(&headers) {
		Ok(t) => t.to_string(),
		Err(resp) => return resp.into_response(),
	};

	let claims = match svid_issuer.verify_svid(&token).await {
		Ok(c) => c,
		Err(e) => {
			warn!(error = %e, "SVID validation failed");
			return (
				StatusCode::UNAUTHORIZED,
				Json(ErrorResponse {
					error: "svid_validation_failed".to_string(),
					message: "SVID validation failed".to_string(),
				}),
			)
				.into_response();
		}
	};

	if claims.weaver_id != id {
		return (
			StatusCode::FORBIDDEN,
			Json(ErrorResponse {
				error: "weaver_id_mismatch".to_string(),
				message: "SVID weaver_id does not match request".to_string(),
			}),
		)
			.into_response();
	}

	let weaver_id: Uuid = match id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ErrorResponse {
					error: "invalid_weaver_id".to_string(),
					message: "Invalid weaver ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	let rx = services.peer_notifier.subscribe(weaver_id).await;

	info!(weaver_id = %id, "Starting peer stream");

	let stream = peer_event_stream(rx);
	Sse::new(stream).into_response()
}

fn peer_event_stream(
	mut rx: tokio::sync::broadcast::Receiver<PeerEvent>,
) -> impl Stream<Item = Result<Event, Infallible>> {
	async_stream::stream! {
		loop {
			match rx.recv().await {
				Ok(event) => {
					let event_type = match &event {
						PeerEvent::PeerAdded { .. } => "peer_added",
						PeerEvent::PeerRemoved { .. } => "peer_removed",
					};
					match serde_json::to_string(&event) {
						Ok(data) => {
							yield Ok(Event::default().event(event_type).data(data));
						}
						Err(_) => {
							warn!(event_type = event_type, "Failed to serialize peer event");
						}
					}
				}
				Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
					warn!(lagged = n, "Peer stream lagged");
				}
				Err(tokio::sync::broadcast::error::RecvError::Closed) => {
					break;
				}
			}
		}
	}
}

/// GET /internal/wg/weavers/{id} - Get weaver info (for debugging)
#[utoipa::path(
    get,
    path = "/internal/wg/weavers/{id}",
    params(
        ("id" = String, Path, description = "Weaver ID")
    ),
    responses(
        (status = 200, description = "Weaver info", body = WeaverResponse),
        (status = 401, description = "Invalid SVID", body = ErrorResponse),
        (status = 404, description = "Weaver not found", body = ErrorResponse),
        (status = 503, description = "WG tunnel not enabled", body = ErrorResponse)
    ),
    tag = "wgtunnel-internal"
)]
#[instrument(skip(state, headers), fields(weaver_id = %id))]
pub async fn get_weaver(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(id): Path<String>,
) -> impl IntoResponse {
	let svid_issuer = match state.svid_issuer.as_ref() {
		Some(issuer) => issuer,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(ErrorResponse {
					error: "service_unavailable".to_string(),
					message: "SVID validation not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let services = match state.wg_tunnel_services.as_ref() {
		Some(s) => s,
		None => {
			return (
				StatusCode::SERVICE_UNAVAILABLE,
				Json(ErrorResponse {
					error: "service_unavailable".to_string(),
					message: "WireGuard tunnel not enabled".to_string(),
				}),
			)
				.into_response();
		}
	};

	let token = match extract_bearer_token(&headers) {
		Ok(t) => t,
		Err(resp) => return resp.into_response(),
	};

	let claims = match svid_issuer.verify_svid(token).await {
		Ok(c) => c,
		Err(e) => {
			warn!(error = %e, "SVID validation failed");
			return (
				StatusCode::UNAUTHORIZED,
				Json(ErrorResponse {
					error: "svid_validation_failed".to_string(),
					message: "SVID validation failed".to_string(),
				}),
			)
				.into_response();
		}
	};

	if claims.weaver_id != id {
		return (
			StatusCode::FORBIDDEN,
			Json(ErrorResponse {
				error: "weaver_id_mismatch".to_string(),
				message: "SVID weaver_id does not match request".to_string(),
			}),
		)
			.into_response();
	}

	let weaver_id: Uuid = match id.parse() {
		Ok(id) => id,
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(ErrorResponse {
					error: "invalid_weaver_id".to_string(),
					message: "Invalid weaver ID format".to_string(),
				}),
			)
				.into_response();
		}
	};

	match services.weaver_service.get(weaver_id).await {
		Ok(Some(w)) => (
			StatusCode::OK,
			Json(WeaverResponse {
				weaver_id: w.weaver_id.to_string(),
				public_key: w.public_key_base64(),
				assigned_ip: w.assigned_ip.to_string(),
				derp_home_region: w.derp_home_region,
				endpoint: w.endpoint,
				registered_at: w.registered_at.to_rfc3339(),
				last_seen_at: w.last_seen_at.map(|t| t.to_rfc3339()),
			}),
		)
			.into_response(),
		Ok(None) => (
			StatusCode::NOT_FOUND,
			Json(ErrorResponse {
				error: "not_found".to_string(),
				message: "Weaver not registered".to_string(),
			}),
		)
			.into_response(),
		Err(e) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ErrorResponse {
				error: "internal_error".to_string(),
				message: e.to_string(),
			}),
		)
			.into_response(),
	}
}
