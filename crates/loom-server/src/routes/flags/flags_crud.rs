// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Flag CRUD HTTP handlers.
//!
//! Implements endpoints for flag create, read, update, archive, and restore.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_flags_core::{Flag, FlagConfig, FlagConfigId, FlagId, FlagStreamEvent};
use loom_server_api::flags::{
	CreateFlagRequest, FlagResponse, FlagsErrorResponse, FlagsSuccessResponse, ListFlagsQuery,
	ListFlagsResponse, UpdateFlagRequest,
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

use super::common::{prerequisite_from_api, prerequisite_to_api, variant_from_api, variant_to_api};

// ============================================================================
// Helper Functions
// ============================================================================

pub(crate) fn flag_to_response(flag: &Flag) -> FlagResponse {
	FlagResponse {
		id: flag.id.to_string(),
		org_id: flag.org_id.map(|id| id.to_string()),
		key: flag.key.clone(),
		name: flag.name.clone(),
		description: flag.description.clone(),
		tags: flag.tags.clone(),
		maintainer_user_id: flag.maintainer_user_id.map(|id| id.to_string()),
		variants: flag.variants.iter().map(variant_to_api).collect(),
		default_variant: flag.default_variant.clone(),
		prerequisites: flag.prerequisites.iter().map(prerequisite_to_api).collect(),
		is_archived: flag.is_archived(),
		created_at: flag.created_at,
		updated_at: flag.updated_at,
		archived_at: flag.archived_at,
	}
}

// ============================================================================
// Flag Routes
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("include_archived" = Option<bool>, Query, description = "Include archived flags")
    ),
    responses(
        (status = 200, description = "List of flags", body = ListFlagsResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// List flags for an organization.
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_flags(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Query(query): Query<ListFlagsQuery>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	// Check org membership
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
	let flags = match state
		.flags_repo
		.list_flags(Some(flags_org_id), query.include_archived)
		.await
	{
		Ok(flags) => flags,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list flags");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let flag_responses: Vec<FlagResponse> = flags.iter().map(flag_to_response).collect();

	(
		StatusCode::OK,
		Json(ListFlagsResponse {
			flags: flag_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateFlagRequest,
    responses(
        (status = 201, description = "Flag created", body = FlagResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 409, description = "Flag key already exists", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Create a new flag.
#[tracing::instrument(skip(state, payload), fields(%org_id, key = %payload.key))]
pub async fn create_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateFlagRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	// Check org membership
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

	// Validate flag key
	if !Flag::validate_key(&payload.key) {
		return bad_request::<FlagsErrorResponse>(
			"invalid_key",
			t(locale, "server.api.flags.invalid_flag_key"),
		)
		.into_response();
	}

	// Validate variants
	if payload.variants.is_empty() {
		return bad_request::<FlagsErrorResponse>("no_variants", "At least one variant is required")
			.into_response();
	}

	// Check for duplicate variant names
	let mut variant_names: Vec<&str> = payload.variants.iter().map(|v| v.name.as_str()).collect();
	variant_names.sort();
	for i in 1..variant_names.len() {
		if variant_names[i] == variant_names[i - 1] {
			return bad_request::<FlagsErrorResponse>(
				"duplicate_variant",
				format!("Duplicate variant name: {}", variant_names[i]),
			)
			.into_response();
		}
	}

	// Validate default variant exists
	if !payload
		.variants
		.iter()
		.any(|v| v.name == payload.default_variant)
	{
		return bad_request::<FlagsErrorResponse>(
			"default_variant_missing",
			t(locale, "server.api.flags.default_variant_missing"),
		)
		.into_response();
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	// Check for duplicate key
	if let Ok(Some(_)) = state
		.flags_repo
		.get_flag_by_key(Some(flags_org_id), &payload.key)
		.await
	{
		return conflict::<FlagsErrorResponse>(
			"duplicate_key",
			t(locale, "server.api.flags.duplicate_flag_key"),
		)
		.into_response();
	}

	// Parse maintainer user ID if provided
	let maintainer_user_id = match &payload.maintainer_user_id {
		Some(id) => match id.parse::<uuid::Uuid>() {
			Ok(uuid) => Some(loom_flags_core::UserId(uuid)),
			Err(_) => {
				return bad_request::<FlagsErrorResponse>(
					"invalid_maintainer_id",
					"Invalid maintainer user ID format",
				)
				.into_response();
			}
		},
		None => None,
	};

	let now = Utc::now();
	let flag = Flag {
		id: FlagId::new(),
		org_id: Some(flags_org_id),
		key: payload.key,
		name: payload.name,
		description: payload.description,
		tags: payload.tags,
		maintainer_user_id,
		variants: payload.variants.iter().map(variant_from_api).collect(),
		default_variant: payload.default_variant,
		prerequisites: payload
			.prerequisites
			.iter()
			.map(prerequisite_from_api)
			.collect(),
		exposure_tracking_enabled: false,
		created_at: now,
		updated_at: now,
		archived_at: None,
	};

	if let Err(e) = state.flags_repo.create_flag(&flag).await {
		tracing::error!(error = %e, flag_key = %flag.key, "Failed to create flag");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	// Auto-create FlagConfig for each environment
	let environments = match state.flags_repo.list_environments(flags_org_id).await {
		Ok(envs) => envs,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list environments for flag config creation");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	for env in environments {
		let config = FlagConfig {
			id: FlagConfigId::new(),
			flag_id: flag.id,
			environment_id: env.id,
			enabled: false,
			strategy_id: None,
			created_at: now,
			updated_at: now,
		};

		if let Err(e) = state.flags_repo.create_flag_config(&config).await {
			tracing::error!(error = %e, flag_id = %flag.id, env_id = %env.id, "Failed to create flag config");
		}
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::FlagCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("flag", flag.id.to_string())
			.details(serde_json::json!({
				"org_id": flags_org_id.to_string(),
				"key": flag.key,
				"name": flag.name,
				"description": flag.description,
				"tags": flag.tags,
			}))
			.build(),
	);

	tracing::info!(flag_id = %flag.id, flag_key = %flag.key, "Flag created");

	(StatusCode::CREATED, Json(flag_to_response(&flag))).into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/{flag_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID")
    ),
    responses(
        (status = 200, description = "Flag details", body = FlagResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Get flag details.
#[tracing::instrument(skip(state), fields(%org_id, %flag_id))]
pub async fn get_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
			)
			.into_response();
		}
	};

	// Check org membership
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

	let flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify flag belongs to the org
	match flag.org_id {
		Some(flag_org_id) if flag_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
	}

	(StatusCode::OK, Json(flag_to_response(&flag))).into_response()
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/flags/{flag_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID")
    ),
    request_body = UpdateFlagRequest,
    responses(
        (status = 200, description = "Flag updated", body = FlagResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Update a flag.
#[tracing::instrument(skip(state, payload), fields(%org_id, %flag_id))]
pub async fn update_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id)): Path<(String, String)>,
	Json(payload): Json<UpdateFlagRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
			)
			.into_response();
		}
	};

	// Check org membership
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

	let mut flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Verify flag belongs to the org
	match flag.org_id {
		Some(flag_org_id) if flag_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
	}

	// Check if flag is archived
	if flag.is_archived() {
		return bad_request::<FlagsErrorResponse>(
			"flag_archived",
			"Cannot update an archived flag. Restore it first.",
		)
		.into_response();
	}

	// Update name if provided
	if let Some(ref name) = payload.name {
		flag.name = name.clone();
	}

	// Update description if provided
	if let Some(ref description) = payload.description {
		flag.description = Some(description.clone());
	}

	// Update tags if provided
	if let Some(ref tags) = payload.tags {
		flag.tags = tags.clone();
	}

	// Update maintainer if provided
	if let Some(ref maintainer_id) = payload.maintainer_user_id {
		match maintainer_id.parse::<uuid::Uuid>() {
			Ok(uuid) => flag.maintainer_user_id = Some(loom_flags_core::UserId(uuid)),
			Err(_) => {
				return bad_request::<FlagsErrorResponse>(
					"invalid_maintainer_id",
					"Invalid maintainer user ID format",
				)
				.into_response();
			}
		}
	}

	// Update variants if provided
	if let Some(ref variants) = payload.variants {
		if variants.is_empty() {
			return bad_request::<FlagsErrorResponse>("no_variants", "At least one variant is required")
				.into_response();
		}

		// Check for duplicate variant names
		let mut variant_names: Vec<&str> = variants.iter().map(|v| v.name.as_str()).collect();
		variant_names.sort();
		for i in 1..variant_names.len() {
			if variant_names[i] == variant_names[i - 1] {
				return bad_request::<FlagsErrorResponse>(
					"duplicate_variant",
					format!("Duplicate variant name: {}", variant_names[i]),
				)
				.into_response();
			}
		}

		flag.variants = variants.iter().map(variant_from_api).collect();
	}

	// Update default_variant if provided
	if let Some(ref default_variant) = payload.default_variant {
		flag.default_variant = default_variant.clone();
	}

	// Validate default variant exists in variants
	if !flag.variants.iter().any(|v| v.name == flag.default_variant) {
		return bad_request::<FlagsErrorResponse>(
			"default_variant_missing",
			t(locale, "server.api.flags.default_variant_missing"),
		)
		.into_response();
	}

	// Update prerequisites if provided
	if let Some(ref prerequisites) = payload.prerequisites {
		flag.prerequisites = prerequisites.iter().map(prerequisite_from_api).collect();
	}

	flag.updated_at = Utc::now();

	if let Err(e) = state.flags_repo.update_flag(&flag).await {
		tracing::error!(error = %e, %flag_id, "Failed to update flag");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::FlagUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("flag", flag.id.to_string())
			.details(serde_json::json!({
				"org_id": flag.org_id.map(|o| o.to_string()),
				"key": flag.key,
				"name": flag.name,
				"description": flag.description,
				"tags": flag.tags,
			}))
			.build(),
	);

	tracing::info!(%flag_id, "Flag updated");

	(StatusCode::OK, Json(flag_to_response(&flag))).into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/{flag_id}/archive",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID")
    ),
    responses(
        (status = 200, description = "Flag archived", body = FlagsSuccessResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Archive a flag.
#[tracing::instrument(skip(state), fields(%org_id, %flag_id))]
pub async fn archive_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
			)
			.into_response();
		}
	};

	// Check org membership
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

	// Verify flag exists and belongs to org
	let flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	match flag.org_id {
		Some(flag_org_id) if flag_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
	}

	if flag.is_archived() {
		return bad_request::<FlagsErrorResponse>("already_archived", "Flag is already archived")
			.into_response();
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	match state.flags_repo.archive_flag(flag_id).await {
		Ok(true) => {
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::FlagArchived)
					.actor(AuditUserId::new(current_user.user.id.into_inner()))
					.resource("flag", flag_id.to_string())
					.details(serde_json::json!({
						"org_id": flags_org_id.to_string(),
						"key": flag.key,
						"name": flag.name,
					}))
					.build(),
			);

			tracing::info!(%flag_id, "Flag archived");

			// Broadcast flag archived event to all environments
			let event = FlagStreamEvent::flag_archived(flag.key.clone());
			state
				.flags_broadcaster
				.broadcast_to_org(flags_org_id, event)
				.await;

			(
				StatusCode::OK,
				Json(FlagsSuccessResponse {
					message: t(locale, "server.api.flags.flag_archived").to_string(),
				}),
			)
				.into_response()
		}
		Ok(false) => {
			not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found")).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to archive flag");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/{flag_id}/restore",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_id" = String, Path, description = "Flag ID")
    ),
    responses(
        (status = 200, description = "Flag restored", body = FlagsSuccessResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Restore an archived flag.
#[tracing::instrument(skip(state), fields(%org_id, %flag_id))]
pub async fn restore_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let flag_id: FlagId = match flag_id.parse() {
		Ok(id) => id,
		Err(_) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_id",
				t(locale, "server.api.flags.flag_not_found"),
			)
			.into_response();
		}
	};

	// Check org membership
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

	// Verify flag exists and belongs to org
	let flag = match state.flags_repo.get_flag_by_id(flag_id).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	match flag.org_id {
		Some(flag_org_id) if flag_org_id.0 == org_id.into_inner() => {}
		_ => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
	}

	if !flag.is_archived() {
		return bad_request::<FlagsErrorResponse>("not_archived", "Flag is not archived")
			.into_response();
	}

	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());

	match state.flags_repo.restore_flag(flag_id).await {
		Ok(true) => {
			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::FlagRestored)
					.actor(AuditUserId::new(current_user.user.id.into_inner()))
					.resource("flag", flag_id.to_string())
					.details(serde_json::json!({
						"org_id": flags_org_id.to_string(),
						"key": flag.key,
						"name": flag.name,
					}))
					.build(),
			);

			tracing::info!(%flag_id, "Flag restored");

			// Broadcast flag restored event to all environments
			// We broadcast to all environments since the flag is now available again
			let environments = state
				.flags_repo
				.list_environments(flags_org_id)
				.await
				.unwrap_or_default();
			for env in environments {
				let config = state
					.flags_repo
					.get_flag_config(flag.id, env.id)
					.await
					.ok()
					.flatten();
				let enabled = config.map(|c| c.enabled).unwrap_or(false);
				let event = FlagStreamEvent::flag_restored(flag.key.clone(), env.name, enabled);
				state
					.flags_broadcaster
					.broadcast(flags_org_id, env.id, event)
					.await;
			}

			(
				StatusCode::OK,
				Json(FlagsSuccessResponse {
					message: t(locale, "server.api.flags.flag_restored").to_string(),
				}),
			)
				.into_response()
		}
		Ok(false) => {
			not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found")).into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_id, "Failed to restore flag");
			internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal")).into_response()
		}
	}
}
