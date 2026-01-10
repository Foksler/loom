// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;
use tracing::instrument;

use loom_flags_core::{
	Environment, EnvironmentId, Flag, FlagConfig, FlagId, FlagPrerequisite, KillSwitch,
	KillSwitchId, OrgId, SdkKey, SdkKeyId, SdkKeyType, Strategy, StrategyId, Variant,
};

use crate::error::{FlagsServerError, Result};

/// Repository trait for feature flags operations.
#[async_trait]
pub trait FlagsRepository: Send + Sync {
	// Environment operations
	async fn create_environment(&self, env: &Environment) -> Result<()>;
	async fn get_environment_by_id(&self, id: EnvironmentId) -> Result<Option<Environment>>;
	async fn get_environment_by_name(&self, org_id: OrgId, name: &str)
		-> Result<Option<Environment>>;
	async fn list_environments(&self, org_id: OrgId) -> Result<Vec<Environment>>;
	async fn update_environment(&self, env: &Environment) -> Result<()>;
	async fn delete_environment(&self, id: EnvironmentId) -> Result<bool>;

	// Flag operations
	async fn create_flag(&self, flag: &Flag) -> Result<()>;
	async fn get_flag_by_id(&self, id: FlagId) -> Result<Option<Flag>>;
	async fn get_flag_by_key(&self, org_id: Option<OrgId>, key: &str) -> Result<Option<Flag>>;
	async fn list_flags(&self, org_id: Option<OrgId>, include_archived: bool) -> Result<Vec<Flag>>;
	async fn update_flag(&self, flag: &Flag) -> Result<()>;
	async fn archive_flag(&self, id: FlagId) -> Result<bool>;
	async fn restore_flag(&self, id: FlagId) -> Result<bool>;

	// Flag config operations
	async fn create_flag_config(&self, config: &FlagConfig) -> Result<()>;
	async fn get_flag_config(
		&self,
		flag_id: FlagId,
		environment_id: EnvironmentId,
	) -> Result<Option<FlagConfig>>;
	async fn list_flag_configs(&self, flag_id: FlagId) -> Result<Vec<FlagConfig>>;
	async fn update_flag_config(&self, config: &FlagConfig) -> Result<()>;

	// Strategy operations
	async fn create_strategy(&self, strategy: &Strategy) -> Result<()>;
	async fn get_strategy_by_id(&self, id: StrategyId) -> Result<Option<Strategy>>;
	async fn list_strategies(&self, org_id: Option<OrgId>) -> Result<Vec<Strategy>>;
	async fn update_strategy(&self, strategy: &Strategy) -> Result<()>;
	async fn delete_strategy(&self, id: StrategyId) -> Result<bool>;

	// Kill switch operations
	async fn create_kill_switch(&self, kill_switch: &KillSwitch) -> Result<()>;
	async fn get_kill_switch_by_id(&self, id: KillSwitchId) -> Result<Option<KillSwitch>>;
	async fn get_kill_switch_by_key(
		&self,
		org_id: Option<OrgId>,
		key: &str,
	) -> Result<Option<KillSwitch>>;
	async fn list_kill_switches(&self, org_id: Option<OrgId>) -> Result<Vec<KillSwitch>>;
	async fn list_active_kill_switches(&self, org_id: Option<OrgId>) -> Result<Vec<KillSwitch>>;
	async fn update_kill_switch(&self, kill_switch: &KillSwitch) -> Result<()>;
	async fn delete_kill_switch(&self, id: KillSwitchId) -> Result<bool>;

	// SDK key operations
	async fn create_sdk_key(&self, key: &SdkKey) -> Result<()>;
	async fn get_sdk_key_by_id(&self, id: SdkKeyId) -> Result<Option<SdkKey>>;
	async fn get_sdk_key_by_hash(&self, key_hash: &str) -> Result<Option<SdkKey>>;
	async fn list_sdk_keys(&self, environment_id: EnvironmentId) -> Result<Vec<SdkKey>>;
	async fn revoke_sdk_key(&self, id: SdkKeyId) -> Result<bool>;
	async fn update_sdk_key_last_used(&self, id: SdkKeyId) -> Result<()>;
}

/// SQLite implementation of the flags repository.
#[derive(Clone)]
pub struct SqliteFlagsRepository {
	pool: SqlitePool,
}

impl SqliteFlagsRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

#[async_trait]
impl FlagsRepository for SqliteFlagsRepository {
	// Environment operations

	#[instrument(skip(self, env), fields(env_id = %env.id, org_id = %env.org_id))]
	async fn create_environment(&self, env: &Environment) -> Result<()> {
		sqlx::query(
			r#"
			INSERT INTO flag_environments (id, org_id, name, color, created_at)
			VALUES (?, ?, ?, ?, ?)
			"#,
		)
		.bind(env.id.0.to_string())
		.bind(env.org_id.0.to_string())
		.bind(&env.name)
		.bind(&env.color)
		.bind(env.created_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(env_id = %id))]
	async fn get_environment_by_id(&self, id: EnvironmentId) -> Result<Option<Environment>> {
		let row = sqlx::query_as::<_, EnvironmentRow>(
			r#"
			SELECT id, org_id, name, color, created_at
			FROM flag_environments
			WHERE id = ?
			"#,
		)
		.bind(id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(org_id = %org_id, name = %name))]
	async fn get_environment_by_name(
		&self,
		org_id: OrgId,
		name: &str,
	) -> Result<Option<Environment>> {
		let row = sqlx::query_as::<_, EnvironmentRow>(
			r#"
			SELECT id, org_id, name, color, created_at
			FROM flag_environments
			WHERE org_id = ? AND name = ?
			"#,
		)
		.bind(org_id.0.to_string())
		.bind(name)
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(org_id = %org_id))]
	async fn list_environments(&self, org_id: OrgId) -> Result<Vec<Environment>> {
		let rows = sqlx::query_as::<_, EnvironmentRow>(
			r#"
			SELECT id, org_id, name, color, created_at
			FROM flag_environments
			WHERE org_id = ?
			ORDER BY created_at ASC
			"#,
		)
		.bind(org_id.0.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self, env), fields(env_id = %env.id))]
	async fn update_environment(&self, env: &Environment) -> Result<()> {
		sqlx::query(
			r#"
			UPDATE flag_environments
			SET name = ?, color = ?
			WHERE id = ?
			"#,
		)
		.bind(&env.name)
		.bind(&env.color)
		.bind(env.id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(env_id = %id))]
	async fn delete_environment(&self, id: EnvironmentId) -> Result<bool> {
		let result = sqlx::query(
			r#"
			DELETE FROM flag_environments WHERE id = ?
			"#,
		)
		.bind(id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(result.rows_affected() > 0)
	}

	// Flag operations

	#[instrument(skip(self, flag), fields(flag_id = %flag.id, flag_key = %flag.key))]
	async fn create_flag(&self, flag: &Flag) -> Result<()> {
		let variants_json = serde_json::to_string(&flag.variants)?;
		let tags_json = serde_json::to_string(&flag.tags)?;

		sqlx::query(
			r#"
			INSERT INTO flags (id, org_id, key, name, description, tags, maintainer_user_id,
							   variants, default_variant, created_at, updated_at, archived_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(flag.id.0.to_string())
		.bind(flag.org_id.map(|id| id.0.to_string()))
		.bind(&flag.key)
		.bind(&flag.name)
		.bind(&flag.description)
		.bind(tags_json)
		.bind(flag.maintainer_user_id.map(|id| id.0.to_string()))
		.bind(variants_json)
		.bind(&flag.default_variant)
		.bind(flag.created_at.to_rfc3339())
		.bind(flag.updated_at.to_rfc3339())
		.bind(flag.archived_at.map(|dt| dt.to_rfc3339()))
		.execute(&self.pool)
		.await?;

		// Insert prerequisites
		for prereq in &flag.prerequisites {
			sqlx::query(
				r#"
				INSERT INTO flag_prerequisites (id, flag_id, prerequisite_flag_key, required_variant, created_at)
				VALUES (?, ?, ?, ?, ?)
				"#,
			)
			.bind(uuid::Uuid::new_v4().to_string())
			.bind(flag.id.0.to_string())
			.bind(&prereq.flag_key)
			.bind(&prereq.required_variant)
			.bind(Utc::now().to_rfc3339())
			.execute(&self.pool)
			.await?;
		}

		Ok(())
	}

	#[instrument(skip(self), fields(flag_id = %id))]
	async fn get_flag_by_id(&self, id: FlagId) -> Result<Option<Flag>> {
		let row = sqlx::query_as::<_, FlagRow>(
			r#"
			SELECT id, org_id, key, name, description, tags, maintainer_user_id,
				   variants, default_variant, created_at, updated_at, archived_at
			FROM flags
			WHERE id = ?
			"#,
		)
		.bind(id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		match row {
			Some(row) => {
				let prerequisites = self.get_flag_prerequisites(id).await?;
				Ok(Some(row.into_flag(prerequisites)?))
			}
			None => Ok(None),
		}
	}

	#[instrument(skip(self), fields(org_id = ?org_id, flag_key = %key))]
	async fn get_flag_by_key(&self, org_id: Option<OrgId>, key: &str) -> Result<Option<Flag>> {
		let row = match org_id {
			Some(org) => {
				sqlx::query_as::<_, FlagRow>(
					r#"
					SELECT id, org_id, key, name, description, tags, maintainer_user_id,
						   variants, default_variant, created_at, updated_at, archived_at
					FROM flags
					WHERE org_id = ? AND key = ?
					"#,
				)
				.bind(org.0.to_string())
				.bind(key)
				.fetch_optional(&self.pool)
				.await?
			}
			None => {
				sqlx::query_as::<_, FlagRow>(
					r#"
					SELECT id, org_id, key, name, description, tags, maintainer_user_id,
						   variants, default_variant, created_at, updated_at, archived_at
					FROM flags
					WHERE org_id IS NULL AND key = ?
					"#,
				)
				.bind(key)
				.fetch_optional(&self.pool)
				.await?
			}
		};

		match row {
			Some(row) => {
				let flag_id: FlagId = row.id.parse().map_err(|_| {
					FlagsServerError::Internal("Invalid flag ID in database".to_string())
				})?;
				let prerequisites = self.get_flag_prerequisites(flag_id).await?;
				Ok(Some(row.into_flag(prerequisites)?))
			}
			None => Ok(None),
		}
	}

	#[instrument(skip(self), fields(org_id = ?org_id))]
	async fn list_flags(&self, org_id: Option<OrgId>, include_archived: bool) -> Result<Vec<Flag>> {
		let rows = match org_id {
			Some(org) => {
				if include_archived {
					sqlx::query_as::<_, FlagRow>(
						r#"
						SELECT id, org_id, key, name, description, tags, maintainer_user_id,
							   variants, default_variant, created_at, updated_at, archived_at
						FROM flags
						WHERE org_id = ?
						ORDER BY key ASC
						"#,
					)
					.bind(org.0.to_string())
					.fetch_all(&self.pool)
					.await?
				} else {
					sqlx::query_as::<_, FlagRow>(
						r#"
						SELECT id, org_id, key, name, description, tags, maintainer_user_id,
							   variants, default_variant, created_at, updated_at, archived_at
						FROM flags
						WHERE org_id = ? AND archived_at IS NULL
						ORDER BY key ASC
						"#,
					)
					.bind(org.0.to_string())
					.fetch_all(&self.pool)
					.await?
				}
			}
			None => {
				if include_archived {
					sqlx::query_as::<_, FlagRow>(
						r#"
						SELECT id, org_id, key, name, description, tags, maintainer_user_id,
							   variants, default_variant, created_at, updated_at, archived_at
						FROM flags
						WHERE org_id IS NULL
						ORDER BY key ASC
						"#,
					)
					.fetch_all(&self.pool)
					.await?
				} else {
					sqlx::query_as::<_, FlagRow>(
						r#"
						SELECT id, org_id, key, name, description, tags, maintainer_user_id,
							   variants, default_variant, created_at, updated_at, archived_at
						FROM flags
						WHERE org_id IS NULL AND archived_at IS NULL
						ORDER BY key ASC
						"#,
					)
					.fetch_all(&self.pool)
					.await?
				}
			}
		};

		let mut flags = Vec::with_capacity(rows.len());
		for row in rows {
			let flag_id: FlagId = row.id.parse().map_err(|_| {
				FlagsServerError::Internal("Invalid flag ID in database".to_string())
			})?;
			let prerequisites = self.get_flag_prerequisites(flag_id).await?;
			flags.push(row.into_flag(prerequisites)?);
		}

		Ok(flags)
	}

	#[instrument(skip(self, flag), fields(flag_id = %flag.id))]
	async fn update_flag(&self, flag: &Flag) -> Result<()> {
		let variants_json = serde_json::to_string(&flag.variants)?;
		let tags_json = serde_json::to_string(&flag.tags)?;

		sqlx::query(
			r#"
			UPDATE flags
			SET name = ?, description = ?, tags = ?, maintainer_user_id = ?,
				variants = ?, default_variant = ?, updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&flag.name)
		.bind(&flag.description)
		.bind(tags_json)
		.bind(flag.maintainer_user_id.map(|id| id.0.to_string()))
		.bind(variants_json)
		.bind(&flag.default_variant)
		.bind(Utc::now().to_rfc3339())
		.bind(flag.id.0.to_string())
		.execute(&self.pool)
		.await?;

		// Update prerequisites - delete and re-insert
		sqlx::query("DELETE FROM flag_prerequisites WHERE flag_id = ?")
			.bind(flag.id.0.to_string())
			.execute(&self.pool)
			.await?;

		for prereq in &flag.prerequisites {
			sqlx::query(
				r#"
				INSERT INTO flag_prerequisites (id, flag_id, prerequisite_flag_key, required_variant, created_at)
				VALUES (?, ?, ?, ?, ?)
				"#,
			)
			.bind(uuid::Uuid::new_v4().to_string())
			.bind(flag.id.0.to_string())
			.bind(&prereq.flag_key)
			.bind(&prereq.required_variant)
			.bind(Utc::now().to_rfc3339())
			.execute(&self.pool)
			.await?;
		}

		Ok(())
	}

	#[instrument(skip(self), fields(flag_id = %id))]
	async fn archive_flag(&self, id: FlagId) -> Result<bool> {
		let result = sqlx::query(
			r#"
			UPDATE flags
			SET archived_at = ?, updated_at = ?
			WHERE id = ? AND archived_at IS NULL
			"#,
		)
		.bind(Utc::now().to_rfc3339())
		.bind(Utc::now().to_rfc3339())
		.bind(id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(result.rows_affected() > 0)
	}

	#[instrument(skip(self), fields(flag_id = %id))]
	async fn restore_flag(&self, id: FlagId) -> Result<bool> {
		let result = sqlx::query(
			r#"
			UPDATE flags
			SET archived_at = NULL, updated_at = ?
			WHERE id = ? AND archived_at IS NOT NULL
			"#,
		)
		.bind(Utc::now().to_rfc3339())
		.bind(id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(result.rows_affected() > 0)
	}

	// Flag config operations

	#[instrument(skip(self, config), fields(config_id = %config.id, flag_id = %config.flag_id))]
	async fn create_flag_config(&self, config: &FlagConfig) -> Result<()> {
		sqlx::query(
			r#"
			INSERT INTO flag_configs (id, flag_id, environment_id, enabled, strategy_id, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(config.id.0.to_string())
		.bind(config.flag_id.0.to_string())
		.bind(config.environment_id.0.to_string())
		.bind(config.enabled)
		.bind(config.strategy_id.map(|id| id.0.to_string()))
		.bind(config.created_at.to_rfc3339())
		.bind(config.updated_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(flag_id = %flag_id, env_id = %environment_id))]
	async fn get_flag_config(
		&self,
		flag_id: FlagId,
		environment_id: EnvironmentId,
	) -> Result<Option<FlagConfig>> {
		let row = sqlx::query_as::<_, FlagConfigRow>(
			r#"
			SELECT id, flag_id, environment_id, enabled, strategy_id, created_at, updated_at
			FROM flag_configs
			WHERE flag_id = ? AND environment_id = ?
			"#,
		)
		.bind(flag_id.0.to_string())
		.bind(environment_id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(flag_id = %flag_id))]
	async fn list_flag_configs(&self, flag_id: FlagId) -> Result<Vec<FlagConfig>> {
		let rows = sqlx::query_as::<_, FlagConfigRow>(
			r#"
			SELECT id, flag_id, environment_id, enabled, strategy_id, created_at, updated_at
			FROM flag_configs
			WHERE flag_id = ?
			"#,
		)
		.bind(flag_id.0.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self, config), fields(config_id = %config.id))]
	async fn update_flag_config(&self, config: &FlagConfig) -> Result<()> {
		sqlx::query(
			r#"
			UPDATE flag_configs
			SET enabled = ?, strategy_id = ?, updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(config.enabled)
		.bind(config.strategy_id.map(|id| id.0.to_string()))
		.bind(Utc::now().to_rfc3339())
		.bind(config.id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	// Strategy operations

	#[instrument(skip(self, strategy), fields(strategy_id = %strategy.id))]
	async fn create_strategy(&self, strategy: &Strategy) -> Result<()> {
		let conditions_json = serde_json::to_string(&strategy.conditions)?;
		let schedule_json = strategy
			.schedule
			.as_ref()
			.map(serde_json::to_string)
			.transpose()?;
		let percentage_key_json = serde_json::to_string(&strategy.percentage_key)?;

		sqlx::query(
			r#"
			INSERT INTO flag_strategies (id, org_id, name, description, conditions, percentage,
										 percentage_key, schedule, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(strategy.id.0.to_string())
		.bind(strategy.org_id.map(|id| id.0.to_string()))
		.bind(&strategy.name)
		.bind(&strategy.description)
		.bind(conditions_json)
		.bind(strategy.percentage.map(|p| p as i32))
		.bind(percentage_key_json)
		.bind(schedule_json)
		.bind(strategy.created_at.to_rfc3339())
		.bind(strategy.updated_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(strategy_id = %id))]
	async fn get_strategy_by_id(&self, id: StrategyId) -> Result<Option<Strategy>> {
		let row = sqlx::query_as::<_, StrategyRow>(
			r#"
			SELECT id, org_id, name, description, conditions, percentage, percentage_key,
				   schedule, created_at, updated_at
			FROM flag_strategies
			WHERE id = ?
			"#,
		)
		.bind(id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(org_id = ?org_id))]
	async fn list_strategies(&self, org_id: Option<OrgId>) -> Result<Vec<Strategy>> {
		let rows = match org_id {
			Some(org) => {
				sqlx::query_as::<_, StrategyRow>(
					r#"
					SELECT id, org_id, name, description, conditions, percentage, percentage_key,
						   schedule, created_at, updated_at
					FROM flag_strategies
					WHERE org_id = ?
					ORDER BY name ASC
					"#,
				)
				.bind(org.0.to_string())
				.fetch_all(&self.pool)
				.await?
			}
			None => {
				sqlx::query_as::<_, StrategyRow>(
					r#"
					SELECT id, org_id, name, description, conditions, percentage, percentage_key,
						   schedule, created_at, updated_at
					FROM flag_strategies
					WHERE org_id IS NULL
					ORDER BY name ASC
					"#,
				)
				.fetch_all(&self.pool)
				.await?
			}
		};

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self, strategy), fields(strategy_id = %strategy.id))]
	async fn update_strategy(&self, strategy: &Strategy) -> Result<()> {
		let conditions_json = serde_json::to_string(&strategy.conditions)?;
		let schedule_json = strategy
			.schedule
			.as_ref()
			.map(serde_json::to_string)
			.transpose()?;
		let percentage_key_json = serde_json::to_string(&strategy.percentage_key)?;

		sqlx::query(
			r#"
			UPDATE flag_strategies
			SET name = ?, description = ?, conditions = ?, percentage = ?,
				percentage_key = ?, schedule = ?, updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&strategy.name)
		.bind(&strategy.description)
		.bind(conditions_json)
		.bind(strategy.percentage.map(|p| p as i32))
		.bind(percentage_key_json)
		.bind(schedule_json)
		.bind(Utc::now().to_rfc3339())
		.bind(strategy.id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(strategy_id = %id))]
	async fn delete_strategy(&self, id: StrategyId) -> Result<bool> {
		let result = sqlx::query("DELETE FROM flag_strategies WHERE id = ?")
			.bind(id.0.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected() > 0)
	}

	// Kill switch operations

	#[instrument(skip(self, kill_switch), fields(kill_switch_id = %kill_switch.id, key = %kill_switch.key))]
	async fn create_kill_switch(&self, kill_switch: &KillSwitch) -> Result<()> {
		let linked_flags_json = serde_json::to_string(&kill_switch.linked_flag_keys)?;

		sqlx::query(
			r#"
			INSERT INTO kill_switches (id, org_id, key, name, description, linked_flag_keys,
									   is_active, activated_at, activated_by, activation_reason,
									   created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(kill_switch.id.0.to_string())
		.bind(kill_switch.org_id.map(|id| id.0.to_string()))
		.bind(&kill_switch.key)
		.bind(&kill_switch.name)
		.bind(&kill_switch.description)
		.bind(linked_flags_json)
		.bind(kill_switch.is_active)
		.bind(kill_switch.activated_at.map(|dt| dt.to_rfc3339()))
		.bind(kill_switch.activated_by.map(|id| id.0.to_string()))
		.bind(&kill_switch.activation_reason)
		.bind(kill_switch.created_at.to_rfc3339())
		.bind(kill_switch.updated_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(kill_switch_id = %id))]
	async fn get_kill_switch_by_id(&self, id: KillSwitchId) -> Result<Option<KillSwitch>> {
		let row = sqlx::query_as::<_, KillSwitchRow>(
			r#"
			SELECT id, org_id, key, name, description, linked_flag_keys, is_active,
				   activated_at, activated_by, activation_reason, created_at, updated_at
			FROM kill_switches
			WHERE id = ?
			"#,
		)
		.bind(id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(org_id = ?org_id, key = %key))]
	async fn get_kill_switch_by_key(
		&self,
		org_id: Option<OrgId>,
		key: &str,
	) -> Result<Option<KillSwitch>> {
		let row = match org_id {
			Some(org) => {
				sqlx::query_as::<_, KillSwitchRow>(
					r#"
					SELECT id, org_id, key, name, description, linked_flag_keys, is_active,
						   activated_at, activated_by, activation_reason, created_at, updated_at
					FROM kill_switches
					WHERE org_id = ? AND key = ?
					"#,
				)
				.bind(org.0.to_string())
				.bind(key)
				.fetch_optional(&self.pool)
				.await?
			}
			None => {
				sqlx::query_as::<_, KillSwitchRow>(
					r#"
					SELECT id, org_id, key, name, description, linked_flag_keys, is_active,
						   activated_at, activated_by, activation_reason, created_at, updated_at
					FROM kill_switches
					WHERE org_id IS NULL AND key = ?
					"#,
				)
				.bind(key)
				.fetch_optional(&self.pool)
				.await?
			}
		};

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(org_id = ?org_id))]
	async fn list_kill_switches(&self, org_id: Option<OrgId>) -> Result<Vec<KillSwitch>> {
		let rows = match org_id {
			Some(org) => {
				sqlx::query_as::<_, KillSwitchRow>(
					r#"
					SELECT id, org_id, key, name, description, linked_flag_keys, is_active,
						   activated_at, activated_by, activation_reason, created_at, updated_at
					FROM kill_switches
					WHERE org_id = ?
					ORDER BY key ASC
					"#,
				)
				.bind(org.0.to_string())
				.fetch_all(&self.pool)
				.await?
			}
			None => {
				sqlx::query_as::<_, KillSwitchRow>(
					r#"
					SELECT id, org_id, key, name, description, linked_flag_keys, is_active,
						   activated_at, activated_by, activation_reason, created_at, updated_at
					FROM kill_switches
					WHERE org_id IS NULL
					ORDER BY key ASC
					"#,
				)
				.fetch_all(&self.pool)
				.await?
			}
		};

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(org_id = ?org_id))]
	async fn list_active_kill_switches(&self, org_id: Option<OrgId>) -> Result<Vec<KillSwitch>> {
		let rows = match org_id {
			Some(org) => {
				sqlx::query_as::<_, KillSwitchRow>(
					r#"
					SELECT id, org_id, key, name, description, linked_flag_keys, is_active,
						   activated_at, activated_by, activation_reason, created_at, updated_at
					FROM kill_switches
					WHERE org_id = ? AND is_active = 1
					ORDER BY key ASC
					"#,
				)
				.bind(org.0.to_string())
				.fetch_all(&self.pool)
				.await?
			}
			None => {
				sqlx::query_as::<_, KillSwitchRow>(
					r#"
					SELECT id, org_id, key, name, description, linked_flag_keys, is_active,
						   activated_at, activated_by, activation_reason, created_at, updated_at
					FROM kill_switches
					WHERE org_id IS NULL AND is_active = 1
					ORDER BY key ASC
					"#,
				)
				.fetch_all(&self.pool)
				.await?
			}
		};

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self, kill_switch), fields(kill_switch_id = %kill_switch.id))]
	async fn update_kill_switch(&self, kill_switch: &KillSwitch) -> Result<()> {
		let linked_flags_json = serde_json::to_string(&kill_switch.linked_flag_keys)?;

		sqlx::query(
			r#"
			UPDATE kill_switches
			SET name = ?, description = ?, linked_flag_keys = ?, is_active = ?,
				activated_at = ?, activated_by = ?, activation_reason = ?, updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&kill_switch.name)
		.bind(&kill_switch.description)
		.bind(linked_flags_json)
		.bind(kill_switch.is_active)
		.bind(kill_switch.activated_at.map(|dt| dt.to_rfc3339()))
		.bind(kill_switch.activated_by.map(|id| id.0.to_string()))
		.bind(&kill_switch.activation_reason)
		.bind(Utc::now().to_rfc3339())
		.bind(kill_switch.id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(kill_switch_id = %id))]
	async fn delete_kill_switch(&self, id: KillSwitchId) -> Result<bool> {
		let result = sqlx::query("DELETE FROM kill_switches WHERE id = ?")
			.bind(id.0.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected() > 0)
	}

	// SDK key operations

	#[instrument(skip(self, key), fields(sdk_key_id = %key.id))]
	async fn create_sdk_key(&self, key: &SdkKey) -> Result<()> {
		sqlx::query(
			r#"
			INSERT INTO sdk_keys (id, environment_id, key_type, name, key_hash, created_by,
								  created_at, last_used_at, revoked_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(key.id.0.to_string())
		.bind(key.environment_id.0.to_string())
		.bind(key.key_type.as_str())
		.bind(&key.name)
		.bind(&key.key_hash)
		.bind(key.created_by.0.to_string())
		.bind(key.created_at.to_rfc3339())
		.bind(key.last_used_at.map(|dt| dt.to_rfc3339()))
		.bind(key.revoked_at.map(|dt| dt.to_rfc3339()))
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(sdk_key_id = %id))]
	async fn get_sdk_key_by_id(&self, id: SdkKeyId) -> Result<Option<SdkKey>> {
		let row = sqlx::query_as::<_, SdkKeyRow>(
			r#"
			SELECT id, environment_id, key_type, name, key_hash, created_by,
				   created_at, last_used_at, revoked_at
			FROM sdk_keys
			WHERE id = ?
			"#,
		)
		.bind(id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self, key_hash))]
	async fn get_sdk_key_by_hash(&self, key_hash: &str) -> Result<Option<SdkKey>> {
		let row = sqlx::query_as::<_, SdkKeyRow>(
			r#"
			SELECT id, environment_id, key_type, name, key_hash, created_by,
				   created_at, last_used_at, revoked_at
			FROM sdk_keys
			WHERE key_hash = ?
			"#,
		)
		.bind(key_hash)
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(env_id = %environment_id))]
	async fn list_sdk_keys(&self, environment_id: EnvironmentId) -> Result<Vec<SdkKey>> {
		let rows = sqlx::query_as::<_, SdkKeyRow>(
			r#"
			SELECT id, environment_id, key_type, name, key_hash, created_by,
				   created_at, last_used_at, revoked_at
			FROM sdk_keys
			WHERE environment_id = ?
			ORDER BY created_at DESC
			"#,
		)
		.bind(environment_id.0.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(sdk_key_id = %id))]
	async fn revoke_sdk_key(&self, id: SdkKeyId) -> Result<bool> {
		let result = sqlx::query(
			r#"
			UPDATE sdk_keys
			SET revoked_at = ?
			WHERE id = ? AND revoked_at IS NULL
			"#,
		)
		.bind(Utc::now().to_rfc3339())
		.bind(id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(result.rows_affected() > 0)
	}

	#[instrument(skip(self), fields(sdk_key_id = %id))]
	async fn update_sdk_key_last_used(&self, id: SdkKeyId) -> Result<()> {
		sqlx::query(
			r#"
			UPDATE sdk_keys
			SET last_used_at = ?
			WHERE id = ?
			"#,
		)
		.bind(Utc::now().to_rfc3339())
		.bind(id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}
}

impl SqliteFlagsRepository {
	async fn get_flag_prerequisites(&self, flag_id: FlagId) -> Result<Vec<FlagPrerequisite>> {
		let rows = sqlx::query_as::<_, PrerequisiteRow>(
			r#"
			SELECT prerequisite_flag_key, required_variant
			FROM flag_prerequisites
			WHERE flag_id = ?
			"#,
		)
		.bind(flag_id.0.to_string())
		.fetch_all(&self.pool)
		.await?;

		Ok(rows
			.into_iter()
			.map(|r| FlagPrerequisite {
				flag_key: r.prerequisite_flag_key,
				required_variant: r.required_variant,
			})
			.collect())
	}
}

// Database row types for sqlx

#[derive(sqlx::FromRow)]
struct EnvironmentRow {
	id: String,
	org_id: String,
	name: String,
	color: Option<String>,
	created_at: String,
}

impl TryFrom<EnvironmentRow> for Environment {
	type Error = FlagsServerError;

	fn try_from(row: EnvironmentRow) -> Result<Self> {
		Ok(Environment {
			id: row
				.id
				.parse()
				.map_err(|_| FlagsServerError::Internal("Invalid environment ID".to_string()))?,
			org_id: row
				.org_id
				.parse()
				.map_err(|_| FlagsServerError::Internal("Invalid org ID".to_string()))?,
			name: row.name,
			color: row.color,
			created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
				.map_err(|_| FlagsServerError::Internal("Invalid created_at".to_string()))?
				.with_timezone(&chrono::Utc),
		})
	}
}

#[derive(sqlx::FromRow)]
struct FlagRow {
	id: String,
	org_id: Option<String>,
	key: String,
	name: String,
	description: Option<String>,
	tags: String,
	maintainer_user_id: Option<String>,
	variants: String,
	default_variant: String,
	created_at: String,
	updated_at: String,
	archived_at: Option<String>,
}

impl FlagRow {
	fn into_flag(self, prerequisites: Vec<FlagPrerequisite>) -> Result<Flag> {
		let tags: Vec<String> = serde_json::from_str(&self.tags)?;
		let variants: Vec<Variant> = serde_json::from_str(&self.variants)?;

		Ok(Flag {
			id: self
				.id
				.parse()
				.map_err(|_| FlagsServerError::Internal("Invalid flag ID".to_string()))?,
			org_id: self
				.org_id
				.map(|s| {
					s.parse()
						.map_err(|_| FlagsServerError::Internal("Invalid org ID".to_string()))
				})
				.transpose()?,
			key: self.key,
			name: self.name,
			description: self.description,
			tags,
			maintainer_user_id: self
				.maintainer_user_id
				.map(|s| {
					s.parse()
						.map_err(|_| FlagsServerError::Internal("Invalid user ID".to_string()))
				})
				.transpose()?,
			variants,
			default_variant: self.default_variant,
			prerequisites,
			created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
				.map_err(|_| FlagsServerError::Internal("Invalid created_at".to_string()))?
				.with_timezone(&chrono::Utc),
			updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
				.map_err(|_| FlagsServerError::Internal("Invalid updated_at".to_string()))?
				.with_timezone(&chrono::Utc),
			archived_at: self
				.archived_at
				.map(|s| {
					chrono::DateTime::parse_from_rfc3339(&s)
						.map_err(|_| {
							FlagsServerError::Internal("Invalid archived_at".to_string())
						})
						.map(|dt| dt.with_timezone(&chrono::Utc))
				})
				.transpose()?,
		})
	}
}

#[derive(sqlx::FromRow)]
struct PrerequisiteRow {
	prerequisite_flag_key: String,
	required_variant: String,
}

#[derive(sqlx::FromRow)]
struct FlagConfigRow {
	id: String,
	flag_id: String,
	environment_id: String,
	enabled: bool,
	strategy_id: Option<String>,
	created_at: String,
	updated_at: String,
}

impl TryFrom<FlagConfigRow> for FlagConfig {
	type Error = FlagsServerError;

	fn try_from(row: FlagConfigRow) -> Result<Self> {
		Ok(FlagConfig {
			id: row
				.id
				.parse()
				.map_err(|_| FlagsServerError::Internal("Invalid config ID".to_string()))?,
			flag_id: row
				.flag_id
				.parse()
				.map_err(|_| FlagsServerError::Internal("Invalid flag ID".to_string()))?,
			environment_id: row.environment_id.parse().map_err(|_| {
				FlagsServerError::Internal("Invalid environment ID".to_string())
			})?,
			enabled: row.enabled,
			strategy_id: row
				.strategy_id
				.map(|s| {
					s.parse()
						.map_err(|_| FlagsServerError::Internal("Invalid strategy ID".to_string()))
				})
				.transpose()?,
			created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
				.map_err(|_| FlagsServerError::Internal("Invalid created_at".to_string()))?
				.with_timezone(&chrono::Utc),
			updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
				.map_err(|_| FlagsServerError::Internal("Invalid updated_at".to_string()))?
				.with_timezone(&chrono::Utc),
		})
	}
}

#[derive(sqlx::FromRow)]
struct StrategyRow {
	id: String,
	org_id: Option<String>,
	name: String,
	description: Option<String>,
	conditions: String,
	percentage: Option<i32>,
	percentage_key: String,
	schedule: Option<String>,
	created_at: String,
	updated_at: String,
}

impl TryFrom<StrategyRow> for Strategy {
	type Error = FlagsServerError;

	fn try_from(row: StrategyRow) -> Result<Self> {
		use loom_flags_core::{Condition, PercentageKey, Schedule};

		let conditions: Vec<Condition> = serde_json::from_str(&row.conditions)?;
		let percentage_key: PercentageKey = serde_json::from_str(&row.percentage_key)?;
		let schedule: Option<Schedule> = row
			.schedule
			.map(|s| serde_json::from_str(&s))
			.transpose()?;

		Ok(Strategy {
			id: row
				.id
				.parse()
				.map_err(|_| FlagsServerError::Internal("Invalid strategy ID".to_string()))?,
			org_id: row
				.org_id
				.map(|s| {
					s.parse()
						.map_err(|_| FlagsServerError::Internal("Invalid org ID".to_string()))
				})
				.transpose()?,
			name: row.name,
			description: row.description,
			conditions,
			percentage: row.percentage.map(|p| p as u32),
			percentage_key,
			schedule,
			created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
				.map_err(|_| FlagsServerError::Internal("Invalid created_at".to_string()))?
				.with_timezone(&chrono::Utc),
			updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
				.map_err(|_| FlagsServerError::Internal("Invalid updated_at".to_string()))?
				.with_timezone(&chrono::Utc),
		})
	}
}

#[derive(sqlx::FromRow)]
struct KillSwitchRow {
	id: String,
	org_id: Option<String>,
	key: String,
	name: String,
	description: Option<String>,
	linked_flag_keys: String,
	is_active: bool,
	activated_at: Option<String>,
	activated_by: Option<String>,
	activation_reason: Option<String>,
	created_at: String,
	updated_at: String,
}

impl TryFrom<KillSwitchRow> for KillSwitch {
	type Error = FlagsServerError;

	fn try_from(row: KillSwitchRow) -> Result<Self> {
		let linked_flag_keys: Vec<String> = serde_json::from_str(&row.linked_flag_keys)?;

		Ok(KillSwitch {
			id: row.id.parse().map_err(|_| {
				FlagsServerError::Internal("Invalid kill switch ID".to_string())
			})?,
			org_id: row
				.org_id
				.map(|s| {
					s.parse()
						.map_err(|_| FlagsServerError::Internal("Invalid org ID".to_string()))
				})
				.transpose()?,
			key: row.key,
			name: row.name,
			description: row.description,
			linked_flag_keys,
			is_active: row.is_active,
			activated_at: row
				.activated_at
				.map(|s| {
					chrono::DateTime::parse_from_rfc3339(&s)
						.map_err(|_| {
							FlagsServerError::Internal("Invalid activated_at".to_string())
						})
						.map(|dt| dt.with_timezone(&chrono::Utc))
				})
				.transpose()?,
			activated_by: row
				.activated_by
				.map(|s| {
					s.parse()
						.map_err(|_| FlagsServerError::Internal("Invalid user ID".to_string()))
				})
				.transpose()?,
			activation_reason: row.activation_reason,
			created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
				.map_err(|_| FlagsServerError::Internal("Invalid created_at".to_string()))?
				.with_timezone(&chrono::Utc),
			updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
				.map_err(|_| FlagsServerError::Internal("Invalid updated_at".to_string()))?
				.with_timezone(&chrono::Utc),
		})
	}
}

#[derive(sqlx::FromRow)]
struct SdkKeyRow {
	id: String,
	environment_id: String,
	key_type: String,
	name: String,
	key_hash: String,
	created_by: String,
	created_at: String,
	last_used_at: Option<String>,
	revoked_at: Option<String>,
}

impl TryFrom<SdkKeyRow> for SdkKey {
	type Error = FlagsServerError;

	fn try_from(row: SdkKeyRow) -> Result<Self> {
		let key_type: SdkKeyType = row
			.key_type
			.parse()
			.map_err(|_| FlagsServerError::Internal("Invalid key type".to_string()))?;

		Ok(SdkKey {
			id: row
				.id
				.parse()
				.map_err(|_| FlagsServerError::Internal("Invalid SDK key ID".to_string()))?,
			environment_id: row.environment_id.parse().map_err(|_| {
				FlagsServerError::Internal("Invalid environment ID".to_string())
			})?,
			key_type,
			name: row.name,
			key_hash: row.key_hash,
			created_by: row
				.created_by
				.parse()
				.map_err(|_| FlagsServerError::Internal("Invalid user ID".to_string()))?,
			created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
				.map_err(|_| FlagsServerError::Internal("Invalid created_at".to_string()))?
				.with_timezone(&chrono::Utc),
			last_used_at: row
				.last_used_at
				.map(|s| {
					chrono::DateTime::parse_from_rfc3339(&s)
						.map_err(|_| {
							FlagsServerError::Internal("Invalid last_used_at".to_string())
						})
						.map(|dt| dt.with_timezone(&chrono::Utc))
				})
				.transpose()?,
			revoked_at: row
				.revoked_at
				.map(|s| {
					chrono::DateTime::parse_from_rfc3339(&s)
						.map_err(|_| FlagsServerError::Internal("Invalid revoked_at".to_string()))
						.map(|dt| dt.with_timezone(&chrono::Utc))
				})
				.transpose()?,
		})
	}
}
