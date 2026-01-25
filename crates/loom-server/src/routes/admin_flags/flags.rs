// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Platform flag handlers.

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_flags_core::{Flag, FlagId, UserId};
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
	flag_to_response, prerequisite_from_api, variant_from_api, CreateFlagRequest, FlagResponse,
	FlagsErrorResponse, FlagsSuccessResponse, ListFlagsResponse, PlatformFlagsQuery,
	UpdateFlagRequest,
};

/// List all platform-level flags.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	get,
	path = "/api/admin/flags",
	params(
		("include_archived" = bool, Query, description = "Include archived flags")
	),
	responses(
		(status = 200, description = "List of platform flags", body = ListFlagsResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id))]
pub async fn list_platform_flags(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Query(query): Query<PlatformFlagsQuery>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform flags list attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let flags = match state
		.flags_repo
		.list_flags(None, query.include_archived)
		.await
	{
		Ok(flags) => flags,
		Err(e) => {
			tracing::error!(error = %e, error_debug = ?e, "Failed to list platform flags");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	tracing::info!(
		actor_id = %current_user.user.id,
		flag_count = flags.len(),
		"Listed platform flags"
	);

	let flags_response: Vec<FlagResponse> = flags.iter().map(flag_to_response).collect();

	(
		StatusCode::OK,
		Json(ListFlagsResponse {
			flags: flags_response,
		}),
	)
		.into_response()
}

/// Create a new platform-level flag.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	post,
	path = "/api/admin/flags",
	request_body = CreateFlagRequest,
	responses(
		(status = 201, description = "Platform flag created", body = FlagResponse),
		(status = 400, description = "Invalid request", body = FlagsErrorResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 409, description = "Flag key already exists", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state, payload), fields(actor_id = %current_user.user.id, flag_key = %payload.key))]
pub async fn create_platform_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Json(payload): Json<CreateFlagRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform flag creation attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	// Validate flag key
	if !Flag::validate_key(&payload.key) {
		return bad_request::<FlagsErrorResponse>(
			"invalid_key",
			t(locale, "server.api.flags.invalid_flag_key"),
		)
		.into_response();
	}

	// Check for duplicate key
	if let Ok(Some(_)) = state.flags_repo.get_flag_by_key(None, &payload.key).await {
		return conflict::<FlagsErrorResponse>(
			"duplicate_key",
			t(locale, "server.api.flags.duplicate_flag_key"),
		)
		.into_response();
	}

	// Validate variants
	if payload.variants.is_empty() {
		return bad_request::<FlagsErrorResponse>(
			"no_variants",
			t(locale, "server.api.flags.variant_not_found"),
		)
		.into_response();
	}

	// Validate default variant exists
	if !payload
		.variants
		.iter()
		.any(|v| v.name == payload.default_variant)
	{
		return bad_request::<FlagsErrorResponse>(
			"invalid_default",
			t(locale, "server.api.flags.default_variant_missing"),
		)
		.into_response();
	}

	let now = Utc::now();
	let flag = Flag {
		id: FlagId::new(),
		org_id: None, // Platform flag
		key: payload.key.clone(),
		name: payload.name.clone(),
		description: payload.description.clone(),
		tags: payload.tags.clone(),
		maintainer_user_id: payload
			.maintainer_user_id
			.as_ref()
			.and_then(|id| id.parse().ok().map(UserId)),
		variants: payload.variants.iter().map(variant_from_api).collect(),
		default_variant: payload.default_variant.clone(),
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
		tracing::error!(error = %e, flag_key = %flag.key, "Failed to create platform flag");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::FlagCreated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("platform_flag", flag.id.to_string())
			.details(json!({
				"key": flag.key,
				"name": flag.name,
				"is_platform_flag": true,
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		flag_id = %flag.id,
		flag_key = %flag.key,
		"Created platform flag"
	);

	(StatusCode::CREATED, Json(flag_to_response(&flag))).into_response()
}

/// Get a platform-level flag by key.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	get,
	path = "/api/admin/flags/{key}",
	params(
		("key" = String, Path, description = "Flag key")
	),
	responses(
		(status = 200, description = "Platform flag", body = FlagResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Flag not found", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id, flag_key = %key))]
pub async fn get_platform_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(key): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform flag access attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let flag = match state.flags_repo.get_flag_by_key(None, &key).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, flag_key = %key, "Failed to get platform flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	(StatusCode::OK, Json(flag_to_response(&flag))).into_response()
}

/// Update a platform-level flag.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	patch,
	path = "/api/admin/flags/{key}",
	params(
		("key" = String, Path, description = "Flag key")
	),
	request_body = UpdateFlagRequest,
	responses(
		(status = 200, description = "Platform flag updated", body = FlagResponse),
		(status = 400, description = "Invalid request", body = FlagsErrorResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Flag not found", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state, payload), fields(actor_id = %current_user.user.id, flag_key = %key))]
pub async fn update_platform_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(key): Path<String>,
	Json(payload): Json<UpdateFlagRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform flag update attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let mut flag = match state.flags_repo.get_flag_by_key(None, &key).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, flag_key = %key, "Failed to get platform flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Update fields
	if let Some(name) = payload.name {
		flag.name = name;
	}
	if let Some(description) = payload.description {
		flag.description = Some(description);
	}
	if let Some(tags) = payload.tags {
		flag.tags = tags;
	}
	if let Some(maintainer_user_id) = payload.maintainer_user_id {
		flag.maintainer_user_id = maintainer_user_id.parse().ok().map(UserId);
	}
	if let Some(variants) = payload.variants {
		flag.variants = variants.iter().map(variant_from_api).collect();
	}
	if let Some(default_variant) = payload.default_variant {
		// Validate default variant exists
		if !flag.variants.iter().any(|v| v.name == default_variant) {
			return bad_request::<FlagsErrorResponse>(
				"invalid_default",
				t(locale, "server.api.flags.default_variant_missing"),
			)
			.into_response();
		}
		flag.default_variant = default_variant;
	}
	if let Some(prerequisites) = payload.prerequisites {
		flag.prerequisites = prerequisites.iter().map(prerequisite_from_api).collect();
	}

	flag.updated_at = Utc::now();

	if let Err(e) = state.flags_repo.update_flag(&flag).await {
		tracing::error!(error = %e, flag_key = %key, "Failed to update platform flag");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::FlagUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("platform_flag", flag.id.to_string())
			.details(json!({
				"key": flag.key,
				"is_platform_flag": true,
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		flag_id = %flag.id,
		flag_key = %flag.key,
		"Updated platform flag"
	);

	(StatusCode::OK, Json(flag_to_response(&flag))).into_response()
}

/// Archive a platform-level flag.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	delete,
	path = "/api/admin/flags/{key}",
	params(
		("key" = String, Path, description = "Flag key")
	),
	responses(
		(status = 200, description = "Platform flag archived", body = FlagsSuccessResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Flag not found", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id, flag_key = %key))]
pub async fn archive_platform_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(key): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform flag archive attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let flag = match state.flags_repo.get_flag_by_key(None, &key).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, flag_key = %key, "Failed to get platform flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	if let Err(e) = state.flags_repo.archive_flag(flag.id).await {
		tracing::error!(error = %e, flag_key = %key, "Failed to archive platform flag");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::FlagArchived)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("platform_flag", flag.id.to_string())
			.details(json!({
				"key": flag.key,
				"is_platform_flag": true,
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		flag_id = %flag.id,
		flag_key = %flag.key,
		"Archived platform flag"
	);

	(
		StatusCode::OK,
		Json(FlagsSuccessResponse {
			message: t(locale, "server.api.flags.flag_archived").to_string(),
		}),
	)
		.into_response()
}

/// Restore an archived platform-level flag.
///
/// # Authorization
///
/// Requires `system_admin` role.
#[utoipa::path(
	post,
	path = "/api/admin/flags/{key}/restore",
	params(
		("key" = String, Path, description = "Flag key")
	),
	responses(
		(status = 200, description = "Platform flag restored", body = FlagsSuccessResponse),
		(status = 401, description = "Not authenticated", body = AdminErrorResponse),
		(status = 403, description = "Not authorized", body = AdminErrorResponse),
		(status = 404, description = "Flag not found", body = FlagsErrorResponse)
	),
	tag = "admin-flags"
)]
#[tracing::instrument(skip(state), fields(actor_id = %current_user.user.id, flag_key = %key))]
pub async fn restore_platform_flag(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(key): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if !current_user.user.is_system_admin {
		tracing::warn!(actor_id = %current_user.user.id, "Unauthorized platform flag restore attempt");
		return (
			StatusCode::FORBIDDEN,
			Json(AdminErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.admin.system_admin_required").to_string(),
			}),
		)
			.into_response();
	}

	let flag = match state.flags_repo.get_flag_by_key(None, &key).await {
		Ok(Some(flag)) => flag,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, flag_key = %key, "Failed to get platform flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	if let Err(e) = state.flags_repo.restore_flag(flag.id).await {
		tracing::error!(error = %e, flag_key = %key, "Failed to restore platform flag");
		return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
			.into_response();
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::FlagRestored)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("platform_flag", flag.id.to_string())
			.details(json!({
				"key": flag.key,
				"is_platform_flag": true,
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		flag_id = %flag.id,
		flag_key = %flag.key,
		"Restored platform flag"
	);

	(
		StatusCode::OK,
		Json(FlagsSuccessResponse {
			message: t(locale, "server.api.flags.flag_restored").to_string(),
		}),
	)
		.into_response()
}
