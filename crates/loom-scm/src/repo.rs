// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{Result, ScmError};
use crate::types::{OwnerType, Repository, Visibility};

#[async_trait]
pub trait RepoStore: Send + Sync {
	async fn create(&self, repo: &Repository) -> Result<Repository>;
	async fn get_by_id(&self, id: Uuid) -> Result<Option<Repository>>;
	async fn get_by_owner_and_name(
		&self,
		owner_type: OwnerType,
		owner_id: Uuid,
		name: &str,
	) -> Result<Option<Repository>>;
	async fn list_by_owner(&self, owner_type: OwnerType, owner_id: Uuid)
		-> Result<Vec<Repository>>;
	async fn update(&self, repo: &Repository) -> Result<Repository>;
	async fn soft_delete(&self, id: Uuid) -> Result<()>;
	async fn hard_delete(&self, id: Uuid) -> Result<()>;
}

pub struct SqliteRepoStore {
	pool: SqlitePool,
}

impl SqliteRepoStore {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	fn row_to_repo(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Repository> {
		let id_str: String = row.get("id");
		let owner_type_str: String = row.get("owner_type");
		let owner_id_str: String = row.get("owner_id");
		let visibility_str: String = row.get("visibility");
		let deleted_at_str: Option<String> = row.get("deleted_at");
		let created_at_str: String = row.get("created_at");
		let updated_at_str: String = row.get("updated_at");

		Ok(Repository {
			id: Uuid::parse_str(&id_str).map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			owner_type: owner_type_str.parse::<OwnerType>()
				.map_err(|_| ScmError::Database(sqlx::Error::Decode(format!("invalid owner_type: {}", owner_type_str).into())))?,
			owner_id: Uuid::parse_str(&owner_id_str).map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			name: row.get("name"),
			visibility: visibility_str.parse::<Visibility>()
				.map_err(|_| ScmError::Database(sqlx::Error::Decode(format!("invalid visibility: {}", visibility_str).into())))?,
			default_branch: row.get("default_branch"),
			deleted_at: deleted_at_str
				.map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)))
				.transpose()
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			created_at: DateTime::parse_from_rfc3339(&created_at_str)
				.map(|d| d.with_timezone(&Utc))
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
				.map(|d| d.with_timezone(&Utc))
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
		})
	}
}

#[async_trait]
impl RepoStore for SqliteRepoStore {
	async fn create(&self, repo: &Repository) -> Result<Repository> {
		sqlx::query(
			r#"
			INSERT INTO repos (id, owner_type, owner_id, name, visibility, default_branch, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(repo.id.to_string())
		.bind(repo.owner_type.as_str())
		.bind(repo.owner_id.to_string())
		.bind(&repo.name)
		.bind(repo.visibility.as_str())
		.bind(&repo.default_branch)
		.bind(repo.created_at.to_rfc3339())
		.bind(repo.updated_at.to_rfc3339())
		.execute(&self.pool)
		.await
		.map_err(|e| match e {
			sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
				ScmError::AlreadyExists
			}
			_ => ScmError::Database(e),
		})?;

		Ok(repo.clone())
	}

	async fn get_by_id(&self, id: Uuid) -> Result<Option<Repository>> {
		let row = sqlx::query(
			r#"
			SELECT id, owner_type, owner_id, name, visibility, default_branch, deleted_at, created_at, updated_at
			FROM repos
			WHERE id = ? AND deleted_at IS NULL
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_repo(&r)).transpose()
	}

	async fn get_by_owner_and_name(
		&self,
		owner_type: OwnerType,
		owner_id: Uuid,
		name: &str,
	) -> Result<Option<Repository>> {
		let row = sqlx::query(
			r#"
			SELECT id, owner_type, owner_id, name, visibility, default_branch, deleted_at, created_at, updated_at
			FROM repos
			WHERE owner_type = ? AND owner_id = ? AND name = ? AND deleted_at IS NULL
			"#,
		)
		.bind(owner_type.as_str())
		.bind(owner_id.to_string())
		.bind(name)
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_repo(&r)).transpose()
	}

	async fn list_by_owner(
		&self,
		owner_type: OwnerType,
		owner_id: Uuid,
	) -> Result<Vec<Repository>> {
		let rows = sqlx::query(
			r#"
			SELECT id, owner_type, owner_id, name, visibility, default_branch, deleted_at, created_at, updated_at
			FROM repos
			WHERE owner_type = ? AND owner_id = ? AND deleted_at IS NULL
			ORDER BY name ASC
			"#,
		)
		.bind(owner_type.as_str())
		.bind(owner_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.iter().map(|r| self.row_to_repo(r)).collect()
	}

	async fn update(&self, repo: &Repository) -> Result<Repository> {
		let updated_at = Utc::now().to_rfc3339();

		let result = sqlx::query(
			r#"
			UPDATE repos
			SET name = ?, visibility = ?, default_branch = ?, updated_at = ?
			WHERE id = ? AND deleted_at IS NULL
			"#,
		)
		.bind(&repo.name)
		.bind(repo.visibility.as_str())
		.bind(&repo.default_branch)
		.bind(&updated_at)
		.bind(repo.id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(ScmError::NotFound);
		}

		self.get_by_id(repo.id).await?.ok_or(ScmError::NotFound)
	}

	async fn soft_delete(&self, id: Uuid) -> Result<()> {
		let deleted_at = Utc::now().to_rfc3339();

		let result = sqlx::query(
			r#"
			UPDATE repos
			SET deleted_at = ?, updated_at = ?
			WHERE id = ? AND deleted_at IS NULL
			"#,
		)
		.bind(&deleted_at)
		.bind(&deleted_at)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(ScmError::NotFound);
		}

		Ok(())
	}

	async fn hard_delete(&self, id: Uuid) -> Result<()> {
		let result = sqlx::query(
			r#"
			DELETE FROM repos WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(ScmError::NotFound);
		}

		Ok(())
	}
}
