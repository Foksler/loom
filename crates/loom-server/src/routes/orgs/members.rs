// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Organization member management HTTP handlers.
//!
//! Implements list, add, remove, and update role endpoints.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_api::orgs::{
	AddOrgMemberRequest, ListOrgMembersResponse, OrgErrorResponse, OrgMemberResponse,
	OrgSuccessResponse, UpdateOrgMemberRoleRequest,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::{types::OrgRole, Action};

use crate::{
	abac_middleware::{build_subject_attrs, org_resource},
	api::AppState,
	auth_middleware::RequireAuth,
	authorize,
	i18n::{resolve_user_locale, t},
	parse_id, parse_role,
	validation::{
		parse_org_id as shared_parse_org_id, parse_org_role, parse_user_id as shared_parse_user_id,
	},
};

use super::common::org_visibility_to_abac;

#[utoipa::path(
    get,
    path = "/api/orgs/{id}/members",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of members", body = ListOrgMembersResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Access denied", body = OrgErrorResponse),
        (status = 404, description = "Organization not found", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// List organization members.
///
/// Returns all members of the organization with their roles.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::Read` on the organization.
/// Members can view the member list.
///
/// # Path Parameters
/// - `id`: Organization UUID
///
/// # Response
/// Returns a list of members with user details and roles.
/// Email is only included if the user has `email_visible` enabled.
///
/// # Errors
/// - 400: Invalid organization ID format
/// - 401: Not authenticated
/// - 403: Access denied
/// - 404: Organization not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_org_members(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		OrgErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(OrgErrorResponse {
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
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = org_resource(org.id, org_visibility_to_abac(org.visibility));

	if let Err(e) = authorize!(&subject, Action::Read, &resource) {
		return e.into_response();
	}

	let members = match state.org_repo.list_members(&org_id).await {
		Ok(members) => members,
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to list members");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let member_responses: Vec<OrgMemberResponse> = members
		.into_iter()
		.map(|(membership, user)| OrgMemberResponse {
			user_id: user.id.to_string(),
			display_name: user.display_name,
			email: if user.email_visible {
				user.primary_email
			} else {
				None
			},
			avatar_url: user.avatar_url,
			role: membership.role.to_string(),
			joined_at: membership.created_at,
		})
		.collect();

	(
		StatusCode::OK,
		Json(ListOrgMembersResponse {
			members: member_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/members",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    request_body = AddOrgMemberRequest,
    responses(
        (status = 200, description = "Member invitation sent", body = OrgSuccessResponse),
        (status = 400, description = "Invalid request", body = OrgErrorResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Not authorized to add members", body = OrgErrorResponse),
        (status = 404, description = "Organization not found", body = OrgErrorResponse),
        (status = 409, description = "User already a member", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Add a member to an organization.
///
/// Adds a user to the organization by email address.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::ManageOrg`.
/// Owners and admins can invite new members.
///
/// # Path Parameters
/// - `id`: Organization UUID
///
/// # Request Body
/// - `email`: Email address of the user to add
/// - `role`: Optional role (owner, admin, member). Defaults to member.
///
/// # Response
/// Returns a success message.
///
/// # Errors
/// - 400: Invalid role
/// - 401: Not authenticated
/// - 403: Not authorized (not owner/admin)
/// - 404: Organization or user not found
/// - 409: User already a member
/// - 500: Internal server error
#[tracing::instrument(skip(state, payload), fields(%org_id))]
pub async fn add_org_member(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<AddOrgMemberRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		OrgErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(OrgErrorResponse {
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
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = org_resource(org.id, org_visibility_to_abac(org.visibility));

	if let Err(e) = authorize!(&subject, Action::ManageOrg, &resource) {
		return e.into_response();
	}

	let role = match payload.role.as_deref() {
		Some(r) => parse_role!(
			OrgErrorResponse,
			parse_org_role(r, &t(locale, "server.api.org.invalid_role"))
		),
		None => OrgRole::Member,
	};

	let target_user = match state.user_repo.get_user_by_email(&payload.email).await {
		Ok(Some(user)) => user,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(OrgErrorResponse {
					error: "user_not_found".to_string(),
					message: t(locale, "server.api.user.not_found_by_email").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to find user by email");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let target_user_id = target_user.id;
	if let Ok(Some(_)) = state
		.org_repo
		.get_membership(&org_id, &target_user.id)
		.await
	{
		return (
			StatusCode::CONFLICT,
			Json(OrgErrorResponse {
				error: "already_member".to_string(),
				message: t(locale, "server.api.org.already_member").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state
		.org_repo
		.add_member(&org_id, &target_user.id, role)
		.await
	{
		tracing::error!(error = %e, %org_id, %target_user_id, "Failed to add member");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, %target_user_id, added_by = %current_user.user.id, "Member added to organization");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::MemberAdded)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org_id.to_string())
			.details(serde_json::json!({
				"target_user_id": target_user_id.to_string(),
				"role": format!("{:?}", role),
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(OrgSuccessResponse {
			message: t(locale, "server.api.org.member_added").to_string(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/members/{user_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("user_id" = String, Path, description = "User ID to remove")
    ),
    responses(
        (status = 200, description = "Member removed", body = OrgSuccessResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Not authorized to remove members", body = OrgErrorResponse),
        (status = 404, description = "Organization or member not found", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Remove a member from an organization.
///
/// Removes a user from the organization. Users can remove themselves (leave).
/// The last owner cannot be removed.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::ManageOrg` unless removing self.
/// - Owners and admins can remove members
/// - Users can remove themselves (leave the organization)
///
/// # Path Parameters
/// - `org_id`: Organization UUID
/// - `user_id`: User UUID to remove
///
/// # Response
/// Returns a success message.
///
/// # Errors
/// - 400: Cannot remove last owner
/// - 401: Not authenticated
/// - 403: Not authorized
/// - 404: Organization or member not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id, %user_id))]
pub async fn remove_org_member(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, user_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		OrgErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let target_user_id = parse_id!(
		OrgErrorResponse,
		shared_parse_user_id(&user_id, &t(locale, "server.api.user.invalid_id"))
	);

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(OrgErrorResponse {
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
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let is_self_removal = current_user.user.id == target_user_id;

	if !is_self_removal {
		let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
		let resource = org_resource(org.id, org_visibility_to_abac(org.visibility));

		if let Err(e) = authorize!(&subject, Action::ManageOrg, &resource) {
			return e.into_response();
		}
	}

	let membership = match state
		.org_repo
		.get_membership(&org_id, &target_user_id)
		.await
	{
		Ok(Some(m)) => m,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(OrgErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.member_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %target_user_id, "Failed to get membership");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if membership.role == OrgRole::Owner {
		let owner_count = match state.org_repo.count_owners(&org_id).await {
			Ok(count) => count,
			Err(e) => {
				tracing::error!(error = %e, %org_id, "Failed to count owners");
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(OrgErrorResponse {
						error: "internal_error".to_string(),
						message: t(locale, "server.api.error.internal").to_string(),
					}),
				)
					.into_response();
			}
		};

		if owner_count <= 1 {
			return (
				StatusCode::BAD_REQUEST,
				Json(OrgErrorResponse {
					error: "last_owner".to_string(),
					message: t(locale, "server.api.org.last_owner").to_string(),
				}),
			)
				.into_response();
		}
	}

	if let Err(e) = state.org_repo.remove_member(&org_id, &target_user_id).await {
		tracing::error!(error = %e, %org_id, %target_user_id, "Failed to remove member");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, %target_user_id, removed_by = %current_user.user.id, is_self_removal, "Member removed from organization");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::MemberRemoved)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org_id.to_string())
			.details(serde_json::json!({
				"target_user_id": target_user_id.to_string(),
				"is_self_removal": is_self_removal,
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(OrgSuccessResponse {
			message: t(locale, "server.api.org.member_removed").to_string(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/members/{user_id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("user_id" = String, Path, description = "User ID to update")
    ),
    request_body = UpdateOrgMemberRoleRequest,
    responses(
        (status = 200, description = "Member role updated", body = OrgSuccessResponse),
        (status = 400, description = "Invalid role or cannot demote last owner", body = OrgErrorResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Not authorized to change roles", body = OrgErrorResponse),
        (status = 404, description = "Organization or member not found", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Update a member's role.
///
/// Changes a member's role within the organization.
/// Cannot demote the last owner.
///
/// # Authorization
/// Requires authentication. Only owners can change member roles.
///
/// # Errors
/// - 400: Invalid role or would remove last owner
/// - 401: Not authenticated
/// - 403: Not authorized (not owner)
/// - 404: Organization or member not found
/// - 500: Internal server error
#[tracing::instrument(skip(state, payload), fields(%org_id, %user_id))]
pub async fn update_org_member_role(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, user_id)): Path<(String, String)>,
	Json(payload): Json<UpdateOrgMemberRoleRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		OrgErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let target_user_id = parse_id!(
		OrgErrorResponse,
		shared_parse_user_id(&user_id, &t(locale, "server.api.user.invalid_id"))
	);

	let new_role = parse_role!(
		OrgErrorResponse,
		parse_org_role(&payload.role, &t(locale, "server.api.org.invalid_role"))
	);

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(OrgErrorResponse {
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
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let caller_membership = match state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		Ok(Some(m)) => m,
		Ok(None) => {
			return (
				StatusCode::FORBIDDEN,
				Json(OrgErrorResponse {
					error: "forbidden".to_string(),
					message: t(locale, "server.api.org.not_a_member").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to get caller membership");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if caller_membership.role != OrgRole::Owner {
		return (
			StatusCode::FORBIDDEN,
			Json(OrgErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.owner_required").to_string(),
			}),
		)
			.into_response();
	}

	let target_membership = match state
		.org_repo
		.get_membership(&org_id, &target_user_id)
		.await
	{
		Ok(Some(m)) => m,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(OrgErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.member_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %target_user_id, "Failed to get target membership");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if target_membership.role == OrgRole::Owner && new_role != OrgRole::Owner {
		let owner_count = match state.org_repo.count_owners(&org_id).await {
			Ok(count) => count,
			Err(e) => {
				tracing::error!(error = %e, %org_id, "Failed to count owners");
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(OrgErrorResponse {
						error: "internal_error".to_string(),
						message: t(locale, "server.api.error.internal").to_string(),
					}),
				)
					.into_response();
			}
		};

		if owner_count <= 1 {
			return (
				StatusCode::BAD_REQUEST,
				Json(OrgErrorResponse {
					error: "last_owner".to_string(),
					message: t(locale, "server.api.org.last_owner").to_string(),
				}),
			)
				.into_response();
		}
	}

	if let Err(e) = state
		.org_repo
		.update_member_role(&org_id, &target_user_id, new_role)
		.await
	{
		tracing::error!(error = %e, %org_id, %target_user_id, "Failed to update member role");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(
		%org_id,
		%target_user_id,
		old_role = %target_membership.role,
		%new_role,
		updated_by = %current_user.user.id,
		"Member role updated"
	);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::RoleChanged)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org_id.to_string())
			.details(serde_json::json!({
				"target_user_id": target_user_id.to_string(),
				"old_role": format!("{:?}", target_membership.role),
				"new_role": format!("{:?}", new_role),
			}))
			.build(),
	);

	let _ = org;

	(
		StatusCode::OK,
		Json(OrgSuccessResponse {
			message: t(locale, "server.api.org.role_updated").to_string(),
		}),
	)
		.into_response()
}
