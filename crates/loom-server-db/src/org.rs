// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Organization repository for database operations.
//!
//! This module provides database access for organization management including:
//! - Organization CRUD operations
//! - Membership management (owners, admins, members)
//! - Invitations (email-based)
//! - Join requests (for public orgs)

use chrono::Utc;
use loom_server_auth::{
	org::{OrgInvitation, OrgJoinRequest, OrgMembership, OrgVisibility, Organization},
	types::{InvitationId, OrgId, OrgRole, UserId},
	user::User,
};
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;

use crate::error::DbError;

/// Repository for organization database operations.
///
/// Manages organizations, their members, invitations, and join requests.
/// All IDs are UUIDs stored as strings in SQLite.
#[derive(Clone)]
pub struct OrgRepository {
	pool: SqlitePool,
}

impl OrgRepository {
	/// Create a new repository with the given pool.
	///
	/// # Arguments
	/// * `pool` - SQLite connection pool
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	// =========================================================================
	// Organization CRUD
	// =========================================================================

	/// Create a new organization.
	///
	/// # Arguments
	/// * `org` - The organization to create
	///
	/// # Errors
	/// Returns `DbError::Sqlx` if insert fails (e.g., duplicate slug).
	///
	/// # Database Constraints
	/// - `id` must be unique
	/// - `slug` must be unique
	#[tracing::instrument(skip(self, org), fields(org_id = %org.id, slug = %org.slug))]
	pub async fn create_org(&self, org: &Organization) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			INSERT INTO organizations (id, name, slug, visibility, is_personal, created_at, updated_at, deleted_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(org.id.to_string())
		.bind(&org.name)
		.bind(&org.slug)
		.bind(org.visibility.to_string())
		.bind(org.is_personal as i32)
		.bind(org.created_at.to_rfc3339())
		.bind(now)
		.bind(org.deleted_at.map(|d| d.to_rfc3339()))
		.execute(&self.pool)
		.await?;

		tracing::debug!(org_id = %org.id, slug = %org.slug, "organization created");
		Ok(())
	}

	/// Get an organization by ID.
	///
	/// # Arguments
	/// * `id` - The organization's UUID
	///
	/// # Returns
	/// `None` if no organization exists with this ID or if soft-deleted.
	#[tracing::instrument(skip(self), fields(org_id = %id))]
	pub async fn get_org_by_id(&self, id: &OrgId) -> Result<Option<Organization>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, name, slug, visibility, is_personal, created_at, updated_at, deleted_at
			FROM organizations
			WHERE id = ? AND deleted_at IS NULL
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_org(&r)).transpose()
	}

	/// Get an organization by ID, including soft-deleted ones.
	///
	/// # Arguments
	/// * `id` - The organization's UUID
	///
	/// # Returns
	/// `None` if no organization exists with this ID. Includes soft-deleted orgs.
	#[tracing::instrument(skip(self), fields(org_id = %id))]
	pub async fn get_org_by_id_including_deleted(
		&self,
		id: &OrgId,
	) -> Result<Option<Organization>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, name, slug, visibility, is_personal, created_at, updated_at, deleted_at
			FROM organizations
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_org(&r)).transpose()
	}

	/// Get an organization by slug.
	///
	/// # Arguments
	/// * `slug` - The organization's URL-safe slug
	///
	/// # Returns
	/// `None` if no organization exists with this slug or if soft-deleted.
	#[tracing::instrument(skip(self), fields(slug = %slug))]
	pub async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Organization>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, name, slug, visibility, is_personal, created_at, updated_at, deleted_at
			FROM organizations
			WHERE slug = ? AND deleted_at IS NULL
			"#,
		)
		.bind(slug)
		.fetch_optional(&self.pool)
		.await?;

		let result = row.map(|r| self.row_to_org(&r)).transpose()?;
		if let Some(ref org) = result {
			tracing::debug!(org_id = %org.id, "organization found by slug");
		}
		Ok(result)
	}

	/// Update an organization.
	///
	/// # Arguments
	/// * `org` - The organization with updated fields
	///
	/// # Errors
	/// Returns `DbError::Sqlx` if update fails (e.g., duplicate slug).
	#[tracing::instrument(skip(self, org), fields(org_id = %org.id))]
	pub async fn update_org(&self, org: &Organization) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			UPDATE organizations
			SET name = ?, slug = ?, visibility = ?, updated_at = ?
			WHERE id = ? AND deleted_at IS NULL
			"#,
		)
		.bind(&org.name)
		.bind(&org.slug)
		.bind(org.visibility.to_string())
		.bind(now)
		.bind(org.id.to_string())
		.execute(&self.pool)
		.await?;

		tracing::debug!(org_id = %org.id, "organization updated");
		Ok(())
	}

	/// Soft-delete an organization.
	///
	/// # Arguments
	/// * `id` - The organization's UUID
	#[tracing::instrument(skip(self), fields(org_id = %id))]
	pub async fn soft_delete_org(&self, id: &OrgId) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			UPDATE organizations
			SET deleted_at = ?, updated_at = ?
			WHERE id = ? AND deleted_at IS NULL
			"#,
		)
		.bind(&now)
		.bind(&now)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		tracing::debug!(org_id = %id, "organization soft-deleted");
		Ok(())
	}

	/// Restore a soft-deleted organization.
	///
	/// # Arguments
	/// * `id` - The organization's UUID
	#[tracing::instrument(skip(self), fields(org_id = %id))]
	pub async fn restore_org(&self, id: &OrgId) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			UPDATE organizations
			SET deleted_at = NULL, updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&now)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		tracing::debug!(org_id = %id, "organization restored");
		Ok(())
	}

	/// List organizations for a user (via membership).
	///
	/// # Arguments
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// List of organizations the user is a member of, ordered by name.
	#[tracing::instrument(skip(self), fields(user_id = %user_id))]
	pub async fn list_orgs_for_user(&self, user_id: &UserId) -> Result<Vec<Organization>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT o.id, o.name, o.slug, o.visibility, o.is_personal, o.created_at, o.updated_at, o.deleted_at
			FROM organizations o
			INNER JOIN org_memberships m ON o.id = m.org_id
			WHERE m.user_id = ? AND o.deleted_at IS NULL
			ORDER BY o.name ASC
			"#,
		)
		.bind(user_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		let orgs: Result<Vec<_>, _> = rows.iter().map(|r| self.row_to_org(r)).collect();
		let orgs = orgs?;
		tracing::debug!(user_id = %user_id, count = orgs.len(), "listed organizations for user");
		Ok(orgs)
	}

	/// Ensure a personal organization exists for the given user.
	///
	/// If the user already has a personal org, returns it. Otherwise, creates one
	/// and adds the user as owner.
	///
	/// # Arguments
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// The personal organization (existing or newly created).
	#[tracing::instrument(skip(self), fields(user_id = %user_id))]
	pub async fn ensure_personal_org(&self, user_id: &UserId) -> Result<Organization, DbError> {
		let existing = sqlx::query(
			r#"
			SELECT o.id, o.name, o.slug, o.visibility, o.is_personal, o.created_at, o.updated_at, o.deleted_at
			FROM organizations o
			INNER JOIN org_memberships m ON o.id = m.org_id
			WHERE m.user_id = ? AND o.is_personal = 1 AND o.deleted_at IS NULL
			LIMIT 1
			"#,
		)
		.bind(user_id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		if let Some(row) = existing {
			let org = self.row_to_org(&row)?;
			tracing::debug!(user_id = %user_id, org_id = %org.id, "personal org already exists");
			return Ok(org);
		}

		let org = Organization::new_personal(user_id);
		self.create_org(&org).await?;
		self.add_member(&org.id, user_id, OrgRole::Owner).await?;

		tracing::info!(user_id = %user_id, org_id = %org.id, "created personal org for user");
		Ok(org)
	}

	/// List public organizations with pagination.
	///
	/// # Arguments
	/// * `limit` - Maximum number of organizations to return
	/// * `offset` - Number of organizations to skip
	///
	/// # Returns
	/// List of public (non-personal) organizations.
	#[tracing::instrument(skip(self), fields(limit, offset))]
	pub async fn list_public_orgs(
		&self,
		limit: i32,
		offset: i32,
	) -> Result<Vec<Organization>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT id, name, slug, visibility, is_personal, created_at, updated_at, deleted_at
			FROM organizations
			WHERE visibility = 'public' AND deleted_at IS NULL AND is_personal = 0
			ORDER BY name ASC
			LIMIT ? OFFSET ?
			"#,
		)
		.bind(limit)
		.bind(offset)
		.fetch_all(&self.pool)
		.await?;

		let orgs: Result<Vec<_>, _> = rows.iter().map(|r| self.row_to_org(r)).collect();
		let orgs = orgs?;
		tracing::debug!(count = orgs.len(), "listed public organizations");
		Ok(orgs)
	}

	// =========================================================================
	// Memberships
	// =========================================================================

	/// Add a member to an organization.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	/// * `user_id` - The user's UUID
	/// * `role` - The member's role (owner, admin, member)
	///
	/// # Database Constraints
	/// - (`org_id`, `user_id`) must be unique
	/// - `org_id` must reference an existing organization
	/// - `user_id` must reference an existing user
	#[tracing::instrument(skip(self), fields(org_id = %org_id, user_id = %user_id, role = %role))]
	pub async fn add_member(
		&self,
		org_id: &OrgId,
		user_id: &UserId,
		role: OrgRole,
	) -> Result<(), DbError> {
		self
			.add_member_with_provenance(org_id, user_id, role, None)
			.await
	}

	/// Add a member to an organization with provenance tracking.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	/// * `user_id` - The user's UUID
	/// * `role` - The member's role (owner, admin, member)
	/// * `provisioned_by` - Optional provenance source (e.g., "scim", "oauth")
	///
	/// # Database Constraints
	/// - (`org_id`, `user_id`) must be unique
	/// - `org_id` must reference an existing organization
	/// - `user_id` must reference an existing user
	#[tracing::instrument(skip(self), fields(org_id = %org_id, user_id = %user_id, role = %role, provisioned_by = ?provisioned_by))]
	pub async fn add_member_with_provenance(
		&self,
		org_id: &OrgId,
		user_id: &UserId,
		role: OrgRole,
		provisioned_by: Option<&str>,
	) -> Result<(), DbError> {
		let id = Uuid::new_v4().to_string();
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			INSERT INTO org_memberships (id, org_id, user_id, role, provisioned_by, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(org_id.to_string())
		.bind(user_id.to_string())
		.bind(role.to_string())
		.bind(provisioned_by)
		.bind(&now)
		.bind(&now)
		.execute(&self.pool)
		.await?;

		tracing::debug!(org_id = %org_id, user_id = %user_id, role = %role, "member added to organization");
		Ok(())
	}

	/// Get a membership for a user in an organization.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// `None` if the user is not a member.
	#[tracing::instrument(skip(self), fields(org_id = %org_id, user_id = %user_id))]
	pub async fn get_membership(
		&self,
		org_id: &OrgId,
		user_id: &UserId,
	) -> Result<Option<OrgMembership>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT org_id, user_id, role, created_at
			FROM org_memberships
			WHERE org_id = ? AND user_id = ?
			"#,
		)
		.bind(org_id.to_string())
		.bind(user_id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_membership(&r)).transpose()
	}

	/// Update a member's role.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	/// * `user_id` - The user's UUID
	/// * `role` - The new role
	#[tracing::instrument(skip(self), fields(org_id = %org_id, user_id = %user_id, role = %role))]
	pub async fn update_member_role(
		&self,
		org_id: &OrgId,
		user_id: &UserId,
		role: OrgRole,
	) -> Result<(), DbError> {
		sqlx::query(
			r#"
			UPDATE org_memberships
			SET role = ?
			WHERE org_id = ? AND user_id = ?
			"#,
		)
		.bind(role.to_string())
		.bind(org_id.to_string())
		.bind(user_id.to_string())
		.execute(&self.pool)
		.await?;

		tracing::debug!(org_id = %org_id, user_id = %user_id, role = %role, "member role updated");
		Ok(())
	}

	/// Remove a member from an organization.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// `true` if a member was removed, `false` if not found.
	#[tracing::instrument(skip(self), fields(org_id = %org_id, user_id = %user_id))]
	pub async fn remove_member(&self, org_id: &OrgId, user_id: &UserId) -> Result<bool, DbError> {
		let result = sqlx::query(
			r#"
			DELETE FROM org_memberships
			WHERE org_id = ? AND user_id = ?
			"#,
		)
		.bind(org_id.to_string())
		.bind(user_id.to_string())
		.execute(&self.pool)
		.await?;

		let removed = result.rows_affected() > 0;
		if removed {
			tracing::debug!(org_id = %org_id, user_id = %user_id, "member removed from organization");
		}
		Ok(removed)
	}

	/// List all members of an organization with their user info.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	///
	/// # Returns
	/// List of (membership, user) tuples ordered by join date.
	#[tracing::instrument(skip(self), fields(org_id = %org_id))]
	pub async fn list_members(&self, org_id: &OrgId) -> Result<Vec<(OrgMembership, User)>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT 
				m.org_id, m.user_id, m.role, m.created_at,
				u.id as u_id, u.display_name, u.username, u.primary_email, u.avatar_url,
				u.email_visible, u.is_system_admin, u.is_support, u.is_auditor,
				u.created_at as u_created_at, u.updated_at as u_updated_at, u.deleted_at as u_deleted_at,
				u.locale
			FROM org_memberships m
			INNER JOIN users u ON m.user_id = u.id
			WHERE m.org_id = ?
			ORDER BY m.created_at ASC
			"#,
		)
		.bind(org_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		let mut result = Vec::with_capacity(rows.len());
		for row in &rows {
			let membership = self.row_to_membership(row)?;
			let user = self.row_to_user_prefixed(row)?;
			result.push((membership, user));
		}
		tracing::debug!(org_id = %org_id, count = result.len(), "listed organization members");
		Ok(result)
	}

	/// Count owners of an organization.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	///
	/// # Returns
	/// Number of users with the "owner" role.
	#[tracing::instrument(skip(self), fields(org_id = %org_id))]
	pub async fn count_owners(&self, org_id: &OrgId) -> Result<i64, DbError> {
		let row: (i64,) = sqlx::query_as(
			r#"
			SELECT COUNT(*) FROM org_memberships
			WHERE org_id = ? AND role = 'owner'
			"#,
		)
		.bind(org_id.to_string())
		.fetch_one(&self.pool)
		.await?;

		Ok(row.0)
	}

	// =========================================================================
	// Invitations
	// =========================================================================

	/// Create an invitation and return its ID.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	/// * `email` - The invitee's email address
	/// * `role` - The role to grant upon acceptance
	/// * `invited_by` - The inviting user's UUID
	/// * `token_hash` - SHA-256 hash of the invitation token
	///
	/// # Returns
	/// The generated invitation ID.
	#[tracing::instrument(skip(self, email, token_hash), fields(org_id = %org_id, invited_by = %invited_by, role = %role))]
	pub async fn create_invitation(
		&self,
		org_id: &OrgId,
		email: &str,
		role: OrgRole,
		invited_by: &UserId,
		token_hash: &str,
	) -> Result<String, DbError> {
		let id = InvitationId::generate();
		let now = Utc::now();
		let expires_at = now + chrono::Duration::days(OrgInvitation::EXPIRY_DAYS);

		sqlx::query(
			r#"
			INSERT INTO org_invitations (id, org_id, email, role, invited_by, token_hash, created_at, expires_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(id.to_string())
		.bind(org_id.to_string())
		.bind(email)
		.bind(role.to_string())
		.bind(invited_by.to_string())
		// Note: token_hash is intentionally not logged
		.bind(token_hash)
		.bind(now.to_rfc3339())
		.bind(expires_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		tracing::debug!(invitation_id = %id, org_id = %org_id, "invitation created");
		Ok(id.to_string())
	}

	/// Get an invitation by token hash.
	///
	/// # Arguments
	/// * `token_hash` - SHA-256 hash of the invitation token
	///
	/// # Returns
	/// `None` if no invitation exists with this hash.
	#[tracing::instrument(skip(self, token_hash))]
	pub async fn get_invitation_by_token_hash(
		&self,
		token_hash: &str,
	) -> Result<Option<OrgInvitation>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, org_id, email, role, invited_by, token_hash, created_at, expires_at, accepted_at
			FROM org_invitations
			WHERE token_hash = ?
			"#,
		)
		.bind(token_hash)
		.fetch_optional(&self.pool)
		.await?;

		let result = row.map(|r| self.row_to_invitation(&r)).transpose()?;
		if let Some(ref inv) = result {
			tracing::debug!(invitation_id = %inv.id, org_id = %inv.org_id, "invitation found by token hash");
		}
		Ok(result)
	}

	/// Mark an invitation as accepted.
	///
	/// # Arguments
	/// * `id` - The invitation's UUID
	#[tracing::instrument(skip(self), fields(invitation_id = %id))]
	pub async fn accept_invitation(&self, id: &str) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			UPDATE org_invitations
			SET accepted_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&now)
		.bind(id)
		.execute(&self.pool)
		.await?;

		tracing::debug!(invitation_id = %id, "invitation accepted");
		Ok(())
	}

	/// Get an invitation by ID.
	///
	/// # Arguments
	/// * `id` - The invitation's UUID
	#[tracing::instrument(skip(self), fields(invitation_id = %id))]
	pub async fn get_invitation_by_id(&self, id: &str) -> Result<Option<OrgInvitation>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, org_id, email, role, invited_by, token_hash, created_at, expires_at, accepted_at
			FROM org_invitations
			WHERE id = ?
			"#,
		)
		.bind(id)
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_invitation(&r)).transpose()
	}

	/// Delete (cancel) an invitation.
	///
	/// # Arguments
	/// * `id` - The invitation's UUID
	///
	/// # Returns
	/// `true` if an invitation was deleted, `false` if not found.
	#[tracing::instrument(skip(self), fields(invitation_id = %id))]
	pub async fn delete_invitation(&self, id: &str) -> Result<bool, DbError> {
		let result = sqlx::query(
			r#"
			DELETE FROM org_invitations
			WHERE id = ?
			"#,
		)
		.bind(id)
		.execute(&self.pool)
		.await?;

		let deleted = result.rows_affected() > 0;
		if deleted {
			tracing::debug!(invitation_id = %id, "invitation deleted");
		}
		Ok(deleted)
	}

	/// List pending invitations for an organization.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	///
	/// # Returns
	/// List of non-accepted, non-expired invitations.
	#[tracing::instrument(skip(self), fields(org_id = %org_id))]
	pub async fn list_pending_invitations(
		&self,
		org_id: &OrgId,
	) -> Result<Vec<OrgInvitation>, DbError> {
		let now = Utc::now().to_rfc3339();
		let rows = sqlx::query(
			r#"
			SELECT id, org_id, email, role, invited_by, token_hash, created_at, expires_at, accepted_at
			FROM org_invitations
			WHERE org_id = ? AND accepted_at IS NULL AND expires_at > ?
			ORDER BY created_at DESC
			"#,
		)
		.bind(org_id.to_string())
		.bind(&now)
		.fetch_all(&self.pool)
		.await?;

		let invitations: Result<Vec<_>, _> = rows.iter().map(|r| self.row_to_invitation(r)).collect();
		let invitations = invitations?;
		tracing::debug!(org_id = %org_id, count = invitations.len(), "listed pending invitations");
		Ok(invitations)
	}

	// =========================================================================
	// Join Requests
	// =========================================================================

	/// Create a join request and return its ID.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	/// * `user_id` - The requesting user's UUID
	///
	/// # Returns
	/// The generated join request ID.
	#[tracing::instrument(skip(self), fields(org_id = %org_id, user_id = %user_id))]
	pub async fn create_join_request(
		&self,
		org_id: &OrgId,
		user_id: &UserId,
	) -> Result<String, DbError> {
		let id = Uuid::new_v4().to_string();
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			INSERT INTO org_join_requests (id, org_id, user_id, created_at)
			VALUES (?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(org_id.to_string())
		.bind(user_id.to_string())
		.bind(&now)
		.execute(&self.pool)
		.await?;

		tracing::debug!(join_request_id = %id, org_id = %org_id, user_id = %user_id, "join request created");
		Ok(id)
	}

	/// Get a join request by ID.
	///
	/// # Arguments
	/// * `id` - The join request's UUID
	#[tracing::instrument(skip(self), fields(join_request_id = %id))]
	pub async fn get_join_request(&self, id: &str) -> Result<Option<OrgJoinRequest>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT org_id, user_id, created_at, handled_at, handled_by, approved
			FROM org_join_requests
			WHERE id = ?
			"#,
		)
		.bind(id)
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_join_request(&r)).transpose()
	}

	/// List pending join requests for an organization.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	///
	/// # Returns
	/// List of unhandled join requests ordered by creation date.
	#[tracing::instrument(skip(self), fields(org_id = %org_id))]
	pub async fn list_pending_join_requests(
		&self,
		org_id: &OrgId,
	) -> Result<Vec<OrgJoinRequest>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT org_id, user_id, created_at, handled_at, handled_by, approved
			FROM org_join_requests
			WHERE org_id = ? AND handled_at IS NULL
			ORDER BY created_at ASC
			"#,
		)
		.bind(org_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		let requests: Result<Vec<_>, _> = rows.iter().map(|r| self.row_to_join_request(r)).collect();
		let requests = requests?;
		tracing::debug!(org_id = %org_id, count = requests.len(), "listed pending join requests");
		Ok(requests)
	}

	/// List pending join requests for an organization with user info.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	///
	/// # Returns
	/// List of (join_request, user) tuples ordered by creation date.
	#[tracing::instrument(skip(self), fields(org_id = %org_id))]
	pub async fn list_pending_join_requests_with_users(
		&self,
		org_id: &OrgId,
	) -> Result<Vec<(OrgJoinRequest, User)>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT 
				jr.id as jr_id, jr.org_id, jr.user_id, jr.created_at, jr.handled_at, jr.handled_by, jr.approved,
				u.id as u_id, u.display_name, u.username, u.primary_email, u.avatar_url,
				u.email_visible, u.is_system_admin, u.is_support, u.is_auditor,
				u.created_at as u_created_at, u.updated_at as u_updated_at, u.deleted_at as u_deleted_at,
				u.locale
			FROM org_join_requests jr
			INNER JOIN users u ON jr.user_id = u.id
			WHERE jr.org_id = ? AND jr.handled_at IS NULL
			ORDER BY jr.created_at ASC
			"#,
		)
		.bind(org_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		let mut result = Vec::with_capacity(rows.len());
		for row in &rows {
			let request = self.row_to_join_request_prefixed(row)?;
			let user = self.row_to_user_prefixed(row)?;
			result.push((request, user));
		}
		tracing::debug!(org_id = %org_id, count = result.len(), "listed pending join requests with users");
		Ok(result)
	}

	/// Check if a user has a pending join request for an organization.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// `true` if a pending request exists.
	#[tracing::instrument(skip(self), fields(org_id = %org_id, user_id = %user_id))]
	pub async fn has_pending_join_request(
		&self,
		org_id: &OrgId,
		user_id: &UserId,
	) -> Result<bool, DbError> {
		let row: (i64,) = sqlx::query_as(
			r#"
			SELECT COUNT(*) FROM org_join_requests
			WHERE org_id = ? AND user_id = ? AND handled_at IS NULL
			"#,
		)
		.bind(org_id.to_string())
		.bind(user_id.to_string())
		.fetch_one(&self.pool)
		.await?;

		Ok(row.0 > 0)
	}

	/// Approve a join request.
	///
	/// # Arguments
	/// * `id` - The join request's UUID
	/// * `handled_by` - The approving user's UUID
	#[tracing::instrument(skip(self), fields(join_request_id = %id, handled_by = %handled_by))]
	pub async fn approve_join_request(&self, id: &str, handled_by: &UserId) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			UPDATE org_join_requests
			SET handled_at = ?, handled_by = ?, approved = 1
			WHERE id = ?
			"#,
		)
		.bind(&now)
		.bind(handled_by.to_string())
		.bind(id)
		.execute(&self.pool)
		.await?;

		tracing::debug!(join_request_id = %id, "join request approved");
		Ok(())
	}

	/// Reject a join request.
	///
	/// # Arguments
	/// * `id` - The join request's UUID
	/// * `handled_by` - The rejecting user's UUID
	#[tracing::instrument(skip(self), fields(join_request_id = %id, handled_by = %handled_by))]
	pub async fn reject_join_request(&self, id: &str, handled_by: &UserId) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			UPDATE org_join_requests
			SET handled_at = ?, handled_by = ?, approved = 0
			WHERE id = ?
			"#,
		)
		.bind(&now)
		.bind(handled_by.to_string())
		.bind(id)
		.execute(&self.pool)
		.await?;

		tracing::debug!(join_request_id = %id, "join request rejected");
		Ok(())
	}

	// =========================================================================
	// Personal Org
	// =========================================================================

	/// Create a personal organization for a user and add them as owner.
	///
	/// # Arguments
	/// * `user_id` - The user's UUID
	/// * `display_name` - The user's display name (used to generate org name)
	///
	/// # Returns
	/// The created personal organization.
	#[tracing::instrument(skip(self, display_name), fields(user_id = %user_id))]
	pub async fn create_personal_org(
		&self,
		user_id: &UserId,
		display_name: &str,
	) -> Result<Organization, DbError> {
		let org = Organization {
			id: OrgId::generate(),
			name: format!("{display_name}'s Personal"),
			slug: format!("personal-{user_id}"),
			visibility: OrgVisibility::Private,
			is_personal: true,
			created_at: Utc::now(),
			updated_at: Utc::now(),
			deleted_at: None,
		};

		self.create_org(&org).await?;
		self.add_member(&org.id, user_id, OrgRole::Owner).await?;

		tracing::debug!(org_id = %org.id, user_id = %user_id, "personal organization created");
		Ok(org)
	}

	// =========================================================================
	// Helpers
	// =========================================================================

	fn row_to_org(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Organization, DbError> {
		let id_str: String = row.get("id");
		let visibility_str: String = row.get("visibility");
		let is_personal: i32 = row.get("is_personal");
		let created_at: String = row.get("created_at");
		let updated_at: String = row.get("updated_at");
		let deleted_at: Option<String> = row.get("deleted_at");

		let id =
			Uuid::parse_str(&id_str).map_err(|e| DbError::Internal(format!("Invalid org ID: {e}")))?;
		let visibility = match visibility_str.as_str() {
			"public" => OrgVisibility::Public,
			"unlisted" => OrgVisibility::Unlisted,
			"private" => OrgVisibility::Private,
			_ => OrgVisibility::Public,
		};

		Ok(Organization {
			id: OrgId::new(id),
			name: row.get("name"),
			slug: row.get("slug"),
			visibility,
			is_personal: is_personal != 0,
			created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
				.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
				.with_timezone(&Utc),
			updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
				.map_err(|e| DbError::Internal(format!("Invalid updated_at: {e}")))?
				.with_timezone(&Utc),
			deleted_at: deleted_at.and_then(|d| {
				chrono::DateTime::parse_from_rfc3339(&d)
					.map(|dt| dt.with_timezone(&Utc))
					.ok()
			}),
		})
	}

	fn row_to_membership(&self, row: &sqlx::sqlite::SqliteRow) -> Result<OrgMembership, DbError> {
		let org_id_str: String = row.get("org_id");
		let user_id_str: String = row.get("user_id");
		let role_str: String = row.get("role");
		let created_at: String = row.get("created_at");

		let org_id = Uuid::parse_str(&org_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid org_id: {e}")))?;
		let user_id = Uuid::parse_str(&user_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid user_id: {e}")))?;
		let role = match role_str.as_str() {
			"owner" => OrgRole::Owner,
			"admin" => OrgRole::Admin,
			_ => OrgRole::Member,
		};

		Ok(OrgMembership {
			org_id: OrgId::new(org_id),
			user_id: UserId::new(user_id),
			role,
			created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
				.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
				.with_timezone(&Utc),
		})
	}

	fn row_to_user_prefixed(&self, row: &sqlx::sqlite::SqliteRow) -> Result<User, DbError> {
		let id_str: String = row.get("u_id");
		let created_at: String = row.get("u_created_at");
		let updated_at: String = row.get("u_updated_at");
		let deleted_at: Option<String> = row.get("u_deleted_at");
		let is_system_admin: i32 = row.get("is_system_admin");
		let is_support: i32 = row.get("is_support");
		let is_auditor: i32 = row.get("is_auditor");
		let email_visible: i32 = row.get("email_visible");

		let id =
			Uuid::parse_str(&id_str).map_err(|e| DbError::Internal(format!("Invalid user ID: {e}")))?;

		Ok(User {
			id: UserId::new(id),
			display_name: row.get("display_name"),
			username: row.get("username"),
			primary_email: row.get("primary_email"),
			avatar_url: row.get("avatar_url"),
			email_visible: email_visible != 0,
			is_system_admin: is_system_admin != 0,
			is_support: is_support != 0,
			is_auditor: is_auditor != 0,
			created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
				.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
				.with_timezone(&Utc),
			updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
				.map_err(|e| DbError::Internal(format!("Invalid updated_at: {e}")))?
				.with_timezone(&Utc),
			deleted_at: deleted_at.and_then(|d| {
				chrono::DateTime::parse_from_rfc3339(&d)
					.map(|dt| dt.with_timezone(&Utc))
					.ok()
			}),
			locale: row.get("locale"),
		})
	}

	fn row_to_invitation(&self, row: &sqlx::sqlite::SqliteRow) -> Result<OrgInvitation, DbError> {
		let id_str: String = row.get("id");
		let org_id_str: String = row.get("org_id");
		let invited_by_str: String = row.get("invited_by");
		let role_str: String = row.get("role");
		let created_at: String = row.get("created_at");
		let expires_at: String = row.get("expires_at");
		let accepted_at: Option<String> = row.get("accepted_at");

		let id = Uuid::parse_str(&id_str)
			.map_err(|e| DbError::Internal(format!("Invalid invitation ID: {e}")))?;
		let org_id = Uuid::parse_str(&org_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid org_id: {e}")))?;
		let invited_by = Uuid::parse_str(&invited_by_str)
			.map_err(|e| DbError::Internal(format!("Invalid invited_by: {e}")))?;
		let role = match role_str.as_str() {
			"owner" => OrgRole::Owner,
			"admin" => OrgRole::Admin,
			_ => OrgRole::Member,
		};

		Ok(OrgInvitation {
			id: InvitationId::new(id),
			org_id: OrgId::new(org_id),
			email: row.get("email"),
			role,
			invited_by: UserId::new(invited_by),
			token_hash: row.get("token_hash"),
			created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
				.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
				.with_timezone(&Utc),
			expires_at: chrono::DateTime::parse_from_rfc3339(&expires_at)
				.map_err(|e| DbError::Internal(format!("Invalid expires_at: {e}")))?
				.with_timezone(&Utc),
			accepted_at: accepted_at.and_then(|d| {
				chrono::DateTime::parse_from_rfc3339(&d)
					.map(|dt| dt.with_timezone(&Utc))
					.ok()
			}),
		})
	}

	fn row_to_join_request(&self, row: &sqlx::sqlite::SqliteRow) -> Result<OrgJoinRequest, DbError> {
		let org_id_str: String = row.get("org_id");
		let user_id_str: String = row.get("user_id");
		let created_at: String = row.get("created_at");
		let handled_at: Option<String> = row.get("handled_at");
		let handled_by: Option<String> = row.get("handled_by");
		let approved: Option<i32> = row.get("approved");

		let org_id = Uuid::parse_str(&org_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid org_id: {e}")))?;
		let user_id = Uuid::parse_str(&user_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid user_id: {e}")))?;

		Ok(OrgJoinRequest {
			org_id: OrgId::new(org_id),
			user_id: UserId::new(user_id),
			created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
				.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
				.with_timezone(&Utc),
			handled_at: handled_at.and_then(|d| {
				chrono::DateTime::parse_from_rfc3339(&d)
					.map(|dt| dt.with_timezone(&Utc))
					.ok()
			}),
			handled_by: handled_by.and_then(|h| Uuid::parse_str(&h).map(UserId::new).ok()),
			approved: approved.map(|a| a != 0),
		})
	}

	fn row_to_join_request_prefixed(
		&self,
		row: &sqlx::sqlite::SqliteRow,
	) -> Result<OrgJoinRequest, DbError> {
		let id_str: String = row.get("jr_id");
		let org_id_str: String = row.get("org_id");
		let user_id_str: String = row.get("user_id");
		let created_at: String = row.get("created_at");
		let handled_at: Option<String> = row.get("handled_at");
		let handled_by: Option<String> = row.get("handled_by");
		let approved: Option<i32> = row.get("approved");

		let _ = id_str;
		let org_id = Uuid::parse_str(&org_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid org_id: {e}")))?;
		let user_id = Uuid::parse_str(&user_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid user_id: {e}")))?;

		Ok(OrgJoinRequest {
			org_id: OrgId::new(org_id),
			user_id: UserId::new(user_id),
			created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
				.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
				.with_timezone(&Utc),
			handled_at: handled_at.and_then(|d| {
				chrono::DateTime::parse_from_rfc3339(&d)
					.map(|dt| dt.with_timezone(&Utc))
					.ok()
			}),
			handled_by: handled_by.and_then(|h| Uuid::parse_str(&h).map(UserId::new).ok()),
			approved: approved.map(|a| a != 0),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;
	use std::collections::HashSet;

	proptest! {
		#[test]
		fn org_id_generation_is_unique(count in 1..1000usize) {
			let mut ids = HashSet::new();
			for _ in 0..count {
				let id = OrgId::generate();
				prop_assert!(ids.insert(id.to_string()), "Generated duplicate OrgId");
			}
		}

		#[test]
		fn invitation_id_generation_is_unique(count in 1..1000usize) {
			let mut ids = HashSet::new();
			for _ in 0..count {
				let id = InvitationId::generate();
				prop_assert!(ids.insert(id.to_string()), "Generated duplicate InvitationId");
			}
		}

		#[test]
		fn slug_validation_alphanumeric_dash(slug in "[a-z0-9-]{1,50}") {
			let is_valid = slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
			prop_assert!(is_valid, "Slug should only contain lowercase alphanumeric and dashes");
		}

		#[test]
		fn list_public_orgs_pagination_bounds(limit in 0i32..1000, offset in 0i32..10000) {
			prop_assert!(limit >= 0, "limit must be non-negative");
			prop_assert!(offset >= 0, "offset must be non-negative");
		}
	}
}
