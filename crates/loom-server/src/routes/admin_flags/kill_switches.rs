// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Platform kill switch handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_flags_core::{KillSwitch, KillSwitchId, UserId};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_flags::FlagsRepository;
use serde_json::json;

use crate::{
	api::AppState,
	api_response::{bad_request, conflict, internal_error, not_found},
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	routes::admin::AdminErrorResponse,
};

use super::common::{
	kill_switch_to_response, ActivateKillSwitchRequest, CreateKillSwitchRequest,
	FlagsErrorResponse, FlagsSuccessResponse, KillSwitchResponse, ListKillSwitchesResponse,
	UpdateKillSwitchRequest,
};

/// List all platform-level kill switches.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	get,
	path = "/api/admin/flags/kill-switches",
	responses(
		(status = 200, description = "List of platform kill switches", body = ListKillSwitchesResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id))]
pub async fn list_platform_kill_switches(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform kill switches list attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let kill_switches = match state.flags_repo.list_kill_switches(None).await {
		Ok(ks) => ks,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list platform kill switches");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	tracing::info!(
		actor_id = %current_user.user.id,
		count = kill_switches.len(),
		"Listed platform kill switches"
	);

	let ks_response: Vec<KillSwitchResponse> =
		kill_switches.iter().map(kill_switch_to_response).collect();

	(
		StatusCode::OK,
		Json(ListKillSwitchesResponse {
			kill_switches: ks_response,
		}),
	)
		.into_response()
}

/// Create a new platform-level kill switch.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	post,
	path = "/api/admin/flags/kill-switches",
	request_body = CreateKillSwitchRequest,
	responses(
		(status = 201, description = "Platform kill switch created", body = KillSwitchResponse),
		(status = 400, description = "Invalid request", body = FlagsErrorResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 409, description = "Kill switch key already exists", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state, payload), fields(actor_id = %current_user.user.id, ks_key = %payload.key))]
pub async fn create_platform_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Json(payload): Json<CreateKillSwitchRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform kill switch creation attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	// Validate kill switch key
	if !KillSwitch::validate_key(&payload.key) {
		return bad_request::<FlagsErrorResponse>(
			"invalid_key",
			t(locale, "server.api.flags.invalid_kill_switch_key"),
		)
		.into_response();
	}

	// Check for duplicate key
	if let Ok(Some(_)) = state
		.flags_repo
		.get_kill_switch_by_key(None, &payload.key)
		.await
	{
		return conflict::<FlagsErrorResponse>(
			"duplicate_key",
			t(locale, "server.api.flags.duplicate_kill_switch_key"),
		)
		.into_response();
	}

	let now = Utc::now();
	let kill_switch = KillSwitch {
		id: KillSwitchId::new(),
		org_id: None, // Platform kill switch
		key: payload.key.clone(),
		name: payload.name.clone(),
		description: payload.description.clone(),
		linked_flag_keys: payload.linked_flag_keys.clone(),
		is_active: false,
		activated_at: None,
		activated_by: None,
		activation_reason: None,
		created_at: now,
		updated_at: now,
	};

	if let Err(e) = state.flags_repo.create_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, ks_key = %kill_switch.key, "Failed to create platform kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::KillSwitchCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("platform_kill_switch", kill_switch.id.to_string())
			.details(json!({
				"key": kill_switch.key,
				"name": kill_switch.name,
				"linked_flag_keys": kill_switch.linked_flag_keys,
				"is_platform_kill_switch": true,
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		ks_id = %kill_switch.id,
		ks_key = %kill_switch.key,
		"Created platform kill switch"
	);

	(
		StatusCode::CREATED,
		Json(kill_switch_to_response(&kill_switch)),
	)
		.into_response()
}

/// Get a platform-level kill switch by key.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	get,
	path = "/api/admin/flags/kill-switches/{key}",
	params(
		("key" = String, Path, description = "Kill switch key")
	),
	responses(
		(status = 200, description = "Platform kill switch", body = KillSwitchResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id, ks_key = %key))]
pub async fn get_platform_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(key): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform kill switch access attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let kill_switch = match state.flags_repo.get_kill_switch_by_key(None, &key).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, ks_key = %key, "Failed to get platform kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	(StatusCode::OK, Json(kill_switch_to_response(&kill_switch))).into_response()
}

/// Update a platform-level kill switch.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	patch,
	path = "/api/admin/flags/kill-switches/{key}",
	params(
		("key" = String, Path, description = "Kill switch key")
	),
	request_body = UpdateKillSwitchRequest,
	responses(
		(status = 200, description = "Platform kill switch updated", body = KillSwitchResponse),
		(status = 400, description = "Invalid request", body = FlagsErrorResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state, payload), fields(actor_id = %current_user.user.id, ks_key = %key))]
pub async fn update_platform_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(key): Path<String>,
	Json(payload): Json<UpdateKillSwitchRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform kill switch update attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let mut kill_switch = match state.flags_repo.get_kill_switch_by_key(None, &key).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, ks_key = %key, "Failed to get platform kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Update fields
	if let Some(name) = payload.name {
		kill_switch.name = name;
	}
	if let Some(description) = payload.description {
		kill_switch.description = Some(description);
	}
	if let Some(linked_flag_keys) = payload.linked_flag_keys {
		kill_switch.linked_flag_keys = linked_flag_keys;
	}

	kill_switch.updated_at = Utc::now();

	if let Err(e) = state.flags_repo.update_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, ks_key = %key, "Failed to update platform kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::KillSwitchUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("platform_kill_switch", kill_switch.id.to_string())
			.details(json!({
				"key": kill_switch.key,
				"is_platform_kill_switch": true,
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		ks_id = %kill_switch.id,
		ks_key = %kill_switch.key,
		"Updated platform kill switch"
	);

	(StatusCode::OK, Json(kill_switch_to_response(&kill_switch))).into_response()
}

/// Delete a platform-level kill switch.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	delete,
	path = "/api/admin/flags/kill-switches/{key}",
	params(
		("key" = String, Path, description = "Kill switch key")
	),
	responses(
		(status = 200, description = "Platform kill switch deleted", body = FlagsSuccessResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id, ks_key = %key))]
pub async fn delete_platform_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(key): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform kill switch delete attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let kill_switch = match state.flags_repo.get_kill_switch_by_key(None, &key).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, ks_key = %key, "Failed to get platform kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	if let Err(e) = state.flags_repo.delete_kill_switch(kill_switch.id).await {
		tracing::error!(error = %e, ks_key = %key, "Failed to delete platform kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::KillSwitchDeleted)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("platform_kill_switch", kill_switch.id.to_string())
			.details(json!({
				"key": kill_switch.key,
				"is_platform_kill_switch": true,
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		ks_id = %kill_switch.id,
		ks_key = %kill_switch.key,
		"Deleted platform kill switch"
	);

	(
		StatusCode::OK,
		Json(FlagsSuccessResponse {
			message: t(locale, "server.api.flags.kill_switch_deleted").to_string(),
		}),
	)
		.into_response()
}

/// Activate a platform-level kill switch.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	post,
	path = "/api/admin/flags/kill-switches/{key}/activate",
	params(
		("key" = String, Path, description = "Kill switch key")
	),
	request_body = ActivateKillSwitchRequest,
	responses(
		(status = 200, description = "Platform kill switch activated", body = KillSwitchResponse),
		(status = 400, description = "Activation reason required", body = FlagsErrorResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state, payload), fields(actor_id = %current_user.user.id, ks_key = %key))]
pub async fn activate_platform_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(key): Path<String>,
	Json(payload): Json<ActivateKillSwitchRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform kill switch activation attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	// Validate reason is provided
	if payload.reason.trim().is_empty() {
		return bad_request::<FlagsErrorResponse>(
			"reason_required",
			t(locale, "server.api.flags.activation_reason_required"),
		)
		.into_response();
	}

	let mut kill_switch = match state.flags_repo.get_kill_switch_by_key(None, &key).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, ks_key = %key, "Failed to get platform kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let now = Utc::now();
	kill_switch.is_active = true;
	kill_switch.activated_at = Some(now);
	kill_switch.activated_by = Some(UserId(current_user.user.id.into_inner()));
	kill_switch.activation_reason = Some(payload.reason.clone());
	kill_switch.updated_at = now;

	if let Err(e) = state.flags_repo.update_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, ks_key = %key, "Failed to activate platform kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	// Broadcast SSE event for platform kill switch activation to all connected clients
	let event = loom_flags_core::FlagStreamEvent::kill_switch_activated(
		kill_switch.key.clone(),
		kill_switch.linked_flag_keys.clone(),
		payload.reason.clone(),
	);
	state.flags_broadcaster.broadcast_to_all(event).await;

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::KillSwitchActivated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("platform_kill_switch", kill_switch.id.to_string())
			.details(json!({
				"key": kill_switch.key,
				"reason": payload.reason,
				"linked_flag_keys": kill_switch.linked_flag_keys,
				"is_platform_kill_switch": true,
			}))
			.build(),
	);

	tracing::warn!(
		actor_id = %current_user.user.id,
		ks_id = %kill_switch.id,
		ks_key = %kill_switch.key,
		reason = %payload.reason,
		linked_flags = ?kill_switch.linked_flag_keys,
		"ACTIVATED platform kill switch"
	);

	(StatusCode::OK, Json(kill_switch_to_response(&kill_switch))).into_response()
}

/// Deactivate a platform-level kill switch.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	post,
	path = "/api/admin/flags/kill-switches/{key}/deactivate",
	params(
		("key" = String, Path, description = "Kill switch key")
	),
	responses(
		(status = 200, description = "Platform kill switch deactivated", body = KillSwitchResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id, ks_key = %key))]
pub async fn deactivate_platform_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(key): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform kill switch deactivation attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let mut kill_switch = match state.flags_repo.get_kill_switch_by_key(None, &key).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, ks_key = %key, "Failed to get platform kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	kill_switch.is_active = false;
	kill_switch.activated_at = None;
	kill_switch.activated_by = None;
	kill_switch.activation_reason = None;
	kill_switch.updated_at = Utc::now();

	if let Err(e) = state.flags_repo.update_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, ks_key = %key, "Failed to deactivate platform kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	// Broadcast SSE event for platform kill switch deactivation to all connected clients
	let event = loom_flags_core::FlagStreamEvent::kill_switch_deactivated(
		kill_switch.key.clone(),
		kill_switch.linked_flag_keys.clone(),
	);
	state.flags_broadcaster.broadcast_to_all(event).await;

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::KillSwitchDeactivated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("platform_kill_switch", kill_switch.id.to_string())
			.details(json!({
				"key": kill_switch.key,
				"is_platform_kill_switch": true,
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		ks_id = %kill_switch.id,
		ks_key = %kill_switch.key,
		"Deactivated platform kill switch"
	);

	(StatusCode::OK, Json(kill_switch_to_response(&kill_switch))).into_response()
}
