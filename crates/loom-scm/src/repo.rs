// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{Result, ScmError};
use crate::types::{OwnerType, RepoRole, RepoTeamAccess, Repository, Visibility};

pub fn validate_repo_name(name: &str) -> Result<()> {
	if name.is_empty() || name.len() > 100 {
		return Err(ScmError::InvalidName(
			"Name must be 1-100 characters".into(),
		));
	}

	if name == "." || name == ".." {
		return Err(ScmError::InvalidName("Invalid name".into()));
	}

	if name.starts_with('.') || name.starts_with('-') {
		return Err(ScmError::InvalidName(
			"Name cannot start with '.' or '-'".into(),
		));
	}

	if name.contains("..") {
		return Err(ScmError::InvalidName(
			"Name cannot contain '..'".into(),
		));
	}

	if !name
		.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
	{
		return Err(ScmError::InvalidName(
			"Name can only contain letters, numbers, dash, underscore, dot".into(),
		));
	}

	Ok(())
}

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

#[async_trait]
pub trait RepoTeamAccessStore: Send + Sync {
	async fn grant_team_access(&self, repo_id: Uuid, team_id: Uuid, role: RepoRole) -> Result<()>;
	async fn revoke_team_access(&self, repo_id: Uuid, team_id: Uuid) -> Result<()>;
	async fn list_repo_team_access(&self, repo_id: Uuid) -> Result<Vec<RepoTeamAccess>>;
	async fn get_user_role_via_teams(&self, user_id: Uuid, repo_id: Uuid) -> Result<Option<RepoRole>>;
}

pub struct SqliteRepoTeamAccessStore {
	pool: SqlitePool,
}

impl SqliteRepoTeamAccessStore {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	fn row_to_team_access(&self, row: &sqlx::sqlite::SqliteRow) -> Result<RepoTeamAccess> {
		let repo_id_str: String = row.get("repo_id");
		let team_id_str: String = row.get("team_id");
		let role_str: String = row.get("role");

		Ok(RepoTeamAccess {
			repo_id: Uuid::parse_str(&repo_id_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			team_id: Uuid::parse_str(&team_id_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			role: role_str
				.parse::<RepoRole>()
				.map_err(|_| ScmError::Database(sqlx::Error::Decode(format!("invalid role: {}", role_str).into())))?,
		})
	}
}

#[async_trait]
impl RepoTeamAccessStore for SqliteRepoTeamAccessStore {
	async fn grant_team_access(&self, repo_id: Uuid, team_id: Uuid, role: RepoRole) -> Result<()> {
		sqlx::query(
			r#"
			INSERT INTO repo_team_access (repo_id, team_id, role)
			VALUES (?, ?, ?)
			ON CONFLICT (repo_id, team_id) DO UPDATE SET role = excluded.role
			"#,
		)
		.bind(repo_id.to_string())
		.bind(team_id.to_string())
		.bind(role.as_str())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	async fn revoke_team_access(&self, repo_id: Uuid, team_id: Uuid) -> Result<()> {
		let result = sqlx::query(
			r#"
			DELETE FROM repo_team_access
			WHERE repo_id = ? AND team_id = ?
			"#,
		)
		.bind(repo_id.to_string())
		.bind(team_id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(ScmError::NotFound);
		}

		Ok(())
	}

	async fn list_repo_team_access(&self, repo_id: Uuid) -> Result<Vec<RepoTeamAccess>> {
		let rows = sqlx::query(
			r#"
			SELECT repo_id, team_id, role
			FROM repo_team_access
			WHERE repo_id = ?
			"#,
		)
		.bind(repo_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.iter().map(|r| self.row_to_team_access(r)).collect()
	}

	async fn get_user_role_via_teams(&self, user_id: Uuid, repo_id: Uuid) -> Result<Option<RepoRole>> {
		let rows = sqlx::query(
			r#"
			SELECT rta.role
			FROM repo_team_access rta
			INNER JOIN team_memberships tm ON rta.team_id = tm.team_id
			WHERE tm.user_id = ? AND rta.repo_id = ?
			"#,
		)
		.bind(user_id.to_string())
		.bind(repo_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		let mut highest_role: Option<RepoRole> = None;
		for row in &rows {
			let role_str: String = row.get("role");
			let role = role_str.parse::<RepoRole>().map_err(|_| {
				ScmError::Database(sqlx::Error::Decode(format!("invalid role: {}", role_str).into()))
			})?;

			highest_role = Some(match highest_role {
				None => role,
				Some(current) => {
					if role.has_permission_of(&current) {
						role
					} else {
						current
					}
				}
			});
		}

		Ok(highest_role)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	#[test]
	fn test_valid_names() {
		assert!(validate_repo_name("my-repo").is_ok());
		assert!(validate_repo_name("repo_name").is_ok());
		assert!(validate_repo_name("repo.v2").is_ok());
		assert!(validate_repo_name("MyRepo123").is_ok());
		assert!(validate_repo_name("a").is_ok());
		assert!(validate_repo_name("A123_test-name.v1").is_ok());
	}

	#[test]
	fn test_empty_name() {
		assert!(validate_repo_name("").is_err());
	}

	#[test]
	fn test_name_too_long() {
		let long_name = "a".repeat(101);
		assert!(validate_repo_name(&long_name).is_err());
		let max_name = "a".repeat(100);
		assert!(validate_repo_name(&max_name).is_ok());
	}

	#[test]
	fn test_dot_names() {
		assert!(validate_repo_name(".").is_err());
		assert!(validate_repo_name("..").is_err());
	}

	#[test]
	fn test_starts_with_dot_or_dash() {
		assert!(validate_repo_name(".hidden").is_err());
		assert!(validate_repo_name("-dash").is_err());
	}

	#[test]
	fn test_path_traversal() {
		assert!(validate_repo_name("../etc").is_err());
		assert!(validate_repo_name("foo/../bar").is_err());
		assert!(validate_repo_name("..passwd").is_err());
	}

	#[test]
	fn test_slashes() {
		assert!(validate_repo_name("repo/name").is_err());
		assert!(validate_repo_name("repo\\name").is_err());
	}

	#[test]
	fn test_shell_metacharacters() {
		assert!(validate_repo_name("repo;rm -rf").is_err());
		assert!(validate_repo_name("repo&cmd").is_err());
		assert!(validate_repo_name("repo|cat").is_err());
		assert!(validate_repo_name("repo`cmd`").is_err());
		assert!(validate_repo_name("repo$VAR").is_err());
		assert!(validate_repo_name("repo$(cmd)").is_err());
		assert!(validate_repo_name("repo{a,b}").is_err());
		assert!(validate_repo_name("repo<file").is_err());
		assert!(validate_repo_name("repo>file").is_err());
		assert!(validate_repo_name("repo!cmd").is_err());
	}

	#[test]
	fn test_spaces_and_special() {
		assert!(validate_repo_name("my repo").is_err());
		assert!(validate_repo_name("repo@name").is_err());
		assert!(validate_repo_name("repo#1").is_err());
	}

	proptest! {
		#[test]
		fn valid_names_pass(name in "[a-zA-Z]([a-zA-Z0-9_-]|[.][a-zA-Z0-9_-]){0,49}") {
			prop_assert!(validate_repo_name(&name).is_ok());
		}

		#[test]
		fn path_traversal_rejected(prefix in r"\.\./.*") {
			prop_assert!(validate_repo_name(&prefix).is_err());
		}

		#[test]
		fn shell_metacharacters_rejected(name in r"[a-zA-Z0-9]*[;&|`$(){}\[\]<>!][a-zA-Z0-9]*") {
			prop_assert!(validate_repo_name(&name).is_err());
		}

		#[test]
		fn slashes_rejected(name in r"[a-zA-Z0-9]*[/\\][a-zA-Z0-9]*") {
			prop_assert!(validate_repo_name(&name).is_err());
		}
	}

	mod team_access_tests {
		use super::*;

		#[test]
		fn test_admin_has_highest_permission() {
			assert!(RepoRole::Admin.has_permission_of(&RepoRole::Admin));
			assert!(RepoRole::Admin.has_permission_of(&RepoRole::Write));
			assert!(RepoRole::Admin.has_permission_of(&RepoRole::Read));
		}

		#[test]
		fn test_write_has_mid_permission() {
			assert!(!RepoRole::Write.has_permission_of(&RepoRole::Admin));
			assert!(RepoRole::Write.has_permission_of(&RepoRole::Write));
			assert!(RepoRole::Write.has_permission_of(&RepoRole::Read));
		}

		#[test]
		fn test_read_has_lowest_permission() {
			assert!(!RepoRole::Read.has_permission_of(&RepoRole::Admin));
			assert!(!RepoRole::Read.has_permission_of(&RepoRole::Write));
			assert!(RepoRole::Read.has_permission_of(&RepoRole::Read));
		}

		#[test]
		fn test_repo_team_access_struct() {
			let access = RepoTeamAccess {
				repo_id: Uuid::new_v4(),
				team_id: Uuid::new_v4(),
				role: RepoRole::Write,
			};
			assert_eq!(access.role, RepoRole::Write);
		}
	}
}
