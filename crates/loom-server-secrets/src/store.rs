// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Secret storage with SQLite backend.
//!
//! Secrets are stored encrypted. This module handles:
//! - Secret CRUD operations
//! - Version management
//! - DEK storage
//! - Scope-based queries

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Row, SqlitePool};
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::error::{SecretsError, SecretsResult};
use crate::key_backend::EncryptedDekData;
use crate::types::{SecretId, SecretScope, SecretVersionId, WeaverId};
use loom_server_auth::types::{OrgId, UserId};

/// A stored secret with encrypted value.
#[derive(Debug, Clone)]
pub struct StoredSecret {
	pub id: SecretId,
	pub org_id: OrgId,
	pub scope: SecretScope,
	pub repo_id: Option<Uuid>,
	pub weaver_id: Option<String>,
	pub name: String,
	pub description: Option<String>,
	pub current_version: i32,
	pub created_by: UserId,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

/// A stored secret version with encrypted data.
#[derive(Debug, Clone)]
pub struct StoredSecretVersion {
	pub id: SecretVersionId,
	pub secret_id: SecretId,
	pub version: i32,
	pub ciphertext: Vec<u8>,
	pub nonce: Vec<u8>,
	pub dek_id: String,
	pub created_by: UserId,
	pub created_at: DateTime<Utc>,
	pub expires_at: Option<DateTime<Utc>>,
	pub disabled_at: Option<DateTime<Utc>>,
}

/// A stored encrypted DEK.
#[derive(Debug, Clone, FromRow)]
pub struct StoredDek {
	pub id: String,
	pub encrypted_key: Vec<u8>,
	pub nonce: Vec<u8>,
	pub kek_version: i32,
	pub created_at: String,
}

impl TryFrom<StoredDek> for EncryptedDekData {
	type Error = SecretsError;

	fn try_from(stored: StoredDek) -> Result<Self, Self::Error> {
		if stored.nonce.len() != 12 {
			return Err(SecretsError::InvalidNonce(format!(
				"expected 12 bytes, got {}",
				stored.nonce.len()
			)));
		}
		let mut nonce = [0u8; 12];
		nonce.copy_from_slice(&stored.nonce);
		Ok(Self {
			id: stored.id,
			encrypted_key: stored.encrypted_key,
			nonce,
			kek_version: stored.kek_version as u32,
		})
	}
}

/// Request to create a new secret.
#[derive(Debug, Clone)]
pub struct CreateSecretRequest {
	pub org_id: OrgId,
	pub scope: SecretScope,
	pub repo_id: Option<Uuid>,
	pub weaver_id: Option<String>,
	pub name: String,
	pub description: Option<String>,
	pub ciphertext: Vec<u8>,
	pub nonce: Vec<u8>,
	pub dek_id: String,
	pub created_by: UserId,
}

/// Request to create a new secret version.
#[derive(Debug, Clone)]
pub struct CreateVersionRequest {
	pub secret_id: SecretId,
	pub ciphertext: Vec<u8>,
	pub nonce: Vec<u8>,
	pub dek_id: String,
	pub created_by: UserId,
	pub expires_at: Option<DateTime<Utc>>,
}

/// Filter for listing secrets.
#[derive(Debug, Clone, Default)]
pub struct SecretFilter {
	pub org_id: Option<OrgId>,
	pub scope: Option<SecretScope>,
	pub repo_id: Option<Uuid>,
	pub weaver_id: Option<String>,
	pub name: Option<String>,
}

/// Trait for secret storage operations.
#[async_trait]
pub trait SecretStore: Send + Sync {
	/// Create a new secret with initial version.
	async fn create_secret(&self, request: CreateSecretRequest) -> SecretsResult<StoredSecret>;

	/// Get a secret by ID.
	async fn get_secret(&self, id: SecretId) -> SecretsResult<Option<StoredSecret>>;

	/// Get a secret by name and scope.
	async fn get_secret_by_name(
		&self,
		org_id: OrgId,
		scope: SecretScope,
		repo_id: Option<Uuid>,
		weaver_id: Option<&str>,
		name: &str,
	) -> SecretsResult<Option<StoredSecret>>;

	/// List secrets matching a filter.
	async fn list_secrets(&self, filter: &SecretFilter) -> SecretsResult<Vec<StoredSecret>>;

	/// Create a new version of a secret.
	async fn create_version(&self, request: CreateVersionRequest)
		-> SecretsResult<StoredSecretVersion>;

	/// Get the current version of a secret.
	async fn get_current_version(
		&self,
		secret_id: SecretId,
	) -> SecretsResult<Option<StoredSecretVersion>>;

	/// Get a specific version of a secret.
	async fn get_version(
		&self,
		secret_id: SecretId,
		version: i32,
	) -> SecretsResult<Option<StoredSecretVersion>>;

	/// Disable a secret version (revocation).
	async fn disable_version(&self, version_id: SecretVersionId) -> SecretsResult<()>;

	/// Soft delete a secret.
	async fn delete_secret(&self, id: SecretId) -> SecretsResult<()>;

	/// Store an encrypted DEK.
	async fn store_dek(&self, dek: &EncryptedDekData) -> SecretsResult<()>;

	/// Get an encrypted DEK by ID.
	async fn get_dek(&self, id: &str) -> SecretsResult<Option<EncryptedDekData>>;
}

/// SQLite implementation of SecretStore.
pub struct SqliteSecretStore {
	pool: SqlitePool,
}

impl SqliteSecretStore {
	/// Create a new SQLite secret store.
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

#[async_trait]
impl SecretStore for SqliteSecretStore {
	#[instrument(skip(self, request), fields(name = %request.name, scope = ?request.scope))]
	async fn create_secret(&self, request: CreateSecretRequest) -> SecretsResult<StoredSecret> {
		let secret_id = SecretId::generate();
		let version_id = SecretVersionId::generate();
		let now = Utc::now();
		let now_str = now.to_rfc3339();

		let scope_str = request.scope.as_str();
		let repo_id_str = request.repo_id.map(|id| id.to_string());

		let mut tx = self.pool.begin().await.map_err(SecretsError::Database)?;

		// Insert secret
		sqlx::query(
			r#"
            INSERT INTO secrets (id, org_id, scope, repo_id, weaver_id, name, description, current_version, created_by, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)
            "#,
		)
		.bind(secret_id.to_string())
		.bind(request.org_id.to_string())
		.bind(scope_str)
		.bind(&repo_id_str)
		.bind(&request.weaver_id)
		.bind(&request.name)
		.bind(&request.description)
		.bind(request.created_by.to_string())
		.bind(&now_str)
		.bind(&now_str)
		.execute(&mut *tx)
		.await
		.map_err(|e| {
			if is_unique_constraint_error(&e) {
				SecretsError::SecretAlreadyExists(request.name.clone())
			} else {
				SecretsError::Database(e)
			}
		})?;

		// Insert initial version
		sqlx::query(
			r#"
            INSERT INTO secret_versions (id, secret_id, version, ciphertext, nonce, dek_id, created_by, created_at)
            VALUES (?, ?, 1, ?, ?, ?, ?, ?)
            "#,
		)
		.bind(version_id.to_string())
		.bind(secret_id.to_string())
		.bind(&request.ciphertext)
		.bind(&request.nonce)
		.bind(&request.dek_id)
		.bind(request.created_by.to_string())
		.bind(&now_str)
		.execute(&mut *tx)
		.await
		.map_err(SecretsError::Database)?;

		tx.commit().await.map_err(SecretsError::Database)?;

		debug!(secret_id = %secret_id, name = %request.name, "Created secret");

		Ok(StoredSecret {
			id: secret_id,
			org_id: request.org_id,
			scope: request.scope,
			repo_id: request.repo_id,
			weaver_id: request.weaver_id,
			name: request.name,
			description: request.description,
			current_version: 1,
			created_by: request.created_by,
			created_at: now,
			updated_at: now,
		})
	}

	async fn get_secret(&self, id: SecretId) -> SecretsResult<Option<StoredSecret>> {
		let row = sqlx::query(
			r#"
            SELECT id, org_id, scope, repo_id, weaver_id, name, description, current_version, created_by, created_at, updated_at
            FROM secrets
            WHERE id = ? AND deleted_at IS NULL
            "#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await
		.map_err(SecretsError::Database)?;

		match row {
			Some(row) => Ok(Some(parse_secret_row(&row)?)),
			None => Ok(None),
		}
	}

	async fn get_secret_by_name(
		&self,
		org_id: OrgId,
		scope: SecretScope,
		repo_id: Option<Uuid>,
		weaver_id: Option<&str>,
		name: &str,
	) -> SecretsResult<Option<StoredSecret>> {
		let scope_str = scope.as_str();
		let repo_id_str = repo_id.map(|id| id.to_string());

		let row = sqlx::query(
			r#"
            SELECT id, org_id, scope, repo_id, weaver_id, name, description, current_version, created_by, created_at, updated_at
            FROM secrets
            WHERE org_id = ? AND scope = ? AND (repo_id = ? OR (repo_id IS NULL AND ? IS NULL))
              AND (weaver_id = ? OR (weaver_id IS NULL AND ? IS NULL))
              AND name = ? AND deleted_at IS NULL
            "#,
		)
		.bind(org_id.to_string())
		.bind(scope_str)
		.bind(&repo_id_str)
		.bind(&repo_id_str)
		.bind(weaver_id)
		.bind(weaver_id)
		.bind(name)
		.fetch_optional(&self.pool)
		.await
		.map_err(SecretsError::Database)?;

		match row {
			Some(row) => Ok(Some(parse_secret_row(&row)?)),
			None => Ok(None),
		}
	}

	async fn list_secrets(&self, filter: &SecretFilter) -> SecretsResult<Vec<StoredSecret>> {
		// Build dynamic query based on filter
		let mut query = String::from(
			r#"
            SELECT id, org_id, scope, repo_id, weaver_id, name, description, current_version, created_by, created_at, updated_at
            FROM secrets
            WHERE deleted_at IS NULL
            "#,
		);

		if filter.org_id.is_some() {
			query.push_str(" AND org_id = ?");
		}
		if filter.scope.is_some() {
			query.push_str(" AND scope = ?");
		}
		if filter.repo_id.is_some() {
			query.push_str(" AND repo_id = ?");
		}
		if filter.weaver_id.is_some() {
			query.push_str(" AND weaver_id = ?");
		}
		if filter.name.is_some() {
			query.push_str(" AND name = ?");
		}

		query.push_str(" ORDER BY name ASC");

		let mut q = sqlx::query(&query);

		if let Some(ref org_id) = filter.org_id {
			q = q.bind(org_id.to_string());
		}
		if let Some(ref scope) = filter.scope {
			q = q.bind(scope.as_str());
		}
		if let Some(ref repo_id) = filter.repo_id {
			q = q.bind(repo_id.to_string());
		}
		if let Some(ref weaver_id) = filter.weaver_id {
			q = q.bind(weaver_id);
		}
		if let Some(ref name) = filter.name {
			q = q.bind(name);
		}

		let rows = q.fetch_all(&self.pool).await.map_err(SecretsError::Database)?;

		rows.iter().map(parse_secret_row).collect()
	}

	async fn create_version(
		&self,
		request: CreateVersionRequest,
	) -> SecretsResult<StoredSecretVersion> {
		let version_id = SecretVersionId::generate();
		let now = Utc::now();
		let now_str = now.to_rfc3339();
		let expires_at_str = request.expires_at.map(|dt| dt.to_rfc3339());

		let mut tx = self.pool.begin().await.map_err(SecretsError::Database)?;

		// Get next version number within transaction
		let next_version: i32 = sqlx::query_scalar(
			"SELECT COALESCE(MAX(version), 0) + 1 FROM secret_versions WHERE secret_id = ?",
		)
		.bind(request.secret_id.to_string())
		.fetch_one(&mut *tx)
		.await
		.map_err(SecretsError::Database)?;

		// Insert version within transaction
		sqlx::query(
			r#"
            INSERT INTO secret_versions (id, secret_id, version, ciphertext, nonce, dek_id, created_by, created_at, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
		)
		.bind(version_id.to_string())
		.bind(request.secret_id.to_string())
		.bind(next_version)
		.bind(&request.ciphertext)
		.bind(&request.nonce)
		.bind(&request.dek_id)
		.bind(request.created_by.to_string())
		.bind(&now_str)
		.bind(&expires_at_str)
		.execute(&mut *tx)
		.await
		.map_err(SecretsError::Database)?;

		// Update secret's current_version within transaction
		// Only update if secret is not soft-deleted (defense in depth)
		let update_result =
			sqlx::query("UPDATE secrets SET current_version = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL")
				.bind(next_version)
				.bind(&now_str)
				.bind(request.secret_id.to_string())
				.execute(&mut *tx)
				.await
				.map_err(SecretsError::Database)?;

		if update_result.rows_affected() == 0 {
			return Err(SecretsError::SecretNotFoundById(request.secret_id));
		}

		tx.commit().await.map_err(SecretsError::Database)?;

		debug!(secret_id = %request.secret_id, version = next_version, "Created secret version");

		Ok(StoredSecretVersion {
			id: version_id,
			secret_id: request.secret_id,
			version: next_version,
			ciphertext: request.ciphertext,
			nonce: request.nonce,
			dek_id: request.dek_id,
			created_by: request.created_by,
			created_at: now,
			expires_at: request.expires_at,
			disabled_at: None,
		})
	}

	async fn get_current_version(
		&self,
		secret_id: SecretId,
	) -> SecretsResult<Option<StoredSecretVersion>> {
		let row = sqlx::query(
			r#"
            SELECT v.id, v.secret_id, v.version, v.ciphertext, v.nonce, v.dek_id, v.created_by, v.created_at, v.expires_at, v.disabled_at
            FROM secret_versions v
            JOIN secrets s ON s.id = v.secret_id AND s.current_version = v.version
            WHERE v.secret_id = ? AND v.disabled_at IS NULL AND s.deleted_at IS NULL
            "#,
		)
		.bind(secret_id.to_string())
		.fetch_optional(&self.pool)
		.await
		.map_err(SecretsError::Database)?;

		match row {
			Some(row) => Ok(Some(parse_version_row(&row)?)),
			None => Ok(None),
		}
	}

	async fn get_version(
		&self,
		secret_id: SecretId,
		version: i32,
	) -> SecretsResult<Option<StoredSecretVersion>> {
		let row = sqlx::query(
			r#"
            SELECT v.id, v.secret_id, v.version, v.ciphertext, v.nonce, v.dek_id, v.created_by, v.created_at, v.expires_at, v.disabled_at
            FROM secret_versions v
            JOIN secrets s ON s.id = v.secret_id
            WHERE v.secret_id = ? AND v.version = ? AND s.deleted_at IS NULL
            "#,
		)
		.bind(secret_id.to_string())
		.bind(version)
		.fetch_optional(&self.pool)
		.await
		.map_err(SecretsError::Database)?;

		match row {
			Some(row) => Ok(Some(parse_version_row(&row)?)),
			None => Ok(None),
		}
	}

	async fn disable_version(&self, version_id: SecretVersionId) -> SecretsResult<()> {
		let now_str = Utc::now().to_rfc3339();
		sqlx::query("UPDATE secret_versions SET disabled_at = ? WHERE id = ?")
			.bind(&now_str)
			.bind(version_id.to_string())
			.execute(&self.pool)
			.await
			.map_err(SecretsError::Database)?;

		Ok(())
	}

	async fn delete_secret(&self, id: SecretId) -> SecretsResult<()> {
		let now_str = Utc::now().to_rfc3339();
		sqlx::query("UPDATE secrets SET deleted_at = ? WHERE id = ?")
			.bind(&now_str)
			.bind(id.to_string())
			.execute(&self.pool)
			.await
			.map_err(SecretsError::Database)?;

		Ok(())
	}

	async fn store_dek(&self, dek: &EncryptedDekData) -> SecretsResult<()> {
		let now_str = Utc::now().to_rfc3339();
		sqlx::query(
			r#"
            INSERT INTO encrypted_deks (id, encrypted_key, nonce, kek_version, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
		)
		.bind(&dek.id)
		.bind(&dek.encrypted_key)
		.bind(dek.nonce.to_vec())
		.bind(dek.kek_version as i32)
		.bind(&now_str)
		.execute(&self.pool)
		.await
		.map_err(SecretsError::Database)?;

		Ok(())
	}

	async fn get_dek(&self, id: &str) -> SecretsResult<Option<EncryptedDekData>> {
		let row = sqlx::query_as::<_, StoredDek>(
			"SELECT id, encrypted_key, nonce, kek_version, created_at FROM encrypted_deks WHERE id = ?",
		)
		.bind(id)
		.fetch_optional(&self.pool)
		.await
		.map_err(SecretsError::Database)?;

		match row {
			Some(stored) => Ok(Some(stored.try_into()?)),
			None => Ok(None),
		}
	}
}

fn parse_secret_row(row: &sqlx::sqlite::SqliteRow) -> SecretsResult<StoredSecret> {
	let id: String = row.get("id");
	let org_id: String = row.get("org_id");
	let scope: String = row.get("scope");
	let repo_id: Option<String> = row.get("repo_id");
	let weaver_id: Option<String> = row.get("weaver_id");
	let name: String = row.get("name");
	let description: Option<String> = row.get("description");
	let current_version: i32 = row.get("current_version");
	let created_by: String = row.get("created_by");
	let created_at: String = row.get("created_at");
	let updated_at: String = row.get("updated_at");

	let parsed_org_id = OrgId::new(
		Uuid::parse_str(&org_id)
			.map_err(|_| SecretsError::CorruptedData(format!("invalid org id: {}", org_id)))?,
	);

	let parsed_repo_id = repo_id
		.as_ref()
		.map(|s| {
			Uuid::parse_str(s)
				.map_err(|_| SecretsError::CorruptedData(format!("invalid repo id: {}", s)))
		})
		.transpose()?;

	let parsed_scope = match scope.as_str() {
		"org" => SecretScope::Org {
			org_id: parsed_org_id,
		},
		"repo" => {
			let repo_id_str = repo_id
				.as_ref()
				.ok_or_else(|| SecretsError::CorruptedData("repo scope requires repo_id".into()))?;
			SecretScope::Repo {
				org_id: parsed_org_id,
				repo_id: repo_id_str.clone(),
			}
		}
		"weaver" => {
			let weaver_id_str = weaver_id
				.as_ref()
				.ok_or_else(|| SecretsError::CorruptedData("weaver scope requires weaver_id".into()))?;
			let wid = weaver_id_str
				.parse::<uuid7::Uuid>()
				.map_err(|_| SecretsError::CorruptedData(format!("invalid weaver_id: {}", weaver_id_str)))?;
			SecretScope::Weaver {
				weaver_id: WeaverId::new(wid),
			}
		}
		other => {
			return Err(SecretsError::CorruptedData(format!(
				"unknown scope type: {}",
				other
			)));
		}
	};

	Ok(StoredSecret {
		id: SecretId::new(
			Uuid::parse_str(&id)
				.map_err(|_| SecretsError::CorruptedData(format!("invalid secret id: {}", id)))?,
		),
		org_id: parsed_org_id,
		scope: parsed_scope,
		repo_id: parsed_repo_id,
		weaver_id,
		name,
		description,
		current_version,
		created_by: UserId::new(
			Uuid::parse_str(&created_by).map_err(|_| {
				SecretsError::CorruptedData(format!("invalid created_by id: {}", created_by))
			})?,
		),
		created_at: DateTime::parse_from_rfc3339(&created_at)
			.map(|dt| dt.with_timezone(&Utc))
			.map_err(|_| {
				SecretsError::CorruptedData(format!("invalid created_at timestamp: {}", created_at))
			})?,
		updated_at: DateTime::parse_from_rfc3339(&updated_at)
			.map(|dt| dt.with_timezone(&Utc))
			.map_err(|_| {
				SecretsError::CorruptedData(format!("invalid updated_at timestamp: {}", updated_at))
			})?,
	})
}

fn parse_version_row(row: &sqlx::sqlite::SqliteRow) -> SecretsResult<StoredSecretVersion> {
	let id: String = row.get("id");
	let secret_id: String = row.get("secret_id");
	let version: i32 = row.get("version");
	let ciphertext: Vec<u8> = row.get("ciphertext");
	let nonce: Vec<u8> = row.get("nonce");
	let dek_id: String = row.get("dek_id");
	let created_by: String = row.get("created_by");
	let created_at: String = row.get("created_at");
	let expires_at: Option<String> = row.get("expires_at");
	let disabled_at: Option<String> = row.get("disabled_at");

	Ok(StoredSecretVersion {
		id: SecretVersionId::new(
			Uuid::parse_str(&id)
				.map_err(|_| SecretsError::CorruptedData(format!("invalid version id: {}", id)))?,
		),
		secret_id: SecretId::new(
			Uuid::parse_str(&secret_id).map_err(|_| {
				SecretsError::CorruptedData(format!("invalid secret id: {}", secret_id))
			})?,
		),
		version,
		ciphertext,
		nonce,
		dek_id,
		created_by: UserId::new(
			Uuid::parse_str(&created_by).map_err(|_| {
				SecretsError::CorruptedData(format!("invalid created_by id: {}", created_by))
			})?,
		),
		created_at: DateTime::parse_from_rfc3339(&created_at)
			.map(|dt| dt.with_timezone(&Utc))
			.map_err(|_| {
				SecretsError::CorruptedData(format!("invalid created_at timestamp: {}", created_at))
			})?,
		expires_at: expires_at
			.map(|s| {
				DateTime::parse_from_rfc3339(&s)
					.map(|dt| dt.with_timezone(&Utc))
					.map_err(|_| {
						SecretsError::CorruptedData(format!("invalid expires_at timestamp: {}", s))
					})
			})
			.transpose()?,
		disabled_at: disabled_at
			.map(|s| {
				DateTime::parse_from_rfc3339(&s)
					.map(|dt| dt.with_timezone(&Utc))
					.map_err(|_| {
						SecretsError::CorruptedData(format!("invalid disabled_at timestamp: {}", s))
					})
			})
			.transpose()?,
	})
}

fn is_unique_constraint_error(e: &sqlx::Error) -> bool {
	if let sqlx::Error::Database(ref db_err) = e {
		return db_err.message().contains("UNIQUE constraint failed");
	}
	false
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_server_auth::types::OrgId;

	async fn create_test_pool() -> SqlitePool {
		let pool = SqlitePool::connect(":memory:").await.unwrap();
		run_test_migrations(&pool).await.unwrap();
		pool
	}

	async fn run_test_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
		let migration = include_str!("../../loom-server/migrations/025_weaver_secrets.sql");
		for stmt in migration.split(';').filter(|s| !s.trim().is_empty()) {
			let trimmed = stmt.trim();
			if !trimmed.is_empty() {
				sqlx::query(trimmed).execute(pool).await?;
			}
		}
		Ok(())
	}

	fn test_user_id() -> UserId {
		UserId::new(Uuid::new_v4())
	}

	fn test_org_id() -> OrgId {
		OrgId::new(Uuid::new_v4())
	}

	#[tokio::test]
	async fn test_create_secret() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let request = CreateSecretRequest {
			org_id,
			scope: SecretScope::Org { org_id },
			repo_id: None,
			weaver_id: None,
			name: "TEST_SECRET".to_string(),
			description: Some("A test secret".to_string()),
			ciphertext: vec![0u8; 16],
			nonce: vec![0u8; 12],
			dek_id: dek_id.clone(),
			created_by: user_id,
		};

		let secret = store.create_secret(request).await.unwrap();
		assert_eq!(secret.name, "TEST_SECRET");
		assert_eq!(secret.current_version, 1);
		assert_eq!(secret.org_id, org_id);
	}

	#[tokio::test]
	async fn test_get_secret_by_id() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let request = CreateSecretRequest {
			org_id,
			scope: SecretScope::Org { org_id },
			repo_id: None,
			weaver_id: None,
			name: "GET_TEST".to_string(),
			description: None,
			ciphertext: vec![0u8; 16],
			nonce: vec![0u8; 12],
			dek_id,
			created_by: user_id,
		};

		let created = store.create_secret(request).await.unwrap();
		let fetched = store.get_secret(created.id).await.unwrap();

		assert!(fetched.is_some());
		let fetched = fetched.unwrap();
		assert_eq!(fetched.id, created.id);
		assert_eq!(fetched.name, "GET_TEST");
	}

	#[tokio::test]
	async fn test_get_secret_by_name() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let scope = SecretScope::Org { org_id };
		let request = CreateSecretRequest {
			org_id,
			scope: scope.clone(),
			repo_id: None,
			weaver_id: None,
			name: "API_KEY".to_string(),
			description: None,
			ciphertext: vec![0u8; 16],
			nonce: vec![0u8; 12],
			dek_id,
			created_by: user_id,
		};

		let created = store.create_secret(request).await.unwrap();
		let fetched = store
			.get_secret_by_name(org_id, scope, None, None, "API_KEY")
			.await
			.unwrap();

		assert!(fetched.is_some());
		assert_eq!(fetched.unwrap().id, created.id);
	}

	#[tokio::test]
	async fn test_list_secrets() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();

		for i in 0..3 {
			let dek_id = Uuid::new_v4().to_string();
			store
				.store_dek(&EncryptedDekData {
					id: dek_id.clone(),
					encrypted_key: vec![0u8; 32],
					nonce: [1u8; 12],
					kek_version: 1,
				})
				.await
				.unwrap();

			let request = CreateSecretRequest {
				org_id,
				scope: SecretScope::Org { org_id },
				repo_id: None,
				weaver_id: None,
				name: format!("SECRET_{}", i),
				description: None,
				ciphertext: vec![0u8; 16],
				nonce: vec![0u8; 12],
				dek_id,
				created_by: user_id,
			};
			store.create_secret(request).await.unwrap();
		}

		let filter = SecretFilter {
			org_id: Some(org_id),
			..Default::default()
		};
		let secrets = store.list_secrets(&filter).await.unwrap();
		assert_eq!(secrets.len(), 3);
	}

	#[tokio::test]
	async fn test_create_version() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let request = CreateSecretRequest {
			org_id,
			scope: SecretScope::Org { org_id },
			repo_id: None,
			weaver_id: None,
			name: "VERSIONED_SECRET".to_string(),
			description: None,
			ciphertext: vec![0u8; 16],
			nonce: vec![0u8; 12],
			dek_id: dek_id.clone(),
			created_by: user_id,
		};

		let secret = store.create_secret(request).await.unwrap();
		assert_eq!(secret.current_version, 1);

		let version_request = CreateVersionRequest {
			secret_id: secret.id,
			ciphertext: vec![1u8; 16],
			nonce: vec![1u8; 12],
			dek_id: dek_id.clone(),
			created_by: user_id,
			expires_at: None,
		};

		let version = store.create_version(version_request).await.unwrap();
		assert_eq!(version.version, 2);

		let updated = store.get_secret(secret.id).await.unwrap().unwrap();
		assert_eq!(updated.current_version, 2);
	}

	#[tokio::test]
	async fn test_get_current_version() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let request = CreateSecretRequest {
			org_id,
			scope: SecretScope::Org { org_id },
			repo_id: None,
			weaver_id: None,
			name: "CURRENT_VERSION_TEST".to_string(),
			description: None,
			ciphertext: vec![0u8; 16],
			nonce: vec![0u8; 12],
			dek_id,
			created_by: user_id,
		};

		let secret = store.create_secret(request).await.unwrap();
		let current = store.get_current_version(secret.id).await.unwrap();

		assert!(current.is_some());
		let current = current.unwrap();
		assert_eq!(current.version, 1);
		assert_eq!(current.ciphertext, vec![0u8; 16]);
	}

	#[tokio::test]
	async fn test_get_specific_version() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let request = CreateSecretRequest {
			org_id,
			scope: SecretScope::Org { org_id },
			repo_id: None,
			weaver_id: None,
			name: "SPECIFIC_VERSION_TEST".to_string(),
			description: None,
			ciphertext: vec![1u8; 16],
			nonce: vec![0u8; 12],
			dek_id: dek_id.clone(),
			created_by: user_id,
		};

		let secret = store.create_secret(request).await.unwrap();

		store
			.create_version(CreateVersionRequest {
				secret_id: secret.id,
				ciphertext: vec![2u8; 16],
				nonce: vec![1u8; 12],
				dek_id: dek_id.clone(),
				created_by: user_id,
				expires_at: None,
			})
			.await
			.unwrap();

		let v1 = store.get_version(secret.id, 1).await.unwrap().unwrap();
		let v2 = store.get_version(secret.id, 2).await.unwrap().unwrap();

		assert_eq!(v1.ciphertext, vec![1u8; 16]);
		assert_eq!(v2.ciphertext, vec![2u8; 16]);
	}

	#[tokio::test]
	async fn test_disable_version() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let request = CreateSecretRequest {
			org_id,
			scope: SecretScope::Org { org_id },
			repo_id: None,
			weaver_id: None,
			name: "DISABLE_TEST".to_string(),
			description: None,
			ciphertext: vec![0u8; 16],
			nonce: vec![0u8; 12],
			dek_id,
			created_by: user_id,
		};

		let secret = store.create_secret(request).await.unwrap();
		let version = store.get_current_version(secret.id).await.unwrap().unwrap();

		store.disable_version(version.id).await.unwrap();

		let current = store.get_current_version(secret.id).await.unwrap();
		assert!(current.is_none());
	}

	#[tokio::test]
	async fn test_delete_secret() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let request = CreateSecretRequest {
			org_id,
			scope: SecretScope::Org { org_id },
			repo_id: None,
			weaver_id: None,
			name: "DELETE_TEST".to_string(),
			description: None,
			ciphertext: vec![0u8; 16],
			nonce: vec![0u8; 12],
			dek_id,
			created_by: user_id,
		};

		let secret = store.create_secret(request).await.unwrap();
		store.delete_secret(secret.id).await.unwrap();

		let fetched = store.get_secret(secret.id).await.unwrap();
		assert!(fetched.is_none());
	}

	#[tokio::test]
	async fn test_store_and_get_dek() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let dek = EncryptedDekData {
			id: Uuid::new_v4().to_string(),
			encrypted_key: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32],
			nonce: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
			kek_version: 1,
		};

		store.store_dek(&dek).await.unwrap();

		let fetched = store.get_dek(&dek.id).await.unwrap();
		assert!(fetched.is_some());
		let fetched = fetched.unwrap();
		assert_eq!(fetched.id, dek.id);
		assert_eq!(fetched.encrypted_key, dek.encrypted_key);
		assert_eq!(fetched.nonce, dek.nonce);
		assert_eq!(fetched.kek_version, dek.kek_version);
	}

	#[tokio::test]
	async fn test_secret_id_uniqueness() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let secret1 = store
			.create_secret(CreateSecretRequest {
				org_id,
				scope: SecretScope::Org { org_id },
				repo_id: None,
				weaver_id: None,
				name: "SECRET_A".to_string(),
				description: None,
				ciphertext: vec![0u8; 16],
				nonce: vec![0u8; 12],
				dek_id: dek_id.clone(),
				created_by: user_id,
			})
			.await
			.unwrap();

		let secret2 = store
			.create_secret(CreateSecretRequest {
				org_id,
				scope: SecretScope::Org { org_id },
				repo_id: None,
				weaver_id: None,
				name: "SECRET_B".to_string(),
				description: None,
				ciphertext: vec![0u8; 16],
				nonce: vec![0u8; 12],
				dek_id: dek_id.clone(),
				created_by: user_id,
			})
			.await
			.unwrap();

		assert_ne!(secret1.id, secret2.id);
	}

	#[tokio::test]
	async fn test_repo_scoped_secret() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let repo_id = Uuid::new_v4();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let scope = SecretScope::Repo {
			org_id,
			repo_id: repo_id.to_string(),
		};

		let request = CreateSecretRequest {
			org_id,
			scope: scope.clone(),
			repo_id: Some(repo_id),
			weaver_id: None,
			name: "REPO_SECRET".to_string(),
			description: None,
			ciphertext: vec![0u8; 16],
			nonce: vec![0u8; 12],
			dek_id,
			created_by: user_id,
		};

		let secret = store.create_secret(request).await.unwrap();
		assert_eq!(secret.repo_id, Some(repo_id));

		let filter = SecretFilter {
			org_id: Some(org_id),
			repo_id: Some(repo_id),
			..Default::default()
		};
		let secrets = store.list_secrets(&filter).await.unwrap();
		assert_eq!(secrets.len(), 1);
		assert_eq!(secrets[0].name, "REPO_SECRET");
	}

	#[tokio::test]
	async fn test_weaver_scoped_secret() {
		let pool = create_test_pool().await;
		let store = SqliteSecretStore::new(pool);

		let org_id = test_org_id();
		let user_id = test_user_id();
		let weaver_id = WeaverId::generate();
		let dek_id = Uuid::new_v4().to_string();

		store
			.store_dek(&EncryptedDekData {
				id: dek_id.clone(),
				encrypted_key: vec![0u8; 32],
				nonce: [1u8; 12],
				kek_version: 1,
			})
			.await
			.unwrap();

		let scope = SecretScope::Weaver { weaver_id };

		let request = CreateSecretRequest {
			org_id,
			scope,
			repo_id: None,
			weaver_id: Some(weaver_id.to_string()),
			name: "WEAVER_SECRET".to_string(),
			description: None,
			ciphertext: vec![0u8; 16],
			nonce: vec![0u8; 12],
			dek_id,
			created_by: user_id,
		};

		let secret = store.create_secret(request).await.unwrap();
		assert_eq!(secret.weaver_id, Some(weaver_id.to_string()));

		let filter = SecretFilter {
			weaver_id: Some(weaver_id.to_string()),
			..Default::default()
		};
		let secrets = store.list_secrets(&filter).await.unwrap();
		assert_eq!(secrets.len(), 1);
	}
}
