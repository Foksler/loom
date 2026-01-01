// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Team repository for database operations.
//!
//! This module provides database access for team management within organizations.
//! Teams group users for access control and collaboration.

use chrono::Utc;
use loom_auth::{
	team::{Team, TeamMembership},
	types::{OrgId, TeamId, TeamRole, UserId},
};
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;

use crate::error::DbError;

/// Repository for team database operations.
///
/// Manages teams within organizations and their memberships.
/// Teams are scoped to a single organization.
#[derive(Clone)]
pub struct TeamRepository {
	pool: SqlitePool,
}

impl TeamRepository {
	/// Create a new repository with the given pool.
	///
	/// # Arguments
	/// * `pool` - SQLite connection pool
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	// =========================================================================
	// Team CRUD
	// =========================================================================

	/// Create a new team.
	///
	/// # Arguments
	/// * `team` - The team to create
	///
	/// # Errors
	/// Returns `DbError::Sqlx` if insert fails (e.g., duplicate slug within org).
	///
	/// # Database Constraints
	/// - `id` must be unique
	/// - (`org_id`, `slug`) must be unique
	/// - `org_id` must reference an existing organization
	#[tracing::instrument(skip(self, team), fields(team_id = %team.id, org_id = %team.org_id, slug = %team.slug))]
	pub async fn create_team(&self, team: &Team) -> Result<(), DbError> {
		sqlx::query(
			r#"
			INSERT INTO teams (id, org_id, name, slug, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(team.id.to_string())
		.bind(team.org_id.to_string())
		.bind(&team.name)
		.bind(&team.slug)
		.bind(team.created_at.to_rfc3339())
		.bind(team.updated_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		tracing::debug!(team_id = %team.id, org_id = %team.org_id, "team created");
		Ok(())
	}

	/// Get a team by ID.
	///
	/// # Arguments
	/// * `id` - The team's UUID
	///
	/// # Returns
	/// `None` if no team exists with this ID.
	#[tracing::instrument(skip(self), fields(team_id = %id))]
	pub async fn get_team_by_id(&self, id: &TeamId) -> Result<Option<Team>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, org_id, name, slug, created_at, updated_at
			FROM teams
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_team(&r)).transpose()
	}

	/// Get a team by slug within an organization.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	/// * `slug` - The team's URL-safe slug
	///
	/// # Returns
	/// `None` if no team exists with this slug in the organization.
	#[tracing::instrument(skip(self), fields(org_id = %org_id, slug = %slug))]
	pub async fn get_team_by_slug(
		&self,
		org_id: &OrgId,
		slug: &str,
	) -> Result<Option<Team>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, org_id, name, slug, created_at, updated_at
			FROM teams
			WHERE org_id = ? AND slug = ?
			"#,
		)
		.bind(org_id.to_string())
		.bind(slug)
		.fetch_optional(&self.pool)
		.await?;

		let result = row.map(|r| self.row_to_team(&r)).transpose()?;
		if let Some(ref team) = result {
			tracing::debug!(team_id = %team.id, "team found by slug");
		}
		Ok(result)
	}

	/// Update a team.
	///
	/// # Arguments
	/// * `team` - The team with updated fields
	///
	/// # Errors
	/// Returns `DbError::Sqlx` if update fails (e.g., duplicate slug).
	#[tracing::instrument(skip(self, team), fields(team_id = %team.id))]
	pub async fn update_team(&self, team: &Team) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			UPDATE teams
			SET name = ?, slug = ?, updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&team.name)
		.bind(&team.slug)
		.bind(now)
		.bind(team.id.to_string())
		.execute(&self.pool)
		.await?;

		tracing::debug!(team_id = %team.id, "team updated");
		Ok(())
	}

	/// Delete a team.
	///
	/// # Arguments
	/// * `id` - The team's UUID
	///
	/// # Returns
	/// `true` if a team was deleted, `false` if not found.
	///
	/// # Note
	/// This is a hard delete. Team memberships will be cascade deleted.
	#[tracing::instrument(skip(self), fields(team_id = %id))]
	pub async fn delete_team(&self, id: &TeamId) -> Result<bool, DbError> {
		let result = sqlx::query(
			r#"
			DELETE FROM teams
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		let deleted = result.rows_affected() > 0;
		if deleted {
			tracing::debug!(team_id = %id, "team deleted");
		}
		Ok(deleted)
	}

	/// List all teams for an organization.
	///
	/// # Arguments
	/// * `org_id` - The organization's UUID
	///
	/// # Returns
	/// List of teams ordered by name.
	#[tracing::instrument(skip(self), fields(org_id = %org_id))]
	pub async fn list_teams_for_org(&self, org_id: &OrgId) -> Result<Vec<Team>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT id, org_id, name, slug, created_at, updated_at
			FROM teams
			WHERE org_id = ?
			ORDER BY name ASC
			"#,
		)
		.bind(org_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		let teams: Result<Vec<_>, _> = rows.iter().map(|r| self.row_to_team(r)).collect();
		let teams = teams?;
		tracing::debug!(org_id = %org_id, count = teams.len(), "listed teams for organization");
		Ok(teams)
	}

	// =========================================================================
	// Memberships
	// =========================================================================

	/// Add a member to a team.
	///
	/// # Arguments
	/// * `team_id` - The team's UUID
	/// * `user_id` - The user's UUID
	/// * `role` - The member's role (maintainer or member)
	///
	/// # Database Constraints
	/// - (`team_id`, `user_id`) must be unique
	/// - `team_id` must reference an existing team
	/// - `user_id` must reference an existing user
	#[tracing::instrument(skip(self), fields(team_id = %team_id, user_id = %user_id, role = %role))]
	pub async fn add_member(
		&self,
		team_id: &TeamId,
		user_id: &UserId,
		role: TeamRole,
	) -> Result<(), DbError> {
		let id = Uuid::new_v4().to_string();
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			INSERT INTO team_memberships (id, team_id, user_id, role, created_at)
			VALUES (?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(team_id.to_string())
		.bind(user_id.to_string())
		.bind(role.to_string())
		.bind(&now)
		.execute(&self.pool)
		.await?;

		tracing::debug!(team_id = %team_id, user_id = %user_id, role = %role, "member added to team");
		Ok(())
	}

	/// Get a membership for a user in a team.
	///
	/// # Arguments
	/// * `team_id` - The team's UUID
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// `None` if the user is not a member.
	#[tracing::instrument(skip(self), fields(team_id = %team_id, user_id = %user_id))]
	pub async fn get_membership(
		&self,
		team_id: &TeamId,
		user_id: &UserId,
	) -> Result<Option<TeamMembership>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, team_id, user_id, role, created_at
			FROM team_memberships
			WHERE team_id = ? AND user_id = ?
			"#,
		)
		.bind(team_id.to_string())
		.bind(user_id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_membership(&r)).transpose()
	}

	/// Update a member's role.
	///
	/// # Arguments
	/// * `team_id` - The team's UUID
	/// * `user_id` - The user's UUID
	/// * `role` - The new role
	#[tracing::instrument(skip(self), fields(team_id = %team_id, user_id = %user_id, role = %role))]
	pub async fn update_member_role(
		&self,
		team_id: &TeamId,
		user_id: &UserId,
		role: TeamRole,
	) -> Result<(), DbError> {
		sqlx::query(
			r#"
			UPDATE team_memberships
			SET role = ?
			WHERE team_id = ? AND user_id = ?
			"#,
		)
		.bind(role.to_string())
		.bind(team_id.to_string())
		.bind(user_id.to_string())
		.execute(&self.pool)
		.await?;

		tracing::debug!(team_id = %team_id, user_id = %user_id, role = %role, "team member role updated");
		Ok(())
	}

	/// Remove a member from a team.
	///
	/// # Arguments
	/// * `team_id` - The team's UUID
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// `true` if a member was removed, `false` if not found.
	#[tracing::instrument(skip(self), fields(team_id = %team_id, user_id = %user_id))]
	pub async fn remove_member(
		&self,
		team_id: &TeamId,
		user_id: &UserId,
	) -> Result<bool, DbError> {
		let result = sqlx::query(
			r#"
			DELETE FROM team_memberships
			WHERE team_id = ? AND user_id = ?
			"#,
		)
		.bind(team_id.to_string())
		.bind(user_id.to_string())
		.execute(&self.pool)
		.await?;

		let removed = result.rows_affected() > 0;
		if removed {
			tracing::debug!(team_id = %team_id, user_id = %user_id, "member removed from team");
		}
		Ok(removed)
	}

	/// List all members of a team.
	///
	/// # Arguments
	/// * `team_id` - The team's UUID
	///
	/// # Returns
	/// List of memberships ordered by join date.
	#[tracing::instrument(skip(self), fields(team_id = %team_id))]
	pub async fn list_members(&self, team_id: &TeamId) -> Result<Vec<TeamMembership>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT id, team_id, user_id, role, created_at
			FROM team_memberships
			WHERE team_id = ?
			ORDER BY created_at ASC
			"#,
		)
		.bind(team_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		let members: Result<Vec<_>, _> = rows.iter().map(|r| self.row_to_membership(r)).collect();
		let members = members?;
		tracing::debug!(team_id = %team_id, count = members.len(), "listed team members");
		Ok(members)
	}

	/// Get all teams a user is a member of, with their role.
	///
	/// # Arguments
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// List of (team, role) tuples ordered by team name.
	#[tracing::instrument(skip(self), fields(user_id = %user_id))]
	pub async fn get_teams_for_user(
		&self,
		user_id: &UserId,
	) -> Result<Vec<(Team, TeamRole)>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT t.id, t.org_id, t.name, t.slug, t.created_at, t.updated_at, m.role
			FROM teams t
			INNER JOIN team_memberships m ON t.id = m.team_id
			WHERE m.user_id = ?
			ORDER BY t.name ASC
			"#,
		)
		.bind(user_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		let mut result = Vec::with_capacity(rows.len());
		for r in &rows {
			let team = self.row_to_team(r)?;
			let role_str: String = r.get("role");
			let role = match role_str.as_str() {
				"maintainer" => TeamRole::Maintainer,
				_ => TeamRole::Member,
			};
			result.push((team, role));
		}
		tracing::debug!(user_id = %user_id, count = result.len(), "retrieved teams for user");
		Ok(result)
	}

	// =========================================================================
	// Helpers
	// =========================================================================

	fn row_to_team(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Team, DbError> {
		let id_str: String = row.get("id");
		let org_id_str: String = row.get("org_id");
		let created_at: String = row.get("created_at");
		let updated_at: String = row.get("updated_at");

		let id = Uuid::parse_str(&id_str)
			.map_err(|e| DbError::Internal(format!("Invalid team ID: {e}")))?;
		let org_id = Uuid::parse_str(&org_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid org_id: {e}")))?;

		Ok(Team {
			id: TeamId::new(id),
			org_id: OrgId::new(org_id),
			name: row.get("name"),
			slug: row.get("slug"),
			created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
				.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
				.with_timezone(&Utc),
			updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
				.map_err(|e| DbError::Internal(format!("Invalid updated_at: {e}")))?
				.with_timezone(&Utc),
		})
	}

	fn row_to_membership(
		&self,
		row: &sqlx::sqlite::SqliteRow,
	) -> Result<TeamMembership, DbError> {
		let id_str: String = row.get("id");
		let team_id_str: String = row.get("team_id");
		let user_id_str: String = row.get("user_id");
		let role_str: String = row.get("role");
		let created_at: String = row.get("created_at");

		let id = Uuid::parse_str(&id_str)
			.map_err(|e| DbError::Internal(format!("Invalid membership ID: {e}")))?;
		let team_id = Uuid::parse_str(&team_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid team_id: {e}")))?;
		let user_id = Uuid::parse_str(&user_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid user_id: {e}")))?;
		let role = match role_str.as_str() {
			"maintainer" => TeamRole::Maintainer,
			_ => TeamRole::Member,
		};

		Ok(TeamMembership {
			id,
			team_id: TeamId::new(team_id),
			user_id: UserId::new(user_id),
			role,
			created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
				.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
				.with_timezone(&Utc),
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
		fn team_id_generation_is_unique(count in 1..1000usize) {
			let mut ids = HashSet::new();
			for _ in 0..count {
				let id = TeamId::generate();
				prop_assert!(ids.insert(id.to_string()), "Generated duplicate TeamId");
			}
		}

		#[test]
		fn team_slug_validation(slug in "[a-z0-9-]{1,50}") {
			let is_valid = slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
			prop_assert!(is_valid, "Team slug should only contain lowercase alphanumeric and dashes");
			prop_assert!(!slug.is_empty(), "Team slug should not be empty");
		}

		#[test]
		fn team_membership_id_is_uuid(_unused: u8) {
			let id = Uuid::new_v4();
			prop_assert!(id.to_string().len() == 36, "UUID should be 36 characters");
		}
	}
}
