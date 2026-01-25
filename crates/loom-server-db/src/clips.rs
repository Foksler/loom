// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Database repository for clips (code snippets).
//!
//! This module provides the database layer for clips storage.
//! The actual git storage is handled by `loom-server-clips`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::{DbError, Result};

/// Clip visibility levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipVisibility {
	Private,
	Internal,
	Public,
}

impl std::fmt::Display for ClipVisibility {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Private => write!(f, "private"),
			Self::Internal => write!(f, "internal"),
			Self::Public => write!(f, "public"),
		}
	}
}

impl std::str::FromStr for ClipVisibility {
	type Err = String;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"private" => Ok(Self::Private),
			"internal" => Ok(Self::Internal),
			"public" => Ok(Self::Public),
			_ => Err(format!("invalid visibility: {}", s)),
		}
	}
}

/// Clip record from the database.
#[derive(Debug, Clone)]
pub struct ClipRecord {
	pub id: Uuid,
	pub owner: String,
	pub name: String,
	pub description: Option<String>,
	pub visibility: ClipVisibility,
	pub created_by: Uuid,
	pub org_id: Option<Uuid>,
	pub is_fork: bool,
	pub forked_from: Option<Uuid>,
	pub file_count: u32,
	pub size_bytes: u64,
	pub language: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

/// Parameters for creating a clip.
#[derive(Debug)]
pub struct CreateClipParams {
	pub id: Uuid,
	pub owner: String,
	pub name: String,
	pub description: Option<String>,
	pub visibility: ClipVisibility,
	pub created_by: Uuid,
	pub org_id: Option<Uuid>,
	pub is_fork: bool,
	pub forked_from: Option<Uuid>,
}

/// Parameters for updating a clip.
#[derive(Debug)]
pub struct UpdateClipParams {
	pub name: Option<String>,
	pub description: Option<String>,
	pub visibility: Option<ClipVisibility>,
}

/// Trait for clips database operations.
#[async_trait]
pub trait ClipsStore: Send + Sync {
	async fn create_clip(&self, params: CreateClipParams) -> Result<ClipRecord>;
	async fn get_clip_by_id(&self, id: Uuid) -> Result<Option<ClipRecord>>;
	async fn get_clip_by_owner_name(&self, owner: &str, name: &str) -> Result<Option<ClipRecord>>;
	async fn list_user_clips(&self, user_id: Uuid, limit: u32, offset: u32)
		-> Result<Vec<ClipRecord>>;
	async fn list_org_clips(&self, org_id: Uuid, limit: u32, offset: u32)
		-> Result<Vec<ClipRecord>>;
	async fn list_public_clips(&self, limit: u32, offset: u32) -> Result<Vec<ClipRecord>>;
	async fn update_clip(&self, id: Uuid, params: UpdateClipParams) -> Result<()>;
	async fn delete_clip(&self, id: Uuid) -> Result<bool>;
	async fn update_clip_stats(
		&self,
		id: Uuid,
		file_count: u32,
		size_bytes: u64,
		language: Option<&str>,
	) -> Result<()>;
	async fn clip_name_exists(&self, owner: &str, name: &str) -> Result<bool>;
}

/// SQLite implementation of clips store.
pub struct ClipsRepository {
	pool: SqlitePool,
}

impl ClipsRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

#[async_trait]
impl ClipsStore for ClipsRepository {
	#[instrument(skip(self, params), fields(clip_id = %params.id, owner = %params.owner, name = %params.name))]
	async fn create_clip(&self, params: CreateClipParams) -> Result<ClipRecord> {
		let now = Utc::now();
		let now_str = now.to_rfc3339();

		sqlx::query(
			r#"
			INSERT INTO clips (
				id, owner, name, description, visibility,
				created_by, org_id, is_fork, forked_from,
				file_count, size_bytes, language,
				created_at, updated_at
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, NULL, ?, ?)
			"#,
		)
		.bind(params.id.to_string())
		.bind(&params.owner)
		.bind(&params.name)
		.bind(&params.description)
		.bind(params.visibility.to_string())
		.bind(params.created_by.to_string())
		.bind(params.org_id.map(|id| id.to_string()))
		.bind(params.is_fork)
		.bind(params.forked_from.map(|id| id.to_string()))
		.bind(&now_str)
		.bind(&now_str)
		.execute(&self.pool)
		.await?;

		Ok(ClipRecord {
			id: params.id,
			owner: params.owner,
			name: params.name,
			description: params.description,
			visibility: params.visibility,
			created_by: params.created_by,
			org_id: params.org_id,
			is_fork: params.is_fork,
			forked_from: params.forked_from,
			file_count: 0,
			size_bytes: 0,
			language: None,
			created_at: now,
			updated_at: now,
		})
	}

	#[instrument(skip(self), fields(clip_id = %id))]
	async fn get_clip_by_id(&self, id: Uuid) -> Result<Option<ClipRecord>> {
		let row = sqlx::query_as::<_, ClipRow>(
			r#"
			SELECT id, owner, name, description, visibility,
				   created_by, org_id, is_fork, forked_from,
				   file_count, size_bytes, language,
				   created_at, updated_at
			FROM clips
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(owner = %owner, name = %name))]
	async fn get_clip_by_owner_name(&self, owner: &str, name: &str) -> Result<Option<ClipRecord>> {
		let row = sqlx::query_as::<_, ClipRow>(
			r#"
			SELECT id, owner, name, description, visibility,
				   created_by, org_id, is_fork, forked_from,
				   file_count, size_bytes, language,
				   created_at, updated_at
			FROM clips
			WHERE owner = ? AND name = ?
			"#,
		)
		.bind(owner)
		.bind(name)
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(user_id = %user_id))]
	async fn list_user_clips(
		&self,
		user_id: Uuid,
		limit: u32,
		offset: u32,
	) -> Result<Vec<ClipRecord>> {
		let rows = sqlx::query_as::<_, ClipRow>(
			r#"
			SELECT id, owner, name, description, visibility,
				   created_by, org_id, is_fork, forked_from,
				   file_count, size_bytes, language,
				   created_at, updated_at
			FROM clips
			WHERE created_by = ? AND org_id IS NULL
			ORDER BY updated_at DESC
			LIMIT ? OFFSET ?
			"#,
		)
		.bind(user_id.to_string())
		.bind(limit as i32)
		.bind(offset as i32)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(org_id = %org_id))]
	async fn list_org_clips(&self, org_id: Uuid, limit: u32, offset: u32) -> Result<Vec<ClipRecord>> {
		let rows = sqlx::query_as::<_, ClipRow>(
			r#"
			SELECT id, owner, name, description, visibility,
				   created_by, org_id, is_fork, forked_from,
				   file_count, size_bytes, language,
				   created_at, updated_at
			FROM clips
			WHERE org_id = ?
			ORDER BY updated_at DESC
			LIMIT ? OFFSET ?
			"#,
		)
		.bind(org_id.to_string())
		.bind(limit as i32)
		.bind(offset as i32)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self))]
	async fn list_public_clips(&self, limit: u32, offset: u32) -> Result<Vec<ClipRecord>> {
		let rows = sqlx::query_as::<_, ClipRow>(
			r#"
			SELECT id, owner, name, description, visibility,
				   created_by, org_id, is_fork, forked_from,
				   file_count, size_bytes, language,
				   created_at, updated_at
			FROM clips
			WHERE visibility = 'public'
			ORDER BY updated_at DESC
			LIMIT ? OFFSET ?
			"#,
		)
		.bind(limit as i32)
		.bind(offset as i32)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self, params), fields(clip_id = %id))]
	async fn update_clip(&self, id: Uuid, params: UpdateClipParams) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		if let Some(name) = params.name {
			sqlx::query("UPDATE clips SET name = ?, updated_at = ? WHERE id = ?")
				.bind(&name)
				.bind(&now)
				.bind(id.to_string())
				.execute(&self.pool)
				.await?;
		}

		if let Some(description) = params.description {
			sqlx::query("UPDATE clips SET description = ?, updated_at = ? WHERE id = ?")
				.bind(&description)
				.bind(&now)
				.bind(id.to_string())
				.execute(&self.pool)
				.await?;
		}

		if let Some(visibility) = params.visibility {
			sqlx::query("UPDATE clips SET visibility = ?, updated_at = ? WHERE id = ?")
				.bind(visibility.to_string())
				.bind(&now)
				.bind(id.to_string())
				.execute(&self.pool)
				.await?;
		}

		Ok(())
	}

	#[instrument(skip(self), fields(clip_id = %id))]
	async fn delete_clip(&self, id: Uuid) -> Result<bool> {
		let result = sqlx::query("DELETE FROM clips WHERE id = ?")
			.bind(id.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected() > 0)
	}

	#[instrument(skip(self), fields(clip_id = %id))]
	async fn update_clip_stats(
		&self,
		id: Uuid,
		file_count: u32,
		size_bytes: u64,
		language: Option<&str>,
	) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			UPDATE clips SET
				file_count = ?, size_bytes = ?, language = ?,
				updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(file_count as i32)
		.bind(size_bytes as i64)
		.bind(language)
		.bind(&now)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(owner = %owner, name = %name))]
	async fn clip_name_exists(&self, owner: &str, name: &str) -> Result<bool> {
		let count = sqlx::query_scalar::<_, i32>(
			r#"
			SELECT COUNT(*) FROM clips WHERE owner = ? AND name = ?
			"#,
		)
		.bind(owner)
		.bind(name)
		.fetch_one(&self.pool)
		.await?;

		Ok(count > 0)
	}
}

/// Row type for SQLite.
#[derive(Debug, sqlx::FromRow)]
struct ClipRow {
	id: String,
	owner: String,
	name: String,
	description: Option<String>,
	visibility: String,
	created_by: String,
	org_id: Option<String>,
	is_fork: bool,
	forked_from: Option<String>,
	file_count: i32,
	size_bytes: i64,
	language: Option<String>,
	created_at: String,
	updated_at: String,
}

impl TryFrom<ClipRow> for ClipRecord {
	type Error = DbError;

	fn try_from(row: ClipRow) -> Result<Self> {
		Ok(ClipRecord {
			id: row.id.parse().map_err(|e| DbError::Internal(format!("{}", e)))?,
			owner: row.owner,
			name: row.name,
			description: row.description,
			visibility: row
				.visibility
				.parse()
				.map_err(|e| DbError::Internal(format!("{}", e)))?,
			created_by: row
				.created_by
				.parse()
				.map_err(|e| DbError::Internal(format!("{}", e)))?,
			org_id: row
				.org_id
				.map(|s| s.parse())
				.transpose()
				.map_err(|e: uuid::Error| DbError::Internal(format!("{}", e)))?,
			is_fork: row.is_fork,
			forked_from: row
				.forked_from
				.map(|s| s.parse())
				.transpose()
				.map_err(|e: uuid::Error| DbError::Internal(format!("{}", e)))?,
			file_count: row.file_count as u32,
			size_bytes: row.size_bytes as u64,
			language: row.language,
			created_at: DateTime::parse_from_rfc3339(&row.created_at)
				.map(|dt| dt.with_timezone(&Utc))
				.map_err(|_| DbError::Internal(format!("invalid datetime: {}", row.created_at)))?,
			updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
				.map(|dt| dt.with_timezone(&Utc))
				.map_err(|_| DbError::Internal(format!("invalid datetime: {}", row.updated_at)))?,
		})
	}
}
