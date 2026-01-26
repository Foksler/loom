// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Join request handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::{org::OrgVisibility, Action, OrgRole};

use crate::{
	abac_middleware::{build_subject_attrs, org_resource},
	api::AppState,
	auth_middleware::RequireAuth,
	authorize,
	i18n::{resolve_user_locale, t},
	parse_id,
	validation::parse_org_id as shared_parse_org_id,
};

use super::common::{
	org_visibility_to_abac, InvitationErrorResponse, InvitationSuccessResponse,
	JoinRequestResponse, ListJoinRequestsResponse,
};

/// List pending join requests for an organization.
///
/// # Authorization
///
/// Requires `ManageOrg` permission on the organization.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/join-requests",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of pending join requests", body = ListJoinRequestsResponse),
        (status = 401, description = "Not authenticated", body = InvitationErrorResponse),
        (status = 403, description = "Not authorized", body = InvitationErrorResponse),
        (status = 404, description = "Organization not found", body = InvitationErrorResponse)
    ),
    tag = "invitations"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id
	)
)]
pub async fn list_join_requests(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = parse_id!(
		InvitationErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(InvitationErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				org_id = %org_id,
				"Failed to get organization"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
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
		tracing::warn!(
			actor_id = %current_user.user.id,
			org_id = %org_id,
			"Unauthorized join request list attempt"
		);
		return e.into_response();
	}

	let requests = match state.org_repo.list_pending_join_requests(&org_id).await {
		Ok(reqs) => reqs,
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				org_id = %org_id,
				"Failed to list join requests"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let mut responses = Vec::with_capacity(requests.len());
	for req in requests {
		let (display_name, email) = match state.user_repo.get_user_by_id(&req.user_id).await {
			Ok(Some(user)) => (user.display_name, user.primary_email),
			_ => ("Unknown".to_string(), None),
		};

		responses.push(JoinRequestResponse {
			id: req.user_id.to_string(),
			user_id: req.user_id.to_string(),
			display_name,
			email,
			created_at: req.created_at,
		});
	}

	tracing::info!(
		actor_id = %current_user.user.id,
		org_id = %org_id,
		request_count = responses.len(),
		"Listed join requests"
	);

	(
		StatusCode::OK,
		Json(ListJoinRequestsResponse {
			requests: responses,
		}),
	)
		.into_response()
}

/// Create a join request for an organization.
///
/// # Authorization
///
/// Requires authentication. Any authenticated user can request to join
/// public or unlisted organizations.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/join-requests",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 201, description = "Join request created", body = InvitationSuccessResponse),
        (status = 401, description = "Not authenticated", body = InvitationErrorResponse),
        (status = 404, description = "Organization not found", body = InvitationErrorResponse),
        (status = 409, description = "Already a member or pending request", body = InvitationErrorResponse)
    ),
    tag = "invitations"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id
	)
)]
pub async fn create_join_request(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = parse_id!(
		InvitationErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(InvitationErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				org_id = %org_id,
				"Failed to get organization"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if org.visibility == OrgVisibility::Private {
		tracing::warn!(
			actor_id = %current_user.user.id,
			org_id = %org_id,
			"Join request denied for private organization"
		);
		return (
			StatusCode::FORBIDDEN,
			Json(InvitationErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.join_request.private_org").to_string(),
			}),
		)
			.into_response();
	}

	if let Ok(Some(_)) = state
		.org_repo
		.get_membership(&org_id, &current_user.user.id)
		.await
	{
		return (
			StatusCode::CONFLICT,
			Json(InvitationErrorResponse {
				error: "already_member".to_string(),
				message: t(locale, "server.api.join_request.already_member").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state
		.org_repo
		.create_join_request(&org_id, &current_user.user.id)
		.await
	{
		tracing::error!(
			error = %e,
			actor_id = %current_user.user.id,
			org_id = %org_id,
			"Failed to create join request"
		);
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(InvitationErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(
		actor_id = %current_user.user.id,
		org_id = %org_id,
		"Join request created"
	);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::MemberAdded)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org_id.to_string())
			.action("join_request_created")
			.details(serde_json::json!({
				"org_slug": org.slug,
			}))
			.build(),
	);

	(
		StatusCode::CREATED,
		Json(InvitationSuccessResponse {
			message: t(locale, "server.api.join_request.created").to_string(),
		}),
	)
		.into_response()
}

/// Approve a join request.
///
/// # Authorization
///
/// Requires `ManageOrg` permission on the organization.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/join-requests/{request_id}/approve",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("request_id" = String, Path, description = "Join request ID")
    ),
    responses(
        (status = 200, description = "Join request approved", body = InvitationSuccessResponse),
        (status = 401, description = "Not authenticated", body = InvitationErrorResponse),
        (status = 403, description = "Not authorized", body = InvitationErrorResponse),
        (status = 404, description = "Request not found", body = InvitationErrorResponse)
    ),
    tag = "invitations"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id,
		request_id = %request_id
	)
)]
pub async fn approve_join_request(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, request_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = parse_id!(
		InvitationErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(InvitationErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				org_id = %org_id,
				"Failed to get organization"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
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
		tracing::warn!(
			actor_id = %current_user.user.id,
			org_id = %org_id,
			request_id = %request_id,
			"Unauthorized join request approval attempt"
		);
		return e.into_response();
	}

	let join_request = match state.org_repo.get_join_request(&request_id).await {
		Ok(Some(req)) => req,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(InvitationErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.join_request.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				request_id = %request_id,
				"Failed to get join request"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if join_request.org_id != org_id {
		return (
			StatusCode::NOT_FOUND,
			Json(InvitationErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.join_request.not_found").to_string(),
			}),
		)
			.into_response();
	}

	if join_request.handled_at.is_some() {
		return (
			StatusCode::BAD_REQUEST,
			Json(InvitationErrorResponse {
				error: "already_handled".to_string(),
				message: t(locale, "server.api.join_request.already_pending").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state
		.org_repo
		.add_member(&org_id, &join_request.user_id, OrgRole::Member)
		.await
	{
		tracing::error!(
			error = %e,
			actor_id = %current_user.user.id,
			target_id = %join_request.user_id,
			org_id = %org_id,
			"Failed to add member"
		);
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(InvitationErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state
		.org_repo
		.approve_join_request(&request_id, &current_user.user.id)
		.await
	{
		tracing::error!(
			error = %e,
			actor_id = %current_user.user.id,
			request_id = %request_id,
			"Failed to mark join request as approved"
		);
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::MemberAdded)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org_id.to_string())
			.details(serde_json::json!({
				"action": "join_request_approved",
				"request_id": &request_id,
				"target_user_id": join_request.user_id.to_string(),
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		target_id = %join_request.user_id,
		org_id = %org_id,
		request_id = %request_id,
		"Join request approved"
	);

	(
		StatusCode::OK,
		Json(InvitationSuccessResponse {
			message: t(locale, "server.api.join_request.approved").to_string(),
		}),
	)
		.into_response()
}

/// Reject a join request.
///
/// # Authorization
///
/// Requires `ManageOrg` permission on the organization.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/join-requests/{request_id}/reject",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("request_id" = String, Path, description = "Join request ID")
    ),
    responses(
        (status = 200, description = "Join request rejected", body = InvitationSuccessResponse),
        (status = 401, description = "Not authenticated", body = InvitationErrorResponse),
        (status = 403, description = "Not authorized", body = InvitationErrorResponse),
        (status = 404, description = "Request not found", body = InvitationErrorResponse)
    ),
    tag = "invitations"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id,
		request_id = %request_id
	)
)]
pub async fn reject_join_request(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, request_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let org_id = parse_id!(
		InvitationErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let org = match state.org_repo.get_org_by_id(&org_id).await {
		Ok(Some(org)) => org,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(InvitationErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.org.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				org_id = %org_id,
				"Failed to get organization"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
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
		tracing::warn!(
			actor_id = %current_user.user.id,
			org_id = %org_id,
			request_id = %request_id,
			"Unauthorized join request rejection attempt"
		);
		return e.into_response();
	}

	let join_request = match state.org_repo.get_join_request(&request_id).await {
		Ok(Some(req)) => req,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(InvitationErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.join_request.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				request_id = %request_id,
				"Failed to get join request"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	if join_request.org_id != org_id {
		return (
			StatusCode::NOT_FOUND,
			Json(InvitationErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.join_request.not_found").to_string(),
			}),
		)
			.into_response();
	}

	if join_request.handled_at.is_some() {
		return (
			StatusCode::BAD_REQUEST,
			Json(InvitationErrorResponse {
				error: "already_handled".to_string(),
				message: t(locale, "server.api.join_request.already_pending").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state
		.org_repo
		.reject_join_request(&request_id, &current_user.user.id)
		.await
	{
		tracing::error!(
			error = %e,
			actor_id = %current_user.user.id,
			request_id = %request_id,
			"Failed to reject join request"
		);
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(InvitationErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(
		actor_id = %current_user.user.id,
		target_id = %join_request.user_id,
		org_id = %org_id,
		request_id = %request_id,
		"Join request rejected"
	);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::MemberRemoved)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org_id.to_string())
			.action("join_request_rejected")
			.details(serde_json::json!({
				"request_id": request_id,
				"target_user_id": join_request.user_id.to_string(),
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(InvitationSuccessResponse {
			message: t(locale, "server.api.join_request.rejected").to_string(),
		}),
	)
		.into_response()
}
