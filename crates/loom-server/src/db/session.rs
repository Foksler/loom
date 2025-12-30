// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Session repository for database operations.
//!
//! This module provides database access for session management including:
//! - Web sessions (browser-based)
//! - Access tokens (CLI/VS Code)
//! - Device code flows (CLI authentication)
//! - Magic links (passwordless email auth)
//! - Impersonation sessions (admin support)

use chrono::{Duration, Utc};
use loom_auth::{
	access_token::ACCESS_TOKEN_EXPIRY_DAYS, device_code::DEVICE_CODE_EXPIRY_MINUTES,
	magic_link::MAGIC_LINK_EXPIRY_MINUTES, Session, SessionId, SessionType, UserId,
};
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;

use crate::error::ServerError;

/// Repository for session database operations.
///
/// Manages authentication sessions across multiple session types.
/// All tokens are stored as hashes, never in plaintext.
#[derive(Clone)]
pub struct SessionRepository {
	pool: SqlitePool,
}

impl SessionRepository {
	/// Create a new session repository with the given pool.
	///
	/// # Arguments
	/// * `pool` - SQLite connection pool
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	/// Create a new web session.
	///
	/// # Arguments
	/// * `session` - The session metadata
	/// * `token_hash` - SHA-256 hash of the session token (never store plaintext)
	///
	/// # Errors
	/// Returns `ServerError::Database` if insert fails (e.g., duplicate ID).
	///
	/// # Database Constraints
	/// - `id` must be unique
	/// - `user_id` must reference an existing user
	#[tracing::instrument(skip(self, session, token_hash), fields(session_id = %session.id, user_id = %session.user_id))]
	pub async fn create_session(
		&self,
		session: &Session,
		token_hash: &str,
	) -> Result<(), ServerError> {
		sqlx::query(
			r#"
			INSERT INTO sessions (
				id, user_id, session_type, token_hash,
				created_at, last_used_at, expires_at,
				ip_address, user_agent, geo_city, geo_country
			) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(session.id.to_string())
		.bind(session.user_id.to_string())
		.bind(session.session_type.to_string())
		// Note: token_hash is intentionally not logged
		.bind(token_hash)
		.bind(session.created_at.to_rfc3339())
		.bind(session.last_used_at.to_rfc3339())
		.bind(session.expires_at.to_rfc3339())
		.bind(&session.ip_address)
		.bind(&session.user_agent)
		.bind(&session.geo_city)
		.bind(&session.geo_country)
		.execute(&self.pool)
		.await?;

		tracing::debug!(session_id = %session.id, user_id = %session.user_id, "session created");
		Ok(())
	}

	/// Get a session by its token hash.
	///
	/// # Arguments
	/// * `token_hash` - SHA-256 hash of the session token
	///
	/// # Returns
	/// `None` if no session exists with this hash.
	///
	/// # Note
	/// Does not check expiry - caller should verify `expires_at`.
	#[tracing::instrument(skip(self, token_hash))]
	pub async fn get_session_by_token_hash(
		&self,
		token_hash: &str,
	) -> Result<Option<Session>, ServerError> {
		let row = sqlx::query(
			r#"
			SELECT id, user_id, session_type, created_at, last_used_at, expires_at,
				   ip_address, user_agent, geo_city, geo_country
			FROM sessions
			WHERE token_hash = ?
			"#,
		)
		.bind(token_hash)
		.fetch_optional(&self.pool)
		.await?;

		match row {
			Some(row) => {
				let session = parse_session_row(&row)?;
				tracing::debug!(session_id = %session.id, user_id = %session.user_id, "session found by token hash");
				Ok(Some(session))
			}
			None => Ok(None),
		}
	}

	/// Get all sessions for a user.
	///
	/// # Arguments
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// List of sessions ordered by most recently used first.
	#[tracing::instrument(skip(self), fields(user_id = %user_id))]
	pub async fn get_sessions_for_user(
		&self,
		user_id: &UserId,
	) -> Result<Vec<Session>, ServerError> {
		let rows = sqlx::query(
			r#"
			SELECT id, user_id, session_type, created_at, last_used_at, expires_at,
				   ip_address, user_agent, geo_city, geo_country
			FROM sessions
			WHERE user_id = ?
			ORDER BY last_used_at DESC
			"#,
		)
		.bind(user_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		let mut sessions = Vec::with_capacity(rows.len());
		for row in rows {
			sessions.push(parse_session_row(&row)?);
		}
		tracing::debug!(user_id = %user_id, count = sessions.len(), "retrieved sessions for user");
		Ok(sessions)
	}

	/// Update the last used timestamp for a session.
	///
	/// Also extends the expiry time by the session lifetime.
	///
	/// # Arguments
	/// * `id` - The session's UUID
	#[tracing::instrument(skip(self), fields(session_id = %id))]
	pub async fn update_session_last_used(&self, id: &SessionId) -> Result<(), ServerError> {
		let now = Utc::now();
		let expires_at = now + Duration::days(loom_auth::SESSION_EXPIRY_DAYS);

		sqlx::query(
			r#"
			UPDATE sessions
			SET last_used_at = ?, expires_at = ?
			WHERE id = ?
			"#,
		)
		.bind(now.to_rfc3339())
		.bind(expires_at.to_rfc3339())
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		tracing::debug!(session_id = %id, "session last_used updated");
		Ok(())
	}

	/// Delete a session by ID.
	///
	/// # Arguments
	/// * `id` - The session's UUID
	///
	/// # Returns
	/// `true` if a session was deleted, `false` if not found.
	#[tracing::instrument(skip(self), fields(session_id = %id))]
	pub async fn delete_session(&self, id: &SessionId) -> Result<bool, ServerError> {
		let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
			.bind(id.to_string())
			.execute(&self.pool)
			.await?;

		let deleted = result.rows_affected() > 0;
		if deleted {
			tracing::debug!(session_id = %id, "session deleted");
		}
		Ok(deleted)
	}

	/// Delete all sessions for a user (logout everywhere).
	///
	/// # Arguments
	/// * `user_id` - The user's UUID
	///
	/// # Returns
	/// Number of sessions deleted.
	#[tracing::instrument(skip(self), fields(user_id = %user_id))]
	pub async fn delete_all_sessions_for_user(&self, user_id: &UserId) -> Result<i64, ServerError> {
		let result = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
			.bind(user_id.to_string())
			.execute(&self.pool)
			.await?;

		let count = result.rows_affected() as i64;
		tracing::debug!(user_id = %user_id, count, "deleted all sessions for user");
		Ok(count)
	}

	/// Create a new access token for CLI/VS Code.
	///
	/// # Arguments
	/// * `user_id` - The user's UUID
	/// * `token_hash` - SHA-256 hash of the token (never store plaintext)
	/// * `label` - User-provided label for the token
	/// * `session_type` - Type of client (CLI, VSCode)
	///
	/// # Returns
	/// The generated token ID.
	#[tracing::instrument(skip(self, token_hash), fields(user_id = %user_id, session_type = %session_type))]
	pub async fn create_access_token(
		&self,
		user_id: &UserId,
		token_hash: &str,
		label: &str,
		session_type: SessionType,
	) -> Result<String, ServerError> {
		let id = Uuid::new_v4().to_string();
		let now = Utc::now();
		let expires_at = now + Duration::days(ACCESS_TOKEN_EXPIRY_DAYS);

		sqlx::query(
			r#"
			INSERT INTO access_tokens (
				id, user_id, token_hash, label, session_type,
				created_at, expires_at
			) VALUES (?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(user_id.to_string())
		// Note: token_hash is intentionally not logged
		.bind(token_hash)
		.bind(label)
		.bind(session_type.to_string())
		.bind(now.to_rfc3339())
		.bind(expires_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		tracing::debug!(token_id = %id, user_id = %user_id, "access token created");
		Ok(id)
	}

	/// Get an access token by its hash.
	///
	/// # Arguments
	/// * `token_hash` - SHA-256 hash of the token
	///
	/// # Returns
	/// Tuple of (token_id, user_id) if found and not revoked/expired.
	#[tracing::instrument(skip(self, token_hash))]
	pub async fn get_access_token_by_hash(
		&self,
		token_hash: &str,
	) -> Result<Option<(String, UserId)>, ServerError> {
		let row = sqlx::query(
			r#"
			SELECT id, user_id
			FROM access_tokens
			WHERE token_hash = ?
			  AND revoked_at IS NULL
			  AND expires_at > datetime('now')
			"#,
		)
		.bind(token_hash)
		.fetch_optional(&self.pool)
		.await?;

		match row {
			Some(row) => {
				let id: String = row.get("id");
				let user_id_str: String = row.get("user_id");
				let user_id = Uuid::parse_str(&user_id_str)
					.map_err(|e| ServerError::Internal(format!("Invalid user_id UUID: {e}")))?;
				tracing::debug!(token_id = %id, user_id = %user_id_str, "access token found");
				Ok(Some((id, UserId::new(user_id))))
			}
			None => Ok(None),
		}
	}

	/// Update the last used timestamp for an access token.
	///
	/// Also extends the expiry time.
	///
	/// # Arguments
	/// * `id` - The token's UUID
	#[tracing::instrument(skip(self), fields(token_id = %id))]
	pub async fn update_access_token_last_used(&self, id: &str) -> Result<(), ServerError> {
		let now = Utc::now();
		let expires_at = now + Duration::days(ACCESS_TOKEN_EXPIRY_DAYS);

		sqlx::query(
			r#"
			UPDATE access_tokens
			SET last_used_at = ?, expires_at = ?
			WHERE id = ?
			"#,
		)
		.bind(now.to_rfc3339())
		.bind(expires_at.to_rfc3339())
		.bind(id)
		.execute(&self.pool)
		.await?;

		tracing::debug!(token_id = %id, "access token last_used updated");
		Ok(())
	}

	/// Revoke an access token.
	///
	/// # Arguments
	/// * `id` - The token's UUID
	///
	/// # Returns
	/// `true` if token was revoked, `false` if already revoked or not found.
	#[tracing::instrument(skip(self), fields(token_id = %id))]
	pub async fn revoke_access_token(&self, id: &str) -> Result<bool, ServerError> {
		let now = Utc::now();

		let result = sqlx::query(
			r#"
			UPDATE access_tokens
			SET revoked_at = ?
			WHERE id = ? AND revoked_at IS NULL
			"#,
		)
		.bind(now.to_rfc3339())
		.bind(id)
		.execute(&self.pool)
		.await?;

		let revoked = result.rows_affected() > 0;
		if revoked {
			tracing::debug!(token_id = %id, "access token revoked");
		}
		Ok(revoked)
	}

	/// Create a new device code for CLI authentication.
	///
	/// # Arguments
	/// * `device_code` - The device code (stored as-is, shown to polling client)
	/// * `user_code` - The user code (shown to user for entry in browser)
	///
	/// # Note
	/// Device codes expire after `DEVICE_CODE_EXPIRY_MINUTES`.
	#[tracing::instrument(skip(self, device_code, user_code))]
	pub async fn create_device_code(
		&self,
		device_code: &str,
		user_code: &str,
	) -> Result<(), ServerError> {
		let now = Utc::now();
		let expires_at = now + Duration::minutes(DEVICE_CODE_EXPIRY_MINUTES);

		sqlx::query(
			r#"
			INSERT INTO device_codes (device_code, user_code, created_at, expires_at)
			VALUES (?, ?, ?, ?)
			"#,
		)
		// Note: device_code and user_code are intentionally not logged
		.bind(device_code)
		.bind(user_code)
		.bind(now.to_rfc3339())
		.bind(expires_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		tracing::debug!("device code created");
		Ok(())
	}

	/// Get a device code by its value.
	///
	/// # Arguments
	/// * `device_code` - The device code
	///
	/// # Returns
	/// Tuple of (user_code, user_id, is_completed) if found.
	/// `user_id` is `None` until user approves.
	#[tracing::instrument(skip(self, device_code))]
	pub async fn get_device_code(
		&self,
		device_code: &str,
	) -> Result<Option<(String, Option<UserId>, bool)>, ServerError> {
		let row = sqlx::query(
			r#"
			SELECT user_code, user_id, completed_at, expires_at
			FROM device_codes
			WHERE device_code = ?
			"#,
		)
		.bind(device_code)
		.fetch_optional(&self.pool)
		.await?;

		match row {
			Some(row) => {
				let user_code: String = row.get("user_code");
				let user_id_str: Option<String> = row.get("user_id");
				let completed_at: Option<String> = row.get("completed_at");

				let user_id = if let Some(uid) = user_id_str {
					let uuid = Uuid::parse_str(&uid)
						.map_err(|e| ServerError::Internal(format!("Invalid user_id UUID: {e}")))?;
					Some(UserId::new(uuid))
				} else {
					None
				};

				let is_completed = completed_at.is_some();

				Ok(Some((user_code, user_id, is_completed)))
			}
			None => Ok(None),
		}
	}

	/// Complete a device code flow after user approval.
	///
	/// # Arguments
	/// * `user_code` - The user code entered by the user
	/// * `user_id` - The approving user's UUID
	///
	/// # Returns
	/// `true` if completed, `false` if code not found, expired, or already used.
	#[tracing::instrument(skip(self, user_code), fields(user_id = %user_id))]
	pub async fn complete_device_code(
		&self,
		user_code: &str,
		user_id: &UserId,
	) -> Result<bool, ServerError> {
		let now = Utc::now();

		let result = sqlx::query(
			r#"
			UPDATE device_codes
			SET user_id = ?, completed_at = ?
			WHERE user_code = ?
			  AND completed_at IS NULL
			  AND expires_at > datetime('now')
			"#,
		)
		.bind(user_id.to_string())
		.bind(now.to_rfc3339())
		.bind(user_code)
		.execute(&self.pool)
		.await?;

		let completed = result.rows_affected() > 0;
		if completed {
			tracing::debug!(user_id = %user_id, "device code completed");
		}
		Ok(completed)
	}

	/// Clean up expired device codes.
	///
	/// # Returns
	/// Number of expired codes deleted.
	#[tracing::instrument(skip(self))]
	pub async fn cleanup_expired_device_codes(&self) -> Result<i64, ServerError> {
		let result = sqlx::query(
			r#"
			DELETE FROM device_codes
			WHERE expires_at < datetime('now')
			"#,
		)
		.execute(&self.pool)
		.await?;

		let count = result.rows_affected() as i64;
		if count > 0 {
			tracing::debug!(count, "cleaned up expired device codes");
		}
		Ok(count)
	}

	/// Create a new magic link for passwordless email authentication.
	///
	/// # Arguments
	/// * `email` - The recipient's email address
	/// * `token_hash` - SHA-256 hash of the magic link token
	///
	/// # Returns
	/// The generated magic link ID.
	///
	/// # Note
	/// Magic links expire after `MAGIC_LINK_EXPIRY_MINUTES`.
	#[tracing::instrument(skip(self, email, token_hash))]
	pub async fn create_magic_link(
		&self,
		email: &str,
		token_hash: &str,
	) -> Result<String, ServerError> {
		let id = Uuid::new_v4().to_string();
		let now = Utc::now();
		let expires_at = now + Duration::minutes(MAGIC_LINK_EXPIRY_MINUTES);

		sqlx::query(
			r#"
			INSERT INTO magic_links (id, email, token_hash, created_at, expires_at)
			VALUES (?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(email)
		// Note: token_hash is intentionally not logged
		.bind(token_hash)
		.bind(now.to_rfc3339())
		.bind(expires_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		tracing::debug!(magic_link_id = %id, "magic link created");
		Ok(id)
	}

	/// Get a magic link by its token hash.
	///
	/// # Arguments
	/// * `token_hash` - SHA-256 hash of the magic link token
	///
	/// # Returns
	/// Tuple of (id, email, is_used) if found.
	#[tracing::instrument(skip(self, token_hash))]
	pub async fn get_magic_link_by_hash(
		&self,
		token_hash: &str,
	) -> Result<Option<(String, String, bool)>, ServerError> {
		let row = sqlx::query(
			r#"
			SELECT id, email, used_at, expires_at
			FROM magic_links
			WHERE token_hash = ?
			"#,
		)
		.bind(token_hash)
		.fetch_optional(&self.pool)
		.await?;

		match row {
			Some(row) => {
				let id: String = row.get("id");
				let email: String = row.get("email");
				let used_at: Option<String> = row.get("used_at");
				let is_used = used_at.is_some();

				tracing::debug!(magic_link_id = %id, "magic link found");
				Ok(Some((id, email, is_used)))
			}
			None => Ok(None),
		}
	}

	/// Get all pending (non-expired, non-used) magic links for Argon2 verification.
	///
	/// # Returns
	/// Vec of (id, email, token_hash) tuples.
	///
	/// # Security Note
	/// This returns token hashes for verification purposes only.
	#[tracing::instrument(skip(self))]
	pub async fn get_pending_magic_links(
		&self,
	) -> Result<Vec<(String, String, String)>, ServerError> {
		let now = Utc::now().to_rfc3339();

		let rows = sqlx::query(
			r#"
			SELECT id, email, token_hash
			FROM magic_links
			WHERE used_at IS NULL AND expires_at > ?
			"#,
		)
		.bind(&now)
		.fetch_all(&self.pool)
		.await?;

		let results = rows
			.iter()
			.map(|row| {
				let id: String = row.get("id");
				let email: String = row.get("email");
				let token_hash: String = row.get("token_hash");
				(id, email, token_hash)
			})
			.collect();

		Ok(results)
	}

	/// Atomically claim a magic link for use.
	///
	/// This uses an atomic UPDATE with `used_at IS NULL` condition to ensure
	/// only ONE request can successfully claim the link, preventing TOCTOU
	/// race conditions where multiple concurrent requests could verify the
	/// same magic link.
	///
	/// # Arguments
	/// * `id` - The magic link's UUID
	///
	/// # Returns
	/// `true` if the link was successfully claimed (was unused), `false` if
	/// already used or not found.
	#[tracing::instrument(skip(self), fields(magic_link_id = %id))]
	pub async fn claim_magic_link(&self, id: &str) -> Result<bool, ServerError> {
		let now = Utc::now();

		let result = sqlx::query(
			r#"
			UPDATE magic_links
			SET used_at = ?
			WHERE id = ? AND used_at IS NULL
			"#,
		)
		.bind(now.to_rfc3339())
		.bind(id)
		.execute(&self.pool)
		.await?;

		let claimed = result.rows_affected() > 0;
		if claimed {
			tracing::debug!(magic_link_id = %id, "magic link claimed successfully");
		} else {
			tracing::debug!(magic_link_id = %id, "magic link already used or not found");
		}
		Ok(claimed)
	}

	/// Invalidate all magic links for an email address.
	///
	/// Called when a new magic link is requested to prevent old links from working.
	///
	/// # Arguments
	/// * `email` - The email address
	#[tracing::instrument(skip(self, email))]
	pub async fn invalidate_magic_links_for_email(&self, email: &str) -> Result<(), ServerError> {
		let now = Utc::now();

		sqlx::query(
			r#"
			UPDATE magic_links
			SET used_at = ?
			WHERE email = ? AND used_at IS NULL
			"#,
		)
		.bind(now.to_rfc3339())
		.bind(email)
		.execute(&self.pool)
		.await?;

		tracing::debug!("invalidated magic links for email");
		Ok(())
	}

	/// Create an impersonation session for admin support.
	///
	/// # Arguments
	/// * `admin_user_id` - The admin user's UUID
	/// * `target_user_id` - The user being impersonated
	/// * `reason` - Justification for impersonation (audit trail)
	///
	/// # Returns
	/// The generated impersonation session ID.
	#[tracing::instrument(skip(self), fields(admin_user_id = %admin_user_id, target_user_id = %target_user_id))]
	pub async fn create_impersonation_session(
		&self,
		admin_user_id: &UserId,
		target_user_id: &UserId,
		reason: &str,
	) -> Result<String, ServerError> {
		let id = Uuid::new_v4().to_string();
		let now = Utc::now();

		sqlx::query(
			r#"
			INSERT INTO impersonation_sessions (
				id, admin_user_id, target_user_id, reason, started_at, created_at
			) VALUES (?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(admin_user_id.to_string())
		.bind(target_user_id.to_string())
		.bind(reason)
		.bind(now.to_rfc3339())
		.bind(now.to_rfc3339())
		.execute(&self.pool)
		.await?;

		tracing::info!(
			impersonation_id = %id,
			admin_user_id = %admin_user_id,
			target_user_id = %target_user_id,
			"impersonation session started"
		);
		Ok(id)
	}

	/// Get an active impersonation session for an admin user.
	///
	/// # Arguments
	/// * `admin_user_id` - The admin user's UUID
	///
	/// # Returns
	/// Tuple of (session_id, target_user_id) if active session exists.
	#[tracing::instrument(skip(self), fields(admin_user_id = %admin_user_id))]
	pub async fn get_active_impersonation_session(
		&self,
		admin_user_id: &UserId,
	) -> Result<Option<(String, UserId)>, ServerError> {
		let row = sqlx::query(
			r#"
			SELECT id, target_user_id
			FROM impersonation_sessions
			WHERE admin_user_id = ? AND ended_at IS NULL
			ORDER BY started_at DESC
			LIMIT 1
			"#,
		)
		.bind(admin_user_id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		match row {
			Some(row) => {
				let id: String = row.get("id");
				let target_user_id_str: String = row.get("target_user_id");
				let target_user_id = Uuid::parse_str(&target_user_id_str)
					.map_err(|e| ServerError::Internal(format!("Invalid target_user_id UUID: {e}")))?;
				tracing::debug!(impersonation_id = %id, target_user_id = %target_user_id_str, "active impersonation session found");
				Ok(Some((id, UserId::new(target_user_id))))
			}
			None => Ok(None),
		}
	}

	/// End an impersonation session.
	///
	/// # Arguments
	/// * `session_id` - The impersonation session's UUID
	///
	/// # Returns
	/// `true` if session was ended, `false` if already ended or not found.
	#[tracing::instrument(skip(self), fields(impersonation_id = %session_id))]
	pub async fn end_impersonation_session(&self, session_id: &str) -> Result<bool, ServerError> {
		let now = Utc::now();

		let result = sqlx::query(
			r#"
			UPDATE impersonation_sessions
			SET ended_at = ?
			WHERE id = ? AND ended_at IS NULL
			"#,
		)
		.bind(now.to_rfc3339())
		.bind(session_id)
		.execute(&self.pool)
		.await?;

		let ended = result.rows_affected() > 0;
		if ended {
			tracing::info!(impersonation_id = %session_id, "impersonation session ended");
		}
		Ok(ended)
	}

	/// End all active impersonation sessions for an admin user.
	///
	/// # Arguments
	/// * `admin_user_id` - The admin user's UUID
	///
	/// # Returns
	/// Number of sessions ended.
	#[tracing::instrument(skip(self), fields(admin_user_id = %admin_user_id))]
	pub async fn end_all_impersonation_sessions(
		&self,
		admin_user_id: &UserId,
	) -> Result<i64, ServerError> {
		let now = Utc::now();

		let result = sqlx::query(
			r#"
			UPDATE impersonation_sessions
			SET ended_at = ?
			WHERE admin_user_id = ? AND ended_at IS NULL
			"#,
		)
		.bind(now.to_rfc3339())
		.bind(admin_user_id.to_string())
		.execute(&self.pool)
		.await?;

		let count = result.rows_affected() as i64;
		if count > 0 {
			tracing::info!(admin_user_id = %admin_user_id, count, "ended all impersonation sessions");
		}
		Ok(count)
	}
}

fn parse_session_row(row: &sqlx::sqlite::SqliteRow) -> Result<Session, ServerError> {
	use chrono::DateTime;

	let id_str: String = row.get("id");
	let user_id_str: String = row.get("user_id");
	let session_type_str: String = row.get("session_type");
	let created_at_str: String = row.get("created_at");
	let last_used_at_str: String = row.get("last_used_at");
	let expires_at_str: String = row.get("expires_at");
	let ip_address: Option<String> = row.get("ip_address");
	let user_agent: Option<String> = row.get("user_agent");
	let geo_city: Option<String> = row.get("geo_city");
	let geo_country: Option<String> = row.get("geo_country");

	let id = Uuid::parse_str(&id_str)
		.map_err(|e| ServerError::Internal(format!("Invalid session id UUID: {e}")))?;
	let user_id = Uuid::parse_str(&user_id_str)
		.map_err(|e| ServerError::Internal(format!("Invalid user_id UUID: {e}")))?;

	let session_type = match session_type_str.as_str() {
		"web" => SessionType::Web,
		"cli" => SessionType::Cli,
		"vscode" => SessionType::VsCode,
		_ => {
			return Err(ServerError::Internal(format!(
				"Unknown session type: {session_type_str}"
			)))
		}
	};

	let created_at = DateTime::parse_from_rfc3339(&created_at_str)
		.map_err(|e| ServerError::Internal(format!("Invalid created_at: {e}")))?
		.with_timezone(&Utc);
	let last_used_at = DateTime::parse_from_rfc3339(&last_used_at_str)
		.map_err(|e| ServerError::Internal(format!("Invalid last_used_at: {e}")))?
		.with_timezone(&Utc);
	let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)
		.map_err(|e| ServerError::Internal(format!("Invalid expires_at: {e}")))?
		.with_timezone(&Utc);

	Ok(Session {
		id: SessionId::new(id),
		user_id: UserId::new(user_id),
		session_type,
		created_at,
		last_used_at,
		expires_at,
		ip_address,
		user_agent,
		geo_city,
		geo_country,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::db::{create_pool, run_migrations};
	use proptest::prelude::*;
	use std::collections::HashSet;
	use tempfile::tempdir;

	async fn create_test_session_repo() -> (SessionRepository, tempfile::TempDir) {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
		let pool = create_pool(&db_url).await.unwrap();
		run_migrations(&pool).await.unwrap();
		let repo = SessionRepository::new(pool);
		(repo, dir)
	}

	#[tokio::test]
	async fn test_magic_link_single_use() {
		let (repo, _dir) = create_test_session_repo().await;

		let link_id = repo
			.create_magic_link("test@example.com", "test_token_hash")
			.await
			.unwrap();

		let result1 = repo.claim_magic_link(&link_id).await.unwrap();
		assert!(result1, "First claim should succeed");

		let result2 = repo.claim_magic_link(&link_id).await.unwrap();
		assert!(!result2, "Second claim should fail");
	}

	#[tokio::test]
	async fn test_magic_link_claim_nonexistent() {
		let (repo, _dir) = create_test_session_repo().await;

		let result = repo
			.claim_magic_link("nonexistent-uuid")
			.await
			.unwrap();
		assert!(!result, "Claiming nonexistent link should return false");
	}

	#[tokio::test]
	async fn test_magic_link_concurrent_claims() {
		let (repo, _dir) = create_test_session_repo().await;

		let link_id = repo
			.create_magic_link("concurrent@example.com", "concurrent_token_hash")
			.await
			.unwrap();

		let handles: Vec<_> = (0..10)
			.map(|_| {
				let repo = repo.clone();
				let id = link_id.clone();
				tokio::spawn(async move { repo.claim_magic_link(&id).await.unwrap() })
			})
			.collect();

		let results: Vec<bool> = futures::future::join_all(handles)
			.await
			.into_iter()
			.map(|r| r.unwrap())
			.collect();

		let success_count = results.iter().filter(|&&r| r).count();
		assert_eq!(
			success_count, 1,
			"Exactly one concurrent claim should succeed, got {success_count}"
		);
	}

	proptest! {
		#[test]
		fn session_id_generation_is_unique(count in 1..1000usize) {
			let mut ids = HashSet::new();
			for _ in 0..count {
				let id = SessionId::generate();
				prop_assert!(ids.insert(id.to_string()), "Generated duplicate SessionId");
			}
		}

		#[test]
		fn token_hash_is_deterministic(input in ".*") {
			use sha2::{Sha256, Digest};
			let hash1 = format!("{:x}", Sha256::digest(input.as_bytes()));
			let hash2 = format!("{:x}", Sha256::digest(input.as_bytes()));
			prop_assert_eq!(hash1, hash2, "Hash should be deterministic");
		}

		#[test]
		fn access_token_expiry_is_positive(_days in 1i64..365) {
			prop_assert!(ACCESS_TOKEN_EXPIRY_DAYS > 0, "Access token expiry must be positive");
		}
	}
}
