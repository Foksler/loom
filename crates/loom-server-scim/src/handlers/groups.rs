// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	Json,
};
use chrono::{DateTime, Utc};
use loom_scim::patch::PatchRequest;
use loom_scim::types::{GroupMember, Meta, SCHEMA_CORE_GROUP};
use loom_scim::{ListResponse, ScimGroup};
use serde::Deserialize;
use sqlx::Row;
use tracing::info;
use uuid::Uuid;

use crate::error::ScimApiError;
use crate::handlers::users::ScimState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGroupsQuery {
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

pub async fn list_groups(
	State(state): State<ScimState>,
	Query(query): Query<ListGroupsQuery>,
) -> Result<Json<ListResponse<ScimGroup>>, ScimApiError> {
	let count = query.count.min(1000);
	let offset = (query.start_index - 1).max(0);
	let org_id_str = state.org_id.to_string();

	let rows = sqlx::query(
		r#"
		SELECT id, name, scim_external_id, created_at, updated_at
		FROM teams
		WHERE org_id = ?
		ORDER BY id ASC
		LIMIT ? OFFSET ?
		"#,
	)
	.bind(&org_id_str)
	.bind(count)
	.bind(offset)
	.fetch_all(&state.pool)
	.await?;

	let total_row = sqlx::query(r#"SELECT COUNT(*) as count FROM teams WHERE org_id = ?"#)
		.bind(&org_id_str)
		.fetch_one(&state.pool)
		.await?;

	let total: i64 = total_row.get("count");

	let mut groups = Vec::new();
	for row in rows {
		let team_id: String = row.get("id");
		let name: String = row.get("name");
		let scim_external_id: Option<String> = row.get("scim_external_id");
		let created_at_str: String = row.get("created_at");
		let updated_at_str: String = row.get("updated_at");

		let members = get_group_members(&state, &team_id).await?;

		let created_at = DateTime::parse_from_rfc3339(&created_at_str)
			.map(|d| d.with_timezone(&Utc))
			.unwrap_or_else(|_| Utc::now());
		let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
			.map(|d| d.with_timezone(&Utc))
			.unwrap_or_else(|_| Utc::now());

		groups.push(ScimGroup {
			schemas: vec![SCHEMA_CORE_GROUP.to_string()],
			id: Some(team_id),
			external_id: scim_external_id,
			display_name: name,
			members,
			meta: Some(Meta {
				resource_type: "Group".to_string(),
				created: created_at,
				last_modified: updated_at,
				location: None,
				version: None,
			}),
		});
	}

	Ok(Json(ListResponse::new(
		groups,
		total,
		query.start_index,
		count,
	)))
}

pub async fn create_group(
	State(state): State<ScimState>,
	Json(scim_group): Json<ScimGroup>,
) -> Result<(StatusCode, Json<ScimGroup>), ScimApiError> {
	let team_id = Uuid::new_v4().to_string();
	let org_id_str = state.org_id.to_string();

	sqlx::query(
		r#"
		INSERT INTO teams (id, org_id, name, scim_external_id, scim_managed, created_at, updated_at)
		VALUES (?, ?, ?, ?, 1, datetime('now'), datetime('now'))
		"#,
	)
	.bind(&team_id)
	.bind(&org_id_str)
	.bind(&scim_group.display_name)
	.bind(&scim_group.external_id)
	.execute(&state.pool)
	.await?;

	for member in &scim_group.members {
		add_member_to_team(&state, &team_id, &member.value).await?;
	}

	info!(team_id = %team_id, name = %scim_group.display_name, "SCIM: created group");
	get_group(State(state), Path(team_id))
		.await
		.map(|g| (StatusCode::CREATED, g))
}

pub async fn get_group(
	State(state): State<ScimState>,
	Path(id): Path<String>,
) -> Result<Json<ScimGroup>, ScimApiError> {
	let org_id_str = state.org_id.to_string();
	let row = sqlx::query(
		r#"
		SELECT id, name, scim_external_id, created_at, updated_at
		FROM teams
		WHERE id = ? AND org_id = ?
		"#,
	)
	.bind(&id)
	.bind(&org_id_str)
	.fetch_optional(&state.pool)
	.await?
	.ok_or_else(|| ScimApiError::NotFound(format!("Group {} not found", id)))?;

	let team_id: String = row.get("id");
	let name: String = row.get("name");
	let scim_external_id: Option<String> = row.get("scim_external_id");
	let created_at_str: String = row.get("created_at");
	let updated_at_str: String = row.get("updated_at");

	let members = get_group_members(&state, &team_id).await?;

	let created_at = DateTime::parse_from_rfc3339(&created_at_str)
		.map(|d| d.with_timezone(&Utc))
		.unwrap_or_else(|_| Utc::now());
	let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
		.map(|d| d.with_timezone(&Utc))
		.unwrap_or_else(|_| Utc::now());

	Ok(Json(ScimGroup {
		schemas: vec![SCHEMA_CORE_GROUP.to_string()],
		id: Some(team_id),
		external_id: scim_external_id,
		display_name: name,
		members,
		meta: Some(Meta {
			resource_type: "Group".to_string(),
			created: created_at,
			last_modified: updated_at,
			location: None,
			version: None,
		}),
	}))
}

pub async fn replace_group(
	State(state): State<ScimState>,
	Path(id): Path<String>,
	Json(scim_group): Json<ScimGroup>,
) -> Result<Json<ScimGroup>, ScimApiError> {
	let org_id_str = state.org_id.to_string();
	sqlx::query(
		r#"
		UPDATE teams SET name = ?, scim_external_id = ?, updated_at = datetime('now')
		WHERE id = ? AND org_id = ?
		"#,
	)
	.bind(&scim_group.display_name)
	.bind(&scim_group.external_id)
	.bind(&id)
	.bind(&org_id_str)
	.execute(&state.pool)
	.await?;

	sqlx::query("DELETE FROM team_memberships WHERE team_id = ?")
		.bind(&id)
		.execute(&state.pool)
		.await?;

	for member in &scim_group.members {
		add_member_to_team(&state, &id, &member.value).await?;
	}

	get_group(State(state), Path(id)).await
}

pub async fn patch_group(
	State(state): State<ScimState>,
	Path(id): Path<String>,
	Json(patch): Json<PatchRequest>,
) -> Result<Json<ScimGroup>, ScimApiError> {
	patch.validate()?;

	for op in &patch.operations {
		match op.path.as_deref() {
			Some("displayName") => {
				if let Some(value) = op.value.as_ref().and_then(|v| v.as_str()) {
					sqlx::query("UPDATE teams SET name = ?, updated_at = datetime('now') WHERE id = ?")
						.bind(value)
						.bind(&id)
						.execute(&state.pool)
						.await?;
				}
			}
			Some("members") => {
				if let Some(members) = op.value.as_ref().and_then(|v| v.as_array()) {
					for member in members {
						if let Some(user_id) = member.get("value").and_then(|v| v.as_str()) {
							match op.op {
								loom_scim::PatchOp::Add => {
									add_member_to_team(&state, &id, user_id).await?;
								}
								loom_scim::PatchOp::Remove => {
									sqlx::query("DELETE FROM team_memberships WHERE team_id = ? AND user_id = ?")
										.bind(&id)
										.bind(user_id)
										.execute(&state.pool)
										.await?;
								}
								_ => {}
							}
						}
					}
				}
			}
			_ => {}
		}
	}

	get_group(State(state), Path(id)).await
}

pub async fn delete_group(
	State(state): State<ScimState>,
	Path(id): Path<String>,
) -> Result<StatusCode, ScimApiError> {
	let org_id_str = state.org_id.to_string();
	sqlx::query("DELETE FROM team_memberships WHERE team_id = ?")
		.bind(&id)
		.execute(&state.pool)
		.await?;

	sqlx::query("DELETE FROM teams WHERE id = ? AND org_id = ? AND scim_managed = 1")
		.bind(&id)
		.bind(&org_id_str)
		.execute(&state.pool)
		.await?;

	info!(team_id = %id, "SCIM: deleted group");
	Ok(StatusCode::NO_CONTENT)
}

async fn get_group_members(
	state: &ScimState,
	team_id: &str,
) -> Result<Vec<GroupMember>, ScimApiError> {
	let rows = sqlx::query(
		r#"
		SELECT u.id, u.display_name
		FROM users u
		JOIN team_memberships tm ON u.id = tm.user_id
		WHERE tm.team_id = ?
		"#,
	)
	.bind(team_id)
	.fetch_all(&state.pool)
	.await?;

	Ok(
		rows
			.into_iter()
			.map(|r| {
				let id: String = r.get("id");
				let display_name: Option<String> = r.get("display_name");
				GroupMember {
					value: id.clone(),
					ref_: Some(format!("/api/scim/Users/{}", id)),
					display: display_name,
				}
			})
			.collect(),
	)
}

async fn add_member_to_team(
	state: &ScimState,
	team_id: &str,
	user_id: &str,
) -> Result<(), ScimApiError> {
	let membership_id = Uuid::new_v4().to_string();
	sqlx::query(
		"INSERT OR IGNORE INTO team_memberships (id, team_id, user_id, role, created_at, updated_at)
		 VALUES (?, ?, ?, 'member', datetime('now'), datetime('now'))",
	)
	.bind(&membership_id)
	.bind(team_id)
	.bind(user_id)
	.execute(&state.pool)
	.await?;
	Ok(())
}
