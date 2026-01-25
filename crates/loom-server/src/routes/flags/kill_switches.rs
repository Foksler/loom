// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Kill switch HTTP handlers.
//!
//! Implements endpoints for kill switch management and activation.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_flags_core::{FlagStreamEvent, KillSwitch, KillSwitchId};
use loom_server_api::flags::{
	ActivateKillSwitchRequest, CreateKillSwitchRequest, FlagsErrorResponse, FlagsSuccessResponse,
	KillSwitchResponse, ListKillSwitchesResponse, UpdateKillSwitchRequest,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_flags::FlagsRepository;

use crate::{
	api::AppState,
	api_response::{bad_request, conflict, internal_error, not_found},
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	parse_id,
	validation::parse_org_id as shared_parse_org_id,
};

// ============================================================================
// Helper Functions
// ============================================================================

fn kill_switch_to_response(kill_switch: KillSwitch) -> KillSwitchResponse {
	KillSwitchResponse {
		id: kill_switch.id.to_string(),
		org_id: kill_switch.org_id.map(|id| id.0.to_string()),
		key: kill_switch.key,
		name: kill_switch.name,
		description: kill_switch.description,
		linked_flag_keys: kill_switch.linked_flag_keys,
		is_active: kill_switch.is_active,
		activated_at: kill_switch.activated_at,
		activated_by: kill_switch.activated_by.map(|id| id.0.to_string()),
		activation_reason: kill_switch.activation_reason,
		created_at: kill_switch.created_at,
		updated_at: kill_switch.updated_at,
	}
}

// ============================================================================
// Kill Switch Routes
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/kill-switches",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of kill switches", body = ListKillSwitchesResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// List kill switches for an organization.
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_kill_switches(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	match state
		.flags_repo
		.list_kill_switches(Some(flags_org_id))
		.await
	{
		Ok(kill_switches) => {
			let response = ListKillSwitchesResponse {
				kill_switches: kill_switches
					.into_iter()
					.map(kill_switch_to_response)
					.collect(),
			};
			(StatusCode::OK, Json(response)).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to list kill switches");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/kill-switches",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateKillSwitchRequest,
    responses(
        (status = 201, description = "Kill switch created", body = KillSwitchResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse),
        (status = 409, description = "Kill switch key already exists", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Create a new kill switch.
#[tracing::instrument(skip(state, payload), fields(%org_id, key = %payload.key))]
pub async fn create_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateKillSwitchRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	if !KillSwitch::validate_key(&payload.key) {
		return bad_request::<FlagsErrorResponse>(
			"invalid_key",
			t(locale, "server.api.flags.invalid_kill_switch_key"),
		)
		.into_response();
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	if let Ok(Some(_)) = state
		.flags_repo
		.get_kill_switch_by_key(Some(flags_org_id), &payload.key)
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
		org_id: Some(flags_org_id),
		key: payload.key,
		name: payload.name,
		description: payload.description,
		linked_flag_keys: payload.linked_flag_keys,
		is_active: false,
		activated_at: None,
		activated_by: None,
		activation_reason: None,
		created_at: now,
		updated_at: now,
	};

	if let Err(e) = state.flags_repo.create_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, kill_switch_id = %kill_switch.id, "Failed to create kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::KillSwitchCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("kill_switch", kill_switch.id.to_string())
			.details(serde_json::json!({
				"org_id": flags_org_id.to_string(),
				"key": kill_switch.key,
				"name": kill_switch.name,
				"linked_flag_keys": kill_switch.linked_flag_keys,
			}))
			.build(),
	);

	tracing::info!(kill_switch_id = %kill_switch.id, key = %kill_switch.key, "Kill switch created");

	(
		StatusCode::CREATED,
		Json(kill_switch_to_response(kill_switch)),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    responses(
        (status = 200, description = "Kill switch details", body = KillSwitchResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Get a kill switch by ID.
#[tracing::instrument(skip(state), fields(%org_id, %kill_switch_id))]
pub async fn get_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(kill_switch)) => {
			// Verify kill switch belongs to the org
			match kill_switch.org_id {
				Some(ks_org_id) if ks_org_id == flags_org_id => {}
				_ => {
					return not_found::<FlagsErrorResponse>(t(
						locale,
						"server.api.flags.kill_switch_not_found",
					))
					.into_response();
				}
			}

			(StatusCode::OK, Json(kill_switch_to_response(kill_switch))).into_response()
		}
		Ok(None) => {
			not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    request_body = UpdateKillSwitchRequest,
    responses(
        (status = 200, description = "Kill switch updated", body = KillSwitchResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Update a kill switch.
#[tracing::instrument(skip(state, payload), fields(%org_id, %kill_switch_id))]
pub async fn update_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
	Json(payload): Json<UpdateKillSwitchRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	let mut kill_switch = match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify kill switch belongs to the org
	match kill_switch.org_id {
		Some(ks_org_id) if ks_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
	}

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
		tracing::error!(error = %e, %kill_switch_id, "Failed to update kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::KillSwitchUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("kill_switch", kill_switch.id.to_string())
			.details(serde_json::json!({
				"org_id": kill_switch.org_id.map(|o| o.to_string()),
				"key": kill_switch.key,
				"name": kill_switch.name,
				"linked_flag_keys": kill_switch.linked_flag_keys,
			}))
			.build(),
	);

	tracing::info!(%kill_switch_id, "Kill switch updated");

	(StatusCode::OK, Json(kill_switch_to_response(kill_switch))).into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}/activate",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    request_body = ActivateKillSwitchRequest,
    responses(
        (status = 200, description = "Kill switch activated", body = KillSwitchResponse),
        (status = 400, description = "Invalid request or missing reason", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Activate a kill switch (emergency shutoff).
#[tracing::instrument(skip(state, payload), fields(%org_id, %kill_switch_id))]
pub async fn activate_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
	Json(payload): Json<ActivateKillSwitchRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

	// Reason is mandatory for activation (audit trail)
	if payload.reason.trim().is_empty() {
		return bad_request::<FlagsErrorResponse>(
			"reason_required",
			t(locale, "server.api.flags.activation_reason_required"),
		)
		.into_response();
	}

	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	let mut kill_switch = match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify kill switch belongs to the org
	match kill_switch.org_id {
		Some(ks_org_id) if ks_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
	}

	let user_id = loom_flags_core::UserId(current_user.user.id.into_inner());
	kill_switch.activate(user_id, payload.reason);

	if let Err(e) = state.flags_repo.update_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, %kill_switch_id, "Failed to activate kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::KillSwitchActivated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("kill_switch", kill_switch.id.to_string())
			.details(serde_json::json!({
				"org_id": flags_org_id.to_string(),
				"key": kill_switch.key,
				"name": kill_switch.name,
				"linked_flag_keys": kill_switch.linked_flag_keys,
				"reason": kill_switch.activation_reason,
			}))
			.build(),
	);

	tracing::warn!(
		%kill_switch_id,
		key = %kill_switch.key,
		user_id = %current_user.user.id,
		reason = ?kill_switch.activation_reason,
		linked_flags = ?kill_switch.linked_flag_keys,
		"Kill switch activated"
	);

	// Broadcast kill switch activation to all environments in the org
	let event = FlagStreamEvent::kill_switch_activated(
		kill_switch.key.clone(),
		kill_switch.linked_flag_keys.clone(),
		kill_switch.activation_reason.clone().unwrap_or_default(),
	);
	state
		.flags_broadcaster
		.broadcast_to_org(flags_org_id, event)
		.await;

	(
		StatusCode::OK,
		Json(FlagsSuccessResponse {
			message: t(locale, "server.api.flags.kill_switch_activated").to_string(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}/deactivate",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    responses(
        (status = 200, description = "Kill switch deactivated", body = KillSwitchResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Deactivate a kill switch.
#[tracing::instrument(skip(state), fields(%org_id, %kill_switch_id))]
pub async fn deactivate_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	let mut kill_switch = match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify kill switch belongs to the org
	match kill_switch.org_id {
		Some(ks_org_id) if ks_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
	}

	kill_switch.deactivate();

	if let Err(e) = state.flags_repo.update_kill_switch(&kill_switch).await {
		tracing::error!(error = %e, %kill_switch_id, "Failed to deactivate kill switch");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::KillSwitchDeactivated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("kill_switch", kill_switch.id.to_string())
			.details(serde_json::json!({
				"org_id": flags_org_id.to_string(),
				"key": kill_switch.key,
				"name": kill_switch.name,
				"linked_flag_keys": kill_switch.linked_flag_keys,
			}))
			.build(),
	);

	tracing::info!(
		%kill_switch_id,
		key = %kill_switch.key,
		user_id = %current_user.user.id,
		"Kill switch deactivated"
	);

	// Broadcast kill switch deactivation to all environments in the org
	let event = FlagStreamEvent::kill_switch_deactivated(
		kill_switch.key.clone(),
		kill_switch.linked_flag_keys.clone(),
	);
	state
		.flags_broadcaster
		.broadcast_to_org(flags_org_id, event)
		.await;

	(
		StatusCode::OK,
		Json(FlagsSuccessResponse {
			message: t(locale, "server.api.flags.kill_switch_deactivated").to_string(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("kill_switch_id" = String, Path, description = "Kill switch ID")
    ),
    responses(
        (status = 200, description = "Kill switch deleted", body = FlagsSuccessResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Kill switch not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Delete a kill switch.
#[tracing::instrument(skip(state), fields(%org_id, %kill_switch_id))]
pub async fn delete_kill_switch(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, kill_switch_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let kill_switch_id: KillSwitchId = match kill_switch_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.kill_switch_not_found"),
			)
			.into_response();
		}
	};

	match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	// Verify kill switch exists and belongs to the org
	let kill_switch = match state.flags_repo.get_kill_switch_by_id(kill_switch_id).await {
		Ok(Some(ks)) => ks,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to get kill switch");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	match kill_switch.org_id {
		Some(ks_org_id) if ks_org_id == flags_org_id => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response();
		}
	}

	match state.flags_repo.delete_kill_switch(kill_switch_id).await {
		Ok(true) => {
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::KillSwitchDeleted)
					.actor(AuditUserId::new(current_user.user.id.into_inner()))
					.resource("kill_switch", kill_switch_id.to_string())
					.details(serde_json::json!({
						"org_id": flags_org_id.to_string(),
						"key": kill_switch.key,
						"name": kill_switch.name,
					}))
					.build(),
			);

			tracing::info!(%kill_switch_id, "Kill switch deleted");
			(
				StatusCode::OK,
				Json(FlagsSuccessResponse {
					message: t(locale, "server.api.flags.kill_switch_deleted").to_string(),
				}),
			)
				.into_response()
		}
		Ok(false) => {
			not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.kill_switch_not_found"))
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %kill_switch_id, "Failed to delete kill switch");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}
