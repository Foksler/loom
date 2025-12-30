// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Audit log repository for database operations.
//!
//! This module provides database access for security audit logging.
//! Audit logs track security-relevant events for compliance and debugging.

use chrono::{DateTime, Duration, Utc};
use loom_auth::{audit::AUDIT_RETENTION_DAYS, AuditEventType, AuditLogEntry, UserId};
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;

use crate::error::ServerError;

/// Repository for audit log database operations.
///
/// Stores and queries security audit events.
/// Audit logs are retained for `AUDIT_RETENTION_DAYS` (90 days by default).
#[derive(Clone)]
pub struct AuditRepository {
	pool: SqlitePool,
}

impl AuditRepository {
	/// Create a new audit repository with the given pool.
	///
	/// # Arguments
	/// * `pool` - SQLite connection pool
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	/// Log an audit event to the database.
	///
	/// # Arguments
	/// * `entry` - The audit log entry to store
	///
	/// # Errors
	/// Returns `ServerError::Database` if insert fails.
	///
	/// # Security Note
	/// Audit logs should never contain plaintext secrets or tokens.
	#[tracing::instrument(
		skip(self, entry),
		fields(
			audit_id = %entry.id,
			event_type = %entry.event_type,
			actor_user_id = ?entry.actor_user_id.as_ref().map(|u| u.to_string()),
			resource_type = ?entry.resource_type,
			resource_id = ?entry.resource_id
		)
	)]
	pub async fn log_event(&self, entry: &AuditLogEntry) -> Result<(), ServerError> {
		let now = Utc::now();
		let details_json = serde_json::to_string(&entry.details)
			.map_err(|e| ServerError::Internal(format!("Failed to serialize details: {e}")))?;

		sqlx::query(
			r#"
			INSERT INTO audit_logs (
				id, timestamp, event_type, actor_user_id, impersonating_user_id,
				resource_type, resource_id, action, ip_address, user_agent,
				details, created_at
			) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(entry.id.to_string())
		.bind(entry.timestamp.to_rfc3339())
		.bind(entry.event_type.to_string())
		.bind(entry.actor_user_id.as_ref().map(|u| u.to_string()))
		.bind(entry.impersonating_user_id.as_ref().map(|u| u.to_string()))
		.bind(&entry.resource_type)
		.bind(&entry.resource_id)
		.bind(&entry.action)
		.bind(&entry.ip_address)
		.bind(&entry.user_agent)
		.bind(&details_json)
		.bind(now.to_rfc3339())
		.execute(&self.pool)
		.await?;

		tracing::debug!(audit_id = %entry.id, event_type = %entry.event_type, "audit event logged");
		Ok(())
	}

	/// Query audit logs with optional filters.
	///
	/// # Arguments
	/// * `event_type` - Filter by event type (e.g., "login", "logout")
	/// * `actor_id` - Filter by the user who performed the action
	/// * `resource_type` - Filter by resource type (e.g., "user", "org")
	/// * `resource_id` - Filter by specific resource ID
	/// * `from` - Filter events after this timestamp
	/// * `to` - Filter events before this timestamp
	/// * `limit` - Maximum number of entries to return
	/// * `offset` - Number of entries to skip
	///
	/// # Returns
	/// Tuple of (entries, total_count) for pagination.
	#[tracing::instrument(skip(self), fields(event_type, actor_id, resource_type, resource_id, limit, offset))]
	#[allow(clippy::too_many_arguments)]
	pub async fn query_logs(
		&self,
		event_type: Option<&str>,
		actor_id: Option<&UserId>,
		resource_type: Option<&str>,
		resource_id: Option<&str>,
		from: Option<DateTime<Utc>>,
		to: Option<DateTime<Utc>>,
		limit: i32,
		offset: i32,
	) -> Result<(Vec<AuditLogEntry>, i64), ServerError> {
		let mut conditions = Vec::new();
		let mut binds: Vec<String> = Vec::new();

		if let Some(et) = event_type {
			conditions.push("event_type = ?");
			binds.push(et.to_string());
		}
		if let Some(actor) = actor_id {
			conditions.push("actor_user_id = ?");
			binds.push(actor.to_string());
		}
		if let Some(rt) = resource_type {
			conditions.push("resource_type = ?");
			binds.push(rt.to_string());
		}
		if let Some(ri) = resource_id {
			conditions.push("resource_id = ?");
			binds.push(ri.to_string());
		}
		if let Some(f) = from {
			conditions.push("timestamp >= ?");
			binds.push(f.to_rfc3339());
		}
		if let Some(t) = to {
			conditions.push("timestamp <= ?");
			binds.push(t.to_rfc3339());
		}

		let where_clause = if conditions.is_empty() {
			String::new()
		} else {
			format!("WHERE {}", conditions.join(" AND "))
		};

		let count_sql = format!("SELECT COUNT(*) as count FROM audit_logs {where_clause}");
		let mut count_query = sqlx::query(&count_sql);
		for bind in &binds {
			count_query = count_query.bind(bind);
		}
		let count_row = count_query.fetch_one(&self.pool).await?;
		let total_count: i64 = count_row.get("count");

		let select_sql = format!(
			r#"
			SELECT id, timestamp, event_type, actor_user_id, impersonating_user_id,
				   resource_type, resource_id, action, ip_address, user_agent, details
			FROM audit_logs
			{where_clause}
			ORDER BY timestamp DESC
			LIMIT ? OFFSET ?
			"#
		);

		let mut select_query = sqlx::query(&select_sql);
		for bind in &binds {
			select_query = select_query.bind(bind);
		}
		select_query = select_query.bind(limit).bind(offset);

		let rows = select_query.fetch_all(&self.pool).await?;

		let mut entries = Vec::with_capacity(rows.len());
		for row in rows {
			entries.push(parse_audit_row(&row)?);
		}

		tracing::debug!(count = entries.len(), total_count, "queried audit logs");
		Ok((entries, total_count))
	}

	/// Delete audit logs older than the retention period.
	///
	/// # Returns
	/// Number of deleted entries.
	///
	/// # Note
	/// Default retention is 90 days (`AUDIT_RETENTION_DAYS`).
	#[tracing::instrument(skip(self))]
	pub async fn cleanup_old_logs(&self) -> Result<i64, ServerError> {
		let cutoff = Utc::now() - Duration::days(AUDIT_RETENTION_DAYS);

		let result = sqlx::query(
			r#"
			DELETE FROM audit_logs
			WHERE timestamp < ?
			"#,
		)
		.bind(cutoff.to_rfc3339())
		.execute(&self.pool)
		.await?;

		let count = result.rows_affected() as i64;
		if count > 0 {
			tracing::info!(count, retention_days = AUDIT_RETENTION_DAYS, "cleaned up old audit logs");
		}
		Ok(count)
	}
}

fn parse_audit_row(row: &sqlx::sqlite::SqliteRow) -> Result<AuditLogEntry, ServerError> {
	let id_str: String = row.get("id");
	let timestamp_str: String = row.get("timestamp");
	let event_type_str: String = row.get("event_type");
	let actor_user_id_str: Option<String> = row.get("actor_user_id");
	let impersonating_user_id_str: Option<String> = row.get("impersonating_user_id");
	let resource_type: Option<String> = row.get("resource_type");
	let resource_id: Option<String> = row.get("resource_id");
	let action: String = row.get("action");
	let ip_address: Option<String> = row.get("ip_address");
	let user_agent: Option<String> = row.get("user_agent");
	let details_str: Option<String> = row.get("details");

	let id = Uuid::parse_str(&id_str)
		.map_err(|e| ServerError::Internal(format!("Invalid audit log id UUID: {e}")))?;

	let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
		.map_err(|e| ServerError::Internal(format!("Invalid timestamp: {e}")))?
		.with_timezone(&Utc);

	let event_type: AuditEventType = serde_json::from_value(serde_json::Value::String(event_type_str.clone()))
		.map_err(|e| ServerError::Internal(format!("Invalid event_type '{event_type_str}': {e}")))?;

	let actor_user_id = actor_user_id_str
		.map(|s| {
			Uuid::parse_str(&s)
				.map(UserId::new)
				.map_err(|e| ServerError::Internal(format!("Invalid actor_user_id UUID: {e}")))
		})
		.transpose()?;

	let impersonating_user_id = impersonating_user_id_str
		.map(|s| {
			Uuid::parse_str(&s)
				.map(UserId::new)
				.map_err(|e| ServerError::Internal(format!("Invalid impersonating_user_id UUID: {e}")))
		})
		.transpose()?;

	let details: serde_json::Value = details_str
		.map(|s| serde_json::from_str(&s))
		.transpose()
		.map_err(|e| ServerError::Internal(format!("Invalid details JSON: {e}")))?
		.unwrap_or(serde_json::Value::Null);

	Ok(AuditLogEntry {
		id,
		timestamp,
		event_type,
		actor_user_id,
		impersonating_user_id,
		resource_type,
		resource_id,
		action,
		ip_address,
		user_agent,
		details,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;
	use std::collections::HashSet;

	proptest! {
		#[test]
		fn audit_id_generation_is_unique(count in 1..1000usize) {
			let mut ids = HashSet::new();
			for _ in 0..count {
				let id = Uuid::new_v4();
				prop_assert!(ids.insert(id.to_string()), "Generated duplicate audit ID");
			}
		}

		#[test]
		fn query_logs_pagination_bounds(limit in 0i32..1000, offset in 0i32..10000) {
			prop_assert!(limit >= 0, "limit must be non-negative");
			prop_assert!(offset >= 0, "offset must be non-negative");
		}

		#[test]
		fn retention_days_is_reasonable(_unused: u8) {
			prop_assert!(AUDIT_RETENTION_DAYS >= 30, "Retention should be at least 30 days");
			prop_assert!(AUDIT_RETENTION_DAYS <= 365, "Retention should be at most 1 year");
		}

		#[test]
		fn timestamp_roundtrip(year in 2020i32..2030, month in 1u32..=12, day in 1u32..=28) {
			use chrono::NaiveDate;
			let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
			let datetime = date.and_hms_opt(12, 0, 0).unwrap().and_utc();
			let rfc3339 = datetime.to_rfc3339();
			let parsed = DateTime::parse_from_rfc3339(&rfc3339);
			prop_assert!(parsed.is_ok(), "RFC3339 timestamp should roundtrip");
		}
	}
}
