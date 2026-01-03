// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	Json,
};
use chrono::{DateTime, Utc};
use loom_scim::patch::PatchRequest;
use loom_scim::{ListResponse, ScimUser};
use loom_server_auth::{OrgId, UserId};
use serde::Deserialize;
use sqlx::{Row, SqlitePool};
use tracing::info;
use uuid::Uuid;

fn parse_user_id(s: &str) -> Result<UserId, ScimApiError> {
	let uuid =
		Uuid::parse_str(s).map_err(|e| ScimApiError::Internal(format!("Invalid user ID: {}", e)))?;
	Ok(UserId::new(uuid))
}

use crate::error::ScimApiError;
use crate::mapping::{scim_user_to_display_name, scim_user_to_email, LoomUser};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersQuery {
	#[serde(default = "default_start_index")]
	pub start_index: i64,
	#[serde(default = "default_count")]
	pub count: i64,
	pub filter: Option<String>,
}

fn default_start_index() -> i64 {
	1
}
fn default_count() -> i64 {
	100
}

#[derive(Clone)]
pub struct ScimState {
	pub pool: SqlitePool,
	pub org_id: OrgId,
}

pub async fn list_users(
	State(state): State<ScimState>,
	Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListResponse<ScimUser>>, ScimApiError> {
	let count = query.count.min(1000);
	let offset = (query.start_index - 1).max(0);
	let org_id_str = state.org_id.to_string();

	let rows = sqlx::query(
		r#"
		SELECT u.id, u.primary_email, u.display_name, u.avatar_url, u.locale,
			   u.scim_external_id, u.deleted_at,
			   u.created_at, u.updated_at
		FROM users u
		JOIN org_memberships om ON u.id = om.user_id
		WHERE om.org_id = ?
		ORDER BY u.id ASC
		LIMIT ? OFFSET ?
		"#,
	)
	.bind(&org_id_str)
	.bind(count)
	.bind(offset)
	.fetch_all(&state.pool)
	.await?;

	let total_row = sqlx::query(
		r#"
		SELECT COUNT(*) as count
		FROM users u
		JOIN org_memberships om ON u.id = om.user_id
		WHERE om.org_id = ?
		"#,
	)
	.bind(&org_id_str)
	.fetch_one(&state.pool)
	.await?;

	let total: i64 = total_row.get("count");

	let users: Vec<ScimUser> = rows
		.into_iter()
		.filter_map(|r| row_to_scim_user(&r).ok())
		.collect();

	Ok(Json(ListResponse::new(
		users,
		total,
		query.start_index,
		count,
	)))
}

pub async fn create_user(
	State(state): State<ScimState>,
	Json(scim_user): Json<ScimUser>,
) -> Result<(StatusCode, Json<ScimUser>), ScimApiError> {
	let email = scim_user_to_email(&scim_user)
		.ok_or_else(|| ScimApiError::BadRequest("userName or email required".to_string()))?;
	let display_name = scim_user_to_display_name(&scim_user);
	let external_id = scim_user.external_id.clone();
	let locale = scim_user.locale.clone();
	let org_id_str = state.org_id.to_string();

	let existing = sqlx::query("SELECT id FROM users WHERE primary_email = ?")
		.bind(&email)
		.fetch_optional(&state.pool)
		.await?;

	let user_id = if let Some(row) = existing {
		let id: String = row.get("id");
		let uid = parse_user_id(&id)?;

		sqlx::query("UPDATE users SET scim_external_id = ?, provisioned_by_scim = 1 WHERE id = ?")
			.bind(&external_id)
			.bind(&id)
			.execute(&state.pool)
			.await?;

		let membership_exists =
			sqlx::query("SELECT 1 FROM org_memberships WHERE user_id = ? AND org_id = ?")
				.bind(&id)
				.bind(&org_id_str)
				.fetch_optional(&state.pool)
				.await?;

		if membership_exists.is_none() {
			let membership_id = Uuid::new_v4().to_string();
			sqlx::query(
				"INSERT INTO org_memberships (id, user_id, org_id, role, provisioned_by, created_at, updated_at)
				 VALUES (?, ?, ?, 'member', 'scim', datetime('now'), datetime('now'))",
			)
			.bind(&membership_id)
			.bind(&id)
			.bind(&org_id_str)
			.execute(&state.pool)
			.await?;
		}

		info!(user_id = %uid, email = %email, "SCIM: linked existing user");
		uid
	} else {
		let new_id = UserId::generate();
		let id_str = new_id.to_string();

		sqlx::query(
			r#"
			INSERT INTO users (id, primary_email, display_name, locale, scim_external_id, provisioned_by_scim, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, 1, datetime('now'), datetime('now'))
			"#,
		)
		.bind(&id_str)
		.bind(&email)
		.bind(&display_name)
		.bind(&locale)
		.bind(&external_id)
		.execute(&state.pool)
		.await?;

		let membership_id = Uuid::new_v4().to_string();
		sqlx::query(
			"INSERT INTO org_memberships (id, user_id, org_id, role, provisioned_by, created_at, updated_at)
			 VALUES (?, ?, ?, 'member', 'scim', datetime('now'), datetime('now'))",
		)
		.bind(&membership_id)
		.bind(&id_str)
		.bind(&org_id_str)
		.execute(&state.pool)
		.await?;

		info!(user_id = %new_id, email = %email, "SCIM: created new user");
		new_id
	};

	let user_id_str = user_id.to_string();
	let row = sqlx::query(
		r#"
		SELECT id, primary_email, display_name, avatar_url, locale,
			   scim_external_id, deleted_at, created_at, updated_at
		FROM users WHERE id = ?
		"#,
	)
	.bind(&user_id_str)
	.fetch_one(&state.pool)
	.await?;

	let scim_user = row_to_scim_user(&row)?;
	Ok((StatusCode::CREATED, Json(scim_user)))
}

pub async fn get_user(
	State(state): State<ScimState>,
	Path(id): Path<String>,
) -> Result<Json<ScimUser>, ScimApiError> {
	let org_id_str = state.org_id.to_string();
	let row = sqlx::query(
		r#"
		SELECT u.id, u.primary_email, u.display_name, u.avatar_url, u.locale,
			   u.scim_external_id, u.deleted_at, u.created_at, u.updated_at
		FROM users u
		JOIN org_memberships om ON u.id = om.user_id
		WHERE u.id = ? AND om.org_id = ?
		"#,
	)
	.bind(&id)
	.bind(&org_id_str)
	.fetch_optional(&state.pool)
	.await?
	.ok_or_else(|| ScimApiError::NotFound(format!("User {} not found", id)))?;

	let scim_user = row_to_scim_user(&row)?;
	Ok(Json(scim_user))
}

pub async fn replace_user(
	State(state): State<ScimState>,
	Path(id): Path<String>,
	Json(scim_user): Json<ScimUser>,
) -> Result<Json<ScimUser>, ScimApiError> {
	let display_name = scim_user_to_display_name(&scim_user);
	let external_id = scim_user.external_id.clone();
	let locale = scim_user.locale.clone();
	let active = scim_user.active;

	let deleted_at: Option<String> = if active {
		None
	} else {
		Some(Utc::now().to_rfc3339())
	};

	sqlx::query(
		r#"
		UPDATE users SET display_name = ?, scim_external_id = ?, locale = ?, deleted_at = ?, updated_at = datetime('now')
		WHERE id = ?
		"#,
	)
	.bind(&display_name)
	.bind(&external_id)
	.bind(&locale)
	.bind(&deleted_at)
	.bind(&id)
	.execute(&state.pool)
	.await?;

	get_user(State(state), Path(id)).await
}

pub async fn patch_user(
	State(state): State<ScimState>,
	Path(id): Path<String>,
	Json(patch): Json<PatchRequest>,
) -> Result<Json<ScimUser>, ScimApiError> {
	patch.validate()?;

	for op in &patch.operations {
		match op.path.as_deref() {
			Some("active") | None if op.value.as_ref().and_then(|v| v.get("active")).is_some() => {
				let active = op
					.value
					.as_ref()
					.and_then(|v| v.get("active"))
					.and_then(|v| v.as_bool())
					.unwrap_or(true);
				let deleted_at: Option<String> = if active {
					None
				} else {
					Some(Utc::now().to_rfc3339())
				};
				sqlx::query("UPDATE users SET deleted_at = ?, updated_at = datetime('now') WHERE id = ?")
					.bind(&deleted_at)
					.bind(&id)
					.execute(&state.pool)
					.await?;
			}
			Some("displayName") => {
				if let Some(value) = op.value.as_ref().and_then(|v| v.as_str()) {
					sqlx::query(
						"UPDATE users SET display_name = ?, updated_at = datetime('now') WHERE id = ?",
					)
					.bind(value)
					.bind(&id)
					.execute(&state.pool)
					.await?;
				}
			}
			_ => {}
		}
	}

	get_user(State(state), Path(id)).await
}

pub async fn delete_user(
	State(state): State<ScimState>,
	Path(id): Path<String>,
) -> Result<StatusCode, ScimApiError> {
	let now = Utc::now().to_rfc3339();
	sqlx::query("UPDATE users SET deleted_at = ?, updated_at = datetime('now') WHERE id = ?")
		.bind(&now)
		.bind(&id)
		.execute(&state.pool)
		.await?;

	let org_id_str = state.org_id.to_string();
	sqlx::query("DELETE FROM org_memberships WHERE user_id = ? AND org_id = ?")
		.bind(&id)
		.bind(&org_id_str)
		.execute(&state.pool)
		.await?;

	info!(user_id = %id, "SCIM: deprovisioned user");
	Ok(StatusCode::NO_CONTENT)
}

fn row_to_scim_user(row: &sqlx::sqlite::SqliteRow) -> Result<ScimUser, ScimApiError> {
	let id: String = row.get("id");
	let email: String = row.get("primary_email");
	let display_name: Option<String> = row.get("display_name");
	let avatar_url: Option<String> = row.get("avatar_url");
	let locale: Option<String> = row.get("locale");
	let scim_external_id: Option<String> = row.get("scim_external_id");
	let deleted_at: Option<String> = row.get("deleted_at");
	let created_at_str: String = row.get("created_at");
	let updated_at_str: String = row.get("updated_at");

	let user_id = parse_user_id(&id)?;
	let active = deleted_at.is_none();

	let created_at = DateTime::parse_from_rfc3339(&created_at_str)
		.map(|d| d.with_timezone(&Utc))
		.unwrap_or_else(|_| Utc::now());
	let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
		.map(|d| d.with_timezone(&Utc))
		.unwrap_or_else(|_| Utc::now());

	let loom_user = LoomUser {
		id: user_id,
		email,
		display_name,
		avatar_url,
		locale,
		scim_external_id,
		active,
		created_at,
		updated_at,
	};

	Ok(loom_user.into())
}
