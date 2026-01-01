// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::{DateTime, Utc};
use loom_auth::types::{OrgId, OrgRole};
use loom_scm::{OwnerType, PayloadFormat, RepoStore, Webhook, WebhookOwnerType, WebhookStore};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{api::AppState, auth_middleware::RequireAuth};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadFormatApi {
	GitHubCompat,
	LoomV1,
}

impl Default for PayloadFormatApi {
	fn default() -> Self {
		PayloadFormatApi::LoomV1
	}
}

impl From<PayloadFormat> for PayloadFormatApi {
	fn from(v: PayloadFormat) -> Self {
		match v {
			PayloadFormat::GitHubCompat => PayloadFormatApi::GitHubCompat,
			PayloadFormat::LoomV1 => PayloadFormatApi::LoomV1,
		}
	}
}

impl From<PayloadFormatApi> for PayloadFormat {
	fn from(v: PayloadFormatApi) -> Self {
		match v {
			PayloadFormatApi::GitHubCompat => PayloadFormat::GitHubCompat,
			PayloadFormatApi::LoomV1 => PayloadFormat::LoomV1,
		}
	}
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWebhookRequest {
	pub url: String,
	pub secret: String,
	#[serde(default)]
	pub payload_format: PayloadFormatApi,
	pub events: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookResponse {
	pub id: Uuid,
	pub url: String,
	pub payload_format: PayloadFormatApi,
	pub events: Vec<String>,
	pub enabled: bool,
	pub created_at: DateTime<Utc>,
}

impl From<Webhook> for WebhookResponse {
	fn from(w: Webhook) -> Self {
		Self {
			id: w.id,
			url: w.url,
			payload_format: w.payload_format.into(),
			events: w.events,
			enabled: w.enabled,
			created_at: w.created_at,
		}
	}
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListWebhooksResponse {
	pub webhooks: Vec<WebhookResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookSuccessResponse {
	pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookErrorResponse {
	pub error: String,
	pub message: String,
}

const VALID_EVENTS: &[&str] = &["push", "repo.created", "repo.deleted"];

fn validate_events(events: &[String]) -> Option<String> {
	if events.is_empty() {
		return Some("At least one event is required".to_string());
	}
	for event in events {
		if !VALID_EVENTS.contains(&event.as_str()) {
			return Some(format!(
				"Invalid event '{}'. Valid events: {}",
				event,
				VALID_EVENTS.join(", ")
			));
		}
	}
	None
}

fn validate_url(url: &str) -> Option<String> {
	if url.is_empty() {
		return Some("URL is required".to_string());
	}
	if !url.starts_with("https://") && !url.starts_with("http://") {
		return Some("URL must start with http:// or https://".to_string());
	}
	if url.len() > 2048 {
		return Some("URL must be less than 2048 characters".to_string());
	}
	None
}

async fn check_repo_admin(
	repo_id: Uuid,
	current_user: &loom_auth::middleware::CurrentUser,
	state: &AppState,
) -> Result<(), (StatusCode, Json<WebhookErrorResponse>)> {
	let scm_store = state.scm_repo_store.as_ref().ok_or_else(|| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(WebhookErrorResponse {
				error: "not_configured".to_string(),
				message: "SCM not configured".to_string(),
			}),
		)
	})?;

	let repo = scm_store.get_by_id(repo_id).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to get repository");
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(WebhookErrorResponse {
				error: "internal_error".to_string(),
				message: "Internal server error".to_string(),
			}),
		)
	})?;

	let repo = repo.ok_or_else(|| {
		(
			StatusCode::NOT_FOUND,
			Json(WebhookErrorResponse {
				error: "not_found".to_string(),
				message: "Repository not found".to_string(),
			}),
		)
	})?;

	let is_admin = match repo.owner_type {
		OwnerType::User => repo.owner_id == current_user.user.id.into_inner(),
		OwnerType::Org => {
			let org_id = OrgId::new(repo.owner_id);
			match state
				.org_repo
				.get_membership(&org_id, &current_user.user.id)
				.await
			{
				Ok(Some(m)) => m.role == OrgRole::Owner || m.role == OrgRole::Admin,
				_ => false,
			}
		}
	};

	if !is_admin {
		return Err((
			StatusCode::FORBIDDEN,
			Json(WebhookErrorResponse {
				error: "forbidden".to_string(),
				message: "Admin access required".to_string(),
			}),
		));
	}

	Ok(())
}

async fn check_org_admin(
	org_id: Uuid,
	current_user: &loom_auth::middleware::CurrentUser,
	state: &AppState,
) -> Result<(), (StatusCode, Json<WebhookErrorResponse>)> {
	let org_id_typed = OrgId::new(org_id);

	let org = state.org_repo.get_org_by_id(&org_id_typed).await.map_err(|e| {
		tracing::error!(error = %e, "Failed to get organization");
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(WebhookErrorResponse {
				error: "internal_error".to_string(),
				message: "Internal server error".to_string(),
			}),
		)
	})?;

	if org.is_none() {
		return Err((
			StatusCode::NOT_FOUND,
			Json(WebhookErrorResponse {
				error: "not_found".to_string(),
				message: "Organization not found".to_string(),
			}),
		));
	}

	let membership = state
		.org_repo
		.get_membership(&org_id_typed, &current_user.user.id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to check org membership");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "internal_error".to_string(),
					message: "Internal server error".to_string(),
				}),
			)
		})?;

	let is_admin = match membership {
		Some(m) => m.role == OrgRole::Owner || m.role == OrgRole::Admin,
		None => false,
	};

	if !is_admin {
		return Err((
			StatusCode::FORBIDDEN,
			Json(WebhookErrorResponse {
				error: "forbidden".to_string(),
				message: "Admin access required".to_string(),
			}),
		));
	}

	Ok(())
}

#[utoipa::path(
	get,
	path = "/api/v1/repos/{id}/webhooks",
	params(
		("id" = Uuid, Path, description = "Repository ID")
	),
	responses(
		(status = 200, description = "List of webhooks", body = ListWebhooksResponse),
		(status = 401, description = "Not authenticated", body = WebhookErrorResponse),
		(status = 403, description = "Not authorized", body = WebhookErrorResponse),
		(status = 404, description = "Repository not found", body = WebhookErrorResponse)
	),
	tag = "webhooks"
)]
#[tracing::instrument(skip(state), fields(repo_id = %id))]
pub async fn list_repo_webhooks(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
) -> impl IntoResponse {
	if let Err(e) = check_repo_admin(id, &current_user, &state).await {
		return e.into_response();
	}

	let webhook_store = match state.scm_webhook_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "not_configured".to_string(),
					message: "SCM not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	match webhook_store.list_by_repo(id).await {
		Ok(webhooks) => {
			let response = ListWebhooksResponse {
				webhooks: webhooks.into_iter().map(Into::into).collect(),
			};
			(StatusCode::OK, Json(response)).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list webhooks");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to list webhooks".to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
	post,
	path = "/api/v1/repos/{id}/webhooks",
	params(
		("id" = Uuid, Path, description = "Repository ID")
	),
	request_body = CreateWebhookRequest,
	responses(
		(status = 201, description = "Webhook created", body = WebhookResponse),
		(status = 400, description = "Invalid request", body = WebhookErrorResponse),
		(status = 401, description = "Not authenticated", body = WebhookErrorResponse),
		(status = 403, description = "Not authorized", body = WebhookErrorResponse),
		(status = 404, description = "Repository not found", body = WebhookErrorResponse)
	),
	tag = "webhooks"
)]
#[tracing::instrument(skip(state, payload), fields(repo_id = %id))]
pub async fn create_repo_webhook(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	Json(payload): Json<CreateWebhookRequest>,
) -> impl IntoResponse {
	if let Err(e) = check_repo_admin(id, &current_user, &state).await {
		return e.into_response();
	}

	if let Some(error) = validate_url(&payload.url) {
		return (
			StatusCode::BAD_REQUEST,
			Json(WebhookErrorResponse {
				error: "invalid_url".to_string(),
				message: error,
			}),
		)
			.into_response();
	}

	if let Some(error) = validate_events(&payload.events) {
		return (
			StatusCode::BAD_REQUEST,
			Json(WebhookErrorResponse {
				error: "invalid_events".to_string(),
				message: error,
			}),
		)
			.into_response();
	}

	if payload.secret.is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(WebhookErrorResponse {
				error: "invalid_secret".to_string(),
				message: "Secret is required".to_string(),
			}),
		)
			.into_response();
	}

	let webhook_store = match state.scm_webhook_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "not_configured".to_string(),
					message: "SCM not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let webhook = Webhook::new(
		WebhookOwnerType::Repo,
		id,
		payload.url,
		payload.secret,
		payload.payload_format.into(),
		payload.events,
	);

	match webhook_store.create(&webhook).await {
		Ok(created) => {
			tracing::info!(
				repo_id = %id,
				webhook_id = %created.id,
				url = %created.url,
				created_by = %current_user.user.id,
				"Webhook created"
			);
			(StatusCode::CREATED, Json(WebhookResponse::from(created))).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to create webhook");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to create webhook".to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
	delete,
	path = "/api/v1/repos/{id}/webhooks/{wid}",
	params(
		("id" = Uuid, Path, description = "Repository ID"),
		("wid" = Uuid, Path, description = "Webhook ID")
	),
	responses(
		(status = 204, description = "Webhook deleted"),
		(status = 401, description = "Not authenticated", body = WebhookErrorResponse),
		(status = 403, description = "Not authorized", body = WebhookErrorResponse),
		(status = 404, description = "Repository or webhook not found", body = WebhookErrorResponse)
	),
	tag = "webhooks"
)]
#[tracing::instrument(skip(state), fields(repo_id = %id, webhook_id = %wid))]
pub async fn delete_repo_webhook(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((id, wid)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
	if let Err(e) = check_repo_admin(id, &current_user, &state).await {
		return e.into_response();
	}

	let webhook_store = match state.scm_webhook_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "not_configured".to_string(),
					message: "SCM not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let webhook = match webhook_store.get_by_id(wid).await {
		Ok(Some(w)) => w,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(WebhookErrorResponse {
					error: "not_found".to_string(),
					message: "Webhook not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get webhook");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "internal_error".to_string(),
					message: "Internal server error".to_string(),
				}),
			)
				.into_response();
		}
	};

	if webhook.owner_type != WebhookOwnerType::Repo || webhook.owner_id != id {
		return (
			StatusCode::NOT_FOUND,
			Json(WebhookErrorResponse {
				error: "not_found".to_string(),
				message: "Webhook not found".to_string(),
			}),
		)
			.into_response();
	}

	match webhook_store.delete(wid).await {
		Ok(()) => {
			tracing::info!(
				repo_id = %id,
				webhook_id = %wid,
				deleted_by = %current_user.user.id,
				"Webhook deleted"
			);
			StatusCode::NO_CONTENT.into_response()
		}
		Err(loom_scm::ScmError::NotFound) => (
			StatusCode::NOT_FOUND,
			Json(WebhookErrorResponse {
				error: "not_found".to_string(),
				message: "Webhook not found".to_string(),
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to delete webhook");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to delete webhook".to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
	get,
	path = "/api/v1/orgs/{id}/webhooks",
	params(
		("id" = Uuid, Path, description = "Organization ID")
	),
	responses(
		(status = 200, description = "List of webhooks", body = ListWebhooksResponse),
		(status = 401, description = "Not authenticated", body = WebhookErrorResponse),
		(status = 403, description = "Not authorized", body = WebhookErrorResponse),
		(status = 404, description = "Organization not found", body = WebhookErrorResponse)
	),
	tag = "webhooks"
)]
#[tracing::instrument(skip(state), fields(org_id = %id))]
pub async fn list_org_webhooks(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
) -> impl IntoResponse {
	if let Err(e) = check_org_admin(id, &current_user, &state).await {
		return e.into_response();
	}

	let webhook_store = match state.scm_webhook_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "not_configured".to_string(),
					message: "SCM not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	match webhook_store.list_by_org(id).await {
		Ok(webhooks) => {
			let response = ListWebhooksResponse {
				webhooks: webhooks.into_iter().map(Into::into).collect(),
			};
			(StatusCode::OK, Json(response)).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list webhooks");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to list webhooks".to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
	post,
	path = "/api/v1/orgs/{id}/webhooks",
	params(
		("id" = Uuid, Path, description = "Organization ID")
	),
	request_body = CreateWebhookRequest,
	responses(
		(status = 201, description = "Webhook created", body = WebhookResponse),
		(status = 400, description = "Invalid request", body = WebhookErrorResponse),
		(status = 401, description = "Not authenticated", body = WebhookErrorResponse),
		(status = 403, description = "Not authorized", body = WebhookErrorResponse),
		(status = 404, description = "Organization not found", body = WebhookErrorResponse)
	),
	tag = "webhooks"
)]
#[tracing::instrument(skip(state, payload), fields(org_id = %id))]
pub async fn create_org_webhook(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	Json(payload): Json<CreateWebhookRequest>,
) -> impl IntoResponse {
	if let Err(e) = check_org_admin(id, &current_user, &state).await {
		return e.into_response();
	}

	if let Some(error) = validate_url(&payload.url) {
		return (
			StatusCode::BAD_REQUEST,
			Json(WebhookErrorResponse {
				error: "invalid_url".to_string(),
				message: error,
			}),
		)
			.into_response();
	}

	if let Some(error) = validate_events(&payload.events) {
		return (
			StatusCode::BAD_REQUEST,
			Json(WebhookErrorResponse {
				error: "invalid_events".to_string(),
				message: error,
			}),
		)
			.into_response();
	}

	if payload.secret.is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(WebhookErrorResponse {
				error: "invalid_secret".to_string(),
				message: "Secret is required".to_string(),
			}),
		)
			.into_response();
	}

	let webhook_store = match state.scm_webhook_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "not_configured".to_string(),
					message: "SCM not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let webhook = Webhook::new(
		WebhookOwnerType::Org,
		id,
		payload.url,
		payload.secret,
		payload.payload_format.into(),
		payload.events,
	);

	match webhook_store.create(&webhook).await {
		Ok(created) => {
			tracing::info!(
				org_id = %id,
				webhook_id = %created.id,
				url = %created.url,
				created_by = %current_user.user.id,
				"Org webhook created"
			);
			(StatusCode::CREATED, Json(WebhookResponse::from(created))).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to create webhook");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to create webhook".to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
	delete,
	path = "/api/v1/orgs/{id}/webhooks/{wid}",
	params(
		("id" = Uuid, Path, description = "Organization ID"),
		("wid" = Uuid, Path, description = "Webhook ID")
	),
	responses(
		(status = 204, description = "Webhook deleted"),
		(status = 401, description = "Not authenticated", body = WebhookErrorResponse),
		(status = 403, description = "Not authorized", body = WebhookErrorResponse),
		(status = 404, description = "Organization or webhook not found", body = WebhookErrorResponse)
	),
	tag = "webhooks"
)]
#[tracing::instrument(skip(state), fields(org_id = %id, webhook_id = %wid))]
pub async fn delete_org_webhook(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((id, wid)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
	if let Err(e) = check_org_admin(id, &current_user, &state).await {
		return e.into_response();
	}

	let webhook_store = match state.scm_webhook_store.as_ref() {
		Some(store) => store,
		None => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "not_configured".to_string(),
					message: "SCM not configured".to_string(),
				}),
			)
				.into_response();
		}
	};

	let webhook = match webhook_store.get_by_id(wid).await {
		Ok(Some(w)) => w,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(WebhookErrorResponse {
					error: "not_found".to_string(),
					message: "Webhook not found".to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get webhook");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "internal_error".to_string(),
					message: "Internal server error".to_string(),
				}),
			)
				.into_response();
		}
	};

	if webhook.owner_type != WebhookOwnerType::Org || webhook.owner_id != id {
		return (
			StatusCode::NOT_FOUND,
			Json(WebhookErrorResponse {
				error: "not_found".to_string(),
				message: "Webhook not found".to_string(),
			}),
		)
			.into_response();
	}

	match webhook_store.delete(wid).await {
		Ok(()) => {
			tracing::info!(
				org_id = %id,
				webhook_id = %wid,
				deleted_by = %current_user.user.id,
				"Org webhook deleted"
			);
			StatusCode::NO_CONTENT.into_response()
		}
		Err(loom_scm::ScmError::NotFound) => (
			StatusCode::NOT_FOUND,
			Json(WebhookErrorResponse {
				error: "not_found".to_string(),
				message: "Webhook not found".to_string(),
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to delete webhook");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(WebhookErrorResponse {
					error: "internal_error".to_string(),
					message: "Failed to delete webhook".to_string(),
				}),
			)
				.into_response()
		}
	}
}
