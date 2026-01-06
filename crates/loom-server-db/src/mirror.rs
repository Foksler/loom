// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{DbError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
	GitHub,
	GitLab,
}

impl Platform {
	pub fn as_str(&self) -> &'static str {
		match self {
			Platform::GitHub => "github",
			Platform::GitLab => "gitlab",
		}
	}

	pub fn parse(s: &str) -> Option<Self> {
		match s.to_lowercase().as_str() {
			"github" => Some(Platform::GitHub),
			"gitlab" => Some(Platform::GitLab),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PushMirror {
	pub id: Uuid,
	pub repo_id: Uuid,
	pub remote_url: String,
	pub credential_key: String,
	pub enabled: bool,
	pub last_pushed_at: Option<DateTime<Utc>>,
	pub last_error: Option<String>,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MirrorBranchRule {
	pub mirror_id: Uuid,
	pub pattern: String,
	pub enabled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExternalMirror {
	pub id: Uuid,
	pub platform: Platform,
	pub external_owner: String,
	pub external_repo: String,
	pub repo_id: Uuid,
	pub last_synced_at: Option<DateTime<Utc>>,
	pub last_accessed_at: Option<DateTime<Utc>>,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreatePushMirror {
	pub repo_id: Uuid,
	pub remote_url: String,
	pub credential_key: String,
	pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct CreateExternalMirror {
	pub platform: Platform,
	pub external_owner: String,
	pub external_repo: String,
	pub repo_id: Uuid,
}

#[async_trait]
pub trait PushMirrorStore: Send + Sync {
	async fn create(&self, mirror: &CreatePushMirror) -> Result<PushMirror>;
	async fn get_by_id(&self, id: Uuid) -> Result<Option<PushMirror>>;
	async fn list_by_repo(&self, repo_id: Uuid) -> Result<Vec<PushMirror>>;
	async fn delete(&self, id: Uuid) -> Result<()>;
	async fn update_push_result(
		&self,
		id: Uuid,
		pushed_at: DateTime<Utc>,
		error: Option<String>,
	) -> Result<()>;
	async fn list_branch_rules(&self, mirror_id: Uuid) -> Result<Vec<MirrorBranchRule>>;
}

#[async_trait]
pub trait ExternalMirrorStore: Send + Sync {
	async fn create(&self, mirror: &CreateExternalMirror) -> Result<ExternalMirror>;
	async fn get_by_id(&self, id: Uuid) -> Result<Option<ExternalMirror>>;
	async fn get_by_repo_id(&self, repo_id: Uuid) -> Result<Option<ExternalMirror>>;
	async fn get_by_external(
		&self,
		platform: Platform,
		owner: &str,
		repo: &str,
	) -> Result<Option<ExternalMirror>>;
	async fn find_stale(&self, stale_threshold: DateTime<Utc>) -> Result<Vec<ExternalMirror>>;
	async fn delete(&self, id: Uuid) -> Result<()>;
	async fn update_last_accessed(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;
	async fn update_last_synced(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;
}

#[derive(Clone)]
pub struct MirrorRepository {
	pool: SqlitePool,
}

impl MirrorRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	#[tracing::instrument(skip(self, mirror))]
	pub async fn create_push_mirror(&self, mirror: &CreatePushMirror) -> Result<PushMirror> {
		let id = Uuid::new_v4();
		let now = Utc::now();
		let id_str = id.to_string();
		let repo_id_str = mirror.repo_id.to_string();
		let now_str = now.to_rfc3339();
		let enabled = mirror.enabled as i32;

		sqlx::query(
			r#"
			INSERT INTO repo_mirrors (id, repo_id, remote_url, credential_key, enabled, created_at)
			VALUES (?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id_str)
		.bind(&repo_id_str)
		.bind(&mirror.remote_url)
		.bind(&mirror.credential_key)
		.bind(enabled)
		.bind(&now_str)
		.execute(&self.pool)
		.await?;

		Ok(PushMirror {
			id,
			repo_id: mirror.repo_id,
			remote_url: mirror.remote_url.clone(),
			credential_key: mirror.credential_key.clone(),
			enabled: mirror.enabled,
			last_pushed_at: None,
			last_error: None,
			created_at: now,
		})
	}

	#[tracing::instrument(skip(self))]
	pub async fn get_push_mirror_by_id(&self, id: Uuid) -> Result<Option<PushMirror>> {
		let id_str = id.to_string();

		let row: Option<(
			String,
			String,
			String,
			String,
			i32,
			Option<String>,
			Option<String>,
			String,
		)> = sqlx::query_as(
			r#"
			SELECT id, repo_id, remote_url, credential_key, enabled, last_pushed_at, last_error, created_at
			FROM repo_mirrors
			WHERE id = ?
			"#,
		)
		.bind(&id_str)
		.fetch_optional(&self.pool)
		.await?;

		row.map(row_to_push_mirror).transpose()
	}

	#[tracing::instrument(skip(self))]
	pub async fn list_push_mirrors_by_repo(&self, repo_id: Uuid) -> Result<Vec<PushMirror>> {
		let repo_id_str = repo_id.to_string();

		let rows: Vec<(
			String,
			String,
			String,
			String,
			i32,
			Option<String>,
			Option<String>,
			String,
		)> = sqlx::query_as(
			r#"
			SELECT id, repo_id, remote_url, credential_key, enabled, last_pushed_at, last_error, created_at
			FROM repo_mirrors
			WHERE repo_id = ?
			ORDER BY created_at DESC
			"#,
		)
		.bind(&repo_id_str)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(row_to_push_mirror).collect()
	}

	#[tracing::instrument(skip(self))]
	pub async fn delete_push_mirror(&self, id: Uuid) -> Result<()> {
		let id_str = id.to_string();

		let result = sqlx::query("DELETE FROM repo_mirrors WHERE id = ?")
			.bind(&id_str)
			.execute(&self.pool)
			.await?;

		if result.rows_affected() == 0 {
			return Err(DbError::NotFound("push mirror not found".to_string()));
		}

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub async fn update_push_result(
		&self,
		id: Uuid,
		pushed_at: DateTime<Utc>,
		error: Option<String>,
	) -> Result<()> {
		let id_str = id.to_string();
		let pushed_at_str = pushed_at.to_rfc3339();

		let result = sqlx::query(
			r#"
			UPDATE repo_mirrors
			SET last_pushed_at = ?, last_error = ?
			WHERE id = ?
			"#,
		)
		.bind(&pushed_at_str)
		.bind(&error)
		.bind(&id_str)
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(DbError::NotFound("push mirror not found".to_string()));
		}

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub async fn list_branch_rules(&self, mirror_id: Uuid) -> Result<Vec<MirrorBranchRule>> {
		let mirror_id_str = mirror_id.to_string();

		let rows: Vec<(String, String, i32)> = sqlx::query_as(
			r#"
			SELECT mirror_id, pattern, enabled
			FROM mirror_branch_rules
			WHERE mirror_id = ?
			"#,
		)
		.bind(&mirror_id_str)
		.fetch_all(&self.pool)
		.await?;

		rows
			.into_iter()
			.map(|(mirror_id, pattern, enabled)| {
				Ok(MirrorBranchRule {
					mirror_id: Uuid::parse_str(&mirror_id)
						.map_err(|e| DbError::Internal(format!("Invalid mirror_id UUID: {e}")))?,
					pattern,
					enabled: enabled != 0,
				})
			})
			.collect()
	}

	#[tracing::instrument(skip(self, mirror))]
	pub async fn create_external_mirror(
		&self,
		mirror: &CreateExternalMirror,
	) -> Result<ExternalMirror> {
		let id = Uuid::new_v4();
		let now = Utc::now();
		let id_str = id.to_string();
		let platform_str = mirror.platform.as_str();
		let repo_id_str = mirror.repo_id.to_string();
		let now_str = now.to_rfc3339();

		sqlx::query(
			r#"
			INSERT INTO external_mirrors (id, platform, external_owner, external_repo, repo_id, last_accessed_at, created_at)
			VALUES (?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id_str)
		.bind(platform_str)
		.bind(&mirror.external_owner)
		.bind(&mirror.external_repo)
		.bind(&repo_id_str)
		.bind(&now_str)
		.bind(&now_str)
		.execute(&self.pool)
		.await?;

		Ok(ExternalMirror {
			id,
			platform: mirror.platform,
			external_owner: mirror.external_owner.clone(),
			external_repo: mirror.external_repo.clone(),
			repo_id: mirror.repo_id,
			last_synced_at: None,
			last_accessed_at: Some(now),
			created_at: now,
		})
	}

	#[tracing::instrument(skip(self))]
	pub async fn get_external_mirror_by_external(
		&self,
		platform: Platform,
		owner: &str,
		repo: &str,
	) -> Result<Option<ExternalMirror>> {
		let platform_str = platform.as_str();

		#[allow(clippy::type_complexity)]
		let row: Option<(
			String,
			String,
			String,
			String,
			String,
			Option<String>,
			Option<String>,
			String,
		)> = sqlx::query_as(
			r#"
			SELECT id, platform, external_owner, external_repo, repo_id, last_synced_at, last_accessed_at, created_at
			FROM external_mirrors
			WHERE platform = ? AND external_owner = ? AND external_repo = ?
			"#,
		)
		.bind(platform_str)
		.bind(owner)
		.bind(repo)
		.fetch_optional(&self.pool)
		.await?;

		row.map(row_to_external_mirror).transpose()
	}

	#[tracing::instrument(skip(self))]
	pub async fn get_external_mirror_by_id(&self, id: Uuid) -> Result<Option<ExternalMirror>> {
		let id_str = id.to_string();

		let row: Option<(
			String,
			String,
			String,
			String,
			String,
			Option<String>,
			Option<String>,
			String,
		)> = sqlx::query_as(
			r#"
			SELECT id, platform, external_owner, external_repo, repo_id, last_synced_at, last_accessed_at, created_at
			FROM external_mirrors
			WHERE id = ?
			"#,
		)
		.bind(&id_str)
		.fetch_optional(&self.pool)
		.await?;

		row.map(row_to_external_mirror).transpose()
	}

	#[tracing::instrument(skip(self))]
	pub async fn get_external_mirror_by_repo_id(
		&self,
		repo_id: Uuid,
	) -> Result<Option<ExternalMirror>> {
		let repo_id_str = repo_id.to_string();

		let row: Option<(
			String,
			String,
			String,
			String,
			String,
			Option<String>,
			Option<String>,
			String,
		)> = sqlx::query_as(
			r#"
			SELECT id, platform, external_owner, external_repo, repo_id, last_synced_at, last_accessed_at, created_at
			FROM external_mirrors
			WHERE repo_id = ?
			"#,
		)
		.bind(&repo_id_str)
		.fetch_optional(&self.pool)
		.await?;

		row.map(row_to_external_mirror).transpose()
	}

	#[tracing::instrument(skip(self))]
	pub async fn find_stale_external_mirrors(
		&self,
		stale_threshold: DateTime<Utc>,
	) -> Result<Vec<ExternalMirror>> {
		let threshold_str = stale_threshold.to_rfc3339();

		let rows: Vec<(
			String,
			String,
			String,
			String,
			String,
			Option<String>,
			Option<String>,
			String,
		)> = sqlx::query_as(
			r#"
			SELECT id, platform, external_owner, external_repo, repo_id, last_synced_at, last_accessed_at, created_at
			FROM external_mirrors
			WHERE last_accessed_at IS NULL OR last_accessed_at < ?
			ORDER BY last_accessed_at ASC
			"#,
		)
		.bind(&threshold_str)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(row_to_external_mirror).collect()
	}

	#[tracing::instrument(skip(self))]
	pub async fn delete_external_mirror(&self, id: Uuid) -> Result<()> {
		let id_str = id.to_string();

		let result = sqlx::query("DELETE FROM external_mirrors WHERE id = ?")
			.bind(&id_str)
			.execute(&self.pool)
			.await?;

		if result.rows_affected() == 0 {
			return Err(DbError::NotFound("external mirror not found".to_string()));
		}

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub async fn update_external_mirror_last_accessed(
		&self,
		id: Uuid,
		at: DateTime<Utc>,
	) -> Result<()> {
		let id_str = id.to_string();
		let at_str = at.to_rfc3339();

		let result = sqlx::query(
			r#"
			UPDATE external_mirrors
			SET last_accessed_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&at_str)
		.bind(&id_str)
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(DbError::NotFound("external mirror not found".to_string()));
		}

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub async fn update_external_mirror_last_synced(
		&self,
		id: Uuid,
		at: DateTime<Utc>,
	) -> Result<()> {
		let id_str = id.to_string();
		let at_str = at.to_rfc3339();

		let result = sqlx::query(
			r#"
			UPDATE external_mirrors
			SET last_synced_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&at_str)
		.bind(&id_str)
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(DbError::NotFound("external mirror not found".to_string()));
		}

		Ok(())
	}
}

#[async_trait]
impl PushMirrorStore for MirrorRepository {
	async fn create(&self, mirror: &CreatePushMirror) -> Result<PushMirror> {
		self.create_push_mirror(mirror).await
	}

	async fn get_by_id(&self, id: Uuid) -> Result<Option<PushMirror>> {
		self.get_push_mirror_by_id(id).await
	}

	async fn list_by_repo(&self, repo_id: Uuid) -> Result<Vec<PushMirror>> {
		self.list_push_mirrors_by_repo(repo_id).await
	}

	async fn delete(&self, id: Uuid) -> Result<()> {
		self.delete_push_mirror(id).await
	}

	async fn update_push_result(
		&self,
		id: Uuid,
		pushed_at: DateTime<Utc>,
		error: Option<String>,
	) -> Result<()> {
		self.update_push_result(id, pushed_at, error).await
	}

	async fn list_branch_rules(&self, mirror_id: Uuid) -> Result<Vec<MirrorBranchRule>> {
		self.list_branch_rules(mirror_id).await
	}
}

#[async_trait]
impl ExternalMirrorStore for MirrorRepository {
	async fn create(&self, mirror: &CreateExternalMirror) -> Result<ExternalMirror> {
		self.create_external_mirror(mirror).await
	}

	async fn get_by_id(&self, id: Uuid) -> Result<Option<ExternalMirror>> {
		self.get_external_mirror_by_id(id).await
	}

	async fn get_by_repo_id(&self, repo_id: Uuid) -> Result<Option<ExternalMirror>> {
		self.get_external_mirror_by_repo_id(repo_id).await
	}

	async fn get_by_external(
		&self,
		platform: Platform,
		owner: &str,
		repo: &str,
	) -> Result<Option<ExternalMirror>> {
		self.get_external_mirror_by_external(platform, owner, repo)
			.await
	}

	async fn find_stale(&self, stale_threshold: DateTime<Utc>) -> Result<Vec<ExternalMirror>> {
		self.find_stale_external_mirrors(stale_threshold).await
	}

	async fn delete(&self, id: Uuid) -> Result<()> {
		self.delete_external_mirror(id).await
	}

	async fn update_last_accessed(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
		self.update_external_mirror_last_accessed(id, at).await
	}

	async fn update_last_synced(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
		self.update_external_mirror_last_synced(id, at).await
	}
}

fn row_to_push_mirror(
	row: (
		String,
		String,
		String,
		String,
		i32,
		Option<String>,
		Option<String>,
		String,
	),
) -> Result<PushMirror> {
	let (id, repo_id, remote_url, credential_key, enabled, last_pushed_at, last_error, created_at) =
		row;

	Ok(PushMirror {
		id: Uuid::parse_str(&id).map_err(|e| DbError::Internal(format!("Invalid id UUID: {e}")))?,
		repo_id: Uuid::parse_str(&repo_id)
			.map_err(|e| DbError::Internal(format!("Invalid repo_id UUID: {e}")))?,
		remote_url,
		credential_key,
		enabled: enabled != 0,
		last_pushed_at: last_pushed_at
			.map(|s| {
				chrono::DateTime::parse_from_rfc3339(&s)
					.map(|dt| dt.with_timezone(&Utc))
					.map_err(|e| DbError::Internal(format!("Invalid last_pushed_at: {e}")))
			})
			.transpose()?,
		last_error,
		created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
			.map(|dt| dt.with_timezone(&Utc))
			.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?,
	})
}

fn row_to_external_mirror(
	row: (
		String,
		String,
		String,
		String,
		String,
		Option<String>,
		Option<String>,
		String,
	),
) -> Result<ExternalMirror> {
	let (
		id,
		platform,
		external_owner,
		external_repo,
		repo_id,
		last_synced_at,
		last_accessed_at,
		created_at,
	) = row;

	Ok(ExternalMirror {
		id: Uuid::parse_str(&id).map_err(|e| DbError::Internal(format!("Invalid id UUID: {e}")))?,
		platform: Platform::parse(&platform)
			.ok_or_else(|| DbError::Internal(format!("Invalid platform: {}", platform)))?,
		external_owner,
		external_repo,
		repo_id: Uuid::parse_str(&repo_id)
			.map_err(|e| DbError::Internal(format!("Invalid repo_id UUID: {e}")))?,
		last_synced_at: last_synced_at
			.map(|s| {
				chrono::DateTime::parse_from_rfc3339(&s)
					.map(|dt| dt.with_timezone(&Utc))
					.map_err(|e| DbError::Internal(format!("Invalid last_synced_at: {e}")))
			})
			.transpose()?,
		last_accessed_at: last_accessed_at
			.map(|s| {
				chrono::DateTime::parse_from_rfc3339(&s)
					.map(|dt| dt.with_timezone(&Utc))
					.map_err(|e| DbError::Internal(format!("Invalid last_accessed_at: {e}")))
			})
			.transpose()?,
		created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
			.map(|dt| dt.with_timezone(&Utc))
			.map_err(|e| DbError::Internal(format!("Invalid created_at: {e}")))?,
	})
}
