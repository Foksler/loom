// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! SQLite database operations for thread persistence.
//!
//! This module provides repository patterns for database access:
//! - [`UserRepository`] - User and identity management
//! - [`SessionRepository`] - Authentication sessions and tokens
//! - [`OrgRepository`] - Organization and membership management
//! - [`TeamRepository`] - Team management within organizations
//! - [`ApiKeyRepository`] - API key management
//! - [`AuditRepository`] - Security audit logging
//! - [`ThreadRepository`] - Thread/conversation persistence
//! - [`ShareRepository`] - Share links and support access

pub mod api_key;
mod audit;
mod org;
mod session;
mod share;
pub mod team;
mod thread;
mod user;

use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteSynchronous};
use std::str::FromStr;

use crate::error::ServerError;

pub use api_key::ApiKeyRepository;
pub use audit::AuditRepository;
pub use org::OrgRepository;
pub use session::SessionRepository;
pub use share::ShareRepository;
pub use team::TeamRepository;
pub use thread::ThreadRepository;
pub use user::UserRepository;

/// GitHub App installation info stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubInstallation {
	pub installation_id: i64,
	pub account_id: i64,
	pub account_login: String,
	pub account_type: String,
	pub app_slug: Option<String>,
	pub repositories_selection: String,
	pub suspended_at: Option<String>,
	pub created_at: String,
	pub updated_at: String,
}

/// GitHub repository linked to an installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubRepo {
	pub repository_id: i64,
	pub owner: String,
	pub name: String,
	pub full_name: String,
	pub private: bool,
	pub default_branch: Option<String>,
}

/// Installation info with minimal fields for lookups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubInstallationInfo {
	pub installation_id: i64,
	pub account_login: String,
	pub account_type: String,
	pub repositories_selection: String,
}

/// A search result hit with relevance score.
#[derive(Debug, Clone)]
pub struct ThreadSearchHit {
	pub summary: loom_thread::ThreadSummary,
	pub score: f64,
}

/// Create a SqlitePool with WAL mode and common settings.
///
/// # Arguments
/// * `database_url` - SQLite connection string (e.g., "sqlite:./loom.db")
///
/// # Errors
/// Returns `ServerError::Internal` if the URL is invalid or connection fails.
#[tracing::instrument(skip(database_url))]
pub async fn create_pool(database_url: &str) -> Result<SqlitePool, ServerError> {
	let options = SqliteConnectOptions::from_str(database_url)
		.map_err(|e| ServerError::Internal(format!("Invalid database URL: {e}")))?
		.journal_mode(SqliteJournalMode::Wal)
		.synchronous(SqliteSynchronous::Normal)
		.create_if_missing(true);

	let pool = SqlitePool::connect_with(options).await?;

	tracing::debug!("database pool created");
	Ok(pool)
}

/// Run all database migrations (001-014).
///
/// # Arguments
/// * `pool` - SQLite connection pool
///
/// # Errors
/// Returns `ServerError::Database` if migrations fail.
///
/// # Note
/// Migrations are idempotent - safe to run multiple times.
#[tracing::instrument(skip(pool))]
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), ServerError> {
	let m1 = include_str!("../../migrations/001_create_threads.sql");
	sqlx::query(m1).execute(pool).await?;

	let m2 = include_str!("../../migrations/002_add_visibility.sql");
	if let Err(e) = sqlx::query(m2).execute(pool).await {
		let msg = e.to_string();
		if !msg.contains("duplicate column name: visibility")
			&& !msg.contains("duplicate column")
			&& !msg.contains("already exists")
		{
			return Err(e.into());
		}
	}

	let m3 = include_str!("../../migrations/003_add_git_metadata.sql");
	if let Err(e) = sqlx::query(m3).execute(pool).await {
		let msg = e.to_string();
		if !msg.contains("duplicate column") && !msg.contains("already exists") {
			return Err(e.into());
		}
	}

	let m4 = include_str!("../../migrations/004_git_repos_and_commits.sql");
	for stmt in m4.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("duplicate column")
				&& !msg.contains("already exists")
				&& !msg.contains("table repos already exists")
				&& !msg.contains("table thread_commits already exists")
			{
				return Err(e.into());
			}
		}
	}

	let m5 = include_str!("../../migrations/005_thread_fts.sql");

	if let Some(vt_end) = m5.find(");") {
		let create_vt = &m5[..vt_end + 2];
		if let Err(e) = sqlx::query(create_vt.trim()).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("table thread_fts already exists") {
				tracing::warn!(error = %e, "FTS CREATE VIRTUAL TABLE failed");
			}
		}

		let remaining = &m5[vt_end + 2..];
		for trigger_block in remaining.split("END;") {
			let trigger = trigger_block.trim();
			if trigger.is_empty() || !trigger.contains("CREATE TRIGGER") {
				continue;
			}
			let full_trigger = format!("{trigger} END;");
			if let Err(e) = sqlx::query(&full_trigger).execute(pool).await {
				let msg = e.to_string();
				if !msg.contains("already exists") && !msg.contains("trigger") {
					tracing::warn!(error = %e, stmt = %full_trigger.chars().take(80).collect::<String>(), "FTS trigger creation failed");
				}
			}
		}
	}

	let m6 = include_str!("../../migrations/006_cse_cache.sql");
	for stmt in m6.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m7 = include_str!("../../migrations/007_github_app.sql");
	for stmt in m7.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists")
				&& !msg.contains("duplicate column")
				&& !msg.contains("table github_installations already exists")
				&& !msg.contains("table github_installation_repos already exists")
			{
				return Err(e.into());
			}
		}
	}

	let m8 = include_str!("../../migrations/008_auth_users.sql");
	for stmt in m8.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m9 = include_str!("../../migrations/009_auth_sessions.sql");
	for stmt in m9.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m10 = include_str!("../../migrations/010_auth_orgs.sql");
	for stmt in m10.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m11 = include_str!("../../migrations/011_auth_teams.sql");
	for stmt in m11.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m12 = include_str!("../../migrations/012_auth_api_keys.sql");
	for stmt in m12.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m13 = include_str!("../../migrations/013_auth_threads_ext.sql");
	for stmt in m13.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m14 = include_str!("../../migrations/014_auth_audit.sql");
	for stmt in m14.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m15 = include_str!("../../migrations/015_impersonation_sessions.sql");
	for stmt in m15.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m16 = include_str!("../../migrations/016_user_locale.sql");
	for stmt in m16.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	let m17 = include_str!("../../migrations/017_fix_sessions_token_hash.sql");
	for stmt in m17.split(';').filter(|s| !s.trim().is_empty()) {
		if let Err(e) = sqlx::query(stmt).execute(pool).await {
			let msg = e.to_string();
			if !msg.contains("already exists") && !msg.contains("duplicate column") {
				return Err(e.into());
			}
		}
	}

	tracing::debug!("database migrations complete");
	Ok(())
}
