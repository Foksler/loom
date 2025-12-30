// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Weaver provisioning HTTP handlers.

use std::collections::HashMap;
use std::convert::Infallible;

use axum::{
	extract::{
		ws::{Message, WebSocket, WebSocketUpgrade},
		Path, Query, State,
	},
	http::StatusCode,
	response::{sse::Event, IntoResponse, Sse},
	routing::{delete, get, post},
	Json, Router,
};
use chrono::{DateTime, Utc};
use futures::{
	stream::{Stream, StreamExt},
	SinkExt,
};
use loom_auth::CurrentUser;
use loom_weaver::{
	CreateWeaverRequest, LogStreamOptions, ResourceSpec, Weaver, WeaverId, WeaverStatus,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use utoipa::{IntoParams, ToSchema};

use crate::{api::AppState, auth_middleware::RequireAuth, error::ServerError};

// ============================================================================
// Request/Response types
// ============================================================================

/// Request to create a new weaver.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateWeaverApiRequest {
	/// Container image to run
	pub image: String,
	/// Environment variables
	#[serde(default)]
	pub env: HashMap<String, String>,
	/// Resource limits
	#[serde(default)]
	pub resources: ResourceSpecApi,
	/// User-defined metadata tags
	#[serde(default)]
	pub tags: HashMap<String, String>,
	/// TTL override in hours (max: 48)
	pub lifetime_hours: Option<u32>,
	/// Override container ENTRYPOINT
	pub command: Option<Vec<String>>,
	/// Override container CMD
	pub args: Option<Vec<String>>,
	/// Override container WORKDIR
	pub workdir: Option<String>,
}

/// Resource limits for a weaver.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct ResourceSpecApi {
	/// Memory limit (e.g., "8Gi")
	pub memory_limit: Option<String>,
	/// CPU limit (e.g., "4")
	pub cpu_limit: Option<String>,
}

/// Response for a single weaver.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WeaverApiResponse {
	/// Unique weaver identifier
	pub id: String,
	/// Kubernetes Pod name
	pub pod_name: String,
	/// Current weaver status
	pub status: WeaverStatusApi,
	/// When the weaver was created
	pub created_at: DateTime<Utc>,
	/// Container image
	#[serde(skip_serializing_if = "Option::is_none")]
	pub image: Option<String>,
	/// User-defined metadata tags
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<HashMap<String, String>>,
	/// Configured lifetime in hours
	#[serde(skip_serializing_if = "Option::is_none")]
	pub lifetime_hours: Option<u32>,
	/// Current age in hours
	#[serde(skip_serializing_if = "Option::is_none")]
	pub age_hours: Option<f64>,
	/// Owner user ID
	#[serde(skip_serializing_if = "Option::is_none")]
	pub owner_user_id: Option<String>,
}

/// Weaver status for API responses.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeaverStatusApi {
	Pending,
	Running,
	Succeeded,
	Failed,
}

impl From<WeaverStatus> for WeaverStatusApi {
	fn from(status: WeaverStatus) -> Self {
		match status {
			WeaverStatus::Pending => WeaverStatusApi::Pending,
			WeaverStatus::Running => WeaverStatusApi::Running,
			WeaverStatus::Succeeded => WeaverStatusApi::Succeeded,
			WeaverStatus::Failed => WeaverStatusApi::Failed,
		}
	}
}

impl From<Weaver> for WeaverApiResponse {
	fn from(weaver: Weaver) -> Self {
		let owner_user_id = if weaver.owner_user_id.is_empty() {
			None
		} else {
			Some(weaver.owner_user_id)
		};
		Self {
			id: weaver.id.to_string(),
			pod_name: weaver.pod_name,
			status: weaver.status.into(),
			created_at: weaver.created_at,
			image: Some(weaver.image),
			tags: Some(weaver.tags),
			lifetime_hours: Some(weaver.lifetime_hours),
			age_hours: Some(weaver.age_hours),
			owner_user_id,
		}
	}
}

/// Response for listing weavers.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListWeaversApiResponse {
	/// List of weavers
	pub weavers: Vec<WeaverApiResponse>,
	/// Total count of weavers returned
	pub count: u32,
}

/// Query parameters for listing weavers.
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct ListWeaversParams {
	/// Filter by tag (format: key:value). Multiple allowed.
	#[serde(default)]
	#[param(value_type = Option<Vec<String>>)]
	pub tag: Option<Vec<String>>,
}

/// Query parameters for log streaming.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LogStreamParams {
	/// Number of lines to tail from the end (default: 256)
	#[serde(default = "default_tail")]
	pub tail: u32,
	/// Whether to include timestamps (default: true)
	#[serde(default = "default_timestamps")]
	pub timestamps: bool,
}

fn default_tail() -> u32 {
	256
}

fn default_timestamps() -> bool {
	true
}

/// Query parameters for cleanup endpoint.
#[derive(Debug, Deserialize, IntoParams)]
pub struct CleanupParams {
	/// If true, only list weavers that would be deleted without actually deleting them
	#[serde(default)]
	pub dry_run: bool,
}

/// Response for cleanup operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CleanupApiResponse {
	/// Whether this was a dry run
	pub dry_run: bool,
	/// Weaver IDs that were deleted (or would be deleted in dry run)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub deleted: Option<Vec<String>>,
	/// Weaver IDs that would be deleted (dry run only)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub would_delete: Option<Vec<String>>,
	/// Number of weavers affected
	pub count: u32,
}

// ============================================================================
// Helper functions
// ============================================================================

fn is_weaver_owner_or_admin(current_user: &CurrentUser, weaver: &Weaver) -> bool {
	if current_user.user.is_system_admin() {
		return true;
	}
	weaver.owner_user_id == current_user.user.id.to_string()
}

// ============================================================================
// Route handlers
// ============================================================================

/// POST /api/weaver - Create a new weaver.
#[utoipa::path(
    post,
    path = "/api/weaver",
    request_body = CreateWeaverApiRequest,
    responses(
        (status = 201, description = "Weaver created", body = WeaverApiResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "weavers",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn create_weaver(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Json(request): Json<CreateWeaverApiRequest>,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Weaver provisioner not configured".to_string()))?;

	let actor_id = current_user.user.id.to_string();
	tracing::info!(image = %request.image, actor_id = %actor_id, "Creating weaver");

	let create_request = CreateWeaverRequest {
		image: request.image,
		env: request.env,
		resources: ResourceSpec {
			memory_limit: request.resources.memory_limit,
			cpu_limit: request.resources.cpu_limit,
		},
		tags: request.tags,
		lifetime_hours: request.lifetime_hours,
		command: request.command,
		args: request.args,
		workdir: request.workdir,
		repo: None,
		branch: None,
		owner_user_id: Some(actor_id.clone()),
	};

	let weaver = provisioner.create_weaver(create_request).await?;

	tracing::info!(weaver_id = %weaver.id, pod_name = %weaver.pod_name, actor_id = %actor_id, "Weaver created");

	Ok((StatusCode::CREATED, Json(WeaverApiResponse::from(weaver))))
}

/// GET /api/weavers - List all weavers.
#[utoipa::path(
    get,
    path = "/api/weavers",
    params(ListWeaversParams),
    responses(
        (status = 200, description = "List of weavers", body = ListWeaversApiResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "weavers",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn list_weavers(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Query(params): Query<ListWeaversParams>,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Weaver provisioner not configured".to_string()))?;

	let tag_filter = parse_tag_filter(params.tag);

	let weavers = if current_user.user.is_system_admin() {
		provisioner.list_weavers(tag_filter).await?
	} else {
		let user_id = current_user.user.id.to_string();
		let all_weavers = provisioner.list_weavers_for_user(&user_id).await?;
		if let Some(tags) = tag_filter {
			all_weavers
				.into_iter()
				.filter(|w| tags.iter().all(|(k, v)| w.tags.get(k) == Some(v)))
				.collect()
		} else {
			all_weavers
		}
	};
	let count = weavers.len() as u32;

	let response = ListWeaversApiResponse {
		weavers: weavers.into_iter().map(WeaverApiResponse::from).collect(),
		count,
	};

	Ok(Json(response))
}

/// GET /api/weaver/{id} - Get a specific weaver.
#[utoipa::path(
    get,
    path = "/api/weaver/{id}",
    params(
        ("id" = String, Path, description = "Weaver ID")
    ),
    responses(
        (status = 200, description = "Weaver details", body = WeaverApiResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "Weaver not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "weavers",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn get_weaver(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Weaver provisioner not configured".to_string()))?;

	let weaver_id: WeaverId = id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid weaver ID: {id}")))?;

	let weaver = provisioner.get_weaver(&weaver_id).await?;

	if !is_weaver_owner_or_admin(&current_user, &weaver) {
		return Err(ServerError::Forbidden(
			"You do not have permission to access this weaver".to_string(),
		));
	}

	Ok(Json(WeaverApiResponse::from(weaver)))
}

/// DELETE /api/weaver/{id} - Delete a weaver.
#[utoipa::path(
    delete,
    path = "/api/weaver/{id}",
    params(
        ("id" = String, Path, description = "Weaver ID")
    ),
    responses(
        (status = 204, description = "Weaver deleted"),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "Weaver not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "weavers",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn delete_weaver(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Weaver provisioner not configured".to_string()))?;

	let weaver_id: WeaverId = id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid weaver ID: {id}")))?;

	let weaver = provisioner.get_weaver(&weaver_id).await?;

	if !is_weaver_owner_or_admin(&current_user, &weaver) {
		return Err(ServerError::Forbidden(
			"You do not have permission to delete this weaver".to_string(),
		));
	}

	let actor_id = current_user.user.id.to_string();
	tracing::info!(weaver_id = %id, actor_id = %actor_id, "Deleting weaver");

	provisioner.delete_weaver(&weaver_id).await?;

	tracing::info!(weaver_id = %id, actor_id = %actor_id, "Weaver deleted");

	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/weaver/{id}/logs - Stream weaver logs via SSE.
#[utoipa::path(
    get,
    path = "/api/weaver/{id}/logs",
    params(
        ("id" = String, Path, description = "Weaver ID"),
        LogStreamParams
    ),
    responses(
        (status = 200, description = "SSE log stream", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "Weaver not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "weavers",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn stream_logs(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(id): Path<String>,
	Query(params): Query<LogStreamParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Weaver provisioner not configured".to_string()))?;

	let weaver_id: WeaverId = id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid weaver ID: {id}")))?;

	let weaver = provisioner.get_weaver(&weaver_id).await?;

	if !is_weaver_owner_or_admin(&current_user, &weaver) {
		return Err(ServerError::Forbidden(
			"You do not have permission to access this weaver's logs".to_string(),
		));
	}

	tracing::debug!(weaver_id = %id, tail = params.tail, timestamps = params.timestamps, "Starting log stream");

	let opts = LogStreamOptions {
		tail: params.tail,
		timestamps: params.timestamps,
	};

	let log_stream = provisioner.stream_logs(&weaver_id, opts).await?;

	let sse_stream = log_stream.map(|result| {
		let event = match result {
			Ok(bytes) => {
				let line = String::from_utf8_lossy(&bytes).into_owned();
				Event::default().data(line)
			}
			Err(e) => Event::default().event("error").data(e.to_string()),
		};
		Ok::<_, Infallible>(event)
	});

	Ok(Sse::new(sse_stream).keep_alive(
		axum::response::sse::KeepAlive::new()
			.interval(std::time::Duration::from_secs(15))
			.text("keep-alive"),
	))
}

/// POST /api/weavers/cleanup - Trigger cleanup of expired weavers.
#[utoipa::path(
    post,
    path = "/api/weavers/cleanup",
    params(CleanupParams),
    responses(
        (status = 200, description = "Cleanup result", body = CleanupApiResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "Forbidden - system admin required", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "weavers",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn trigger_cleanup(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Query(params): Query<CleanupParams>,
) -> Result<impl IntoResponse, ServerError> {
	if !current_user.user.is_system_admin() {
		return Err(ServerError::Forbidden(
			"System admin access required for cleanup operations".to_string(),
		));
	}

	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Weaver provisioner not configured".to_string()))?;

	let actor_id = current_user.user.id.to_string();

	if params.dry_run {
		tracing::info!(actor_id = %actor_id, "Performing dry-run cleanup check");

		let expired = provisioner.find_expired_weavers().await?;
		let weaver_ids: Vec<String> = expired.iter().map(|w| w.id.to_string()).collect();
		let count = weaver_ids.len() as u32;

		tracing::info!(count = count, actor_id = %actor_id, "Found expired weavers (dry run)");

		Ok(Json(CleanupApiResponse {
			dry_run: true,
			deleted: None,
			would_delete: Some(weaver_ids),
			count,
		}))
	} else {
		tracing::info!(actor_id = %actor_id, "Triggering cleanup of expired weavers");

		let result = provisioner.cleanup_expired_weavers().await?;
		let weaver_ids: Vec<String> = result.deleted.iter().map(|id| id.to_string()).collect();

		tracing::info!(count = result.count, actor_id = %actor_id, "Cleanup completed");

		Ok(Json(CleanupApiResponse {
			dry_run: false,
			deleted: Some(weaver_ids),
			would_delete: None,
			count: result.count,
		}))
	}
}

/// GET /api/weaver/{id}/attach - WebSocket terminal attach.
#[utoipa::path(
    get,
    path = "/api/weaver/{id}/attach",
    params(
        ("id" = String, Path, description = "Weaver ID")
    ),
    responses(
        (status = 101, description = "WebSocket upgrade for terminal I/O"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Weaver not found")
    ),
    tag = "weavers"
)]
pub async fn attach_weaver(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(id): Path<String>,
	ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Weaver provisioner not configured".to_string()))?
		.clone();

	let weaver_id: WeaverId = id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid weaver ID: {id}")))?;

	let weaver = provisioner
		.get_weaver(&weaver_id)
		.await
		.map_err(|_| ServerError::NotFound(format!("Weaver not found: {id}")))?;

	if !is_weaver_owner_or_admin(&current_user, &weaver) {
		return Err(ServerError::Forbidden(
			"You do not have permission to attach to this weaver".to_string(),
		));
	}

	let actor_id = current_user.user.id.to_string();
	tracing::info!(weaver_id = %id, actor_id = %actor_id, "WebSocket attach requested");

	Ok(ws.on_upgrade(move |socket| handle_attach_websocket(socket, provisioner, weaver_id)))
}

async fn handle_attach_websocket(
	socket: WebSocket,
	provisioner: std::sync::Arc<loom_weaver::Provisioner>,
	weaver_id: WeaverId,
) {
	if let Err(e) = handle_attach_websocket_inner(socket, provisioner, weaver_id).await {
		tracing::error!(error = %e, "WebSocket attach error");
	}
}

async fn handle_attach_websocket_inner(
	socket: WebSocket,
	provisioner: std::sync::Arc<loom_weaver::Provisioner>,
	weaver_id: WeaverId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let attached = provisioner.attach_weaver(&weaver_id).await?;
	let loom_weaver::AttachedProcess { stdin, stdout } = attached;

	let (mut ws_sender, mut ws_receiver) = socket.split();
	let mut stdin = stdin;
	let mut stdout = stdout;

	let ws_to_pod = async {
		while let Some(msg) = ws_receiver.next().await {
			match msg {
				Ok(Message::Binary(data)) => {
					if stdin.write_all(&data).await.is_err() {
						break;
					}
				}
				Ok(Message::Text(text)) => {
					if stdin.write_all(text.as_bytes()).await.is_err() {
						break;
					}
				}
				Ok(Message::Close(_)) | Err(_) => break,
				_ => {}
			}
		}
	};

	let pod_to_ws = async {
		let mut buf = [0u8; 4096];
		loop {
			match stdout.read(&mut buf).await {
				Ok(0) => break,
				Ok(n) => {
					if ws_sender
						.send(Message::Binary(buf[..n].to_vec().into()))
						.await
						.is_err()
					{
						break;
					}
				}
				Err(_) => break,
			}
		}
	};

	tokio::select! {
		_ = ws_to_pod => {}
		_ = pod_to_ws => {}
	}

	tracing::debug!(weaver_id = %weaver_id, "WebSocket attach session ended");
	Ok(())
}

// ============================================================================
// Router
// ============================================================================

/// Create the weaver routes router.
pub fn weaver_routes(state: AppState) -> Router {
	Router::new()
		.route("/api/weaver", post(create_weaver))
		.route("/api/weavers", get(list_weavers))
		.route("/api/weaver/{id}", get(get_weaver))
		.route("/api/weaver/{id}", delete(delete_weaver))
		.route("/api/weaver/{id}/logs", get(stream_logs))
		.route("/api/weaver/{id}/attach", get(attach_weaver))
		.route("/api/weavers/cleanup", post(trigger_cleanup))
		.with_state(state)
}

// ============================================================================
// Helper functions
// ============================================================================

/// Parse tag filter from query parameters.
/// Tags are provided as "key:value" strings.
fn parse_tag_filter(tags: Option<Vec<String>>) -> Option<HashMap<String, String>> {
	tags.map(|tag_list| {
		tag_list
			.into_iter()
			.filter_map(|t| {
				let parts: Vec<&str> = t.splitn(2, ':').collect();
				if parts.len() == 2 {
					Some((parts[0].to_string(), parts[1].to_string()))
				} else {
					None
				}
			})
			.collect()
	})
	.filter(|m: &HashMap<String, String>| !m.is_empty())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_tag_filter_none() {
		assert_eq!(parse_tag_filter(None), None);
	}

	#[test]
	fn test_parse_tag_filter_empty() {
		assert_eq!(parse_tag_filter(Some(vec![])), None);
	}

	#[test]
	fn test_parse_tag_filter_single() {
		let result = parse_tag_filter(Some(vec!["project:ai-worker".to_string()]));
		let mut expected = HashMap::new();
		expected.insert("project".to_string(), "ai-worker".to_string());
		assert_eq!(result, Some(expected));
	}

	#[test]
	fn test_parse_tag_filter_multiple() {
		let result = parse_tag_filter(Some(vec![
			"project:ai-worker".to_string(),
			"env:prod".to_string(),
		]));
		let mut expected = HashMap::new();
		expected.insert("project".to_string(), "ai-worker".to_string());
		expected.insert("env".to_string(), "prod".to_string());
		assert_eq!(result, Some(expected));
	}

	#[test]
	fn test_parse_tag_filter_invalid() {
		let result = parse_tag_filter(Some(vec!["invalid-no-colon".to_string()]));
		assert_eq!(result, None);
	}

	#[test]
	fn test_parse_tag_filter_value_with_colon() {
		let result = parse_tag_filter(Some(vec!["url:https://example.com".to_string()]));
		let mut expected = HashMap::new();
		expected.insert("url".to_string(), "https://example.com".to_string());
		assert_eq!(result, Some(expected));
	}
}
