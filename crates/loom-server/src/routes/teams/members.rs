// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Team member management HTTP handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::{types::TeamRole, Action};

use crate::{
	abac_middleware::{build_subject_attrs, team_resource},
	api::AppState,
	auth_middleware::RequireAuth,
	authorize,
	i18n::{resolve_user_locale, t},
	parse_id,
	validation::{
		parse_org_id as shared_parse_org_id, parse_team_id as shared_parse_team_id,
		parse_user_id as shared_parse_user_id,
	},
};

use super::common::{
	AddTeamMemberRequest, ListTeamMembersResponse, TeamErrorResponse, TeamMemberResponse,
	TeamSuccessResponse,
};

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/teams/{team_id}/members",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "List of team members", body = ListTeamMembersResponse),
        (status = 401, description = "Not authenticated", body = TeamErrorResponse),
        (status = 403, description = "Access denied", body = TeamErrorResponse),
        (status = 404, description = "Team not found", body = TeamErrorResponse)
    ),
    tag = "teams"
)]
/// List team members.
///
/// Returns all members of the specified team with their roles.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::Read` on the team.
/// Organization members can view team membership.
///
/// # Path Parameters
/// - `org_id`: Organization UUID
/// - `team_id`: Team UUID
///
/// # Response
/// Returns a list of members with user details and roles.
/// Email is only included if the user has `email_visible` enabled.
///
/// # Errors
/// - 400: Invalid ID format
/// - 401: Not authenticated
/// - 403: Access denied
/// - 404: Team not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id, %team_id))]
pub async fn list_team_members(
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

	let memberships = match state.team_repo.list_members(&team.id).await {
		Ok(members) => members,
		Err(e) => {
			tracing::error!(error = %e, %org_id, %team_id, "Failed to list team members");
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

	let mut members = Vec::with_capacity(memberships.len());
	for membership in memberships {
		let user = match state.user_repo.get_user_by_id(&membership.user_id).await {
			Ok(Some(user)) => user,
			Ok(None) => continue,
			Err(e) => {
				tracing::warn!(error = %e, user_id = %membership.user_id, "Failed to get user");
				continue;
			}
		};

		members.push(TeamMemberResponse {
			user_id: user.id.to_string(),
			display_name: user.display_name,
			email: if user.email_visible {
				user.primary_email
			} else {
				None
			},
			avatar_url: user.avatar_url,
			role: membership.role.into(),
			joined_at: membership.created_at,
		});
	}

	(StatusCode::OK, Json(ListTeamMembersResponse { members })).into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/teams/{team_id}/members",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("team_id" = String, Path, description = "Team ID")
    ),
    request_body = AddTeamMemberRequest,
    responses(
        (status = 200, description = "Member added", body = TeamSuccessResponse),
        (status = 400, description = "Invalid request", body = TeamErrorResponse),
        (status = 401, description = "Not authenticated", body = TeamErrorResponse),
        (status = 403, description = "Not authorized to add members", body = TeamErrorResponse),
        (status = 404, description = "Team or user not found", body = TeamErrorResponse),
        (status = 409, description = "User already a team member", body = TeamErrorResponse)
    ),
    tag = "teams"
)]
/// Add a member to a team.
///
/// Adds a user to the team. The user must already be a member of the organization.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::ManageTeam`.
/// Only org owners, admins, and team maintainers can add members.
///
/// # Path Parameters
/// - `org_id`: Organization UUID
/// - `team_id`: Team UUID
///
/// # Request Body
/// - `user_id`: UUID of the user to add (must be org member)
/// - `role`: Optional role (maintainer, member). Defaults to member.
///
/// # Response
/// Returns a success message.
///
/// # Errors
/// - 400: User not an org member, invalid user ID
/// - 401: Not authenticated
/// - 403: Not authorized
/// - 404: Team or user not found
/// - 409: User already a team member
/// - 500: Internal server error
#[tracing::instrument(skip(state, payload), fields(%org_id, %team_id, target_user_id = %payload.user_id))]
pub async fn add_team_member(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, team_id)): Path<(String, String)>,
	Json(payload): Json<AddTeamMemberRequest>,
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

	let target_user_id = parse_id!(
		TeamErrorResponse,
		shared_parse_user_id(&payload.user_id, &t(locale, "server.api.user.invalid_id"))
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

	if let Err(e) = authorize!(&subject, Action::ManageTeam, &resource) {
		return e.into_response();
	}

	match state
		.org_repo
		.get_membership(&org_id, &target_user_id)
		.await
	{
		Ok(Some(_)) => {}
		Ok(None) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(TeamErrorResponse {
					error: "not_org_member".to_string(),
					message: t(locale, "server.api.team.user_not_org_member").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %target_user_id, "Failed to check org membership");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(TeamErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.team.membership_check_failed").to_string(),
				}),
			)
				.into_response();
		}
	}

	if let Ok(Some(_)) = state
		.team_repo
		.get_membership(&team.id, &target_user_id)
		.await
	{
		return (
			StatusCode::CONFLICT,
			Json(TeamErrorResponse {
				error: "already_member".to_string(),
				message: t(locale, "server.api.team.already_member").to_string(),
			}),
		)
			.into_response();
	}

	let role: TeamRole = payload.role.into();
	if let Err(e) = state
		.team_repo
		.add_member(&team.id, &target_user_id, role)
		.await
	{
		tracing::error!(error = %e, %org_id, %team_id, %target_user_id, "Failed to add team member");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(TeamErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, %team_id, %target_user_id, added_by = %current_user.user.id, "Team member added");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::TeamMemberAdded)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("team", team.id.to_string())
			.details(serde_json::json!({
				"org_id": org_id.to_string(),
				"target_user_id": target_user_id.to_string(),
				"role": format!("{:?}", role),
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(TeamSuccessResponse {
			message: t(locale, "server.api.team.member_added").to_string(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/teams/{team_id}/members/{user_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("team_id" = String, Path, description = "Team ID"),
        ("user_id" = String, Path, description = "User ID to remove")
    ),
    responses(
        (status = 200, description = "Member removed", body = TeamSuccessResponse),
        (status = 401, description = "Not authenticated", body = TeamErrorResponse),
        (status = 403, description = "Not authorized to remove members", body = TeamErrorResponse),
        (status = 404, description = "Team or member not found", body = TeamErrorResponse)
    ),
    tag = "teams"
)]
/// Remove a member from a team.
///
/// Removes a user from the team. Users can remove themselves.
/// Cannot remove the last maintainer of a team.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::ManageTeam` unless removing self.
/// - Org owners, admins, and team maintainers can remove members
/// - Users can remove themselves from a team
///
/// # Path Parameters
/// - `org_id`: Organization UUID
/// - `team_id`: Team UUID
/// - `user_id`: UUID of the user to remove
///
/// # Response
/// Returns a success message.
///
/// # Errors
/// - 400: Cannot remove last maintainer
/// - 401: Not authenticated
/// - 403: Not authorized
/// - 404: Team or member not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id, %team_id, %user_id))]
pub async fn remove_team_member(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, team_id, user_id)): Path<(String, String, String)>,
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

	let target_user_id = parse_id!(
		TeamErrorResponse,
		shared_parse_user_id(&user_id, &t(locale, "server.api.user.invalid_id"))
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

	let is_self_removal = target_user_id == current_user.user.id;

	if !is_self_removal {
		let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
		let resource = team_resource(team.id, team.org_id);

		if let Err(e) = authorize!(&subject, Action::ManageTeam, &resource) {
			return e.into_response();
		}
	}

	let membership = match state
		.team_repo
		.get_membership(&team.id, &target_user_id)
		.await
	{
		Ok(Some(m)) => m,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(TeamErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.team.user_not_member").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %team_id, %target_user_id, "Failed to get team membership");
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

	if membership.role == TeamRole::Maintainer {
		let all_members = match state.team_repo.list_members(&team.id).await {
			Ok(members) => members,
			Err(e) => {
				tracing::error!(error = %e, %org_id, %team_id, "Failed to list team members");
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

		let maintainer_count = all_members
			.iter()
			.filter(|m| m.role == TeamRole::Maintainer)
			.count();

		if maintainer_count <= 1 {
			return (
				StatusCode::BAD_REQUEST,
				Json(TeamErrorResponse {
					error: "last_maintainer".to_string(),
					message: t(locale, "server.api.team.last_maintainer").to_string(),
				}),
			)
				.into_response();
		}
	}

	if let Err(e) = state
		.team_repo
		.remove_member(&team.id, &target_user_id)
		.await
	{
		tracing::error!(error = %e, %org_id, %team_id, %target_user_id, "Failed to remove team member");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(TeamErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, %team_id, %target_user_id, removed_by = %current_user.user.id, is_self_removal, "Team member removed");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::TeamMemberRemoved)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("team", team.id.to_string())
			.details(serde_json::json!({
				"org_id": org_id.to_string(),
				"target_user_id": target_user_id.to_string(),
				"is_self_removal": is_self_removal,
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(TeamSuccessResponse {
			message: t(locale, "server.api.team.member_removed").to_string(),
		}),
	)
		.into_response()
}
