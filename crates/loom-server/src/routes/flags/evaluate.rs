// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Flag evaluation HTTP handlers.
//!
//! Implements endpoints for evaluating feature flags.

use axum::{
	extract::{Path, State},
	http::{HeaderMap, StatusCode},
	response::IntoResponse,
	Json,
};
use loom_flags_core::VariantValue;
use loom_server_api::flags::{
	EvaluateAllFlagsRequest, EvaluateAllFlagsResponse, EvaluateFlagRequest, EvaluationContextApi,
	EvaluationReasonApi, EvaluationResultApi, FlagsErrorResponse, VariantValueApi,
};
#[cfg(test)]
use loom_server_api::flags::GeoContextApi;
use loom_server_flags::{
	evaluate_flag, EvaluationContext, EvaluationReason, FlagsRepository, GeoContext,
};

use crate::{
	api::AppState,
	api_response::{bad_request, internal_error, not_found},
	auth_middleware::RequireAuth,
	client_info::ClientInfo,
	i18n::{resolve_user_locale, t},
	parse_id,
	validation::parse_org_id as shared_parse_org_id,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper to convert API context to core context with optional server-resolved GeoIP.
///
/// Server-resolved GeoIP data takes precedence over client-provided geo context,
/// ensuring geographic targeting cannot be spoofed by clients.
fn to_core_context_with_geo(
	api_ctx: &EvaluationContextApi,
	server_geo: Option<&ClientInfo>,
) -> EvaluationContext {
	let mut ctx = EvaluationContext::new(&api_ctx.environment);

	if let Some(ref user_id) = api_ctx.user_id {
		ctx = ctx.with_user_id(user_id);
	}
	if let Some(ref org_id) = api_ctx.org_id {
		ctx = ctx.with_org_id(org_id);
	}
	if let Some(ref session_id) = api_ctx.session_id {
		ctx = ctx.with_session_id(session_id);
	}

	for (key, value) in &api_ctx.attributes {
		ctx = ctx.with_attribute(key, value.clone());
	}

	// Server-resolved GeoIP takes precedence over client-provided geo context
	// to prevent clients from spoofing their geographic location
	if let Some(client_info) = server_geo {
		if client_info.geo_country.is_some()
			|| client_info.geo_region.is_some()
			|| client_info.geo_city.is_some()
		{
			let mut geo_ctx = GeoContext::new();
			if let Some(ref country) = client_info.geo_country {
				geo_ctx = geo_ctx.with_country(country);
			}
			if let Some(ref region) = client_info.geo_region {
				geo_ctx = geo_ctx.with_region(region);
			}
			if let Some(ref city) = client_info.geo_city {
				geo_ctx = geo_ctx.with_city(city);
			}
			ctx = ctx.with_geo(geo_ctx);
			return ctx;
		}
	}

	// Fall back to client-provided geo context if server resolution failed
	if let Some(ref geo) = api_ctx.geo {
		let mut geo_ctx = GeoContext::new();
		if let Some(ref country) = geo.country {
			geo_ctx = geo_ctx.with_country(country);
		}
		if let Some(ref region) = geo.region {
			geo_ctx = geo_ctx.with_region(region);
		}
		if let Some(ref city) = geo.city {
			geo_ctx = geo_ctx.with_city(city);
		}
		ctx = ctx.with_geo(geo_ctx);
	}

	ctx
}

/// Helper to convert API context to core context (without server GeoIP).
#[allow(dead_code)] // Used in tests
fn to_core_context(api_ctx: &EvaluationContextApi) -> EvaluationContext {
	to_core_context_with_geo(api_ctx, None)
}

/// Helper to convert core evaluation reason to API reason.
fn to_api_reason(reason: &EvaluationReason) -> EvaluationReasonApi {
	match reason {
		EvaluationReason::Default => EvaluationReasonApi::Default,
		EvaluationReason::Strategy { strategy_id } => EvaluationReasonApi::Strategy {
			strategy_id: strategy_id.to_string(),
		},
		EvaluationReason::KillSwitch { kill_switch_id } => EvaluationReasonApi::KillSwitch {
			kill_switch_id: kill_switch_id.to_string(),
		},
		EvaluationReason::Prerequisite { missing_flag } => EvaluationReasonApi::Prerequisite {
			missing_flag: missing_flag.clone(),
		},
		EvaluationReason::Disabled => EvaluationReasonApi::Disabled,
		EvaluationReason::Error { message } => EvaluationReasonApi::Error {
			message: message.clone(),
		},
	}
}

/// Helper to convert core variant value to API variant value.
fn to_api_variant_value(value: &VariantValue) -> VariantValueApi {
	match value {
		VariantValue::Boolean(b) => VariantValueApi::Boolean(*b),
		VariantValue::String(s) => VariantValueApi::String(s.clone()),
		VariantValue::Json(v) => VariantValueApi::Json(v.clone()),
	}
}

// ============================================================================
// Evaluation Routes
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/evaluate",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = EvaluateAllFlagsRequest,
    responses(
        (status = 200, description = "All flags evaluated", body = EvaluateAllFlagsResponse),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Evaluate all flags for a given context.
///
/// This endpoint evaluates all non-archived flags for the organization
/// and returns the evaluation results for each flag. The evaluation
/// takes into account:
/// - Environment configuration (enabled/disabled)
/// - Kill switches (platform and org level)
/// - Prerequisites
/// - Strategy conditions, percentage targeting, and schedules
/// - GeoIP (resolved from client IP via proxy headers)
#[tracing::instrument(skip(state, headers, payload), fields(%org_id, environment = %payload.context.environment))]
pub async fn evaluate_all_flags(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(org_id): Path<String>,
	Json(payload): Json<EvaluateAllFlagsRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	// Extract client info including GeoIP from request headers
	let client_info = ClientInfo::from_headers(&headers, state.geoip_service.as_ref());

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
	// Use server-resolved GeoIP for evaluation context
	let context = to_core_context_with_geo(&payload.context, Some(&client_info));

	// Get the environment for this context
	let environment = match state
		.flags_repo
		.get_environment_by_name(flags_org_id, &payload.context.environment)
		.await
	{
		Ok(Some(env)) => env,
		Ok(None) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_environment",
				t(locale, "server.api.flags.environment_not_found"),
			)
			.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, environment = %payload.context.environment, "Failed to get environment");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get all flags for this org (non-archived)
	let flags = match state.flags_repo.list_flags(Some(flags_org_id), false).await {
		Ok(flags) => flags,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list flags");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get platform flags as well (they override org flags)
	let platform_flags = match state.flags_repo.list_flags(None, false).await {
		Ok(flags) => flags,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list platform flags");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get active kill switches (platform first, then org)
	let platform_kill_switches = match state.flags_repo.list_active_kill_switches(None).await {
		Ok(ks) => ks,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list platform kill switches");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let org_kill_switches = match state
		.flags_repo
		.list_active_kill_switches(Some(flags_org_id))
		.await
	{
		Ok(ks) => ks,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list org kill switches");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Combine kill switches (platform first for precedence)
	let all_kill_switches: Vec<_> = platform_kill_switches
		.into_iter()
		.chain(org_kill_switches.into_iter())
		.collect();

	// Build a map of flag keys to flags, with platform flags taking precedence
	let mut flag_map: std::collections::HashMap<String, &loom_flags_core::Flag> =
		std::collections::HashMap::new();

	// Add org flags first
	for flag in &flags {
		flag_map.insert(flag.key.clone(), flag);
	}

	// Platform flags override org flags
	for flag in &platform_flags {
		flag_map.insert(flag.key.clone(), flag);
	}

	// Evaluate each flag
	let mut results = Vec::with_capacity(flag_map.len());

	for flag in flag_map.values() {
		// Get config for this environment
		let config = match state
			.flags_repo
			.get_flag_config(flag.id, environment.id)
			.await
		{
			Ok(config) => config,
			Err(e) => {
				tracing::error!(error = %e, flag_key = %flag.key, "Failed to get flag config");
				// Return an error result for this flag
				results.push(EvaluationResultApi {
					flag_key: flag.key.clone(),
					variant: flag.default_variant.clone(),
					value: to_api_variant_value(
						&flag
							.get_default_variant()
							.map(|v| v.value.clone())
							.unwrap_or(VariantValue::Boolean(false)),
					),
					reason: EvaluationReasonApi::Error {
						message: "Failed to get flag config".to_string(),
					},
				});
				continue;
			}
		};

		// Get strategy if configured
		let strategy = match &config {
			Some(c) => match c.strategy_id {
				Some(strategy_id) => match state.flags_repo.get_strategy_by_id(strategy_id).await {
					Ok(s) => s,
					Err(e) => {
						tracing::error!(error = %e, %strategy_id, "Failed to get strategy");
						None
					}
				},
				None => None,
			},
			None => None,
		};

		// Evaluate prerequisites - collect owned strings to avoid lifetime issues
		let mut prereq_results: Vec<(String, String)> = Vec::new();
		for prereq in &flag.prerequisites {
			if let Some(prereq_flag) = flag_map.get(&prereq.flag_key) {
				let prereq_config = state
					.flags_repo
					.get_flag_config(prereq_flag.id, environment.id)
					.await
					.ok()
					.flatten();
				let prereq_result = evaluate_flag(
					prereq_flag,
					prereq_config.as_ref(),
					None,
					&all_kill_switches,
					&[],
					&context,
				);
				prereq_results.push((prereq.flag_key.clone(), prereq_result.variant.clone()));
			}
		}

		// Convert prereq_results to the expected format
		let prereq_refs: Vec<(&str, &str)> = prereq_results
			.iter()
			.map(|(k, v)| (k.as_str(), v.as_str()))
			.collect();

		let result = evaluate_flag(
			flag,
			config.as_ref(),
			strategy.as_ref(),
			&all_kill_switches,
			&prereq_refs,
			&context,
		);

		// Record evaluation stats (fire and forget, don't block on errors)
		let flag_id = flag.id;
		let flag_key_for_stats = flag.key.clone();
		let repo = state.flags_repo.clone();
		tokio::spawn(async move {
			if let Err(e) = repo
				.record_flag_evaluation(flag_id, &flag_key_for_stats)
				.await
			{
				tracing::warn!(error = %e, %flag_id, "Failed to record flag evaluation stats");
			}
		});

		results.push(EvaluationResultApi {
			flag_key: result.flag_key,
			variant: result.variant,
			value: to_api_variant_value(&result.value),
			reason: to_api_reason(&result.reason),
		});
	}

	tracing::debug!(
		?org_id,
		environment = %payload.context.environment,
		flag_count = results.len(),
		"Evaluated all flags"
	);

	(
		StatusCode::OK,
		Json(EvaluateAllFlagsResponse {
			results,
			evaluated_at: chrono::Utc::now(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/flags/{flag_key}/evaluate",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("flag_key" = String, Path, description = "Flag key to evaluate")
    ),
    request_body = EvaluateFlagRequest,
    responses(
        (status = 200, description = "Flag evaluated", body = EvaluationResultApi),
        (status = 400, description = "Invalid request", body = FlagsErrorResponse),
        (status = 401, description = "Not authenticated", body = FlagsErrorResponse),
        (status = 404, description = "Flag or organization not found", body = FlagsErrorResponse)
    ),
    tag = "flags"
)]
/// Evaluate a single flag for a given context.
///
/// This endpoint evaluates a specific flag and returns the evaluation result.
/// Platform flags take precedence over org flags with the same key.
/// GeoIP is resolved server-side from the client IP address.
#[tracing::instrument(skip(state, headers, payload), fields(%org_id, %flag_key, environment = %payload.context.environment))]
pub async fn evaluate_flag_endpoint(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	headers: HeaderMap,
	Path((org_id, flag_key)): Path<(String, String)>,
	Json(payload): Json<EvaluateFlagRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		FlagsErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	// Extract client info including GeoIP from request headers
	let client_info = ClientInfo::from_headers(&headers, state.geoip_service.as_ref());

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
	// Use server-resolved GeoIP for evaluation context
	let context = to_core_context_with_geo(&payload.context, Some(&client_info));

	// Get the environment for this context
	let environment = match state
		.flags_repo
		.get_environment_by_name(flags_org_id, &payload.context.environment)
		.await
	{
		Ok(Some(env)) => env,
		Ok(None) => {
			return bad_request::<FlagsErrorResponse>(
				"invalid_environment",
				t(locale, "server.api.flags.environment_not_found"),
			)
			.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, environment = %payload.context.environment, "Failed to get environment");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Check for platform flag first (takes precedence)
	let flag = match state.flags_repo.get_flag_by_key(None, &flag_key).await {
		Ok(Some(f)) => f,
		Ok(None) => {
			// Try org flag
			match state
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
			}
		}
		Err(e) => {
			tracing::error!(error = %e, %flag_key, "Failed to get platform flag");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get active kill switches (platform first, then org)
	let platform_kill_switches = match state.flags_repo.list_active_kill_switches(None).await {
		Ok(ks) => ks,
		Err(e) => {
			tracing::error!(error = %e, "Failed to list platform kill switches");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let org_kill_switches = match state
		.flags_repo
		.list_active_kill_switches(Some(flags_org_id))
		.await
	{
		Ok(ks) => ks,
		Err(e) => {
			tracing::error!(error = %e, ?org_id, "Failed to list org kill switches");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let all_kill_switches: Vec<_> = platform_kill_switches
		.into_iter()
		.chain(org_kill_switches.into_iter())
		.collect();

	// Get config for this environment
	let config = match state
		.flags_repo
		.get_flag_config(flag.id, environment.id)
		.await
	{
		Ok(config) => config,
		Err(e) => {
			tracing::error!(error = %e, %flag_key, "Failed to get flag config");
			return internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	// Get strategy if configured
	let strategy = match &config {
		Some(c) => match c.strategy_id {
			Some(strategy_id) => match state.flags_repo.get_strategy_by_id(strategy_id).await {
				Ok(s) => s,
				Err(e) => {
					tracing::error!(error = %e, %strategy_id, "Failed to get strategy");
					None
				}
			},
			None => None,
		},
		None => None,
	};

	// Evaluate prerequisites
	let mut prereq_results: Vec<(String, String)> = Vec::new();
	for prereq in &flag.prerequisites {
		// Try platform flag first, then org flag
		let prereq_flag = match state
			.flags_repo
			.get_flag_by_key(None, &prereq.flag_key)
			.await
		{
			Ok(Some(f)) => Some(f),
			Ok(None) => state
				.flags_repo
				.get_flag_by_key(Some(flags_org_id), &prereq.flag_key)
				.await
				.ok()
				.flatten(),
			Err(_) => None,
		};

		if let Some(prereq_flag) = prereq_flag {
			let prereq_config = state
				.flags_repo
				.get_flag_config(prereq_flag.id, environment.id)
				.await
				.ok()
				.flatten();
			let prereq_result = evaluate_flag(
				&prereq_flag,
				prereq_config.as_ref(),
				None,
				&all_kill_switches,
				&[],
				&context,
			);
			prereq_results.push((prereq.flag_key.clone(), prereq_result.variant));
		}
	}

	let prereq_refs: Vec<(&str, &str)> = prereq_results
		.iter()
		.map(|(k, v)| (k.as_str(), v.as_str()))
		.collect();

	let result = evaluate_flag(
		&flag,
		config.as_ref(),
		strategy.as_ref(),
		&all_kill_switches,
		&prereq_refs,
		&context,
	);

	// Record evaluation stats (fire and forget, don't block on errors)
	let flag_id = flag.id;
	let flag_key_for_stats = flag.key.clone();
	let repo = state.flags_repo.clone();
	tokio::spawn(async move {
		if let Err(e) = repo
			.record_flag_evaluation(flag_id, &flag_key_for_stats)
			.await
		{
			tracing::warn!(error = %e, %flag_id, "Failed to record flag evaluation stats");
		}
	});

	tracing::debug!(
		%flag_key,
		variant = %result.variant,
		?result.reason,
		"Flag evaluated"
	);

	(
		StatusCode::OK,
		Json(EvaluationResultApi {
			flag_key: result.flag_key,
			variant: result.variant,
			value: to_api_variant_value(&result.value),
			reason: to_api_reason(&result.reason),
		}),
	)
		.into_response()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_to_core_context_without_geo() {
		let api_ctx = EvaluationContextApi {
			environment: "prod".to_string(),
			user_id: Some("user123".to_string()),
			org_id: Some("org456".to_string()),
			session_id: None,
			attributes: std::collections::HashMap::new(),
			geo: None,
		};

		let ctx = to_core_context(&api_ctx);

		assert_eq!(ctx.environment, "prod");
		assert_eq!(ctx.user_id, Some("user123".to_string()));
		assert_eq!(ctx.org_id, Some("org456".to_string()));
		assert!(ctx.geo.is_none());
	}

	#[test]
	fn test_to_core_context_with_client_geo() {
		let api_ctx = EvaluationContextApi {
			environment: "prod".to_string(),
			user_id: None,
			org_id: None,
			session_id: None,
			attributes: std::collections::HashMap::new(),
			geo: Some(GeoContextApi {
				country: Some("United States".to_string()),
				region: Some("California".to_string()),
				city: Some("San Francisco".to_string()),
			}),
		};

		let ctx = to_core_context(&api_ctx);

		assert!(ctx.geo.is_some());
		let geo = ctx.geo.unwrap();
		assert_eq!(geo.country, Some("United States".to_string()));
		assert_eq!(geo.region, Some("California".to_string()));
		assert_eq!(geo.city, Some("San Francisco".to_string()));
	}

	#[test]
	fn test_to_core_context_with_geo_server_overrides_client() {
		let api_ctx = EvaluationContextApi {
			environment: "prod".to_string(),
			user_id: None,
			org_id: None,
			session_id: None,
			attributes: std::collections::HashMap::new(),
			geo: Some(GeoContextApi {
				country: Some("Fake Country".to_string()),
				region: Some("Fake Region".to_string()),
				city: Some("Fake City".to_string()),
			}),
		};

		let server_geo = ClientInfo {
			ip_address: Some("8.8.8.8".to_string()),
			user_agent: Some("Mozilla/5.0".to_string()),
			geo_city: Some("Mountain View".to_string()),
			geo_region: Some("California".to_string()),
			geo_country: Some("United States".to_string()),
		};

		// Server-resolved GeoIP should take precedence
		let ctx = to_core_context_with_geo(&api_ctx, Some(&server_geo));

		assert!(ctx.geo.is_some());
		let geo = ctx.geo.unwrap();
		assert_eq!(geo.country, Some("United States".to_string()));
		assert_eq!(geo.region, Some("California".to_string()));
		assert_eq!(geo.city, Some("Mountain View".to_string()));
	}

	#[test]
	fn test_to_core_context_with_geo_fallback_to_client() {
		let api_ctx = EvaluationContextApi {
			environment: "prod".to_string(),
			user_id: None,
			org_id: None,
			session_id: None,
			attributes: std::collections::HashMap::new(),
			geo: Some(GeoContextApi {
				country: Some("Japan".to_string()),
				region: Some("Tokyo".to_string()),
				city: Some("Shibuya".to_string()),
			}),
		};

		// Server GeoIP lookup failed (no geo data)
		let server_geo = ClientInfo {
			ip_address: Some("192.168.1.1".to_string()), // Private IP - no geo data
			user_agent: Some("Mozilla/5.0".to_string()),
			geo_city: None,
			geo_region: None,
			geo_country: None,
		};

		// Should fall back to client-provided geo
		let ctx = to_core_context_with_geo(&api_ctx, Some(&server_geo));

		assert!(ctx.geo.is_some());
		let geo = ctx.geo.unwrap();
		assert_eq!(geo.country, Some("Japan".to_string()));
		assert_eq!(geo.region, Some("Tokyo".to_string()));
		assert_eq!(geo.city, Some("Shibuya".to_string()));
	}

	#[test]
	fn test_to_core_context_with_geo_partial_server_data() {
		let api_ctx = EvaluationContextApi {
			environment: "prod".to_string(),
			user_id: None,
			org_id: None,
			session_id: None,
			attributes: std::collections::HashMap::new(),
			geo: Some(GeoContextApi {
				country: Some("Client Country".to_string()),
				region: Some("Client Region".to_string()),
				city: Some("Client City".to_string()),
			}),
		};

		// Server has country only
		let server_geo = ClientInfo {
			ip_address: Some("8.8.8.8".to_string()),
			user_agent: None,
			geo_city: None,
			geo_region: None,
			geo_country: Some("Germany".to_string()),
		};

		// Server data takes precedence (even if partial)
		let ctx = to_core_context_with_geo(&api_ctx, Some(&server_geo));

		assert!(ctx.geo.is_some());
		let geo = ctx.geo.unwrap();
		assert_eq!(geo.country, Some("Germany".to_string()));
		assert_eq!(geo.region, None);
		assert_eq!(geo.city, None);
	}
}
