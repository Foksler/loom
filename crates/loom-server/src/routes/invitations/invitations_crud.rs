// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Invitation CRUD handlers.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::{hash_token, Action, OrgRole};
use loom_server_email::EmailRequest;

use crate::{
	abac_middleware::{build_subject_attrs, org_resource},
	api::AppState,
	auth_middleware::RequireAuth,
	authorize,
	i18n::{resolve_user_locale, t},
	impl_api_error_response, parse_id, parse_role,
	validation::{parse_org_id as shared_parse_org_id, parse_org_role},
};

use super::common::{
	generate_invitation_token, org_visibility_to_abac, AcceptInvitationRequest,
	AcceptInvitationResponse, CreateInvitationRequest, CreateInvitationResponse,
	InvitationErrorResponse, InvitationResponse, InvitationSuccessResponse,
	ListInvitationsResponse,
};

impl_api_error_response!(InvitationErrorResponse);

/// List pending invitations for an organization.
///
/// # Authorization
///
/// Requires `ManageOrg` permission on the organization.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/invitations",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "List of pending invitations", body = ListInvitationsResponse),
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
pub async fn list_invitations(
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
					message: t(locale, "server.api.invitation.list_failed").to_string(),
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
			"Unauthorized invitation list attempt"
		);
		return e.into_response();
	}

	let invitations = match state.org_repo.list_pending_invitations(&org_id).await {
		Ok(invs) => invs,
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				org_id = %org_id,
				"Failed to list invitations"
			);
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.invitation.list_failed").to_string(),
				}),
			)
				.into_response();
		}
	};

	let mut responses = Vec::with_capacity(invitations.len());
	for inv in invitations {
		let invited_by_name = match state.user_repo.get_user_by_id(&inv.invited_by).await {
			Ok(Some(user)) => user.display_name,
			_ => "Unknown".to_string(),
		};
		let is_expired = inv.is_expired();

		responses.push(InvitationResponse {
			id: inv.id.to_string(),
			org_id: inv.org_id.to_string(),
			org_name: org.name.clone(),
			email: inv.email,
			role: inv.role.to_string(),
			invited_by: inv.invited_by.to_string(),
			invited_by_name,
			created_at: inv.created_at,
			expires_at: inv.expires_at,
			is_expired,
		});
	}

	tracing::info!(
		actor_id = %current_user.user.id,
		org_id = %org_id,
		invitation_count = responses.len(),
		"Listed invitations"
	);

	(
		StatusCode::OK,
		Json(ListInvitationsResponse {
			invitations: responses,
		}),
	)
		.into_response()
}

/// Create an invitation to join an organization.
///
/// # Authorization
///
/// Requires `ManageOrg` permission on the organization.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/invitations",
    params(
        ("org_id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateInvitationRequest,
    responses(
        (status = 201, description = "Invitation created", body = CreateInvitationResponse),
        (status = 400, description = "Invalid request", body = InvitationErrorResponse),
        (status = 401, description = "Not authenticated", body = InvitationErrorResponse),
        (status = 403, description = "Not authorized", body = InvitationErrorResponse),
        (status = 404, description = "Organization not found", body = InvitationErrorResponse)
    ),
    tag = "invitations"
)]
#[tracing::instrument(
	skip(state, payload),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id
	)
)]
pub async fn create_invitation(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<CreateInvitationRequest>,
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
			"Unauthorized invitation creation attempt"
		);
		return e.into_response();
	}

	let role = match payload.role.as_deref() {
		Some(r) => parse_role!(
			InvitationErrorResponse,
			parse_org_role(r, &t(locale, "server.api.org.invalid_role"))
		),
		None => OrgRole::Member,
	};

	let token = generate_invitation_token();
	let token_hash = hash_token(&token);

	let invitation_id = match state
		.org_repo
		.create_invitation(
			&org_id,
			&payload.email,
			role,
			&current_user.user.id,
			&token_hash,
		)
		.await
	{
		Ok(id) => id,
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				org_id = %org_id,
				"Failed to create invitation"
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

	if let Some(email_service) = &state.email_service {
		let request = EmailRequest::OrgInvitation {
			org_name: org.name.clone(),
			inviter_name: current_user.user.display_name.clone(),
			token: token.clone(),
		};
		if let Err(e) = email_service
			.send(&payload.email, request, current_user.user.locale.as_deref())
			.await
		{
			tracing::warn!(error = %e, "Failed to send invitation email");
		}
	}

	let expires_at =
		Utc::now() + chrono::Duration::days(loom_server_auth::OrgInvitation::EXPIRY_DAYS);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::MemberAdded)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("invitation", invitation_id.clone())
			.details(serde_json::json!({
				"action": "invitation_created",
				"org_id": org_id.to_string(),
				"email": &payload.email,
				"role": role.to_string(),
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		org_id = %org_id,
		invitation_id = %invitation_id,
		role = %role,
		"Invitation created"
	);

	(
		StatusCode::CREATED,
		Json(CreateInvitationResponse {
			id: invitation_id,
			email: payload.email,
			role: role.to_string(),
			expires_at,
		}),
	)
		.into_response()
}

/// Cancel a pending invitation.
///
/// # Authorization
///
/// Requires `ManageOrg` permission on the organization.
#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/invitations/{id}",
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("id" = String, Path, description = "Invitation ID")
    ),
    responses(
        (status = 200, description = "Invitation cancelled", body = InvitationSuccessResponse),
        (status = 401, description = "Not authenticated", body = InvitationErrorResponse),
        (status = 403, description = "Not authorized", body = InvitationErrorResponse),
        (status = 404, description = "Invitation not found", body = InvitationErrorResponse)
    ),
    tag = "invitations"
)]
#[tracing::instrument(
	skip(state),
	fields(
		actor_id = %current_user.user.id,
		org_id = %org_id,
		invitation_id = %invitation_id
	)
)]
pub async fn cancel_invitation(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path((org_id, invitation_id)): Path<(String, String)>,
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
			invitation_id = %invitation_id,
			"Unauthorized invitation cancellation attempt"
		);
		return e.into_response();
	}

	let invitation = match state.org_repo.get_invitation_by_id(&invitation_id).await {
		Ok(Some(inv)) => inv,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(InvitationErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.invitation.not_found").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(
				error = %e,
				actor_id = %current_user.user.id,
				invitation_id = %invitation_id,
				"Failed to get invitation"
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

	if invitation.org_id != org_id {
		return (
			StatusCode::NOT_FOUND,
			Json(InvitationErrorResponse {
				error: "not_found".to_string(),
				message: t(locale, "server.api.invitation.not_found").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state.org_repo.delete_invitation(&invitation_id).await {
		tracing::error!(
			error = %e,
			actor_id = %current_user.user.id,
			invitation_id = %invitation_id,
			"Failed to delete invitation"
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
		invitation_id = %invitation_id,
		"Invitation cancelled"
	);

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::MemberRemoved)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("invitation", invitation_id.to_string())
			.action("invitation_cancelled")
			.details(serde_json::json!({
				"org_id": org_id.to_string(),
				"email": invitation.email,
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(InvitationSuccessResponse {
			message: t(locale, "server.api.invitation.cancelled").to_string(),
		}),
	)
		.into_response()
}

/// Accept an invitation to join an organization.
///
/// # Authorization
///
/// Requires authentication. Any authenticated user can accept an invitation
/// sent to their email.
#[utoipa::path(
    post,
    path = "/api/invitations/accept",
    request_body = AcceptInvitationRequest,
    responses(
        (status = 200, description = "Invitation accepted", body = AcceptInvitationResponse),
        (status = 400, description = "Invalid invitation", body = InvitationErrorResponse),
        (status = 401, description = "Not authenticated", body = InvitationErrorResponse),
        (status = 404, description = "Invitation not found or expired", body = InvitationErrorResponse),
        (status = 409, description = "Already a member", body = InvitationErrorResponse)
    ),
    tag = "invitations"
)]
#[tracing::instrument(
	skip(state, payload),
	fields(actor_id = %current_user.user.id)
)]
pub async fn accept_invitation(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Json(payload): Json<AcceptInvitationRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let token_hash = hash_token(&payload.token);

	let invitation = match state
		.org_repo
		.get_invitation_by_token_hash(&token_hash)
		.await
	{
		Ok(Some(inv)) => inv,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(InvitationErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.invitation.invalid_token").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, actor_id = %current_user.user.id, "Failed to get invitation");
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

	if !invitation.is_valid() {
		return (
			StatusCode::BAD_REQUEST,
			Json(InvitationErrorResponse {
				error: "invalid_invitation".to_string(),
				message: if invitation.is_expired() {
					t(locale, "server.api.invitation.expired").to_string()
				} else {
					t(locale, "server.api.invitation.already_accepted").to_string()
				},
			}),
		)
			.into_response();
	}

	if let Ok(Some(_)) = state
		.org_repo
		.get_membership(&invitation.org_id, &current_user.user.id)
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

	let org = match state.org_repo.get_org_by_id(&invitation.org_id).await {
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
				org_id = %invitation.org_id,
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

	if let Err(e) = state
		.org_repo
		.add_member(&invitation.org_id, &current_user.user.id, invitation.role)
		.await
	{
		tracing::error!(
			error = %e,
			actor_id = %current_user.user.id,
			org_id = %invitation.org_id,
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
		.accept_invitation(&invitation.id.to_string())
		.await
	{
		tracing::error!(
			error = %e,
			actor_id = %current_user.user.id,
			invitation_id = %invitation.id,
			"Failed to mark invitation as accepted"
		);
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::MemberAdded)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org.id.to_string())
			.details(serde_json::json!({
				"action": "invitation_accepted",
				"invitation_id": invitation.id.to_string(),
				"role": invitation.role.to_string(),
			}))
			.build(),
	);

	tracing::info!(
		actor_id = %current_user.user.id,
		org_id = %org.id,
		invitation_id = %invitation.id,
		role = %invitation.role,
		"Invitation accepted"
	);

	(
		StatusCode::OK,
		Json(AcceptInvitationResponse {
			org_id: org.id.to_string(),
			org_name: org.name,
			role: invitation.role.to_string(),
		}),
	)
		.into_response()
}

/// Get invitation details by token.
///
/// # Authorization
///
/// Public endpoint. Anyone with the token can view invitation details.
#[utoipa::path(
    get,
    path = "/api/invitations/{token}",
    params(
        ("token" = String, Path, description = "Invitation token")
    ),
    responses(
        (status = 200, description = "Invitation details", body = InvitationResponse),
        (status = 404, description = "Invitation not found or expired", body = InvitationErrorResponse)
    ),
    tag = "invitations"
)]
#[tracing::instrument(skip(state, token))]
pub async fn get_invitation(
	State(state): State<AppState>,
	Path(token): Path<String>,
) -> impl IntoResponse {
	let locale = &state.default_locale;

	let token_hash = hash_token(&token);

	let invitation = match state
		.org_repo
		.get_invitation_by_token_hash(&token_hash)
		.await
	{
		Ok(Some(inv)) => inv,
		Ok(None) => {
			return (
				StatusCode::NOT_FOUND,
				Json(InvitationErrorResponse {
					error: "not_found".to_string(),
					message: t(locale, "server.api.invitation.invalid_token").to_string(),
				}),
			)
				.into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, "Failed to get invitation");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.invitation.list_failed").to_string(),
				}),
			)
				.into_response();
		}
	};

	let org = match state.org_repo.get_org_by_id(&invitation.org_id).await {
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
			tracing::error!(error = %e, org_id = %invitation.org_id, "Failed to get organization");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(InvitationErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.invitation.list_failed").to_string(),
				}),
			)
				.into_response();
		}
	};

	let invited_by_name = match state.user_repo.get_user_by_id(&invitation.invited_by).await {
		Ok(Some(user)) => user.display_name,
		_ => "Unknown".to_string(),
	};
	let is_expired = invitation.is_expired();

	(
		StatusCode::OK,
		Json(InvitationResponse {
			id: invitation.id.to_string(),
			org_id: invitation.org_id.to_string(),
			org_name: org.name,
			email: invitation.email,
			role: invitation.role.to_string(),
			invited_by: invitation.invited_by.to_string(),
			invited_by_name,
			created_at: invitation.created_at,
			expires_at: invitation.expires_at,
			is_expired,
		}),
	)
		.into_response()
}
