// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;

use crate::error::DbError;

#[derive(Debug, Clone)]
pub struct BranchProtectionRuleRecord {
	pub id: Uuid,
	pub repo_id: Uuid,
	pub pattern: String,
	pub block_direct_push: bool,
	pub block_force_push: bool,
	pub block_deletion: bool,
	pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait ProtectionStore: Send + Sync {
	async fn create(
		&self,
		rule: &BranchProtectionRuleRecord,
	) -> Result<BranchProtectionRuleRecord, DbError>;
	async fn list_by_repo(&self, repo_id: Uuid) -> Result<Vec<BranchProtectionRuleRecord>, DbError>;
	async fn get_by_id(&self, id: Uuid) -> Result<Option<BranchProtectionRuleRecord>, DbError>;
	async fn delete(&self, id: Uuid) -> Result<(), DbError>;
}

#[derive(Clone)]
pub struct ProtectionRepository {
	pool: SqlitePool,
}

impl ProtectionRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

#[async_trait]
impl ProtectionStore for ProtectionRepository {
	#[tracing::instrument(skip(self, rule), fields(rule_id = %rule.id, repo_id = %rule.repo_id))]
	async fn create(
		&self,
		rule: &BranchProtectionRuleRecord,
	) -> Result<BranchProtectionRuleRecord, DbError> {
		sqlx::query(
			r#"
			INSERT INTO branch_protection_rules (id, repo_id, pattern, block_direct_push, block_force_push, block_deletion, created_at)
			VALUES (?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(rule.id.to_string())
		.bind(rule.repo_id.to_string())
		.bind(&rule.pattern)
		.bind(rule.block_direct_push as i32)
		.bind(rule.block_force_push as i32)
		.bind(rule.block_deletion as i32)
		.bind(rule.created_at.to_rfc3339())
		.execute(&self.pool)
		.await
		.map_err(|e| match e {
			sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
				DbError::Conflict("Branch protection rule already exists".to_string())
			}
			_ => DbError::Sqlx(e),
		})?;

		Ok(rule.clone())
	}

	#[tracing::instrument(skip(self), fields(repo_id = %repo_id))]
	async fn list_by_repo(&self, repo_id: Uuid) -> Result<Vec<BranchProtectionRuleRecord>, DbError> {
		let rows = sqlx::query(
			r#"
			SELECT id, repo_id, pattern, block_direct_push, block_force_push, block_deletion, created_at
			FROM branch_protection_rules
			WHERE repo_id = ?
			ORDER BY created_at ASC
			"#,
		)
		.bind(repo_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.iter().map(row_to_rule).collect()
	}

	#[tracing::instrument(skip(self), fields(rule_id = %id))]
	async fn get_by_id(&self, id: Uuid) -> Result<Option<BranchProtectionRuleRecord>, DbError> {
		let row = sqlx::query(
			r#"
			SELECT id, repo_id, pattern, block_direct_push, block_force_push, block_deletion, created_at
			FROM branch_protection_rules
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| row_to_rule(&r)).transpose()
	}

	#[tracing::instrument(skip(self), fields(rule_id = %id))]
	async fn delete(&self, id: Uuid) -> Result<(), DbError> {
		let result = sqlx::query(
			r#"
			DELETE FROM branch_protection_rules WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(DbError::NotFound(
				"Branch protection rule not found".to_string(),
			));
		}

		Ok(())
	}
}

fn row_to_rule(row: &sqlx::sqlite::SqliteRow) -> Result<BranchProtectionRuleRecord, DbError> {
	let id_str: String = row.get("id");
	let repo_id_str: String = row.get("repo_id");
	let created_at_str: String = row.get("created_at");
	let block_direct_push: i32 = row.get("block_direct_push");
	let block_force_push: i32 = row.get("block_force_push");
	let block_deletion: i32 = row.get("block_deletion");

	Ok(BranchProtectionRuleRecord {
		id: Uuid::parse_str(&id_str).map_err(|e| DbError::Internal(e.to_string()))?,
		repo_id: Uuid::parse_str(&repo_id_str).map_err(|e| DbError::Internal(e.to_string()))?,
		pattern: row.get("pattern"),
		block_direct_push: block_direct_push != 0,
		block_force_push: block_force_push != 0,
		block_deletion: block_deletion != 0,
		created_at: DateTime::parse_from_rfc3339(&created_at_str)
			.map(|d| d.with_timezone(&Utc))
			.map_err(|e| DbError::Internal(e.to_string()))?,
	})
}
