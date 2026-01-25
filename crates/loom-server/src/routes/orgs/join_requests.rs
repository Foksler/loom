// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Organization join request HTTP handlers.
//!
//! Implements create, list, approve, and reject endpoints.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_api::orgs::{
	JoinRequestResponse, ListJoinRequestsResponse, OrgErrorResponse, OrgSuccessResponse,
};
use loom_server_auth::{types::OrgRole, Action};

use crate::{
	abac_middleware::{build_subject_attrs, org_resource},
	api::AppState,
	auth_middleware::RequireAuth,
	authorize,
	i18n::{resolve_user_locale, t},
	parse_id,
	validation::parse_org_id as shared_parse_org_id,
};

use super::common::org_visibility_to_abac;

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/join-requests",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 201, description = "Join request created", body = OrgSuccessResponse),
        (status = 400, description = "Organization is private", body = OrgErrorResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 404, description = "Organization not found", body = OrgErrorResponse),
        (status = 409, description = "Already a member or pending request", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Request to join an organization.
///
/// Creates a pending join request for the current user.
/// Only works for public and unlisted organizations.
///
/// # Authorization
/// Requires authentication.
///
/// # Errors
/// - 400: Organization is private
/// - 401: Not authenticated
/// - 404: Organization not found
/// - 409: Already a member or pending request exists
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn create_join_request(
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

	if org.visibility == loom_server_auth::org::OrgVisibility::Private {
		return (
			StatusCode::BAD_REQUEST,
			Json(OrgErrorResponse {
				error: "private_org".to_string(),
				message: t(locale, "server.api.org.join_request_private").to_string(),
			}),
		)
			.into_response();
	}

	let user_id = &current_user.user.id;
	if let Ok(Some(_)) = state.org_repo.get_membership(&org_id, user_id).await {
		return (
			StatusCode::CONFLICT,
			Json(OrgErrorResponse {
				error: "already_member".to_string(),
				message: t(locale, "server.api.org.already_member").to_string(),
			}),
		)
			.into_response();
	}

	match state
		.org_repo
		.has_pending_join_request(&org_id, user_id)
		.await
	{
		Ok(true) => {
			return (
				StatusCode::CONFLICT,
				Json(OrgErrorResponse {
					error: "pending_request".to_string(),
					message: t(locale, "server.api.org.join_request_pending").to_string(),
				}),
			)
				.into_response();
		}
		Ok(false) => {}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check pending join request");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	}

	match state.org_repo.create_join_request(&org_id, user_id).await {
		Ok(_id) => {
			tracing::info!(%org_id, %user_id, "Join request created");
			(
				StatusCode::CREATED,
				Json(OrgSuccessResponse {
					message: t(locale, "server.api.org.join_request_created").to_string(),
				}),
			)
				.into_response()
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to create join request");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
    get,
    path = "/api/orgs/{id}/join-requests",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of pending join requests", body = ListJoinRequestsResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Not authorized to view join requests", body = OrgErrorResponse),
        (status = 404, description = "Organization not found", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// List pending join requests.
///
/// Returns all pending join requests for the organization.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::ManageOrg`.
/// Owners and admins can view join requests.
///
/// # Errors
/// - 401: Not authenticated
/// - 403: Not authorized (not owner/admin)
/// - 404: Organization not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn list_join_requests(
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

	if let Err(e) = authorize!(&subject, Action::ManageOrg, &resource) {
		return e.into_response();
	}

	let requests = match state
		.org_repo
		.list_pending_join_requests_with_users(&org_id)
		.await
	{
		Ok(requests) => requests,
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to list join requests");
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

	let request_responses: Vec<JoinRequestResponse> = requests
		.into_iter()
		.map(|(request, user)| JoinRequestResponse {
			id: request.user_id.to_string(),
			user_id: user.id.to_string(),
			display_name: user.display_name,
			email: if user.email_visible {
				user.primary_email
			} else {
				None
			},
			created_at: request.created_at,
		})
		.collect();

	(
		StatusCode::OK,
		Json(ListJoinRequestsResponse {
			requests: request_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/join-requests/{request_id}/approve",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("request_id" = String, Path, description = "Join request ID")
    ),
    responses(
        (status = 200, description = "Join request approved", body = OrgSuccessResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Not authorized to approve requests", body = OrgErrorResponse),
        (status = 404, description = "Organization or request not found", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Approve a join request.
///
/// Approves a pending join request and adds the user as a member.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::ManageOrg`.
/// Owners and admins can approve join requests.
///
/// # Errors
/// - 401: Not authenticated
/// - 403: Not authorized (not owner/admin)
/// - 404: Organization or request not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id, %request_id))]
pub async fn approve_join_request(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, request_id)): Path<(String, String)>,
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

	let join_request = match state.org_repo.get_join_request(&request_id).await {
		Ok(Some(req)) if req.is_pending() && req.org_id == org_id => req,
		Ok(Some(_)) | Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(OrgErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.join_request_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %request_id, "Failed to get join request");
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

	if let Err(e) = state
		.org_repo
		.approve_join_request(&request_id, &current_user.user.id)
		.await
	{
		tracing::error!(error = %e, %org_id, %request_id, "Failed to approve join request");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state
		.org_repo
		.add_member(&org_id, &join_request.user_id, OrgRole::Member)
		.await
	{
		tracing::error!(error = %e, %org_id, user_id = %join_request.user_id, "Failed to add member after approval");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, %request_id, approved_by = %current_user.user.id, "Join request approved");

	(
		StatusCode::OK,
		Json(OrgSuccessResponse {
			message: t(locale, "server.api.org.join_request_approved").to_string(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/join-requests/{request_id}/reject",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("request_id" = String, Path, description = "Join request ID")
    ),
    responses(
        (status = 200, description = "Join request rejected", body = OrgSuccessResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Not authorized to reject requests", body = OrgErrorResponse),
        (status = 404, description = "Organization or request not found", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Reject a join request.
///
/// Rejects a pending join request.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::ManageOrg`.
/// Owners and admins can reject join requests.
///
/// # Errors
/// - 401: Not authenticated
/// - 403: Not authorized (not owner/admin)
/// - 404: Organization or request not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id, %request_id))]
pub async fn reject_join_request(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, request_id)): Path<(String, String)>,
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

	match state.org_repo.get_join_request(&request_id).await {
		Ok(Some(req)) if req.is_pending() && req.org_id == org_id => {}
		Ok(Some(_)) | Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(OrgErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.join_request_not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, %request_id, "Failed to get join request");
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

	if let Err(e) = state
		.org_repo
		.reject_join_request(&request_id, &current_user.user.id)
		.await
	{
		tracing::error!(error = %e, %org_id, %request_id, "Failed to reject join request");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, %request_id, rejected_by = %current_user.user.id, "Join request rejected");

	(
		StatusCode::OK,
		Json(OrgSuccessResponse {
			message: t(locale, "server.api.org.join_request_rejected").to_string(),
		}),
	)
		.into_response()
}
