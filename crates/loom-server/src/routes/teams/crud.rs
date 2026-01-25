// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Team CRUD HTTP handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::{
	team::Team,
	types::{TeamId, TeamRole},
	Action,
};

use crate::{
	abac_middleware::{build_subject_attrs, team_resource},
	api::AppState,
	api_response::{bad_request, conflict},
	auth_middleware::RequireAuth,
	authorize,
	i18n::{resolve_user_locale, t},
	parse_id, validate_slug_or_error,
	validation::{parse_org_id as shared_parse_org_id, parse_team_id as shared_parse_team_id, validate_slug_with_error},
};

use super::common::{
	CreateTeamRequest, ListTeamsResponse, TeamErrorResponse, TeamResponse, TeamSuccessResponse,
	UpdateTeamRequest,
};

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/teams",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of teams", body = ListTeamsResponse),
        (status = 401, description = "Not authenticated", body = TeamErrorResponse),
        (status = 403, description = "Access denied", body = TeamErrorResponse),
        (status = 404, description = "Organization not found", body = TeamErrorResponse)
    ),
    tag = "teams"
)]
/// List teams in an organization.
///
/// Returns all teams within the specified organization.
///
/// # Authorization
/// Requires authentication. User must be a member of the organization.
///
/// # Path Parameters
/// - `org_id`: Organization UUID
///
/// # Response
/// Returns a list of teams with basic details and member counts.
///
/// # Errors
/// - 400: Invalid organization ID format
/// - 401: Not authenticated
/// - 403: Not a member of the organization
/// - 404: Organization not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_teams(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = parse_id!(
		TeamErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(TeamErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to get organization");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(TeamErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let user_id = current_user.user.id;
	let membership = match state
		.org_repo
		.get_membership(&org.id, &current_user.user.id)
		.await
	{
		Ok(Some(m)) => m,
		Ok(None) => {
			return (
				StatusCode::FORBIDDEN,
				Json(TeamErrorResponse {
					error: "forbidden".to_string(),
					message: t(locale, "server.api.org.not_a_member").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %user_id, "Failed to check org membership");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(TeamErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.team.membership_check_failed").to_string(),
				}),
			)
				.into_response();
		}
	};

	let _ = membership;

	let teams = match state.team_repo.list_teams_for_org(&org_id).await {
		Ok(teams) => teams,
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to list teams");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(TeamErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.team.list_failed").to_string(),
				}),
			)
				.into_response();
		}
	};

	let mut team_responses = Vec::with_capacity(teams.len());
	for team in teams {
		let member_count = match state.team_repo.list_members(&team.id).await {
			Ok(members) => Some(members.len() as i64),
			Err(_) => None,
		};
		team_responses.push(TeamResponse::from_team(team, member_count));
	}

	(
		StatusCode::OK,
		Json(ListTeamsResponse {
			teams: team_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/teams",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateTeamRequest,
    responses(
        (status = 201, description = "Team created", body = TeamResponse),
        (status = 400, description = "Invalid request", body = TeamErrorResponse),
        (status = 401, description = "Not authenticated", body = TeamErrorResponse),
        (status = 403, description = "Not authorized to create teams", body = TeamErrorResponse),
        (status = 404, description = "Organization not found", body = TeamErrorResponse),
        (status = 409, description = "Team slug already exists", body = TeamErrorResponse)
    ),
    tag = "teams"
)]
/// Create a new team.
///
/// Creates a new team within the organization. The creator becomes a maintainer.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::ManageTeam`.
/// Only org owners and admins can create teams.
///
/// # Path Parameters
/// - `org_id`: Organization UUID
///
/// # Request Body
/// - `name`: Team display name (1-100 characters)
/// - `slug`: URL-safe identifier (2-50 chars, lowercase alphanumeric and hyphens)
///
/// # Response
/// Returns the created team with member count of 1.
///
/// # Errors
/// - 400: Invalid slug format or name
/// - 401: Not authenticated
/// - 403: Not authorized (not org owner/admin)
/// - 404: Organization not found
/// - 409: Slug already exists in this organization
/// - 500: Internal server error
#[tracing::instrument(skip(state, payload), fields(%org_id, slug = %payload.slug))]
pub async fn create_team(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateTeamRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = parse_id!(
		TeamErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	validate_slug_or_error!(
		TeamErrorResponse,
		validate_slug_with_error(
			&payload.slug,
			2,
			50,
			&t(locale, "server.api.team.invalid_slug_length"),
			&t(locale, "server.api.team.invalid_slug_format")
		)
	);

	if payload.name.is_empty() || payload.name.len() > 100 {
		return bad_request::<TeamErrorResponse>(
			"invalid_name",
			t(locale, "server.api.team.invalid_name_length"),
		)
		.into_response();
	}

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(TeamErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to get organization");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(TeamErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = team_resource(TeamId::generate(), org.id);

	if let Err(e) = authorize!(&subject, Action::ManageTeam, &resource) {
		return e.into_response();
	}

	if let Ok(Some(_)) = state
		.team_repo
		.get_team_by_slug(&org_id, &payload.slug)
		.await
	{
		return (
			StatusCode::CONFLICT,
			Json(TeamErrorResponse {
				error: "slug_exists".to_string(),
				message: t(locale, "server.api.team.slug_exists").to_string(),
			}),
		)
			.into_response();
	}

	let team = Team::new(org_id, &payload.name, &payload.slug);
	let team_id = team.id;
	let user_id = current_user.user.id;

	if let Err(e) = state.team_repo.create_team(&team).await {
		tracing::error!(error = %e, %org_id, %team_id, "Failed to create team");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(TeamErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state
		.team_repo
		.add_member(&team.id, &current_user.user.id, TeamRole::Maintainer)
		.await
	{
		tracing::error!(error = %e, %org_id, %team_id, %user_id, "Failed to add creator as team lead");
	}

	tracing::info!(%org_id, %team_id, %user_id, "Team created");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::TeamCreated)
			.actor(AuditUserId::new(user_id.into_inner()))
			.resource("team", team.id.to_string())
			.details(serde_json::json!({
				"org_id": org_id.to_string(),
				"name": team.name,
				"slug": team.slug,
			}))
			.build(),
	);

	(
		StatusCode::CREATED,
		Json(TeamResponse::from_team(team, Some(1))),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/teams/{team_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Team details", body = TeamResponse),
        (status = 401, description = "Not authenticated", body = TeamErrorResponse),
        (status = 403, description = "Access denied", body = TeamErrorResponse),
        (status = 404, description = "Team not found", body = TeamErrorResponse)
    ),
    tag = "teams"
)]
/// Get team details.
///
/// Returns detailed information about a team.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::Read` on the team.
/// Organization members can view teams.
///
/// # Path Parameters
/// - `org_id`: Organization UUID
/// - `team_id`: Team UUID
///
/// # Response
/// Returns team details including member count.
///
/// # Errors
/// - 400: Invalid ID format
/// - 401: Not authenticated
/// - 403: Access denied
/// - 404: Team not found or not in this organization
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id, %team_id))]
pub async fn get_team(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, team_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = parse_id!(
		TeamErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let team_id = parse_id!(
		TeamErrorResponse,
		shared_parse_team_id(&team_id, &t(locale, "server.api.team.invalid_id"))
	);

	let team = match state.team_repo.get_team_by_id(&team_id).await {
		Ok(Some(team)) => team,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(TeamErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.team.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %team_id, "Failed to get team");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(TeamErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if team.org_id != org_id {
		return (
			StatusCode::NOT_FOUND,
			Json(TeamErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.team.not_found_in_org").to_string(),
			}),
		)
			.into_response();
	}

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = team_resource(team.id, team.org_id);

	if let Err(e) = authorize!(&subject, Action::Read, &resource) {
		return e.into_response();
	}

	let member_count = match state.team_repo.list_members(&team.id).await {
		Ok(members) => Some(members.len() as i64),
		Err(_) => None,
	};

	(
		StatusCode::OK,
		Json(TeamResponse::from_team(team, member_count)),
	)
		.into_response()
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/teams/{team_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("team_id" = String, Path, description = "Team ID")
    ),
    request_body = UpdateTeamRequest,
    responses(
        (status = 200, description = "Team updated", body = TeamResponse),
        (status = 400, description = "Invalid request", body = TeamErrorResponse),
        (status = 401, description = "Not authenticated", body = TeamErrorResponse),
        (status = 403, description = "Not authorized to update team", body = TeamErrorResponse),
        (status = 404, description = "Team not found", body = TeamErrorResponse),
        (status = 409, description = "Team slug already exists", body = TeamErrorResponse)
    ),
    tag = "teams"
)]
/// Update a team.
///
/// Updates team settings like name and slug.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::Write` on the team.
/// Only org owners, admins, and team maintainers can update.
///
/// # Path Parameters
/// - `org_id`: Organization UUID
/// - `team_id`: Team UUID
///
/// # Request Body
/// All fields are optional:
/// - `name`: New display name (1-100 characters)
/// - `slug`: New URL-safe identifier (must be unique within org)
///
/// # Response
/// Returns the updated team.
///
/// # Errors
/// - 400: Invalid slug format or name
/// - 401: Not authenticated
/// - 403: Not authorized
/// - 404: Team not found
/// - 409: New slug already exists
/// - 500: Internal server error
#[tracing::instrument(skip(state, payload), fields(%org_id, %team_id))]
pub async fn update_team(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, team_id)): Path<(String, String)>,
	Json(payload): Json<UpdateTeamRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = parse_id!(
		TeamErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let team_id = parse_id!(
		TeamErrorResponse,
		shared_parse_team_id(&team_id, &t(locale, "server.api.team.invalid_id"))
	);

	let mut team = match state.team_repo.get_team_by_id(&team_id).await {
		Ok(Some(team)) => team,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(TeamErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.team.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %team_id, "Failed to get team");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(TeamErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if team.org_id != org_id {
		return (
			StatusCode::NOT_FOUND,
			Json(TeamErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.team.not_found_in_org").to_string(),
			}),
		)
			.into_response();
	}

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = team_resource(team.id, team.org_id);

	if let Err(e) = authorize!(&subject, Action::Write, &resource) {
		return e.into_response();
	}

	if let Some(ref name) = payload.name {
		if name.is_empty() || name.len() > 100 {
			return (
				StatusCode::BAD_REQUEST,
				Json(TeamErrorResponse {
					error: "invalid_name".to_string(),
					message: t(locale, "server.api.team.invalid_name_length").to_string(),
				}),
			)
				.into_response();
		}
		team.name = name.clone();
	}

	if let Some(ref slug) = payload.slug {
		validate_slug_or_error!(
			TeamErrorResponse,
			validate_slug_with_error(
				slug,
				2,
				50,
				&t(locale, "server.api.team.invalid_slug_length"),
				&t(locale, "server.api.team.invalid_slug_format")
			)
		);

		if slug != &team.slug {
			if let Ok(Some(_)) = state.team_repo.get_team_by_slug(&org_id, slug).await {
				return conflict::<TeamErrorResponse>(
					"slug_exists",
					t(locale, "server.api.team.slug_exists"),
				)
				.into_response();
			}
		}
		team.slug = slug.clone();
	}

	if let Err(e) = state.team_repo.update_team(&team).await {
		tracing::error!(error = %e, %org_id, %team_id, "Failed to update team");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(TeamErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, %team_id, user_id = %current_user.user.id, "Team updated");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::TeamUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("team", team.id.to_string())
			.details(serde_json::json!({
				"org_id": org_id.to_string(),
				"name": team.name,
				"slug": team.slug,
			}))
			.build(),
	);

	let member_count = match state.team_repo.list_members(&team.id).await {
		Ok(members) => Some(members.len() as i64),
		Err(_) => None,
	};

	(
		StatusCode::OK,
		Json(TeamResponse::from_team(team, member_count)),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/teams/{team_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Team deleted", body = TeamSuccessResponse),
        (status = 401, description = "Not authenticated", body = TeamErrorResponse),
        (status = 403, description = "Not authorized to delete team", body = TeamErrorResponse),
        (status = 404, description = "Team not found", body = TeamErrorResponse)
    ),
    tag = "teams"
)]
/// Delete a team.
///
/// Permanently deletes a team and all its memberships.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::Delete` on the team.
/// Only org owners and admins can delete teams.
///
/// # Path Parameters
/// - `org_id`: Organization UUID
/// - `team_id`: Team UUID
///
/// # Response
/// Returns a success message.
///
/// # Errors
/// - 400: Invalid ID format
/// - 401: Not authenticated
/// - 403: Not authorized
/// - 404: Team not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id, %team_id))]
pub async fn delete_team(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, team_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = parse_id!(
		TeamErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let team_id = parse_id!(
		TeamErrorResponse,
		shared_parse_team_id(&team_id, &t(locale, "server.api.team.invalid_id"))
	);

	let team = match state.team_repo.get_team_by_id(&team_id).await {
		Ok(Some(team)) => team,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(TeamErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.team.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %team_id, "Failed to get team");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(TeamErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if team.org_id != org_id {
		return (
			StatusCode::NOT_FOUND,
			Json(TeamErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.team.not_found_in_org").to_string(),
			}),
		)
			.into_response();
	}

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = team_resource(team.id, team.org_id);

	if let Err(e) = authorize!(&subject, Action::Delete, &resource) {
		return e.into_response();
	}

	if let Err(e) = state.team_repo.delete_team(&team.id).await {
		tracing::error!(error = %e, %org_id, %team_id, "Failed to delete team");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(TeamErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, %team_id, user_id = %current_user.user.id, "Team deleted");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::TeamDeleted)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("team", team.id.to_string())
			.details(serde_json::json!({
				"org_id": org_id.to_string(),
				"name": team.name,
				"slug": team.slug,
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(TeamSuccessResponse {
			message: t(locale, "server.api.team.deleted").to_string(),
		}),
	)
		.into_response()
}
