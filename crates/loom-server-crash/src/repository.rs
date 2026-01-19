// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Repository layer for crash database operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tracing::instrument;

use loom_crash_core::{
	CrashEvent, CrashEventId, CrashProject, Issue, IssueId, OrgId, PersonId, ProjectId, UserId,
};

use crate::error::{CrashServerError, Result};

/// Repository trait for crash operations.
#[async_trait]
pub trait CrashRepository: Send + Sync {
	// Project operations
	async fn create_project(&self, project: &CrashProject) -> Result<()>;
	async fn get_project_by_id(&self, id: ProjectId) -> Result<Option<CrashProject>>;
	async fn get_project_by_slug(&self, org_id: OrgId, slug: &str) -> Result<Option<CrashProject>>;
	async fn list_projects(&self, org_id: OrgId) -> Result<Vec<CrashProject>>;
	async fn delete_project(&self, id: ProjectId) -> Result<bool>;

	// Issue operations
	async fn create_issue(&self, issue: &Issue) -> Result<()>;
	async fn get_issue_by_id(&self, id: IssueId) -> Result<Option<Issue>>;
	async fn get_issue_by_fingerprint(
		&self,
		project_id: ProjectId,
		fingerprint: &str,
	) -> Result<Option<Issue>>;
	async fn list_issues(&self, project_id: ProjectId, limit: u32) -> Result<Vec<Issue>>;
	async fn update_issue(&self, issue: &Issue) -> Result<()>;
	async fn delete_issue(&self, id: IssueId) -> Result<bool>;

	// Event operations
	async fn create_event(&self, event: &CrashEvent) -> Result<()>;
	async fn get_event_by_id(&self, id: CrashEventId) -> Result<Option<CrashEvent>>;
	async fn list_events_for_issue(&self, issue_id: IssueId, limit: u32) -> Result<Vec<CrashEvent>>;

	// Issue state updates
	async fn increment_issue_event_count(&self, id: IssueId) -> Result<()>;
	async fn add_issue_person(&self, issue_id: IssueId, person_id: PersonId) -> Result<()>;
	async fn issue_has_person(&self, issue_id: IssueId, person_id: PersonId) -> Result<bool>;

	// Short ID generation
	async fn get_next_short_id(&self, project_id: ProjectId) -> Result<String>;
}

/// SQLite implementation of the crash repository.
#[derive(Clone)]
pub struct SqliteCrashRepository {
	pool: SqlitePool,
}

impl SqliteCrashRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

#[async_trait]
impl CrashRepository for SqliteCrashRepository {
	#[instrument(skip(self, project), fields(project_id = %project.id, slug = %project.slug))]
	async fn create_project(&self, project: &CrashProject) -> Result<()> {
		let fingerprint_rules_json = serde_json::to_string(&project.fingerprint_rules)?;

		sqlx::query(
			r#"
			INSERT INTO crash_projects (
				id, org_id, name, slug, platform,
				auto_resolve_age_days, fingerprint_rules,
				created_at, updated_at
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(project.id.0.to_string())
		.bind(project.org_id.0.to_string())
		.bind(&project.name)
		.bind(&project.slug)
		.bind(project.platform.to_string())
		.bind(project.auto_resolve_age_days.map(|d| d as i32))
		.bind(fingerprint_rules_json)
		.bind(project.created_at.to_rfc3339())
		.bind(project.updated_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(project_id = %id))]
	async fn get_project_by_id(&self, id: ProjectId) -> Result<Option<CrashProject>> {
		let row = sqlx::query_as::<_, ProjectRow>(
			r#"
			SELECT id, org_id, name, slug, platform,
				   auto_resolve_age_days, fingerprint_rules,
				   created_at, updated_at
			FROM crash_projects
			WHERE id = ?
			"#,
		)
		.bind(id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(org_id = %org_id, slug = %slug))]
	async fn get_project_by_slug(&self, org_id: OrgId, slug: &str) -> Result<Option<CrashProject>> {
		let row = sqlx::query_as::<_, ProjectRow>(
			r#"
			SELECT id, org_id, name, slug, platform,
				   auto_resolve_age_days, fingerprint_rules,
				   created_at, updated_at
			FROM crash_projects
			WHERE org_id = ? AND slug = ?
			"#,
		)
		.bind(org_id.0.to_string())
		.bind(slug)
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(org_id = %org_id))]
	async fn list_projects(&self, org_id: OrgId) -> Result<Vec<CrashProject>> {
		let rows = sqlx::query_as::<_, ProjectRow>(
			r#"
			SELECT id, org_id, name, slug, platform,
				   auto_resolve_age_days, fingerprint_rules,
				   created_at, updated_at
			FROM crash_projects
			WHERE org_id = ?
			ORDER BY name
			"#,
		)
		.bind(org_id.0.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(project_id = %id))]
	async fn delete_project(&self, id: ProjectId) -> Result<bool> {
		let result = sqlx::query("DELETE FROM crash_projects WHERE id = ?")
			.bind(id.0.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected() > 0)
	}

	#[instrument(skip(self, issue), fields(issue_id = %issue.id, fingerprint = %issue.fingerprint))]
	async fn create_issue(&self, issue: &Issue) -> Result<()> {
		let metadata_json = serde_json::to_string(&issue.metadata)?;

		sqlx::query(
			r#"
			INSERT INTO crash_issues (
				id, org_id, project_id, short_id, fingerprint,
				title, culprit, metadata,
				status, level, priority,
				event_count, user_count,
				first_seen, last_seen,
				resolved_at, resolved_by, resolved_in_release,
				times_regressed, last_regressed_at, regressed_in_release,
				assigned_to, created_at, updated_at
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(issue.id.0.to_string())
		.bind(issue.org_id.0.to_string())
		.bind(issue.project_id.0.to_string())
		.bind(&issue.short_id)
		.bind(&issue.fingerprint)
		.bind(&issue.title)
		.bind(&issue.culprit)
		.bind(metadata_json)
		.bind(issue.status.to_string())
		.bind(issue.level.to_string())
		.bind(issue.priority.to_string())
		.bind(issue.event_count as i64)
		.bind(issue.user_count as i64)
		.bind(issue.first_seen.to_rfc3339())
		.bind(issue.last_seen.to_rfc3339())
		.bind(issue.resolved_at.map(|dt| dt.to_rfc3339()))
		.bind(issue.resolved_by.map(|u| u.0.to_string()))
		.bind(&issue.resolved_in_release)
		.bind(issue.times_regressed as i32)
		.bind(issue.last_regressed_at.map(|dt| dt.to_rfc3339()))
		.bind(&issue.regressed_in_release)
		.bind(issue.assigned_to.map(|u| u.0.to_string()))
		.bind(issue.created_at.to_rfc3339())
		.bind(issue.updated_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(issue_id = %id))]
	async fn get_issue_by_id(&self, id: IssueId) -> Result<Option<Issue>> {
		let row = sqlx::query_as::<_, IssueRow>(
			r#"
			SELECT id, org_id, project_id, short_id, fingerprint,
				   title, culprit, metadata,
				   status, level, priority,
				   event_count, user_count,
				   first_seen, last_seen,
				   resolved_at, resolved_by, resolved_in_release,
				   times_regressed, last_regressed_at, regressed_in_release,
				   assigned_to, created_at, updated_at
			FROM crash_issues
			WHERE id = ?
			"#,
		)
		.bind(id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(project_id = %project_id))]
	async fn get_issue_by_fingerprint(
		&self,
		project_id: ProjectId,
		fingerprint: &str,
	) -> Result<Option<Issue>> {
		let row = sqlx::query_as::<_, IssueRow>(
			r#"
			SELECT id, org_id, project_id, short_id, fingerprint,
				   title, culprit, metadata,
				   status, level, priority,
				   event_count, user_count,
				   first_seen, last_seen,
				   resolved_at, resolved_by, resolved_in_release,
				   times_regressed, last_regressed_at, regressed_in_release,
				   assigned_to, created_at, updated_at
			FROM crash_issues
			WHERE project_id = ? AND fingerprint = ?
			"#,
		)
		.bind(project_id.0.to_string())
		.bind(fingerprint)
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(project_id = %project_id))]
	async fn list_issues(&self, project_id: ProjectId, limit: u32) -> Result<Vec<Issue>> {
		let rows = sqlx::query_as::<_, IssueRow>(
			r#"
			SELECT id, org_id, project_id, short_id, fingerprint,
				   title, culprit, metadata,
				   status, level, priority,
				   event_count, user_count,
				   first_seen, last_seen,
				   resolved_at, resolved_by, resolved_in_release,
				   times_regressed, last_regressed_at, regressed_in_release,
				   assigned_to, created_at, updated_at
			FROM crash_issues
			WHERE project_id = ?
			ORDER BY last_seen DESC
			LIMIT ?
			"#,
		)
		.bind(project_id.0.to_string())
		.bind(limit as i32)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self, issue), fields(issue_id = %issue.id))]
	async fn update_issue(&self, issue: &Issue) -> Result<()> {
		let metadata_json = serde_json::to_string(&issue.metadata)?;

		sqlx::query(
			r#"
			UPDATE crash_issues SET
				title = ?, culprit = ?, metadata = ?,
				status = ?, level = ?, priority = ?,
				event_count = ?, user_count = ?,
				last_seen = ?,
				resolved_at = ?, resolved_by = ?, resolved_in_release = ?,
				times_regressed = ?, last_regressed_at = ?, regressed_in_release = ?,
				assigned_to = ?, updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&issue.title)
		.bind(&issue.culprit)
		.bind(metadata_json)
		.bind(issue.status.to_string())
		.bind(issue.level.to_string())
		.bind(issue.priority.to_string())
		.bind(issue.event_count as i64)
		.bind(issue.user_count as i64)
		.bind(issue.last_seen.to_rfc3339())
		.bind(issue.resolved_at.map(|dt| dt.to_rfc3339()))
		.bind(issue.resolved_by.map(|u| u.0.to_string()))
		.bind(&issue.resolved_in_release)
		.bind(issue.times_regressed as i32)
		.bind(issue.last_regressed_at.map(|dt| dt.to_rfc3339()))
		.bind(&issue.regressed_in_release)
		.bind(issue.assigned_to.map(|u| u.0.to_string()))
		.bind(Utc::now().to_rfc3339())
		.bind(issue.id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(issue_id = %id))]
	async fn delete_issue(&self, id: IssueId) -> Result<bool> {
		let result = sqlx::query("DELETE FROM crash_issues WHERE id = ?")
			.bind(id.0.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected() > 0)
	}

	#[instrument(skip(self, event), fields(event_id = %event.id, exception_type = %event.exception_type))]
	async fn create_event(&self, event: &CrashEvent) -> Result<()> {
		let stacktrace_json = serde_json::to_string(&event.stacktrace)?;
		let raw_stacktrace_json = event
			.raw_stacktrace
			.as_ref()
			.map(|s| serde_json::to_string(s))
			.transpose()?;
		let runtime_json = event
			.runtime
			.as_ref()
			.map(|r| serde_json::to_string(r))
			.transpose()?;
		let tags_json = serde_json::to_string(&event.tags)?;
		let extra_json = serde_json::to_string(&event.extra)?;
		let user_context_json = event
			.user_context
			.as_ref()
			.map(|c| serde_json::to_string(c))
			.transpose()?;
		let device_context_json = event
			.device_context
			.as_ref()
			.map(|c| serde_json::to_string(c))
			.transpose()?;
		let browser_context_json = event
			.browser_context
			.as_ref()
			.map(|c| serde_json::to_string(c))
			.transpose()?;
		let os_context_json = event
			.os_context
			.as_ref()
			.map(|c| serde_json::to_string(c))
			.transpose()?;
		let active_flags_json = serde_json::to_string(&event.active_flags)?;
		let request_json = event
			.request
			.as_ref()
			.map(|r| serde_json::to_string(r))
			.transpose()?;
		let breadcrumbs_json = serde_json::to_string(&event.breadcrumbs)?;

		sqlx::query(
			r#"
			INSERT INTO crash_events (
				id, org_id, project_id, issue_id,
				person_id, distinct_id,
				exception_type, exception_value, stacktrace, raw_stacktrace,
				release, dist, environment, platform, runtime, server_name,
				tags, extra, user_context, device_context, browser_context, os_context,
				active_flags, request, breadcrumbs,
				timestamp, received_at
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(event.id.0.to_string())
		.bind(event.org_id.0.to_string())
		.bind(event.project_id.0.to_string())
		.bind(event.issue_id.map(|i| i.0.to_string()))
		.bind(event.person_id.map(|p| p.0.to_string()))
		.bind(&event.distinct_id)
		.bind(&event.exception_type)
		.bind(&event.exception_value)
		.bind(stacktrace_json)
		.bind(raw_stacktrace_json)
		.bind(&event.release)
		.bind(&event.dist)
		.bind(&event.environment)
		.bind(event.platform.to_string())
		.bind(runtime_json)
		.bind(&event.server_name)
		.bind(tags_json)
		.bind(extra_json)
		.bind(user_context_json)
		.bind(device_context_json)
		.bind(browser_context_json)
		.bind(os_context_json)
		.bind(active_flags_json)
		.bind(request_json)
		.bind(breadcrumbs_json)
		.bind(event.timestamp.to_rfc3339())
		.bind(event.received_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(event_id = %id))]
	async fn get_event_by_id(&self, id: CrashEventId) -> Result<Option<CrashEvent>> {
		let row = sqlx::query_as::<_, EventRow>(
			r#"
			SELECT id, org_id, project_id, issue_id,
				   person_id, distinct_id,
				   exception_type, exception_value, stacktrace, raw_stacktrace,
				   release, dist, environment, platform, runtime, server_name,
				   tags, extra, user_context, device_context, browser_context, os_context,
				   active_flags, request, breadcrumbs,
				   timestamp, received_at
			FROM crash_events
			WHERE id = ?
			"#,
		)
		.bind(id.0.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(issue_id = %issue_id))]
	async fn list_events_for_issue(
		&self,
		issue_id: IssueId,
		limit: u32,
	) -> Result<Vec<CrashEvent>> {
		let rows = sqlx::query_as::<_, EventRow>(
			r#"
			SELECT id, org_id, project_id, issue_id,
				   person_id, distinct_id,
				   exception_type, exception_value, stacktrace, raw_stacktrace,
				   release, dist, environment, platform, runtime, server_name,
				   tags, extra, user_context, device_context, browser_context, os_context,
				   active_flags, request, breadcrumbs,
				   timestamp, received_at
			FROM crash_events
			WHERE issue_id = ?
			ORDER BY timestamp DESC
			LIMIT ?
			"#,
		)
		.bind(issue_id.0.to_string())
		.bind(limit as i32)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(issue_id = %id))]
	async fn increment_issue_event_count(&self, id: IssueId) -> Result<()> {
		sqlx::query(
			r#"
			UPDATE crash_issues SET
				event_count = event_count + 1,
				last_seen = ?,
				updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(Utc::now().to_rfc3339())
		.bind(Utc::now().to_rfc3339())
		.bind(id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(issue_id = %issue_id, person_id = %person_id))]
	async fn add_issue_person(&self, issue_id: IssueId, person_id: PersonId) -> Result<()> {
		sqlx::query(
			r#"
			INSERT OR IGNORE INTO crash_issue_persons (issue_id, person_id, first_seen)
			VALUES (?, ?, ?)
			"#,
		)
		.bind(issue_id.0.to_string())
		.bind(person_id.0.to_string())
		.bind(Utc::now().to_rfc3339())
		.execute(&self.pool)
		.await?;

		// Update user_count
		sqlx::query(
			r#"
			UPDATE crash_issues SET
				user_count = (SELECT COUNT(*) FROM crash_issue_persons WHERE issue_id = ?),
				updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(issue_id.0.to_string())
		.bind(Utc::now().to_rfc3339())
		.bind(issue_id.0.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(issue_id = %issue_id, person_id = %person_id))]
	async fn issue_has_person(&self, issue_id: IssueId, person_id: PersonId) -> Result<bool> {
		let row = sqlx::query_scalar::<_, i32>(
			r#"
			SELECT COUNT(*) FROM crash_issue_persons
			WHERE issue_id = ? AND person_id = ?
			"#,
		)
		.bind(issue_id.0.to_string())
		.bind(person_id.0.to_string())
		.fetch_one(&self.pool)
		.await?;

		Ok(row > 0)
	}

	#[instrument(skip(self), fields(project_id = %project_id))]
	async fn get_next_short_id(&self, project_id: ProjectId) -> Result<String> {
		// Get project slug for the short ID prefix
		let project = self
			.get_project_by_id(project_id)
			.await?
			.ok_or_else(|| CrashServerError::ProjectNotFound(project_id.to_string()))?;

		// Count existing issues for this project
		let count = sqlx::query_scalar::<_, i32>(
			r#"
			SELECT COUNT(*) FROM crash_issues WHERE project_id = ?
			"#,
		)
		.bind(project_id.0.to_string())
		.fetch_one(&self.pool)
		.await?;

		// Generate short ID like "PROJ-123"
		let prefix = project.slug.to_uppercase();
		let prefix = if prefix.len() > 4 {
			&prefix[..4]
		} else {
			&prefix
		};

		Ok(format!("{}-{}", prefix, count + 1))
	}
}

// ============================================================================
// Row types for SQLite
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct ProjectRow {
	id: String,
	org_id: String,
	name: String,
	slug: String,
	platform: String,
	auto_resolve_age_days: Option<i32>,
	fingerprint_rules: String,
	created_at: String,
	updated_at: String,
}

impl TryFrom<ProjectRow> for CrashProject {
	type Error = CrashServerError;

	fn try_from(row: ProjectRow) -> Result<Self> {
		Ok(CrashProject {
			id: ProjectId(row.id.parse()?),
			org_id: OrgId(row.org_id.parse()?),
			name: row.name,
			slug: row.slug,
			platform: row
				.platform
				.parse()
				.map_err(|_| CrashServerError::Parse(format!("invalid platform: {}", row.platform)))?,
			auto_resolve_age_days: row.auto_resolve_age_days.map(|d| d as u32),
			fingerprint_rules: serde_json::from_str(&row.fingerprint_rules)?,
			created_at: parse_datetime(&row.created_at)?,
			updated_at: parse_datetime(&row.updated_at)?,
		})
	}
}

#[derive(Debug, sqlx::FromRow)]
struct IssueRow {
	id: String,
	org_id: String,
	project_id: String,
	short_id: String,
	fingerprint: String,
	title: String,
	culprit: Option<String>,
	metadata: String,
	status: String,
	level: String,
	priority: String,
	event_count: i64,
	user_count: i64,
	first_seen: String,
	last_seen: String,
	resolved_at: Option<String>,
	resolved_by: Option<String>,
	resolved_in_release: Option<String>,
	times_regressed: i32,
	last_regressed_at: Option<String>,
	regressed_in_release: Option<String>,
	assigned_to: Option<String>,
	created_at: String,
	updated_at: String,
}

impl TryFrom<IssueRow> for Issue {
	type Error = CrashServerError;

	fn try_from(row: IssueRow) -> Result<Self> {
		Ok(Issue {
			id: IssueId(row.id.parse()?),
			org_id: OrgId(row.org_id.parse()?),
			project_id: ProjectId(row.project_id.parse()?),
			short_id: row.short_id,
			fingerprint: row.fingerprint,
			title: row.title,
			culprit: row.culprit,
			metadata: serde_json::from_str(&row.metadata)?,
			status: row
				.status
				.parse()
				.map_err(|_| CrashServerError::Parse(format!("invalid status: {}", row.status)))?,
			level: row
				.level
				.parse()
				.map_err(|_| CrashServerError::Parse(format!("invalid level: {}", row.level)))?,
			priority: row.priority.parse().map_err(|_| {
				CrashServerError::Parse(format!("invalid priority: {}", row.priority))
			})?,
			event_count: row.event_count as u64,
			user_count: row.user_count as u64,
			first_seen: parse_datetime(&row.first_seen)?,
			last_seen: parse_datetime(&row.last_seen)?,
			resolved_at: row.resolved_at.map(|s| parse_datetime(&s)).transpose()?,
			resolved_by: row
				.resolved_by
				.map(|s| Ok::<_, CrashServerError>(UserId(s.parse()?)))
				.transpose()?,
			resolved_in_release: row.resolved_in_release,
			times_regressed: row.times_regressed as u32,
			last_regressed_at: row.last_regressed_at.map(|s| parse_datetime(&s)).transpose()?,
			regressed_in_release: row.regressed_in_release,
			assigned_to: row
				.assigned_to
				.map(|s| Ok::<_, CrashServerError>(UserId(s.parse()?)))
				.transpose()?,
			created_at: parse_datetime(&row.created_at)?,
			updated_at: parse_datetime(&row.updated_at)?,
		})
	}
}

#[derive(Debug, sqlx::FromRow)]
struct EventRow {
	id: String,
	org_id: String,
	project_id: String,
	issue_id: Option<String>,
	person_id: Option<String>,
	distinct_id: String,
	exception_type: String,
	exception_value: String,
	stacktrace: String,
	raw_stacktrace: Option<String>,
	release: Option<String>,
	dist: Option<String>,
	environment: String,
	platform: String,
	runtime: Option<String>,
	server_name: Option<String>,
	tags: String,
	extra: String,
	user_context: Option<String>,
	device_context: Option<String>,
	browser_context: Option<String>,
	os_context: Option<String>,
	active_flags: String,
	request: Option<String>,
	breadcrumbs: String,
	timestamp: String,
	received_at: String,
}

impl TryFrom<EventRow> for CrashEvent {
	type Error = CrashServerError;

	fn try_from(row: EventRow) -> Result<Self> {
		Ok(CrashEvent {
			id: CrashEventId(row.id.parse()?),
			org_id: OrgId(row.org_id.parse()?),
			project_id: ProjectId(row.project_id.parse()?),
			issue_id: row
				.issue_id
				.map(|s| Ok::<_, CrashServerError>(IssueId(s.parse()?)))
				.transpose()?,
			person_id: row
				.person_id
				.map(|s| Ok::<_, CrashServerError>(PersonId(s.parse()?)))
				.transpose()?,
			distinct_id: row.distinct_id,
			exception_type: row.exception_type,
			exception_value: row.exception_value,
			stacktrace: serde_json::from_str(&row.stacktrace)?,
			raw_stacktrace: row.raw_stacktrace.map(|s| serde_json::from_str(&s)).transpose()?,
			release: row.release,
			dist: row.dist,
			environment: row.environment,
			platform: row.platform.parse().map_err(|_| {
				CrashServerError::Parse(format!("invalid platform: {}", row.platform))
			})?,
			runtime: row.runtime.map(|s| serde_json::from_str(&s)).transpose()?,
			server_name: row.server_name,
			tags: serde_json::from_str(&row.tags)?,
			extra: serde_json::from_str(&row.extra)?,
			user_context: row.user_context.map(|s| serde_json::from_str(&s)).transpose()?,
			device_context: row.device_context.map(|s| serde_json::from_str(&s)).transpose()?,
			browser_context: row.browser_context.map(|s| serde_json::from_str(&s)).transpose()?,
			os_context: row.os_context.map(|s| serde_json::from_str(&s)).transpose()?,
			active_flags: serde_json::from_str(&row.active_flags)?,
			request: row.request.map(|s| serde_json::from_str(&s)).transpose()?,
			breadcrumbs: serde_json::from_str(&row.breadcrumbs)?,
			timestamp: parse_datetime(&row.timestamp)?,
			received_at: parse_datetime(&row.received_at)?,
		})
	}
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
	DateTime::parse_from_rfc3339(s)
		.map(|dt| dt.with_timezone(&Utc))
		.map_err(|_| CrashServerError::InvalidDateTime(s.to_string()))
}
