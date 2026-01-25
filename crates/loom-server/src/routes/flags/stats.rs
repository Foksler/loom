// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Flag statistics HTTP handlers.
//!
//! Implements endpoints for flag usage statistics.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_server_api::flags::{
	FlagStatsResponse, FlagsErrorResponse, ListStaleFlagsResponse, StaleFlagResponse,
};
use loom_server_flags::FlagsRepository;

use crate::{
	api::AppState,
	api_response::{internal_error, not_found},
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	parse_id,
	validation::parse_org_id as shared_parse_org_id,
};

// ============================================================================
// Constants
// ============================================================================

/// Default stale threshold in days if not configured.
const DEFAULT_STALE_THRESHOLD_DAYS: u32 = 30;

// ============================================================================
// Flag Stats Routes
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/stale",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of stale flags", body = ListStaleFlagsResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// List flags that haven't been evaluated recently.
///
/// A flag is considered stale if it hasn't been evaluated within the configured
/// stale threshold (default: 30 days).
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_stale_flags(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
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

	// Get stale threshold from config or use default
	let stale_threshold_days = std::env::var("LOOM_FLAGS_STALE_THRESHOLD_DAYS")
		.ok()
		.and_then(|s| s.parse().ok())
		.unwrap_or(DEFAULT_STALE_THRESHOLD_DAYS);

	let stale_flags = match state
		.flags_repo
		.list_stale_flags(Some(flags_org_id), stale_threshold_days)
		.await
	{
		Ok(flags) => flags,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list stale flags");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let now = Utc::now();
	let stale_flag_responses: Vec<StaleFlagResponse> = stale_flags
		.into_iter()
		.map(|(flag, last_evaluated_at)| {
			let days_since = last_evaluated_at.map(|dt| (now - dt).num_days());
			StaleFlagResponse {
				flag_id: flag.id.to_string(),
				flag_key: flag.key,
				name: flag.name,
				last_evaluated_at,
				days_since_evaluated: days_since,
			}
		})
		.collect();

	(
		StatusCode::OK,
		Json(ListStaleFlagsResponse {
			stale_flags: stale_flag_responses,
			stale_threshold_days,
		}),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/flags/{flag_key}/stats",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_key" = String, Path, description = "Flag key")
    ),
    responses(
        (status = 200, description = "Flag statistics", body = FlagStatsResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Get statistics for a specific flag.
///
/// Returns evaluation counts and last evaluation timestamp.
#[tracing::instrument(skip(state), fields(%org_id, %flag_key))]
pub async fn get_flag_stats(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, flag_key)): Path<(String, String)>,
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

	// Get the flag by key
	let flag = match state
		.flags_repo
		.get_flag_by_key(Some(flags_org_id), &flag_key)
		.await
	{
		Ok(Some(f)) => f,
		Ok(None) => {
			return not_found::<FlagsErrorResponse>(t(locale, "server.api.flags.flag_not_found"))
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_key, "Failed to get flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get flag stats
	let stats = match state.flags_repo.get_flag_stats(flag.id).await {
		Ok(Some(s)) => s,
		Ok(None) => {
			// No stats yet - return zeroes
			loom_flags_core::FlagStats {
				flag_key: flag.key.clone(),
				last_evaluated_at: None,
				evaluation_count_24h: 0,
				evaluation_count_7d: 0,
				evaluation_count_30d: 0,
			}
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_key, "Failed to get flag stats");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	(
		StatusCode::OK,
		Json(FlagStatsResponse {
			flag_key: stats.flag_key,
			last_evaluated_at: stats.last_evaluated_at,
			evaluation_count_24h: stats.evaluation_count_24h,
			evaluation_count_7d: stats.evaluation_count_7d,
			evaluation_count_30d: stats.evaluation_count_30d,
		}),
	)
		.into_response()
}
