// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use chrono::{DateTime, Utc};
use loom_server_audit::AuditLogEntry;
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;

use crate::error::ServerError;

pub struct AuditQueryRepository {
	pool: SqlitePool,
}

impl AuditQueryRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	#[allow(clippy::too_many_arguments)]
	pub async fn query_logs(
		&self,
		event_type: Option<&str>,
		actor_id: Option<&str>,
		resource_type: Option<&str>,
		resource_id: Option<&str>,
		from: Option<DateTime<Utc>>,
		to: Option<DateTime<Utc>>,
		limit: Option<i64>,
		offset: Option<i64>,
	) -> Result<(Vec<AuditLogEntry>, i64), ServerError> {
		let limit = limit.unwrap_or(50).min(1000);
		let offset = offset.unwrap_or(0);

		let mut conditions = vec!["1=1".to_string()];
		if event_type.is_some() {
			conditions.push("event_type = ?".to_string());
		}
		if actor_id.is_some() {
			conditions.push("actor_user_id = ?".to_string());
		}
		if resource_type.is_some() {
			conditions.push("resource_type = ?".to_string());
		}
		if resource_id.is_some() {
			conditions.push("resource_id = ?".to_string());
		}
		if from.is_some() {
			conditions.push("timestamp >= ?".to_string());
		}
		if to.is_some() {
			conditions.push("timestamp <= ?".to_string());
		}

		let where_clause = conditions.join(" AND ");

		let count_sql = format!(
			"SELECT COUNT(*) as cnt FROM audit_logs WHERE {}",
			where_clause
		);
		let mut count_query = sqlx::query(&count_sql);
		if let Some(v) = event_type {
			count_query = count_query.bind(v);
		}
		if let Some(v) = actor_id {
			count_query = count_query.bind(v);
		}
		if let Some(v) = resource_type {
			count_query = count_query.bind(v);
		}
		if let Some(v) = resource_id {
			count_query = count_query.bind(v);
		}
		if let Some(v) = from {
			count_query = count_query.bind(v.to_rfc3339());
		}
		if let Some(v) = to {
			count_query = count_query.bind(v.to_rfc3339());
		}

		let count_row = count_query.fetch_one(&self.pool).await?;
		let total: i64 = count_row.get("cnt");

		let data_sql = format!(
			"SELECT id, timestamp, event_type, actor_user_id, impersonating_user_id, \
			 resource_type, resource_id, action, ip_address, user_agent, details \
			 FROM audit_logs WHERE {} ORDER BY timestamp DESC LIMIT ? OFFSET ?",
			where_clause
		);
		let mut data_query = sqlx::query(&data_sql);
		if let Some(v) = event_type {
			data_query = data_query.bind(v);
		}
		if let Some(v) = actor_id {
			data_query = data_query.bind(v);
		}
		if let Some(v) = resource_type {
			data_query = data_query.bind(v);
		}
		if let Some(v) = resource_id {
			data_query = data_query.bind(v);
		}
		if let Some(v) = from {
			data_query = data_query.bind(v.to_rfc3339());
		}
		if let Some(v) = to {
			data_query = data_query.bind(v.to_rfc3339());
		}
		data_query = data_query.bind(limit).bind(offset);

		let rows = data_query.fetch_all(&self.pool).await?;
		let logs: Vec<AuditLogEntry> = rows
			.into_iter()
			.filter_map(|row| {
				let id_str: String = row.get("id");
				let id = Uuid::parse_str(&id_str).ok()?;

				let ts_str: String = row.get("timestamp");
				let timestamp = DateTime::parse_from_rfc3339(&ts_str)
					.map(|dt| dt.with_timezone(&Utc))
					.unwrap_or_else(|_| Utc::now());

				let event_type_str: String = row.get("event_type");
				let event_type = parse_event_type(&event_type_str)?;

				let actor_user_id: Option<String> = row.get("actor_user_id");
				let impersonating_user_id: Option<String> = row.get("impersonating_user_id");
				let details_str: Option<String> = row.get("details");

				Some(AuditLogEntry {
					id,
					timestamp,
					event_type,
					severity: event_type.default_severity(),
					actor_user_id: actor_user_id
						.and_then(|s| Uuid::parse_str(&s).ok())
						.map(loom_server_audit::UserId::new),
					impersonating_user_id: impersonating_user_id
						.and_then(|s| Uuid::parse_str(&s).ok())
						.map(loom_server_audit::UserId::new),
					resource_type: row.get("resource_type"),
					resource_id: row.get("resource_id"),
					action: row.get("action"),
					ip_address: row.get("ip_address"),
					user_agent: row.get("user_agent"),
					details: details_str
						.and_then(|s| serde_json::from_str(&s).ok())
						.unwrap_or(serde_json::Value::Null),
					trace_id: None,
					span_id: None,
					request_id: None,
				})
			})
			.collect();

		Ok((logs, total))
	}
}

fn parse_event_type(s: &str) -> Option<loom_server_audit::AuditEventType> {
	use loom_server_audit::AuditEventType;
	match s {
		"login" => Some(AuditEventType::Login),
		"logout" => Some(AuditEventType::Logout),
		"login_failed" => Some(AuditEventType::LoginFailed),
		"magic_link_requested" => Some(AuditEventType::MagicLinkRequested),
		"magic_link_used" => Some(AuditEventType::MagicLinkUsed),
		"device_code_started" => Some(AuditEventType::DeviceCodeStarted),
		"device_code_completed" => Some(AuditEventType::DeviceCodeCompleted),
		"session_created" => Some(AuditEventType::SessionCreated),
		"session_revoked" => Some(AuditEventType::SessionRevoked),
		"session_expired" => Some(AuditEventType::SessionExpired),
		"api_key_created" => Some(AuditEventType::ApiKeyCreated),
		"api_key_used" => Some(AuditEventType::ApiKeyUsed),
		"api_key_revoked" => Some(AuditEventType::ApiKeyRevoked),
		"access_granted" => Some(AuditEventType::AccessGranted),
		"access_denied" => Some(AuditEventType::AccessDenied),
		"org_created" => Some(AuditEventType::OrgCreated),
		"org_updated" => Some(AuditEventType::OrgUpdated),
		"org_deleted" => Some(AuditEventType::OrgDeleted),
		"org_restored" => Some(AuditEventType::OrgRestored),
		"member_added" => Some(AuditEventType::MemberAdded),
		"member_removed" => Some(AuditEventType::MemberRemoved),
		"role_changed" => Some(AuditEventType::RoleChanged),
		"team_created" => Some(AuditEventType::TeamCreated),
		"team_updated" => Some(AuditEventType::TeamUpdated),
		"team_deleted" => Some(AuditEventType::TeamDeleted),
		"team_member_added" => Some(AuditEventType::TeamMemberAdded),
		"team_member_removed" => Some(AuditEventType::TeamMemberRemoved),
		"thread_created" => Some(AuditEventType::ThreadCreated),
		"thread_deleted" => Some(AuditEventType::ThreadDeleted),
		"thread_shared" => Some(AuditEventType::ThreadShared),
		"thread_unshared" => Some(AuditEventType::ThreadUnshared),
		"thread_visibility_changed" => Some(AuditEventType::ThreadVisibilityChanged),
		"support_access_requested" => Some(AuditEventType::SupportAccessRequested),
		"support_access_approved" => Some(AuditEventType::SupportAccessApproved),
		"support_access_revoked" => Some(AuditEventType::SupportAccessRevoked),
		"impersonation_started" => Some(AuditEventType::ImpersonationStarted),
		"impersonation_ended" => Some(AuditEventType::ImpersonationEnded),
		"global_role_changed" => Some(AuditEventType::GlobalRoleChanged),
		"weaver_created" => Some(AuditEventType::WeaverCreated),
		"weaver_deleted" => Some(AuditEventType::WeaverDeleted),
		"weaver_attached" => Some(AuditEventType::WeaverAttached),
		"llm_request_started" => Some(AuditEventType::LlmRequestStarted),
		"llm_request_completed" => Some(AuditEventType::LlmRequestCompleted),
		"llm_request_failed" => Some(AuditEventType::LlmRequestFailed),
		"repo_created" => Some(AuditEventType::RepoCreated),
		"repo_deleted" => Some(AuditEventType::RepoDeleted),
		"mirror_created" => Some(AuditEventType::MirrorCreated),
		"mirror_synced" => Some(AuditEventType::MirrorSynced),
		"webhook_received" => Some(AuditEventType::WebhookReceived),
		_ => None,
	}
}
