// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Repository layer for clips database operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tracing::instrument;

use crate::error::{ClipsError, Result};
use crate::types::{Clip, ClipId, OrgId, UserId};

/// Repository trait for clip operations.
#[async_trait]
pub trait ClipsRepository: Send + Sync {
	/// Create a new clip.
	async fn create_clip(&self, clip: &Clip) -> Result<()>;

	/// Get a clip by ID.
	async fn get_clip_by_id(&self, id: ClipId) -> Result<Option<Clip>>;

	/// Get a clip by owner and name.
	async fn get_clip_by_owner_name(&self, owner: &str, name: &str) -> Result<Option<Clip>>;

	/// List clips for a user.
	async fn list_user_clips(&self, user_id: UserId, limit: u32, offset: u32) -> Result<Vec<Clip>>;

	/// List clips for an organization.
	async fn list_org_clips(&self, org_id: OrgId, limit: u32, offset: u32) -> Result<Vec<Clip>>;

	/// List public clips.
	async fn list_public_clips(&self, limit: u32, offset: u32) -> Result<Vec<Clip>>;

	/// Update a clip's metadata.
	async fn update_clip(&self, clip: &Clip) -> Result<()>;

	/// Delete a clip.
	async fn delete_clip(&self, id: ClipId) -> Result<bool>;

	/// Update clip statistics (file count, size).
	async fn update_clip_stats(
		&self,
		id: ClipId,
		file_count: u32,
		size_bytes: u64,
		language: Option<&str>,
	) -> Result<()>;

	/// Check if a clip name exists for an owner.
	async fn clip_name_exists(&self, owner: &str, name: &str) -> Result<bool>;
}

/// Store trait that combines repository functionality.
#[async_trait]
pub trait ClipsStore: ClipsRepository {}

/// SQLite implementation of the clips repository.
#[derive(Clone)]
pub struct SqliteClipsRepository {
	pool: SqlitePool,
}

impl SqliteClipsRepository {
	/// Create a new SQLite clips repository.
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

#[async_trait]
impl ClipsStore for SqliteClipsRepository {}

#[async_trait]
impl ClipsRepository for SqliteClipsRepository {
	#[instrument(skip(self, clip), fields(clip_id = %clip.id, owner = %clip.owner, name = %clip.name))]
	async fn create_clip(&self, clip: &Clip) -> Result<()> {
		sqlx::query(
			r#"
			INSERT INTO clips (
				id, owner, name, description, visibility,
				created_by, org_id, is_fork, forked_from,
				file_count, size_bytes, language,
				created_at, updated_at
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(clip.id.0.to_string())
		.bind(&clip.owner)
		.bind(&clip.name)
		.bind(&clip.description)
		.bind(clip.visibility.to_string())
		.bind(clip.created_by.0.to_string())
		.bind(clip.org_id.map(|id| id.0.to_string()))
		.bind(clip.is_fork)
		.bind(clip.forked_from.map(|id| id.0.to_string()))
		.bind(clip.file_count as i32)
		.bind(clip.size_bytes as i64)
		.bind(&clip.language)
		.bind(clip.created_at.to_rfc3339())
		.bind(clip.updated_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(clip_id = %id))]
	async fn get_clip_by_id(&self, id: ClipId) -> Result<Option<Clip>> {
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
		.bind(id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(owner = %owner, name = %name))]
	async fn get_clip_by_owner_name(&self, owner: &str, name: &str) -> Result<Option<Clip>> {
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
	async fn list_user_clips(&self, user_id: UserId, limit: u32, offset: u32) -> Result<Vec<Clip>> {
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
		.bind(user_id.0.to_string())
		.bind(limit as i32)
		.bind(offset as i32)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(org_id = %org_id))]
	async fn list_org_clips(&self, org_id: OrgId, limit: u32, offset: u32) -> Result<Vec<Clip>> {
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
		.bind(org_id.0.to_string())
		.bind(limit as i32)
		.bind(offset as i32)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self))]
	async fn list_public_clips(&self, limit: u32, offset: u32) -> Result<Vec<Clip>> {
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

	#[instrument(skip(self, clip), fields(clip_id = %clip.id))]
	async fn update_clip(&self, clip: &Clip) -> Result<()> {
		sqlx::query(
			r#"
			UPDATE clips SET
				name = ?, description = ?, visibility = ?,
				updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&clip.name)
		.bind(&clip.description)
		.bind(clip.visibility.to_string())
		.bind(Utc::now().to_rfc3339())
		.bind(clip.id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(clip_id = %id))]
	async fn delete_clip(&self, id: ClipId) -> Result<bool> {
		let result = sqlx::query("DELETE FROM clips WHERE id = ?")
			.bind(id.0.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected() > 0)
	}

	#[instrument(skip(self), fields(clip_id = %id))]
	async fn update_clip_stats(
		&self,
		id: ClipId,
		file_count: u32,
		size_bytes: u64,
		language: Option<&str>,
	) -> Result<()> {
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
		.bind(Utc::now().to_rfc3339())
		.bind(id.0.to_string())
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

impl TryFrom<ClipRow> for Clip {
	type Error = ClipsError;

	fn try_from(row: ClipRow) -> Result<Self> {
		Ok(Clip {
			id: ClipId(row.id.parse()?),
			owner: row.owner,
			name: row.name,
			description: row.description,
			visibility: row
				.visibility
				.parse()
				.map_err(|e| ClipsError::Parse(format!("{}", e)))?,
			created_by: UserId(row.created_by.parse()?),
			org_id: row
				.org_id
				.map(|s| Ok::<_, ClipsError>(OrgId(s.parse()?)))
				.transpose()?,
			is_fork: row.is_fork,
			forked_from: row
				.forked_from
				.map(|s| Ok::<_, ClipsError>(ClipId(s.parse()?)))
				.transpose()?,
			file_count: row.file_count as u32,
			size_bytes: row.size_bytes as u64,
			language: row.language,
			created_at: parse_datetime(&row.created_at)?,
			updated_at: parse_datetime(&row.updated_at)?,
		})
	}
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
	DateTime::parse_from_rfc3339(s)
		.map(|dt| dt.with_timezone(&Utc))
		.map_err(|_| ClipsError::InvalidDateTime(s.to_string()))
}
