// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{Result, ScmError};
use crate::types::BranchProtectionRule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectionViolation {
	DirectPushBlocked { branch: String, pattern: String },
	ForcePushBlocked { branch: String, pattern: String },
	DeletionBlocked { branch: String, pattern: String },
}

impl std::fmt::Display for ProtectionViolation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ProtectionViolation::DirectPushBlocked { branch, pattern } => {
				write!(
					f,
					"Direct push to branch '{}' is blocked by protection rule '{}'",
					branch, pattern
				)
			}
			ProtectionViolation::ForcePushBlocked { branch, pattern } => {
				write!(
					f,
					"Force push to branch '{}' is blocked by protection rule '{}'",
					branch, pattern
				)
			}
			ProtectionViolation::DeletionBlocked { branch, pattern } => {
				write!(
					f,
					"Deletion of branch '{}' is blocked by protection rule '{}'",
					branch, pattern
				)
			}
		}
	}
}

#[derive(Debug, Clone)]
pub struct PushCheck {
	pub branch: String,
	pub is_force_push: bool,
	pub is_deletion: bool,
	pub user_is_admin: bool,
}

#[async_trait]
pub trait ProtectionStore: Send + Sync {
	async fn create(&self, rule: &BranchProtectionRule) -> Result<BranchProtectionRule>;
	async fn list_by_repo(&self, repo_id: Uuid) -> Result<Vec<BranchProtectionRule>>;
	async fn get_by_id(&self, id: Uuid) -> Result<Option<BranchProtectionRule>>;
	async fn delete(&self, id: Uuid) -> Result<()>;
}

pub struct SqliteProtectionStore {
	pool: SqlitePool,
}

impl SqliteProtectionStore {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	fn row_to_rule(&self, row: &sqlx::sqlite::SqliteRow) -> Result<BranchProtectionRule> {
		let id_str: String = row.get("id");
		let repo_id_str: String = row.get("repo_id");
		let created_at_str: String = row.get("created_at");
		let block_direct_push: i32 = row.get("block_direct_push");
		let block_force_push: i32 = row.get("block_force_push");
		let block_deletion: i32 = row.get("block_deletion");

		Ok(BranchProtectionRule {
			id: Uuid::parse_str(&id_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			repo_id: Uuid::parse_str(&repo_id_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			pattern: row.get("pattern"),
			block_direct_push: block_direct_push != 0,
			block_force_push: block_force_push != 0,
			block_deletion: block_deletion != 0,
			created_at: DateTime::parse_from_rfc3339(&created_at_str)
				.map(|d| d.with_timezone(&Utc))
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
		})
	}
}

#[async_trait]
impl ProtectionStore for SqliteProtectionStore {
	async fn create(&self, rule: &BranchProtectionRule) -> Result<BranchProtectionRule> {
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
				ScmError::AlreadyExists
			}
			_ => ScmError::Database(e),
		})?;

		Ok(rule.clone())
	}

	async fn list_by_repo(&self, repo_id: Uuid) -> Result<Vec<BranchProtectionRule>> {
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

		rows.iter().map(|r| self.row_to_rule(r)).collect()
	}

	async fn get_by_id(&self, id: Uuid) -> Result<Option<BranchProtectionRule>> {
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

		row.map(|r| self.row_to_rule(&r)).transpose()
	}

	async fn delete(&self, id: Uuid) -> Result<()> {
		let result = sqlx::query(
			r#"
			DELETE FROM branch_protection_rules WHERE id = ?
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

pub fn matches_pattern(pattern: &str, branch: &str) -> bool {
	if pattern == branch {
		return true;
	}

	if let Some(prefix) = pattern.strip_suffix("/*") {
		return branch.starts_with(&format!("{}/", prefix));
	}

	if let Some(prefix) = pattern.strip_suffix('*') {
		return branch.starts_with(prefix);
	}

	false
}

pub fn check_push_allowed(
	rules: &[BranchProtectionRule],
	check: &PushCheck,
) -> std::result::Result<(), ProtectionViolation> {
	if check.user_is_admin {
		return Ok(());
	}

	for rule in rules {
		if !matches_pattern(&rule.pattern, &check.branch) {
			continue;
		}

		if check.is_deletion && rule.block_deletion {
			return Err(ProtectionViolation::DeletionBlocked {
				branch: check.branch.clone(),
				pattern: rule.pattern.clone(),
			});
		}

		if check.is_force_push && rule.block_force_push {
			return Err(ProtectionViolation::ForcePushBlocked {
				branch: check.branch.clone(),
				pattern: rule.pattern.clone(),
			});
		}

		if rule.block_direct_push {
			return Err(ProtectionViolation::DirectPushBlocked {
				branch: check.branch.clone(),
				pattern: rule.pattern.clone(),
			});
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_matches_pattern_exact() {
		assert!(matches_pattern("cannon", "cannon"));
		assert!(!matches_pattern("cannon", "main"));
		assert!(!matches_pattern("cannon", "cannons"));
	}

	#[test]
	fn test_matches_pattern_wildcard() {
		assert!(matches_pattern("release/*", "release/v1.0"));
		assert!(matches_pattern("release/*", "release/"));
		assert!(!matches_pattern("release/*", "release"));
		assert!(!matches_pattern("release/*", "releases/v1.0"));
	}

	#[test]
	fn test_matches_pattern_prefix_wildcard() {
		assert!(matches_pattern("feat*", "feature"));
		assert!(matches_pattern("feat*", "feat"));
		assert!(matches_pattern("feat*", "feat-new"));
		assert!(!matches_pattern("feat*", "fix"));
	}

	#[test]
	fn test_check_push_allowed_admin_bypass() {
		let rules = vec![BranchProtectionRule::new(
			Uuid::new_v4(),
			"cannon".to_string(),
		)];
		let check = PushCheck {
			branch: "cannon".to_string(),
			is_force_push: true,
			is_deletion: true,
			user_is_admin: true,
		};
		assert!(check_push_allowed(&rules, &check).is_ok());
	}

	#[test]
	fn test_check_push_allowed_direct_push_blocked() {
		let rules = vec![BranchProtectionRule::new(
			Uuid::new_v4(),
			"cannon".to_string(),
		)];
		let check = PushCheck {
			branch: "cannon".to_string(),
			is_force_push: false,
			is_deletion: false,
			user_is_admin: false,
		};
		let result = check_push_allowed(&rules, &check);
		assert!(matches!(
			result,
			Err(ProtectionViolation::DirectPushBlocked { .. })
		));
	}

	#[test]
	fn test_check_push_allowed_force_push_blocked() {
		let mut rule = BranchProtectionRule::new(Uuid::new_v4(), "cannon".to_string());
		rule.block_direct_push = false;
		let rules = vec![rule];
		let check = PushCheck {
			branch: "cannon".to_string(),
			is_force_push: true,
			is_deletion: false,
			user_is_admin: false,
		};
		let result = check_push_allowed(&rules, &check);
		assert!(matches!(
			result,
			Err(ProtectionViolation::ForcePushBlocked { .. })
		));
	}

	#[test]
	fn test_check_push_allowed_deletion_blocked() {
		let mut rule = BranchProtectionRule::new(Uuid::new_v4(), "cannon".to_string());
		rule.block_direct_push = false;
		rule.block_force_push = false;
		let rules = vec![rule];
		let check = PushCheck {
			branch: "cannon".to_string(),
			is_force_push: false,
			is_deletion: true,
			user_is_admin: false,
		};
		let result = check_push_allowed(&rules, &check);
		assert!(matches!(
			result,
			Err(ProtectionViolation::DeletionBlocked { .. })
		));
	}

	#[test]
	fn test_check_push_allowed_unprotected_branch() {
		let rules = vec![BranchProtectionRule::new(
			Uuid::new_v4(),
			"cannon".to_string(),
		)];
		let check = PushCheck {
			branch: "feature/new-thing".to_string(),
			is_force_push: true,
			is_deletion: true,
			user_is_admin: false,
		};
		assert!(check_push_allowed(&rules, &check).is_ok());
	}

	#[test]
	fn test_check_push_allowed_wildcard_pattern() {
		let rules = vec![BranchProtectionRule::new(
			Uuid::new_v4(),
			"release/*".to_string(),
		)];
		let check = PushCheck {
			branch: "release/v1.0".to_string(),
			is_force_push: false,
			is_deletion: false,
			user_is_admin: false,
		};
		let result = check_push_allowed(&rules, &check);
		assert!(matches!(
			result,
			Err(ProtectionViolation::DirectPushBlocked { .. })
		));
	}
}
