// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::cleanup::ExternalMirrorStore;
use crate::error::{MirrorError, Result};
use crate::types::{
	CreateExternalMirror, CreatePushMirror, ExternalMirror, MirrorBranchRule, Platform, PushMirror,
};

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

pub struct SqlitePushMirrorStore {
	pool: SqlitePool,
}

impl SqlitePushMirrorStore {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

#[async_trait]
impl PushMirrorStore for SqlitePushMirrorStore {
	async fn create(&self, mirror: &CreatePushMirror) -> Result<PushMirror> {
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

	async fn get_by_id(&self, id: Uuid) -> Result<Option<PushMirror>> {
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

	async fn list_by_repo(&self, repo_id: Uuid) -> Result<Vec<PushMirror>> {
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

	async fn delete(&self, id: Uuid) -> Result<()> {
		let id_str = id.to_string();

		let result = sqlx::query("DELETE FROM repo_mirrors WHERE id = ?")
			.bind(&id_str)
			.execute(&self.pool)
			.await?;

		if result.rows_affected() == 0 {
			return Err(MirrorError::NotFound);
		}

		Ok(())
	}

	async fn update_push_result(
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
			return Err(MirrorError::NotFound);
		}

		Ok(())
	}

	async fn list_branch_rules(&self, mirror_id: Uuid) -> Result<Vec<MirrorBranchRule>> {
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
						.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
					pattern,
					enabled: enabled != 0,
				})
			})
			.collect()
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
		id: Uuid::parse_str(&id)
			.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
		repo_id: Uuid::parse_str(&repo_id)
			.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
		remote_url,
		credential_key,
		enabled: enabled != 0,
		last_pushed_at: last_pushed_at
			.map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
			.transpose()
			.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
		last_error,
		created_at: DateTime::parse_from_rfc3339(&created_at)
			.map(|dt| dt.with_timezone(&Utc))
			.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	async fn create_test_pool() -> SqlitePool {
		let pool = SqlitePool::connect(":memory:").await.unwrap();
		crate::schema::run_test_migrations(&pool).await.unwrap();
		pool
	}

	#[tokio::test]
	async fn test_create_and_get_mirror() {
		let pool = create_test_pool().await;
		let store = SqlitePushMirrorStore::new(pool);

		let create = CreatePushMirror {
			repo_id: Uuid::new_v4(),
			remote_url: "https://github.com/test/repo.git".to_string(),
			credential_key: "mirror:test:github".to_string(),
			enabled: true,
		};

		let created = store.create(&create).await.unwrap();
		assert_eq!(created.remote_url, create.remote_url);
		assert_eq!(created.enabled, true);

		let fetched = store.get_by_id(created.id).await.unwrap().unwrap();
		assert_eq!(fetched.id, created.id);
		assert_eq!(fetched.remote_url, created.remote_url);
	}

	#[tokio::test]
	async fn test_list_by_repo() {
		let pool = create_test_pool().await;
		let store = SqlitePushMirrorStore::new(pool);

		let repo_id = Uuid::new_v4();

		store
			.create(&CreatePushMirror {
				repo_id,
				remote_url: "https://github.com/test/repo1.git".to_string(),
				credential_key: "key1".to_string(),
				enabled: true,
			})
			.await
			.unwrap();

		store
			.create(&CreatePushMirror {
				repo_id,
				remote_url: "https://gitlab.com/test/repo2.git".to_string(),
				credential_key: "key2".to_string(),
				enabled: false,
			})
			.await
			.unwrap();

		let mirrors = store.list_by_repo(repo_id).await.unwrap();
		assert_eq!(mirrors.len(), 2);
	}

	#[tokio::test]
	async fn test_delete_mirror() {
		let pool = create_test_pool().await;
		let store = SqlitePushMirrorStore::new(pool);

		let created = store
			.create(&CreatePushMirror {
				repo_id: Uuid::new_v4(),
				remote_url: "https://github.com/test/repo.git".to_string(),
				credential_key: "key".to_string(),
				enabled: true,
			})
			.await
			.unwrap();

		store.delete(created.id).await.unwrap();

		let fetched = store.get_by_id(created.id).await.unwrap();
		assert!(fetched.is_none());
	}

	#[tokio::test]
	async fn test_delete_nonexistent_returns_not_found() {
		let pool = create_test_pool().await;
		let store = SqlitePushMirrorStore::new(pool);

		let result = store.delete(Uuid::new_v4()).await;
		assert!(matches!(result, Err(MirrorError::NotFound)));
	}

	/// Verifies that list_by_repo returns mirrors ordered by created_at descending (newest first).
	#[tokio::test]
	async fn test_list_by_repo_ordered_by_created_at_desc() {
		let pool = create_test_pool().await;
		let store = SqlitePushMirrorStore::new(pool);

		let repo_id = Uuid::new_v4();

		let mirror1 = store
			.create(&CreatePushMirror {
				repo_id,
				remote_url: "https://github.com/test/first.git".to_string(),
				credential_key: "key1".to_string(),
				enabled: true,
			})
			.await
			.unwrap();

		tokio::time::sleep(std::time::Duration::from_millis(10)).await;

		let mirror2 = store
			.create(&CreatePushMirror {
				repo_id,
				remote_url: "https://github.com/test/second.git".to_string(),
				credential_key: "key2".to_string(),
				enabled: true,
			})
			.await
			.unwrap();

		tokio::time::sleep(std::time::Duration::from_millis(10)).await;

		let mirror3 = store
			.create(&CreatePushMirror {
				repo_id,
				remote_url: "https://github.com/test/third.git".to_string(),
				credential_key: "key3".to_string(),
				enabled: true,
			})
			.await
			.unwrap();

		let mirrors = store.list_by_repo(repo_id).await.unwrap();
		assert_eq!(mirrors.len(), 3);
		assert_eq!(mirrors[0].id, mirror3.id);
		assert_eq!(mirrors[1].id, mirror2.id);
		assert_eq!(mirrors[2].id, mirror1.id);
	}

	/// Verifies that update_push_result correctly updates last_pushed_at and last_error fields.
	#[tokio::test]
	async fn test_update_push_result_updates_fields() {
		let pool = create_test_pool().await;
		let store = SqlitePushMirrorStore::new(pool);

		let created = store
			.create(&CreatePushMirror {
				repo_id: Uuid::new_v4(),
				remote_url: "https://github.com/test/repo.git".to_string(),
				credential_key: "key".to_string(),
				enabled: true,
			})
			.await
			.unwrap();

		assert!(created.last_pushed_at.is_none());
		assert!(created.last_error.is_none());

		let pushed_at = Utc::now();
		let error_msg = Some("connection refused".to_string());
		store
			.update_push_result(created.id, pushed_at, error_msg.clone())
			.await
			.unwrap();

		let fetched = store.get_by_id(created.id).await.unwrap().unwrap();
		assert!(fetched.last_pushed_at.is_some());
		assert_eq!(fetched.last_error, error_msg);

		let pushed_at2 = Utc::now();
		store
			.update_push_result(created.id, pushed_at2, None)
			.await
			.unwrap();

		let fetched2 = store.get_by_id(created.id).await.unwrap().unwrap();
		assert!(fetched2.last_pushed_at.is_some());
		assert!(fetched2.last_error.is_none());
	}

	/// Verifies that update_push_result returns NotFound for a non-existent mirror.
	#[tokio::test]
	async fn test_update_push_result_missing_returns_not_found() {
		let pool = create_test_pool().await;
		let store = SqlitePushMirrorStore::new(pool);

		let result = store
			.update_push_result(Uuid::new_v4(), Utc::now(), None)
			.await;
		assert!(matches!(result, Err(MirrorError::NotFound)));
	}

	/// Verifies that list_branch_rules returns inserted rules correctly.
	#[tokio::test]
	async fn test_list_branch_rules_returns_rules() {
		let pool = create_test_pool().await;
		let store = SqlitePushMirrorStore::new(pool.clone());

		let created = store
			.create(&CreatePushMirror {
				repo_id: Uuid::new_v4(),
				remote_url: "https://github.com/test/repo.git".to_string(),
				credential_key: "key".to_string(),
				enabled: true,
			})
			.await
			.unwrap();

		let mirror_id_str = created.id.to_string();
		sqlx::query("INSERT INTO mirror_branch_rules (mirror_id, pattern, enabled) VALUES (?, ?, ?)")
			.bind(&mirror_id_str)
			.bind("main")
			.bind(1)
			.execute(&pool)
			.await
			.unwrap();

		sqlx::query("INSERT INTO mirror_branch_rules (mirror_id, pattern, enabled) VALUES (?, ?, ?)")
			.bind(&mirror_id_str)
			.bind("release/*")
			.bind(0)
			.execute(&pool)
			.await
			.unwrap();

		let rules = store.list_branch_rules(created.id).await.unwrap();
		assert_eq!(rules.len(), 2);

		let main_rule = rules.iter().find(|r| r.pattern == "main").unwrap();
		assert!(main_rule.enabled);
		assert_eq!(main_rule.mirror_id, created.id);

		let release_rule = rules.iter().find(|r| r.pattern == "release/*").unwrap();
		assert!(!release_rule.enabled);
	}
}

pub struct SqliteExternalMirrorStore {
	pool: SqlitePool,
}

impl SqliteExternalMirrorStore {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	pub async fn create(&self, mirror: &CreateExternalMirror) -> Result<ExternalMirror> {
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

	pub async fn get_by_external(
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
}

#[async_trait]
impl ExternalMirrorStore for SqliteExternalMirrorStore {
	async fn get_by_id(&self, id: Uuid) -> Result<Option<ExternalMirror>> {
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

	async fn get_by_repo_id(&self, repo_id: Uuid) -> Result<Option<ExternalMirror>> {
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

	async fn find_stale(&self, stale_threshold: DateTime<Utc>) -> Result<Vec<ExternalMirror>> {
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

	async fn delete(&self, id: Uuid) -> Result<()> {
		let id_str = id.to_string();

		let result = sqlx::query("DELETE FROM external_mirrors WHERE id = ?")
			.bind(&id_str)
			.execute(&self.pool)
			.await?;

		if result.rows_affected() == 0 {
			return Err(MirrorError::NotFound);
		}

		Ok(())
	}

	async fn update_last_accessed(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
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
			return Err(MirrorError::NotFound);
		}

		Ok(())
	}

	async fn update_last_synced(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
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
			return Err(MirrorError::NotFound);
		}

		Ok(())
	}
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
		id: Uuid::parse_str(&id)
			.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
		platform: Platform::parse(&platform).ok_or_else(|| {
			MirrorError::Database(sqlx::Error::Decode(Box::new(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				format!("Invalid platform: {}", platform),
			))))
		})?,
		external_owner,
		external_repo,
		repo_id: Uuid::parse_str(&repo_id)
			.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
		last_synced_at: last_synced_at
			.map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
			.transpose()
			.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
		last_accessed_at: last_accessed_at
			.map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
			.transpose()
			.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
		created_at: DateTime::parse_from_rfc3339(&created_at)
			.map(|dt| dt.with_timezone(&Utc))
			.map_err(|e| MirrorError::Database(sqlx::Error::Decode(Box::new(e))))?,
	})
}

#[cfg(test)]
mod external_mirror_tests {
	use super::*;

	async fn create_test_pool() -> SqlitePool {
		let pool = SqlitePool::connect(":memory:").await.unwrap();
		crate::schema::run_test_migrations(&pool).await.unwrap();
		pool
	}

	#[tokio::test]
	async fn test_create_and_get_external_mirror() {
		let pool = create_test_pool().await;
		let store = SqliteExternalMirrorStore::new(pool);

		let create = CreateExternalMirror {
			platform: Platform::GitHub,
			external_owner: "torvalds".to_string(),
			external_repo: "linux".to_string(),
			repo_id: Uuid::new_v4(),
		};

		let created = store.create(&create).await.unwrap();
		assert_eq!(created.platform, Platform::GitHub);
		assert_eq!(created.external_owner, "torvalds");
		assert_eq!(created.external_repo, "linux");
		assert!(created.last_accessed_at.is_some());

		let fetched = store.get_by_id(created.id).await.unwrap().unwrap();
		assert_eq!(fetched.id, created.id);
		assert_eq!(fetched.external_owner, "torvalds");
	}

	#[tokio::test]
	async fn test_get_by_repo_id() {
		let pool = create_test_pool().await;
		let store = SqliteExternalMirrorStore::new(pool);

		let repo_id = Uuid::new_v4();
		let create = CreateExternalMirror {
			platform: Platform::GitLab,
			external_owner: "gitlab-org".to_string(),
			external_repo: "gitlab".to_string(),
			repo_id,
		};

		store.create(&create).await.unwrap();

		let fetched = store.get_by_repo_id(repo_id).await.unwrap().unwrap();
		assert_eq!(fetched.repo_id, repo_id);
		assert_eq!(fetched.platform, Platform::GitLab);
	}

	#[tokio::test]
	async fn test_get_by_external() {
		let pool = create_test_pool().await;
		let store = SqliteExternalMirrorStore::new(pool);

		let create = CreateExternalMirror {
			platform: Platform::GitHub,
			external_owner: "rust-lang".to_string(),
			external_repo: "rust".to_string(),
			repo_id: Uuid::new_v4(),
		};

		store.create(&create).await.unwrap();

		let fetched = store
			.get_by_external(Platform::GitHub, "rust-lang", "rust")
			.await
			.unwrap()
			.unwrap();
		assert_eq!(fetched.external_owner, "rust-lang");
		assert_eq!(fetched.external_repo, "rust");
	}

	#[tokio::test]
	async fn test_update_last_accessed() {
		let pool = create_test_pool().await;
		let store = SqliteExternalMirrorStore::new(pool);

		let create = CreateExternalMirror {
			platform: Platform::GitHub,
			external_owner: "owner".to_string(),
			external_repo: "repo".to_string(),
			repo_id: Uuid::new_v4(),
		};

		let created = store.create(&create).await.unwrap();
		let original_accessed = created.last_accessed_at;

		tokio::time::sleep(std::time::Duration::from_millis(10)).await;

		let new_time = Utc::now();
		store
			.update_last_accessed(created.id, new_time)
			.await
			.unwrap();

		let fetched = store.get_by_id(created.id).await.unwrap().unwrap();
		assert!(fetched.last_accessed_at.unwrap() > original_accessed.unwrap());
	}

	#[tokio::test]
	async fn test_find_stale_mirrors() {
		let pool = create_test_pool().await;
		let store = SqliteExternalMirrorStore::new(pool);

		let create1 = CreateExternalMirror {
			platform: Platform::GitHub,
			external_owner: "owner1".to_string(),
			external_repo: "repo1".to_string(),
			repo_id: Uuid::new_v4(),
		};
		store.create(&create1).await.unwrap();

		let threshold = Utc::now() + chrono::Duration::hours(1);
		let stale = store.find_stale(threshold).await.unwrap();
		assert_eq!(stale.len(), 1);

		let threshold_past = Utc::now() - chrono::Duration::hours(1);
		let stale = store.find_stale(threshold_past).await.unwrap();
		assert_eq!(stale.len(), 0);
	}

	#[tokio::test]
	async fn test_delete_external_mirror() {
		let pool = create_test_pool().await;
		let store = SqliteExternalMirrorStore::new(pool);

		let create = CreateExternalMirror {
			platform: Platform::GitHub,
			external_owner: "owner".to_string(),
			external_repo: "repo".to_string(),
			repo_id: Uuid::new_v4(),
		};

		let created = store.create(&create).await.unwrap();
		store.delete(created.id).await.unwrap();

		let fetched = store.get_by_id(created.id).await.unwrap();
		assert!(fetched.is_none());
	}

	/// Verifies that update_last_synced correctly updates the last_synced_at field.
	#[tokio::test]
	async fn test_update_last_synced() {
		let pool = create_test_pool().await;
		let store = SqliteExternalMirrorStore::new(pool);

		let create = CreateExternalMirror {
			platform: Platform::GitHub,
			external_owner: "owner".to_string(),
			external_repo: "repo".to_string(),
			repo_id: Uuid::new_v4(),
		};

		let created = store.create(&create).await.unwrap();
		assert!(created.last_synced_at.is_none());

		let sync_time = Utc::now();
		store
			.update_last_synced(created.id, sync_time)
			.await
			.unwrap();

		let fetched = store.get_by_id(created.id).await.unwrap().unwrap();
		assert!(fetched.last_synced_at.is_some());
	}

	/// Verifies that update_last_accessed returns NotFound for a non-existent mirror.
	#[tokio::test]
	async fn test_update_last_accessed_not_found() {
		let pool = create_test_pool().await;
		let store = SqliteExternalMirrorStore::new(pool);

		let result = store.update_last_accessed(Uuid::new_v4(), Utc::now()).await;
		assert!(matches!(result, Err(MirrorError::NotFound)));
	}

	/// Verifies that update_last_synced returns NotFound for a non-existent mirror.
	#[tokio::test]
	async fn test_update_last_synced_not_found() {
		let pool = create_test_pool().await;
		let store = SqliteExternalMirrorStore::new(pool);

		let result = store.update_last_synced(Uuid::new_v4(), Utc::now()).await;
		assert!(matches!(result, Err(MirrorError::NotFound)));
	}
}
