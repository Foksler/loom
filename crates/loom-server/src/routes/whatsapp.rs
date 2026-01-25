// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! WhatsApp webhook HTTP handlers.

use axum::{
	body::Bytes,
	extract::{Query, State},
	http::{HeaderMap, StatusCode},
	response::IntoResponse,
};
use loom_whatsapp::{verify_webhook_signature, WebhookPayload, WebhookVerifyParams};
use tracing::{debug, info, warn};

use crate::{api::AppState, error::ServerError};

/// GET /api/whatsapp/webhook - Handle WhatsApp webhook verification challenge.
///
/// Meta sends a verification request when configuring a webhook:
/// - hub.mode = "subscribe"
/// - hub.challenge = random string to echo back
/// - hub.verify_token = token configured in Meta dashboard
#[axum::debug_handler]
#[tracing::instrument(skip(state, params))]
pub async fn whatsapp_webhook_verify(
	State(state): State<AppState>,
	Query(params): Query<WebhookVerifyParams>,
) -> Result<impl IntoResponse, ServerError> {
	debug!(mode = %params.mode, "WhatsApp webhook verification request");

	// 1. Check mode
	if params.mode != "subscribe" {
		warn!(mode = %params.mode, "Invalid webhook mode");
		return Err(ServerError::BadRequest("Invalid mode".into()));
	}

	// 2. Verify WhatsApp is configured
	if state.whatsapp_service.is_none() {
		warn!("WhatsApp not configured");
		return Err(ServerError::ServiceUnavailable("WhatsApp is not configured".into()));
	}

	// 3. Verify token against stored hash
	// For now, we verify against all configs and accept if any match
	let whatsapp_repo = state.whatsapp_repo.as_ref().ok_or_else(|| {
		ServerError::ServiceUnavailable("WhatsApp is not configured".into())
	})?;

	let configs = whatsapp_repo
		.list_enabled_configs()
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to list configs: {e}")))?;

	let mut valid = false;
	for config in configs {
		// Verify token matches hash using argon2
		if loom_server_whatsapp::verify_token(&params.verify_token, &config.verify_token_hash) {
			valid = true;
			break;
		}
	}

	if !valid {
		warn!("Webhook verification token mismatch");
		return Err(ServerError::Unauthorized("Invalid verify_token".into()));
	}

	// 4. Echo back the challenge
	info!("WhatsApp webhook verification successful");
	Ok(params.challenge)
}

/// POST /api/whatsapp/webhook - Handle incoming WhatsApp messages and status updates.
///
/// Security: Verifies X-Hub-Signature-256 HMAC signature before processing.
#[axum::debug_handler]
#[tracing::instrument(skip(state, headers, body))]
pub async fn whatsapp_webhook(
	State(state): State<AppState>,
	headers: HeaderMap,
	body: Bytes,
) -> Result<impl IntoResponse, ServerError> {
	debug!("WhatsApp webhook received");

	// 1. Ensure WhatsApp is configured
	let whatsapp_service = state.whatsapp_service.as_ref().ok_or_else(|| {
		warn!("WhatsApp webhook received but not configured");
		ServerError::ServiceUnavailable("WhatsApp is not configured".into())
	})?;

	let whatsapp_repo = state.whatsapp_repo.as_ref().ok_or_else(|| {
		ServerError::ServiceUnavailable("WhatsApp is not configured".into())
	})?;

	// 2. Extract signature header
	let sig_header = headers
		.get("X-Hub-Signature-256")
		.and_then(|v| v.to_str().ok())
		.ok_or_else(|| {
			warn!("Missing X-Hub-Signature-256 header");
			ServerError::BadRequest("Missing X-Hub-Signature-256 header".into())
		})?;

	// 3. Parse payload to find phone_number_id for config lookup
	let payload: WebhookPayload = serde_json::from_slice(&body).map_err(|e| {
		warn!(error = %e, "Failed to parse webhook payload");
		ServerError::BadRequest(format!("Invalid webhook payload: {e}"))
	})?;

	// 4. Find the phone_number_id from the payload
	let phone_number_id = payload
		.entry
		.first()
		.and_then(|e| e.changes.first())
		.map(|c| c.value.metadata.phone_number_id.as_str())
		.ok_or_else(|| {
			warn!("No phone_number_id in webhook payload");
			ServerError::BadRequest("No phone_number_id in payload".into())
		})?;

	// 5. Look up config by phone_number_id to get app_secret for signature verification
	let config = whatsapp_repo
		.find_config_by_phone_number_id(phone_number_id)
		.await
		.map_err(|e| ServerError::Internal(format!("Failed to lookup config: {e}")))?
		.ok_or_else(|| {
			warn!(phone_number_id = %phone_number_id, "No config found for phone_number_id");
			ServerError::NotFound(format!(
				"No WhatsApp config for phone_number_id: {phone_number_id}"
			))
		})?;

	// 6. Verify signature using the app_secret from config
	if let Err(e) = verify_webhook_signature(&config.app_secret_hash, sig_header, &body) {
		warn!(error = %e, "Webhook signature verification failed");
		return Err(ServerError::Unauthorized("Invalid webhook signature".into()));
	}

	// 7. Process the webhook payload asynchronously
	// Return 200 OK immediately per WhatsApp requirements
	let service = whatsapp_service.clone();
	tokio::spawn(async move {
		if let Err(e) = service.process_webhook(payload).await {
			tracing::error!(error = %e, "Failed to process WhatsApp webhook");
		}
	});

	info!("WhatsApp webhook accepted for processing");
	Ok(StatusCode::OK)
}
