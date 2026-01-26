// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Organization CRUD HTTP handlers.
//!
//! Implements list, create, get, update, delete, and restore endpoints.

use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use loom_flags_core::{Environment, EnvironmentId};
use loom_server_api::orgs::{
	CreateOrgRequest, ListOrgsResponse, OrgErrorResponse, OrgResponse, OrgSuccessResponse,
	UpdateOrgRequest,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::{
	is_username_reserved,
	org::{OrgVisibility, Organization},
	types::{OrgId, OrgRole},
	Action,
};
use loom_server_flags::FlagsRepository;

use crate::{
	abac_middleware::{build_subject_attrs, org_resource},
	api::AppState,
	api_response::{bad_request, conflict, internal_error, not_found},
	auth_middleware::RequireAuth,
	authorize,
	i18n::{resolve_user_locale, t},
	parse_id, validate_slug_or_error,
	validation::{parse_org_id as shared_parse_org_id, validate_slug_with_error},
};

use super::common::org_visibility_to_abac;

#[utoipa::path(
    get,
    path = "/api/orgs",
    responses(
        (status = 200, description = "List of organizations", body = ListOrgsResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// List organizations the user belongs to.
///
/// Returns all organizations where the current user is a member,
/// including their personal organization.
///
/// # Authorization
/// Requires authentication. No additional ABAC checks - users can only
/// see organizations they are members of.
///
/// # Response
/// Returns a list of organizations with basic details and member counts.
///
/// # Errors
/// - 401: Not authenticated
/// - 500: Internal server error
#[tracing::instrument(skip(state))]
pub async fn list_orgs(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let user_id = current_user.user.id;
	let orgs = match state.org_repo.list_orgs_for_user(&user_id).await {
		Ok(orgs) => orgs,
		Err(e) => {
			tracing::error!(error = %e, %user_id, "Failed to list orgs for user");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(OrgErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.org.list_failed").to_string(),
				}),
			)
				.into_response();
		}
	};

	let mut org_responses = Vec::with_capacity(orgs.len());
	for org in orgs {
		let member_count = match state.org_repo.list_members(&org.id).await {
			Ok(members) => Some(members.len() as i64),
			Err(_) => None,
		};
		org_responses.push(OrgResponse::from_org(org, member_count));
	}

	(
		StatusCode::OK,
		Json(ListOrgsResponse {
			orgs: org_responses,
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs",
    request_body = CreateOrgRequest,
    responses(
        (status = 201, description = "Organization created", body = OrgResponse),
        (status = 400, description = "Invalid request", body = OrgErrorResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 409, description = "Slug already exists", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Create a new organization.
///
/// Creates a new organization with the current user as the owner.
///
/// # Authorization
/// Requires authentication. Any authenticated user can create organizations.
///
/// # Request Body
/// - `name`: Organization display name (required)
/// - `slug`: URL-safe identifier (required, 3-50 chars, lowercase alphanumeric and hyphens)
/// - `visibility`: Optional visibility setting (public, unlisted, private)
///
/// # Response
/// Returns the created organization with member count of 1.
///
/// # Errors
/// - 400: Invalid slug format or name
/// - 401: Not authenticated
/// - 409: Slug already exists
/// - 500: Internal server error
#[tracing::instrument(skip(state, payload), fields(slug = %payload.slug))]
pub async fn create_org(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Json(payload): Json<CreateOrgRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let user_id = current_user.user.id;

	validate_slug_or_error!(
		OrgErrorResponse,
		validate_slug_with_error(
			&payload.slug,
			3,
			50,
			&t(locale, "server.api.org.invalid_slug_length"),
			&t(locale, "server.api.org.invalid_slug_format")
		)
	);

	if is_username_reserved(&payload.slug) {
		return bad_request::<OrgErrorResponse>(
			"slug_reserved",
			t(locale, "server.api.org.slug_reserved"),
		)
		.into_response();
	}

	match state.org_repo.get_org_by_slug(&payload.slug).await {
		Ok(Some(_)) => {
			return conflict::<OrgErrorResponse>("slug_exists", t(locale, "server.api.org.slug_exists"))
				.into_response();
		}
		Ok(None) => {}
		Err(e) => {
			tracing::error!(error = %e, %user_id, "Failed to check slug uniqueness");
			return internal_error::<OrgErrorResponse>(t(locale, "server.api.org.create_failed"))
				.into_response();
		}
	}

	let visibility = payload
		.visibility
		.map(OrgVisibility::from)
		.unwrap_or(OrgVisibility::Public);

	let org = Organization {
		id: OrgId::generate(),
		name: payload.name,
		slug: payload.slug,
		visibility,
		is_personal: false,
		created_at: Utc::now(),
		updated_at: Utc::now(),
		deleted_at: None,
	};

	let org_id = org.id;
	if let Err(e) = state.org_repo.create_org(&org).await {
		tracing::error!(error = %e, %user_id, %org_id, "Failed to create organization");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.org.create_failed").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state
		.org_repo
		.add_member(&org.id, &current_user.user.id, OrgRole::Owner)
		.await
	{
		tracing::error!(error = %e, %user_id, %org_id, "Failed to add owner to organization");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.org.create_failed").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%user_id, %org_id, "Organization created");

	// Auto-create dev and prod environments for the new organization
	let flags_org_id = loom_flags_core::OrgId(org_id.into_inner());
	for (name, color) in Environment::default_environments() {
		let env = Environment {
			id: EnvironmentId::new(),
			org_id: flags_org_id,
			name: name.to_string(),
			color: Some(color.to_string()),
			created_at: Utc::now(),
		};
		if let Err(e) = state.flags_repo.create_environment(&env).await {
			tracing::warn!(error = %e, env_name = %name, %org_id, "Failed to create default environment");
			// Non-fatal: org creation still succeeds even if env creation fails
		} else {
			tracing::info!(env_id = %env.id, env_name = %name, %org_id, "Default environment created");
		}
	}

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::OrgCreated)
			.actor(AuditUserId::new(user_id.into_inner()))
			.resource("org", org.id.to_string())
			.details(serde_json::json!({
				"name": org.name,
				"slug": org.slug,
				"visibility": format!("{:?}", org.visibility),
			}))
			.build(),
	);

	(
		StatusCode::CREATED,
		Json(OrgResponse::from_org(org, Some(1))),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{id}",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "Organization details", body = OrgResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Access denied", body = OrgErrorResponse),
        (status = 404, description = "Organization not found", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Get organization details.
///
/// Returns detailed information about an organization if the user has access.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::Read` on the organization resource.
/// - Public orgs: Anyone can read
/// - Unlisted orgs: Members only
/// - Private orgs: Members only
///
/// # Path Parameters
/// - `id`: Organization UUID
///
/// # Response
/// Returns organization details including member count.
///
/// # Errors
/// - 400: Invalid organization ID format
/// - 401: Not authenticated
/// - 403: Access denied (ABAC check failed)
/// - 404: Organization not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn get_org(
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
			return not_found::<OrgErrorResponse>(t(locale, "server.api.org.not_found")).into_response();
		}
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to get organization");
			return internal_error::<OrgErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response();
		}
	};

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = org_resource(org.id, org_visibility_to_abac(org.visibility));

	if let Err(e) = authorize!(&subject, Action::Read, &resource) {
		return e.into_response();
	}

	let member_count = match state.org_repo.list_members(&org.id).await {
		Ok(members) => Some(members.len() as i64),
		Err(_) => None,
	};

	(
		StatusCode::OK,
		Json(OrgResponse::from_org(org, member_count)),
	)
		.into_response()
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{id}",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    request_body = UpdateOrgRequest,
    responses(
        (status = 200, description = "Organization updated", body = OrgResponse),
        (status = 400, description = "Invalid request", body = OrgErrorResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Not authorized to update", body = OrgErrorResponse),
        (status = 404, description = "Organization not found", body = OrgErrorResponse),
        (status = 409, description = "Slug already exists", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Update an organization.
///
/// Updates organization settings like name, slug, and visibility.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::Write` on the organization.
/// Only owners and admins can update organization settings.
///
/// # Path Parameters
/// - `id`: Organization UUID
///
/// # Request Body
/// All fields are optional:
/// - `name`: New display name
/// - `slug`: New URL-safe identifier (must be unique)
/// - `visibility`: New visibility setting
///
/// # Response
/// Returns the updated organization.
///
/// # Errors
/// - 400: Invalid slug format
/// - 401: Not authenticated
/// - 403: Not authorized (not owner/admin)
/// - 404: Organization not found
/// - 409: New slug already exists
/// - 500: Internal server error
#[tracing::instrument(skip(state, payload), fields(%org_id))]
pub async fn update_org(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
	Json(payload): Json<UpdateOrgRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		OrgErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let mut org = match state.org_repo.get_org_by_id(&org_id).await {
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

	if let Err(e) = authorize!(&subject, Action::Write, &resource) {
		return e.into_response();
	}

	if let Some(ref new_slug) = payload.slug {
		if new_slug != &org.slug {
			validate_slug_or_error!(
				OrgErrorResponse,
				validate_slug_with_error(
					new_slug,
					3,
					50,
					&t(locale, "server.api.org.invalid_slug_length"),
					&t(locale, "server.api.org.invalid_slug_format")
				)
			);

			if is_username_reserved(new_slug) {
				return bad_request::<OrgErrorResponse>(
					"slug_reserved",
					t(locale, "server.api.org.slug_reserved"),
				)
				.into_response();
			}

			match state.org_repo.get_org_by_slug(new_slug).await {
				Ok(Some(_)) => {
					return (
						StatusCode::CONFLICT,
						Json(OrgErrorResponse {
							error: "slug_exists".to_string(),
							message: t(locale, "server.api.org.slug_exists").to_string(),
						}),
					)
						.into_response();
				}
				Ok(None) => {}
				Err(e) => {
					tracing::error!(error = %e, %org_id, "Failed to check slug uniqueness");
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
			org.slug = new_slug.clone();
		}
	}

	if let Some(name) = payload.name {
		org.name = name;
	}

	if let Some(visibility) = payload.visibility {
		org.visibility = visibility.into();
	}

	org.updated_at = Utc::now();

	if let Err(e) = state.org_repo.update_org(&org).await {
		tracing::error!(error = %e, %org_id, "Failed to update organization");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, user_id = %current_user.user.id, "Organization updated");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::OrgUpdated)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org.id.to_string())
			.details(serde_json::json!({
				"name": org.name,
				"slug": org.slug,
				"visibility": format!("{:?}", org.visibility),
			}))
			.build(),
	);

	let member_count = match state.org_repo.list_members(&org.id).await {
		Ok(members) => Some(members.len() as i64),
		Err(_) => None,
	};

	(
		StatusCode::OK,
		Json(OrgResponse::from_org(org, member_count)),
	)
		.into_response()
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{id}",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "Organization deleted", body = OrgSuccessResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Not authorized to delete", body = OrgErrorResponse),
        (status = 404, description = "Organization not found", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Soft-delete an organization.
///
/// Marks an organization for deletion. Personal organizations cannot be deleted.
/// The deletion is soft and can be restored within a grace period.
///
/// # Authorization
/// Requires authentication. ABAC check for `Action::Delete` on the organization.
/// Only owners can delete an organization.
///
/// # Path Parameters
/// - `id`: Organization UUID
///
/// # Response
/// Returns a success message.
///
/// # Errors
/// - 400: Cannot delete personal organization
/// - 401: Not authenticated
/// - 403: Not authorized (not owner)
/// - 404: Organization not found
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn delete_org(
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

	if org.is_personal {
		return (
			StatusCode::BAD_REQUEST,
			Json(OrgErrorResponse {
				error: "cannot_delete_personal".to_string(),
				message: t(locale, "server.api.org.cannot_delete_personal").to_string(),
			}),
		)
			.into_response();
	}

	let subject = build_subject_attrs(&current_user, &state.org_repo, &state.team_repo).await;
	let resource = org_resource(org.id, org_visibility_to_abac(org.visibility));

	if let Err(e) = authorize!(&subject, Action::Delete, &resource) {
		return e.into_response();
	}

	if let Err(e) = state.org_repo.soft_delete_org(&org_id).await {
		tracing::error!(error = %e, %org_id, "Failed to delete organization");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, user_id = %current_user.user.id, "Organization deleted");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::OrgDeleted)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org.id.to_string())
			.details(serde_json::json!({
				"name": org.name,
				"slug": org.slug,
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(OrgSuccessResponse {
			message: t(locale, "server.api.org.deleted").to_string(),
		}),
	)
		.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/restore",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "Organization restored", body = OrgSuccessResponse),
        (status = 401, description = "Not authenticated", body = OrgErrorResponse),
        (status = 403, description = "Not authorized to restore", body = OrgErrorResponse),
        (status = 404, description = "Organization not found", body = OrgErrorResponse),
        (status = 410, description = "Grace period expired", body = OrgErrorResponse)
    ),
    tag = "orgs"
)]
/// Restore a soft-deleted organization.
///
/// Restores an organization that was previously deleted, if within the 90-day grace period.
///
/// # Authorization
/// Requires authentication. Only owners can restore.
///
/// # Errors
/// - 401: Not authenticated
/// - 403: Not authorized (not owner)
/// - 404: Organization not found
/// - 410: Grace period expired (>90 days since deletion)
/// - 500: Internal server error
#[tracing::instrument(skip(state), fields(%org_id))]
pub async fn restore_org(
	RequireAuth(current_user): RequireAuth,
	State(state): State<AppState>,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let org_id = parse_id!(
		OrgErrorResponse,
		shared_parse_org_id(&org_id, &t(locale, "server.api.org.invalid_id"))
	);

	let org = match state
		.org_repo
		.get_org_by_id_including_deleted(&org_id)
		.await
	{
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

	let deleted_at = match org.deleted_at {
		Some(dt) => dt,
		None => {
			return (
				StatusCode::BAD_REQUEST,
				Json(OrgErrorResponse {
					error: "not_deleted".to_string(),
					message: t(locale, "server.api.org.not_deleted").to_string(),
				}),
			)
				.into_response();
		}
	};

	let grace_period_days = 90;
	let grace_period_expired = Utc::now() > deleted_at + chrono::Duration::days(grace_period_days);
	if grace_period_expired {
		return (
			StatusCode::GONE,
			Json(OrgErrorResponse {
				error: "grace_period_expired".to_string(),
				message: t(locale, "server.api.org.grace_period_expired").to_string(),
			}),
		)
			.into_response();
	}

	let membership = match state
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
			tracing::error!(error = %e, %org_id, "Failed to get membership");
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

	if membership.role != OrgRole::Owner {
		return (
			StatusCode::FORBIDDEN,
			Json(OrgErrorResponse {
				error: "forbidden".to_string(),
				message: t(locale, "server.api.org.owner_required").to_string(),
			}),
		)
			.into_response();
	}

	if let Err(e) = state.org_repo.restore_org(&org_id).await {
		tracing::error!(error = %e, %org_id, "Failed to restore organization");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(OrgErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	tracing::info!(%org_id, restored_by = %current_user.user.id, "Organization restored");

	state.audit_service.log(
		AuditLogBuilder::new(AuditEventType::OrgRestored)
			.actor(AuditUserId::new(current_user.user.id.into_inner()))
			.resource("org", org.id.to_string())
			.details(serde_json::json!({
				"name": org.name,
				"slug": org.slug,
			}))
			.build(),
	);

	(
		StatusCode::OK,
		Json(OrgSuccessResponse {
			message: t(locale, "server.api.org.restored").to_string(),
		}),
	)
		.into_response()
}
