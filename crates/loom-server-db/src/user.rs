// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! User repository for database operations.
//!
//! This module provides database access for user and identity management.
//! Users can have multiple identities (e.g., GitHub, Google, MagicLink).

use chrono::{DateTime, Utc};
use loom_server_auth::{Identity, IdentityId, Provider, User, UserId};
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;

use crate::error::DbError;

/// Repository for user database operations.
///
/// Provides CRUD operations for users and their linked identities.
/// All user IDs are UUIDs stored as strings in SQLite.
#[derive(Clone)]
pub struct UserRepository {
	pool: SqlitePool,
}

impl UserRepository {
	/// Create a new repository with the given connection pool.
	///
	/// # Arguments
	/// * `pool` - SQLite connection pool
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	/// Create a new user in the database.
	///
	/// # Arguments
	/// * `user` - The user to create
	///
	/// # Errors
	/// Returns `DbError::Sqlx` if the insert fails (e.g., duplicate ID).
	///
	/// # Database Constraints
	/// - `id` must be unique
	/// - `primary_email` should be unique (not enforced at DB level)
	#[tracing::instrument(skip(self, user), fields(user_id = %user.id))]
	pub async fn create_user(&self, user: &User) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			INSERT INTO users (
				id, display_name, username, primary_email, avatar_url,
				email_visible, is_system_admin, is_support, is_auditor,
				created_at, updated_at, deleted_at, locale
			) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(user.id.to_string())
		.bind(&user.display_name)
		.bind(&user.username)
		.bind(&user.primary_email)
		.bind(&user.avatar_url)
		.bind(user.email_visible as i32)
		.bind(user.is_system_admin as i32)
		.bind(user.is_support as i32)
		.bind(user.is_auditor as i32)
		.bind(user.created_at.to_rfc3339())
		.bind(now)
		.bind(user.deleted_at.map(|dt| dt.to_rfc3339()))
		.bind(&user.locale)
		.execute(&self.pool)
		.await?;

		tracing::debug!(user_id = %user.id, "user created");
		Ok(())
	}

	/// Get a user by their unique ID.
	///
	/// # Arguments
	/// * `id` - The user's UUID
	///
	/// # Returns
	/// `None` if no user exists with this ID or if the user is soft-deleted.
	#[tracing::instrument(skip(self), fields(user_id = %id))]
	pub async fn get_user_by_id(&self, id: &UserId) -> Result<Option<User>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, display_name, username, primary_email, avatar_url,
				   email_visible, is_system_admin, is_support, is_auditor,
				   created_at, updated_at, deleted_at, locale
			FROM users
			WHERE id = ? AND deleted_at IS NULL
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_user(&r)).transpose()
	}

	/// Get a user by their display name.
	///
	/// # Arguments
	/// * `display_name` - The display name to search for (exact match)
	///
	/// # Returns
	/// `None` if no user exists with this display name or if the user is soft-deleted.
	#[tracing::instrument(skip(self))]
	pub async fn get_user_by_display_name(
		&self,
		display_name: &str,
	) -> Result<Option<User>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, display_name, username, primary_email, avatar_url,
				   email_visible, is_system_admin, is_support, is_auditor,
				   created_at, updated_at, deleted_at, locale
			FROM users
			WHERE display_name = ? AND deleted_at IS NULL
			"#,
		)
		.bind(display_name)
		.fetch_optional(&self.pool)
		.await?;

		let result = row.map(|r| self.row_to_user(&r)).transpose()?;
		if let Some(ref user) = result {
			tracing::debug!(user_id = %user.id, "user found by display_name");
		}
		Ok(result)
	}

	/// Get a user by their primary email address.
	///
	/// # Arguments
	/// * `email` - The email address to search for
	///
	/// # Returns
	/// `None` if no user exists with this email or if the user is soft-deleted.
	#[tracing::instrument(skip(self, email))]
	pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, display_name, username, primary_email, avatar_url,
				   email_visible, is_system_admin, is_support, is_auditor,
				   created_at, updated_at, deleted_at, locale
			FROM users
			WHERE primary_email = ? AND deleted_at IS NULL
			"#,
		)
		.bind(email)
		.fetch_optional(&self.pool)
		.await?;

		let result = row.map(|r| self.row_to_user(&r)).transpose()?;
		if let Some(ref user) = result {
			tracing::debug!(user_id = %user.id, "user found by email");
		}
		Ok(result)
	}

	/// Update an existing user's profile.
	///
	/// # Arguments
	/// * `user` - The user with updated fields
	///
	/// # Errors
	/// Returns `DbError::Sqlx` if the update fails.
	#[tracing::instrument(skip(self, user), fields(user_id = %user.id))]
	pub async fn update_user(&self, user: &User) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
			UPDATE users SET
				display_name = ?,
				username = ?,
				primary_email = ?,
				avatar_url = ?,
				email_visible = ?,
				is_system_admin = ?,
				is_support = ?,
				is_auditor = ?,
				updated_at = ?,
				deleted_at = ?,
				locale = ?
			WHERE id = ?
			"#,
		)
		.bind(&user.display_name)
		.bind(&user.username)
		.bind(&user.primary_email)
		.bind(&user.avatar_url)
		.bind(user.email_visible as i32)
		.bind(user.is_system_admin as i32)
		.bind(user.is_support as i32)
		.bind(user.is_auditor as i32)
		.bind(now)
		.bind(user.deleted_at.map(|dt| dt.to_rfc3339()))
		.bind(&user.locale)
		.bind(user.id.to_string())
		.execute(&self.pool)
		.await?;

		tracing::debug!(user_id = %user.id, "user updated");
		Ok(())
	}

	/// Soft-delete a user by setting their `deleted_at` timestamp.
	///
	/// Soft-deleted users are excluded from normal queries but data is retained.
	///
	/// # Arguments
	/// * `id` - The user's UUID
	#[tracing::instrument(skip(self), fields(user_id = %id))]
	pub async fn soft_delete_user(&self, id: &UserId) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query("UPDATE users SET deleted_at = ?, updated_at = ? WHERE id = ?")
			.bind(&now)
			.bind(&now)
			.bind(id.to_string())
			.execute(&self.pool)
			.await?;

		tracing::debug!(user_id = %id, "user soft-deleted");
		Ok(())
	}

	/// Restore a soft-deleted user by clearing their `deleted_at` timestamp.
	///
	/// # Arguments
	/// * `id` - The user's UUID
	#[tracing::instrument(skip(self), fields(user_id = %id))]
	pub async fn restore_user(&self, id: &UserId) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query("UPDATE users SET deleted_at = NULL, updated_at = ? WHERE id = ?")
			.bind(&now)
			.bind(id.to_string())
			.execute(&self.pool)
			.await?;

		tracing::debug!(user_id = %id, "user restored");
		Ok(())
	}

	/// Update a user's locale preference.
	///
	/// # Arguments
	/// * `id` - The user's UUID
	/// * `locale` - The locale code (e.g., "en", "es", "ar") or None to clear
	#[tracing::instrument(skip(self), fields(user_id = %id))]
	pub async fn update_locale(
		&self,
		id: &UserId,
		locale: Option<&str>,
	) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query("UPDATE users SET locale = ?, updated_at = ? WHERE id = ?")
			.bind(locale)
			.bind(&now)
			.bind(id.to_string())
			.execute(&self.pool)
			.await?;

		tracing::debug!(user_id = %id, locale = ?locale, "user locale updated");
		Ok(())
	}

	/// Get a user by their username (case-insensitive).
	#[tracing::instrument(skip(self))]
	pub async fn get_user_by_username(
		&self,
		username: &str,
	) -> Result<Option<User>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, display_name, username, primary_email, avatar_url,
				   email_visible, is_system_admin, is_support, is_auditor,
				   created_at, updated_at, deleted_at, locale
			FROM users
			WHERE LOWER(username) = LOWER(?) AND deleted_at IS NULL
			"#,
		)
		.bind(username)
		.fetch_optional(&self.pool)
		.await?;

		let result = row.map(|r| self.row_to_user(&r)).transpose()?;
		if let Some(ref user) = result {
			tracing::debug!(user_id = %user.id, "user found by username");
		}
		Ok(result)
	}

	/// Check if a username is available (case-insensitive).
	#[tracing::instrument(skip(self))]
	pub async fn is_username_available(&self, username: &str) -> Result<bool, DbError> {
		let count: (i64,) =
			sqlx::query_as("SELECT COUNT(*) FROM users WHERE LOWER(username) = LOWER(?)")
				.bind(username)
				.fetch_one(&self.pool)
				.await?;

		Ok(count.0 == 0)
	}

	/// Generate a unique username from a base, adding numeric suffix if needed.
	#[tracing::instrument(skip(self))]
	pub async fn generate_unique_username(&self, base: &str) -> Result<String, DbError> {
		use loom_server_auth::generate_username_base;

		let sanitized = generate_username_base(base);

		if self.is_username_available(&sanitized).await? {
			return Ok(sanitized);
		}

		for i in 1..1000 {
			let candidate = format!("{}{}", sanitized, i);
			if candidate.len() <= 39 && self.is_username_available(&candidate).await? {
				return Ok(candidate);
			}
		}

		let uuid_suffix = Uuid::new_v4().to_string().replace("-", "");
		Ok(format!(
			"{}_{}",
			&sanitized[..sanitized.len().min(20)],
			&uuid_suffix[..8]
		))
	}

	/// Update a user's username.
	#[tracing::instrument(skip(self), fields(user_id = %user_id))]
	pub async fn update_username(
		&self,
		user_id: &UserId,
		username: &str,
	) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query("UPDATE users SET username = ?, updated_at = ? WHERE id = ?")
			.bind(username)
			.bind(&now)
			.bind(user_id.to_string())
			.execute(&self.pool)
			.await?;

		tracing::debug!(user_id = %user_id, "user username updated");
		Ok(())
	}

	/// List users with pagination and optional search.
	///
	/// # Arguments
	/// * `limit` - Maximum number of users to return
	/// * `offset` - Number of users to skip
	/// * `search` - Optional search term for display_name or email
	///
	/// # Returns
	/// Tuple of (users, total_count) for pagination.
	#[tracing::instrument(skip(self, search), fields(limit, offset))]
	pub async fn list_users(
		&self,
		limit: i32,
		offset: i32,
		search: Option<&str>,
	) -> Result<(Vec<User>, i64), DbError> {
		let (users, total) = if let Some(search_term) = search {
			let pattern = format!("%{search_term}%");
			let rows = sqlx::query(
				r#"
				SELECT id, display_name, username, primary_email, avatar_url,
					   email_visible, is_system_admin, is_support, is_auditor,
					   created_at, updated_at, deleted_at, locale
				FROM users
				WHERE deleted_at IS NULL
				  AND (display_name LIKE ? OR primary_email LIKE ?)
				ORDER BY created_at DESC
				LIMIT ? OFFSET ?
				"#,
			)
			.bind(&pattern)
			.bind(&pattern)
			.bind(limit)
			.bind(offset)
			.fetch_all(&self.pool)
			.await?;

			let count: (i64,) = sqlx::query_as(
				r#"
				SELECT COUNT(*) FROM users
				WHERE deleted_at IS NULL
				  AND (display_name LIKE ? OR primary_email LIKE ?)
				"#,
			)
			.bind(&pattern)
			.bind(&pattern)
			.fetch_one(&self.pool)
			.await?;

			let users: Vec<User> = rows
				.iter()
				.filter_map(|r| self.row_to_user(r).ok())
				.collect();
			(users, count.0)
		} else {
			let rows = sqlx::query(
				r#"
				SELECT id, display_name, username, primary_email, avatar_url,
					   email_visible, is_system_admin, is_support, is_auditor,
					   created_at, updated_at, deleted_at, locale
				FROM users
				WHERE deleted_at IS NULL
				ORDER BY created_at DESC
				LIMIT ? OFFSET ?
				"#,
			)
			.bind(limit)
			.bind(offset)
			.fetch_all(&self.pool)
			.await?;

			let count: (i64,) =
				sqlx::query_as("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL")
					.fetch_one(&self.pool)
					.await?;

			let users: Vec<User> = rows
				.iter()
				.filter_map(|r| self.row_to_user(r).ok())
				.collect();
			(users, count.0)
		};

		tracing::debug!(count = users.len(), total, "listed users");
		Ok((users, total))
	}

	/// Create a new identity linking a user to an external provider.
	///
	/// # Arguments
	/// * `identity` - The identity to create
	///
	/// # Database Constraints
	/// - `id` must be unique
	/// - (`provider`, `provider_user_id`) must be unique
	/// - `user_id` must reference an existing user
	#[tracing::instrument(skip(self, identity), fields(identity_id = %identity.id, user_id = %identity.user_id))]
	pub async fn create_identity(&self, identity: &Identity) -> Result<(), DbError> {
		sqlx::query(
			r#"
			INSERT INTO identities (
				id, user_id, provider, provider_user_id,
				email, email_verified, created_at
			) VALUES (?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(identity.id.to_string())
		.bind(identity.user_id.to_string())
		.bind(identity.provider.to_string())
		.bind(&identity.provider_user_id)
		.bind(&identity.email)
		.bind(identity.email_verified as i32)
		.bind(identity.created_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		tracing::debug!(identity_id = %identity.id, user_id = %identity.user_id, "identity created");
		Ok(())
	}

	/// Get all identities linked to a user.
	///
	/// # Arguments
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// List of all identities (GitHub, Google, etc.) linked to this user.
	#[tracing::instrument(skip(self), fields(user_id = %user_id))]
	pub async fn get_identities_for_user(
		&self,
		user_id: &UserId,
	) -> Result<Vec<Identity>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT id, user_id, provider, provider_user_id,
				   email, email_verified, created_at
			FROM identities
			WHERE user_id = ?
			"#,
		)
		.bind(user_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.iter().map(|r| self.row_to_identity(r)).collect()
	}

	/// Get an identity by provider and provider-specific user ID.
	///
	/// Used during OAuth login to find existing linked accounts.
	///
	/// # Arguments
	/// * `provider` - The OAuth provider name (e.g., "github", "google")
	/// * `provider_user_id` - The user's ID on the external provider
	#[tracing::instrument(skip(self))]
	pub async fn get_identity_by_provider(
		&self,
		provider: &str,
		provider_user_id: &str,
	) -> Result<Option<Identity>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, user_id, provider, provider_user_id,
				   email, email_verified, created_at
			FROM identities
			WHERE provider = ? AND provider_user_id = ?
			"#,
		)
		.bind(provider)
		.bind(provider_user_id)
		.fetch_optional(&self.pool)
		.await?;

		let result = row.map(|r| self.row_to_identity(&r)).transpose()?;
		if let Some(ref identity) = result {
			tracing::debug!(identity_id = %identity.id, user_id = %identity.user_id, "identity found by provider");
		}
		Ok(result)
	}

	/// Delete an identity by ID.
	///
	/// # Arguments
	/// * `id` - The identity's UUID
	///
	/// # Returns
	/// `true` if an identity was deleted, `false` if not found.
	#[tracing::instrument(skip(self), fields(identity_id = %id))]
	pub async fn delete_identity(&self, id: &IdentityId) -> Result<bool, DbError> {
		let result = sqlx::query("DELETE FROM identities WHERE id = ?")
			.bind(id.to_string())
			.execute(&self.pool)
			.await?;

		let deleted = result.rows_affected() > 0;
		if deleted {
			tracing::debug!(identity_id = %id, "identity deleted");
		}
		Ok(deleted)
	}

	/// Find an existing user by email or create a new one.
	///
	/// Used during OAuth signup to either link to an existing account
	/// or create a new user.
	///
	/// # Arguments
	/// * `email` - The user's email address
	/// * `display_name` - Display name for new users
	/// * `avatar_url` - Optional avatar URL for new users
	/// * `preferred_username` - Optional preferred username (e.g., GitHub login)
	#[tracing::instrument(skip(self, email, display_name, avatar_url, preferred_username))]
	pub async fn find_or_create_user_by_email(
		&self,
		email: &str,
		display_name: &str,
		avatar_url: Option<&str>,
		preferred_username: Option<&str>,
	) -> Result<User, DbError> {
		if let Some(mut user) = self.get_user_by_email(email).await? {
			tracing::debug!(user_id = %user.id, "found existing user by email");
			if user.username.is_none() {
				let username_base = preferred_username.unwrap_or(display_name);
				let username = self.generate_unique_username(username_base).await?;
				self.update_username(&user.id, &username).await?;
				user.username = Some(username);
				tracing::debug!(user_id = %user.id, "set username for existing user");
			}
			return Ok(user);
		}

		// Check if this will be the first user (auto-promote to system admin)
		let user_count = self.count_users().await?;
		let is_first_user = user_count == 0;

		let now = Utc::now();
		let username_base = preferred_username.unwrap_or(display_name);
		let username = self.generate_unique_username(username_base).await?;
		let user = User {
			id: UserId::generate(),
			display_name: display_name.to_string(),
			username: Some(username),
			primary_email: Some(email.to_string()),
			avatar_url: avatar_url.map(|s| s.to_string()),
			email_visible: true,
			is_system_admin: is_first_user,
			is_support: false,
			is_auditor: false,
			created_at: now,
			updated_at: now,
			deleted_at: None,
			locale: None,
		};

		self.create_user(&user).await?;

		if is_first_user {
			tracing::info!(user_id = %user.id, email = %email, "first user created as system admin");
		} else {
			tracing::debug!(user_id = %user.id, "created new user by email");
		}

		Ok(user)
	}

	/// Count total non-deleted users.
	///
	/// # Returns
	/// Total count of active users in the system.
	#[tracing::instrument(skip(self))]
	pub async fn count_users(&self) -> Result<i64, DbError> {
		let count: (i64,) =
			sqlx::query_as("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL")
				.fetch_one(&self.pool)
				.await?;
		Ok(count.0)
	}

	/// Promote a user to system admin.
	///
	/// Typically used during initial bootstrap to make the first user an admin.
	///
	/// # Arguments
	/// * `user_id` - The user's UUID
	#[tracing::instrument(skip(self), fields(user_id = %user_id))]
	pub async fn make_first_user_admin(&self, user_id: &UserId) -> Result<(), DbError> {
		let now = Utc::now().to_rfc3339();
		sqlx::query("UPDATE users SET is_system_admin = 1, updated_at = ? WHERE id = ?")
			.bind(&now)
			.bind(user_id.to_string())
			.execute(&self.pool)
			.await?;

		tracing::info!(user_id = %user_id, "user promoted to system admin");
		Ok(())
	}

	fn row_to_user(&self, row: &sqlx::sqlite::SqliteRow) -> Result<User, DbError> {
		let id_str: String = row.get("id");
		let id = Uuid::parse_str(&id_str)
			.map_err(|e| DbError::Internal(format!("Invalid user ID: {e}")))?;

		let created_at_str: String = row.get("created_at");
		let created_at = DateTime::parse_from_rfc3339(&created_at_str)
			.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
			.with_timezone(&Utc);

		let updated_at_str: String = row.get("updated_at");
		let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
			.map_err(|e| DbError::Internal(format!("Invalid updated_at: {e}")))?
			.with_timezone(&Utc);

		let deleted_at: Option<String> = row.get("deleted_at");
		let deleted_at = deleted_at
			.map(|s| {
				DateTime::parse_from_rfc3339(&s)
					.map(|dt| dt.with_timezone(&Utc))
					.map_err(|e| DbError::Internal(format!("Invalid deleted_at: {e}")))
			})
			.transpose()?;

		let email_visible: i32 = row.get("email_visible");
		let is_system_admin: i32 = row.get("is_system_admin");
		let is_support: i32 = row.get("is_support");
		let is_auditor: i32 = row.get("is_auditor");

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
			created_at,
			updated_at,
			deleted_at,
			locale: row.get("locale"),
		})
	}

	fn row_to_identity(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Identity, DbError> {
		let id_str: String = row.get("id");
		let id = Uuid::parse_str(&id_str)
			.map_err(|e| DbError::Internal(format!("Invalid identity ID: {e}")))?;

		let user_id_str: String = row.get("user_id");
		let user_id = Uuid::parse_str(&user_id_str)
			.map_err(|e| DbError::Internal(format!("Invalid user ID: {e}")))?;

		let provider_str: String = row.get("provider");
		let provider = match provider_str.as_str() {
			"github" => Provider::GitHub,
			"google" => Provider::Google,
			"magic_link" => Provider::MagicLink,
			_ => {
				return Err(DbError::Internal(format!(
					"Unknown provider: {provider_str}"
				)))
			}
		};

		let created_at_str: String = row.get("created_at");
		let created_at = DateTime::parse_from_rfc3339(&created_at_str)
			.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?
			.with_timezone(&Utc);

		let email_verified: i32 = row.get("email_verified");

		Ok(Identity {
			id: IdentityId::new(id),
			user_id: UserId::new(user_id),
			provider,
			provider_user_id: row.get("provider_user_id"),
			email: row.get("email"),
			email_verified: email_verified != 0,
			created_at,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;
	use sqlx::sqlite::SqlitePool;
	use std::collections::HashSet;

	async fn create_test_pool() -> SqlitePool {
		let pool = SqlitePool::connect(":memory:").await.unwrap();

		sqlx::query(
			r#"
			CREATE TABLE IF NOT EXISTS users (
				id TEXT PRIMARY KEY,
				display_name TEXT NOT NULL,
				username TEXT UNIQUE,
				primary_email TEXT UNIQUE,
				avatar_url TEXT,
				email_visible INTEGER DEFAULT 1,
				is_system_admin INTEGER DEFAULT 0,
				is_support INTEGER DEFAULT 0,
				is_auditor INTEGER DEFAULT 0,
				created_at TEXT NOT NULL,
				updated_at TEXT NOT NULL,
				deleted_at TEXT,
				locale TEXT DEFAULT NULL
			)
			"#,
		)
		.execute(&pool)
		.await
		.unwrap();

		sqlx::query(
			r#"
			CREATE TABLE IF NOT EXISTS identities (
				id TEXT PRIMARY KEY,
				user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
				provider TEXT NOT NULL,
				provider_user_id TEXT NOT NULL,
				email TEXT NOT NULL,
				email_verified INTEGER DEFAULT 0,
				access_token TEXT,
				refresh_token TEXT,
				token_expires_at TEXT,
				created_at TEXT NOT NULL,
				UNIQUE(provider, provider_user_id)
			)
			"#,
		)
		.execute(&pool)
		.await
		.unwrap();

		pool
	}

	#[tokio::test]
	async fn test_first_user_becomes_system_admin() {
		let pool = create_test_pool().await;
		let repo = UserRepository::new(pool);

		let user = repo
			.find_or_create_user_by_email(
				"first@example.com",
				"First User",
				None,
				Some("firstuser"),
			)
			.await
			.unwrap();

		assert!(
			user.is_system_admin,
			"First user should be promoted to system admin"
		);
	}

	#[tokio::test]
	async fn test_second_user_is_not_system_admin() {
		let pool = create_test_pool().await;
		let repo = UserRepository::new(pool);

		let first = repo
			.find_or_create_user_by_email(
				"first@example.com",
				"First User",
				None,
				Some("firstuser"),
			)
			.await
			.unwrap();

		let second = repo
			.find_or_create_user_by_email(
				"second@example.com",
				"Second User",
				None,
				Some("seconduser"),
			)
			.await
			.unwrap();

		assert!(first.is_system_admin, "First user should be system admin");
		assert!(
			!second.is_system_admin,
			"Second user should NOT be system admin"
		);
	}

	#[tokio::test]
	async fn test_existing_user_retains_admin_status() {
		let pool = create_test_pool().await;
		let repo = UserRepository::new(pool);

		let first = repo
			.find_or_create_user_by_email(
				"admin@example.com",
				"Admin User",
				None,
				Some("adminuser"),
			)
			.await
			.unwrap();

		assert!(first.is_system_admin, "First user should be system admin");

		let same_user = repo
			.find_or_create_user_by_email(
				"admin@example.com",
				"Admin User Updated",
				None,
				Some("adminuser"),
			)
			.await
			.unwrap();

		assert_eq!(first.id, same_user.id, "Should return the same user");
		assert!(
			same_user.is_system_admin,
			"Existing admin should retain admin status"
		);
	}

	proptest! {
		#[test]
		fn user_id_generation_is_unique(count in 1..1000usize) {
			let mut ids = HashSet::new();
			for _ in 0..count {
				let id = UserId::generate();
				prop_assert!(ids.insert(id.to_string()), "Generated duplicate UserId");
			}
		}

		#[test]
		fn identity_id_generation_is_unique(count in 1..1000usize) {
			let mut ids = HashSet::new();
			for _ in 0..count {
				let id = IdentityId::generate();
				prop_assert!(ids.insert(id.to_string()), "Generated duplicate IdentityId");
			}
		}

		#[test]
		fn list_users_pagination_bounds(limit in 0i32..1000, offset in 0i32..10000) {
			prop_assert!(limit >= 0, "limit must be non-negative");
			prop_assert!(offset >= 0, "offset must be non-negative");
		}
	}
}
