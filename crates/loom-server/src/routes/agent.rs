// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Agent provisioning HTTP handlers.

use std::collections::HashMap;
use std::convert::Infallible;

use axum::{
	extract::{Path, Query, State},
	http::{HeaderMap, StatusCode},
	middleware::{self, Next},
	response::{sse::Event, IntoResponse, Response, Sse},
	routing::{delete, get, post},
	Json, Router,
};
use chrono::{DateTime, Utc};
use futures::stream::{Stream, StreamExt};
use loom_agent_provisioner::{
	Agent, AgentId, AgentStatus, CreateAgentRequest, LogStreamOptions, ProvisionerError,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{api::AppState, error::ServerError};

// ============================================================================
// Request/Response types
// ============================================================================

/// Request to create a new agent.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateAgentApiRequest {
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

/// Resource limits for an agent.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct ResourceSpecApi {
	/// Memory limit (e.g., "8Gi")
	pub memory_limit: Option<String>,
	/// CPU limit (e.g., "4")
	pub cpu_limit: Option<String>,
}

/// Response for a single agent.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AgentApiResponse {
	/// Unique agent identifier
	pub id: String,
	/// Kubernetes Pod name
	pub pod_name: String,
	/// Current agent status
	pub status: AgentStatusApi,
	/// When the agent was created
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
}

/// Agent status for API responses.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatusApi {
	Pending,
	Running,
	Succeeded,
	Failed,
}

impl From<AgentStatus> for AgentStatusApi {
	fn from(status: AgentStatus) -> Self {
		match status {
			AgentStatus::Pending => AgentStatusApi::Pending,
			AgentStatus::Running => AgentStatusApi::Running,
			AgentStatus::Succeeded => AgentStatusApi::Succeeded,
			AgentStatus::Failed => AgentStatusApi::Failed,
		}
	}
}

impl From<Agent> for AgentApiResponse {
	fn from(agent: Agent) -> Self {
		Self {
			id: agent.id.to_string(),
			pod_name: agent.pod_name,
			status: agent.status.into(),
			created_at: agent.created_at,
			image: Some(agent.image),
			tags: Some(agent.tags),
			lifetime_hours: Some(agent.lifetime_hours),
			age_hours: Some(agent.age_hours),
		}
	}
}

/// Response for listing agents.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListAgentsApiResponse {
	/// List of agents
	pub agents: Vec<AgentApiResponse>,
	/// Total count of agents returned
	pub count: u32,
}

/// Query parameters for listing agents.
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct ListAgentsParams {
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
	/// If true, only list agents that would be deleted without actually deleting them
	#[serde(default)]
	pub dry_run: bool,
}

/// Response for cleanup operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CleanupApiResponse {
	/// Whether this was a dry run
	pub dry_run: bool,
	/// Agent IDs that were deleted (or would be deleted in dry run)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub deleted: Option<Vec<String>>,
	/// Agent IDs that would be deleted (dry run only)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub would_delete: Option<Vec<String>>,
	/// Number of agents affected
	pub count: u32,
}

// ============================================================================
// API Key middleware
// ============================================================================

/// Extension type for storing the validated API key
#[derive(Clone)]
pub struct ValidatedApiKey;

/// Middleware to require a valid API key in the X-API-Key header.
pub async fn require_agent_api_key(
	State(state): State<AppState>,
	headers: HeaderMap,
	request: axum::http::Request<axum::body::Body>,
	next: Next,
) -> Response {
	let expected_key = match &state.agent_api_key {
		Some(key) if !key.expose().is_empty() => key,
		_ => {
			tracing::error!("Agent API key not configured");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(crate::error::ErrorResponse {
					error: "configuration_error".to_string(),
					message: "Agent API key not configured".to_string(),
					server_version: None,
					client_version: None,
				}),
			)
				.into_response();
		}
	};

	let provided_key = headers
		.get("X-API-Key")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");

	if provided_key.is_empty() {
		return (
			StatusCode::UNAUTHORIZED,
			Json(crate::error::ErrorResponse {
				error: "unauthorized".to_string(),
				message: "Missing X-API-Key header".to_string(),
				server_version: None,
				client_version: None,
			}),
		)
			.into_response();
	}

	if provided_key != expected_key.expose() {
		tracing::warn!("Invalid API key provided for agent endpoint");
		return (
			StatusCode::UNAUTHORIZED,
			Json(crate::error::ErrorResponse {
				error: "unauthorized".to_string(),
				message: "Invalid API key".to_string(),
				server_version: None,
				client_version: None,
			}),
		)
			.into_response();
	}

	next.run(request).await
}

// ============================================================================
// Error conversion
// ============================================================================

impl From<ProvisionerError> for ServerError {
	fn from(err: ProvisionerError) -> Self {
		match err {
			ProvisionerError::AgentNotFound { id } => {
				ServerError::NotFound(format!("Agent not found: {}", id))
			}
			ProvisionerError::TooManyAgents { current, max } => ServerError::ServiceUnavailable(
				format!("Too many agents: {} running (max: {})", current, max),
			),
			ProvisionerError::InvalidLifetime { requested, max } => ServerError::BadRequest(
				format!("Invalid lifetime: {} hours (max: {} hours)", requested, max),
			),
			ProvisionerError::AgentFailed { id, reason } => {
				ServerError::Internal(format!("Agent {} failed: {}", id, reason))
			}
			ProvisionerError::AgentTimeout { id } => {
				ServerError::UpstreamTimeout(format!("Agent {} timed out waiting for ready state", id))
			}
			ProvisionerError::K8sError(e) => {
				ServerError::UpstreamError(format!("Kubernetes error: {}", e))
			}
			ProvisionerError::NamespaceNotFound { name } => {
				ServerError::Internal(format!("Namespace not found: {}", name))
			}
		}
	}
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/agent - Create a new agent.
#[utoipa::path(
    post,
    path = "/api/agent",
    request_body = CreateAgentApiRequest,
    responses(
        (status = 201, description = "Agent created", body = AgentApiResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 429, description = "Too many agents", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "agents",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn create_agent(
	State(state): State<AppState>,
	Json(req): Json<CreateAgentApiRequest>,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Agent provisioner not configured".to_string()))?;

	tracing::info!(image = %req.image, "Creating new agent");

	let create_req = CreateAgentRequest {
		image: req.image,
		env: req.env,
		resources: loom_agent_provisioner::ResourceSpec {
			memory_limit: req.resources.memory_limit,
			cpu_limit: req.resources.cpu_limit,
		},
		tags: req.tags,
		lifetime_hours: req.lifetime_hours,
		command: req.command,
		args: req.args,
		workdir: req.workdir,
	};

	let agent = provisioner.create_agent(create_req).await?;

	tracing::info!(agent_id = %agent.id, pod_name = %agent.pod_name, "Agent created");

	let response = AgentApiResponse {
		id: agent.id.to_string(),
		pod_name: agent.pod_name,
		status: agent.status.into(),
		created_at: agent.created_at,
		image: None,
		tags: None,
		lifetime_hours: None,
		age_hours: None,
	};

	Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/agents - List all agents.
#[utoipa::path(
    get,
    path = "/api/agents",
    params(ListAgentsParams),
    responses(
        (status = 200, description = "List of agents", body = ListAgentsApiResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "agents",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn list_agents(
	State(state): State<AppState>,
	Query(params): Query<ListAgentsParams>,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Agent provisioner not configured".to_string()))?;

	let tag_filter = parse_tag_filter(params.tag);

	tracing::debug!(tag_filter = ?tag_filter, "Listing agents");

	let agents = provisioner.list_agents(tag_filter).await?;
	let count = agents.len() as u32;

	let response = ListAgentsApiResponse {
		agents: agents.into_iter().map(Into::into).collect(),
		count,
	};

	Ok(Json(response))
}

/// GET /api/agent/{id} - Get agent details.
#[utoipa::path(
    get,
    path = "/api/agent/{id}",
    params(
        ("id" = String, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Agent details", body = AgentApiResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Agent not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "agents",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn get_agent(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Agent provisioner not configured".to_string()))?;

	let agent_id: AgentId = id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid agent ID: {}", id)))?;

	tracing::debug!(agent_id = %id, "Getting agent");

	let agent = provisioner.get_agent(&agent_id).await?;

	Ok(Json(AgentApiResponse::from(agent)))
}

/// DELETE /api/agent/{id} - Delete an agent.
#[utoipa::path(
    delete,
    path = "/api/agent/{id}",
    params(
        ("id" = String, Path, description = "Agent ID")
    ),
    responses(
        (status = 204, description = "Agent deleted"),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Agent not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "agents",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn delete_agent(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Agent provisioner not configured".to_string()))?;

	let agent_id: AgentId = id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid agent ID: {}", id)))?;

	tracing::info!(agent_id = %id, "Deleting agent");

	provisioner.delete_agent(&agent_id).await?;

	tracing::info!(agent_id = %id, "Agent deleted");

	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/agent/{id}/logs - Stream agent logs via SSE.
#[utoipa::path(
    get,
    path = "/api/agent/{id}/logs",
    params(
        ("id" = String, Path, description = "Agent ID"),
        LogStreamParams
    ),
    responses(
        (status = 200, description = "SSE log stream", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 404, description = "Agent not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "agents",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn stream_logs(
	State(state): State<AppState>,
	Path(id): Path<String>,
	Query(params): Query<LogStreamParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Agent provisioner not configured".to_string()))?;

	let agent_id: AgentId = id
		.parse()
		.map_err(|_| ServerError::BadRequest(format!("Invalid agent ID: {}", id)))?;

	tracing::debug!(agent_id = %id, tail = params.tail, timestamps = params.timestamps, "Starting log stream");

	let opts = LogStreamOptions {
		tail: params.tail,
		timestamps: params.timestamps,
	};

	let log_stream = provisioner.stream_logs(&agent_id, opts).await?;

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

/// POST /api/agents/cleanup - Trigger cleanup of expired agents.
#[utoipa::path(
    post,
    path = "/api/agents/cleanup",
    params(CleanupParams),
    responses(
        (status = 200, description = "Cleanup result", body = CleanupApiResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "agents",
    security(("api_key" = []))
)]
#[axum::debug_handler]
pub async fn trigger_cleanup(
	State(state): State<AppState>,
	Query(params): Query<CleanupParams>,
) -> Result<impl IntoResponse, ServerError> {
	let provisioner = state
		.provisioner
		.as_ref()
		.ok_or_else(|| ServerError::Internal("Agent provisioner not configured".to_string()))?;

	if params.dry_run {
		tracing::info!("Performing dry-run cleanup check");

		let expired = provisioner.find_expired_agents().await?;
		let agent_ids: Vec<String> = expired.iter().map(|a| a.id.to_string()).collect();
		let count = agent_ids.len() as u32;

		tracing::info!(count = count, "Found expired agents (dry run)");

		Ok(Json(CleanupApiResponse {
			dry_run: true,
			deleted: None,
			would_delete: Some(agent_ids),
			count,
		}))
	} else {
		tracing::info!("Triggering cleanup of expired agents");

		let result = provisioner.cleanup_expired_agents().await?;
		let agent_ids: Vec<String> = result.deleted.iter().map(|id| id.to_string()).collect();

		tracing::info!(count = result.count, "Cleanup completed");

		Ok(Json(CleanupApiResponse {
			dry_run: false,
			deleted: Some(agent_ids),
			would_delete: None,
			count: result.count,
		}))
	}
}

// ============================================================================
// Router
// ============================================================================

/// Create the agent routes router with API key authentication.
pub fn agent_routes(state: AppState) -> Router {
	Router::new()
		.route("/api/agent", post(create_agent))
		.route("/api/agents", get(list_agents))
		.route("/api/agent/{id}", get(get_agent))
		.route("/api/agent/{id}", delete(delete_agent))
		.route("/api/agent/{id}/logs", get(stream_logs))
		.route("/api/agents/cleanup", post(trigger_cleanup))
		.layer(middleware::from_fn_with_state(
			state.clone(),
			require_agent_api_key,
		))
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
