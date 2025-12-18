//! SQLite database operations for thread persistence.

use loom_google_cse::CseResponse;
use loom_thread::{Thread, ThreadId, ThreadSummary};
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteSynchronous},
    Row,
};
use std::str::FromStr;

use crate::error::ServerError;

/// GitHub App installation info stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubInstallation {
    pub installation_id: i64,
    pub account_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub app_slug: Option<String>,
    pub repositories_selection: String,
    pub suspended_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// GitHub repository linked to an installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubRepo {
    pub repository_id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub default_branch: Option<String>,
}

/// Installation info with minimal fields for lookups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubInstallationInfo {
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub repositories_selection: String,
}

/// A search result hit with relevance score
#[derive(Debug, Clone)]
pub struct ThreadSearchHit {
    pub summary: ThreadSummary,
    pub score: f64,
}

/// Get or create a repo entry in the repos table, returning its id.
async fn get_or_create_repo_id(pool: &SqlitePool, slug: &str) -> Result<Option<i64>, ServerError> {
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query("INSERT OR IGNORE INTO repos (slug, created_at) VALUES(?, ?)")
        .bind(slug)
        .bind(&now)
        .execute(pool)
        .await?;

    let result: Option<(i64,)> = sqlx::query_as("SELECT id FROM repos WHERE slug = ?")
        .bind(slug)
        .fetch_optional(pool)
        .await?;

    Ok(result.map(|(id,)| id))
}

/// Record commit SHAs associated with a thread in the thread_commits table.
async fn record_thread_commits(
    pool: &SqlitePool,
    thread: &Thread,
    repo_id: i64,
) -> Result<(), ServerError> {
    for sha in &thread.git_commits {
        let is_initial = Some(sha) == thread.git_initial_commit_sha.as_ref();
        let is_final = Some(sha) == thread.git_current_commit_sha.as_ref();

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO thread_commits (
                thread_id, repo_id, commit_sha, branch, is_dirty,
                observed_at, is_initial, is_final
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(thread.id.as_str())
        .bind(repo_id)
        .bind(sha)
        .bind(&thread.git_branch)
        .bind(thread.git_end_dirty.unwrap_or(false) as i32)
        .bind(&thread.updated_at)
        .bind(is_initial as i32)
        .bind(is_final as i32)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Repository for thread database operations.
#[derive(Clone)]
pub struct ThreadRepository {
    pool: SqlitePool,
}

impl ThreadRepository {
    /// Create a new repository with the given database URL.
    ///
    /// Configures SQLite with WAL mode for multi-reader, single-writer.
    pub async fn new(database_url: &str) -> Result<Self, ServerError> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| ServerError::Internal(format!("Invalid database URL: {}", e)))?
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migrations
        Self::run_migrations(&pool).await?;

        tracing::info!(database_url = %database_url, "database connection established");

        Ok(Self { pool })
    }

    /// Run database migrations.
    async fn run_migrations(pool: &SqlitePool) -> Result<(), ServerError> {
        let m1 = include_str!("../migrations/001_create_threads.sql");
        sqlx::query(m1).execute(pool).await?;

        let m2 = include_str!("../migrations/002_add_visibility.sql");
        if let Err(e) = sqlx::query(m2).execute(pool).await {
            let msg = e.to_string();
            // Gracefully handle if column already exists
            if !msg.contains("duplicate column name: visibility")
                && !msg.contains("duplicate column")
                && !msg.contains("already exists")
            {
                return Err(e.into());
            }
        }

        let m3 = include_str!("../migrations/003_add_git_metadata.sql");
        if let Err(e) = sqlx::query(m3).execute(pool).await {
            let msg = e.to_string();
            if !msg.contains("duplicate column") && !msg.contains("already exists") {
                return Err(e.into());
            }
        }

        // Migration 004: repos table and thread_commits
        let m4 = include_str!("../migrations/004_git_repos_and_commits.sql");
        // Split by semicolons and execute each statement
        for stmt in m4.split(';').filter(|s| !s.trim().is_empty()) {
            if let Err(e) = sqlx::query(stmt).execute(pool).await {
                let msg = e.to_string();
                if !msg.contains("duplicate column")
                    && !msg.contains("already exists")
                    && !msg.contains("table repos already exists")
                    && !msg.contains("table thread_commits already exists")
                {
                    return Err(e.into());
                }
            }
        }

        // Migration 005: FTS5 search
        // Parse statements carefully: split CREATE VIRTUAL TABLE from triggers
        let m5 = include_str!("../migrations/005_thread_fts.sql");

        // Find the CREATE VIRTUAL TABLE statement (ends with ");")
        if let Some(vt_end) = m5.find(");") {
            let create_vt = &m5[..vt_end + 2];
            if let Err(e) = sqlx::query(create_vt.trim()).execute(pool).await {
                let msg = e.to_string();
                if !msg.contains("already exists")
                    && !msg.contains("table thread_fts already exists")
                {
                    tracing::warn!(error = %e, "FTS CREATE VIRTUAL TABLE failed");
                }
            }

            // Parse triggers (split remaining text by "END;")
            let remaining = &m5[vt_end + 2..];
            for trigger_block in remaining.split("END;") {
                let trigger = trigger_block.trim();
                if trigger.is_empty() || !trigger.contains("CREATE TRIGGER") {
                    continue;
                }
                let full_trigger = format!("{} END;", trigger);
                if let Err(e) = sqlx::query(&full_trigger).execute(pool).await {
                    let msg = e.to_string();
                    if !msg.contains("already exists") && !msg.contains("trigger") {
                        tracing::warn!(error = %e, stmt = %full_trigger.chars().take(80).collect::<String>(), "FTS trigger creation failed");
                    }
                }
            }
        }

        // Migration 006: CSE cache table
        let m6 = include_str!("../migrations/006_cse_cache.sql");
        for stmt in m6.split(';').filter(|s| !s.trim().is_empty()) {
            if let Err(e) = sqlx::query(stmt).execute(pool).await {
                let msg = e.to_string();
                if !msg.contains("already exists") && !msg.contains("duplicate column") {
                    return Err(e.into());
                }
            }
        }

        // Migration 007: GitHub App tables
        let m7 = include_str!("../migrations/007_github_app.sql");
        for stmt in m7.split(';').filter(|s| !s.trim().is_empty()) {
            if let Err(e) = sqlx::query(stmt).execute(pool).await {
                let msg = e.to_string();
                if !msg.contains("already exists")
                    && !msg.contains("duplicate column")
                    && !msg.contains("table github_installations already exists")
                    && !msg.contains("table github_installation_repos already exists")
                {
                    return Err(e.into());
                }
            }
        }

        tracing::debug!("database migrations complete");
        Ok(())
    }

    /// Upsert a thread with optional version checking.
    ///
    /// If `expected_version` is Some, the update will fail with Conflict if
    /// the stored version doesn't match.
    pub async fn upsert(
        &self,
        thread: &Thread,
        expected_version: Option<u64>,
    ) -> Result<Thread, ServerError> {
        // Check if thread exists
        let existing = self.get(&thread.id).await?;

        if let Some(existing_thread) = existing {
            // Update existing thread
            if let Some(expected) = expected_version {
                if existing_thread.version != expected {
                    return Err(ServerError::Conflict {
                        expected: existing_thread.version,
                        actual: expected,
                    });
                }
            }

            self.update(thread).await?;
        } else {
            // Insert new thread
            self.insert(thread).await?;
        }

        // Return the stored thread
        self.get(&thread.id)
            .await?
            .ok_or_else(|| ServerError::Internal("Thread not found after upsert".to_string()))
    }

    /// Insert a new thread.
    async fn insert(&self, thread: &Thread) -> Result<(), ServerError> {
        let full_json = serde_json::to_string(thread)?;
        let tags_json = serde_json::to_string(&thread.metadata.tags)?;
        let agent_state_json = serde_json::to_string(&thread.agent_state)?;
        let conversation_json = serde_json::to_string(&thread.conversation)?;
        let metadata_json = serde_json::to_string(&thread.metadata)?;

        let repo_id = if let Some(ref slug) = thread.git_remote_url {
            get_or_create_repo_id(&self.pool, slug).await?
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO threads (
                id, version, created_at, updated_at, last_activity_at,
                workspace_root, cwd, loom_version, provider, model,
                git_branch, git_remote_url, repo_id,
                git_initial_branch, git_initial_commit_sha, git_current_commit_sha,
                git_start_dirty, git_end_dirty,
                title, tags, is_pinned, message_count,
                agent_state_kind, agent_state, conversation, metadata, full_json,
                visibility, is_shared_with_support
            ) VALUES (
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?
            )
            "#,
        )
        .bind(thread.id.as_str())
        .bind(thread.version as i64)
        .bind(&thread.created_at)
        .bind(&thread.updated_at)
        .bind(&thread.last_activity_at)
        .bind(&thread.workspace_root)
        .bind(&thread.cwd)
        .bind(&thread.loom_version)
        .bind(&thread.provider)
        .bind(&thread.model)
        .bind(&thread.git_branch)
        .bind(&thread.git_remote_url)
        .bind(repo_id)
        .bind(&thread.git_initial_branch)
        .bind(&thread.git_initial_commit_sha)
        .bind(&thread.git_current_commit_sha)
        .bind(thread.git_start_dirty.map(|d| d as i32))
        .bind(thread.git_end_dirty.map(|d| d as i32))
        .bind(&thread.metadata.title)
        .bind(&tags_json)
        .bind(thread.metadata.is_pinned as i32)
        .bind(thread.conversation.messages.len() as i32)
        .bind(thread.agent_state.kind.as_str())
        .bind(&agent_state_json)
        .bind(&conversation_json)
        .bind(&metadata_json)
        .bind(&full_json)
        .bind(thread.visibility.as_str())
        .bind(thread.is_shared_with_support as i32)
        .execute(&self.pool)
        .await?;

        if let Some(repo_id) = repo_id {
            record_thread_commits(&self.pool, thread, repo_id).await?;
        }

        tracing::debug!(thread_id = %thread.id, version = thread.version, "thread inserted");

        Ok(())
    }

    /// Update an existing thread.
    async fn update(&self, thread: &Thread) -> Result<(), ServerError> {
        let full_json = serde_json::to_string(thread)?;
        let tags_json = serde_json::to_string(&thread.metadata.tags)?;
        let agent_state_json = serde_json::to_string(&thread.agent_state)?;
        let conversation_json = serde_json::to_string(&thread.conversation)?;
        let metadata_json = serde_json::to_string(&thread.metadata)?;

        let repo_id = if let Some(ref slug) = thread.git_remote_url {
            get_or_create_repo_id(&self.pool, slug).await?
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE threads SET
                version = ?,
                updated_at = ?,
                last_activity_at = ?,
                workspace_root = ?,
                cwd = ?,
                loom_version = ?,
                provider = ?,
                model = ?,
                git_branch = ?,
                git_remote_url = ?,
                repo_id = ?,
                git_initial_branch = ?,
                git_initial_commit_sha = ?,
                git_current_commit_sha = ?,
                git_start_dirty = ?,
                git_end_dirty = ?,
                title = ?,
                tags = ?,
                is_pinned = ?,
                message_count = ?,
                agent_state_kind = ?,
                agent_state = ?,
                conversation = ?,
                metadata = ?,
                full_json = ?,
                visibility = ?,
                is_shared_with_support = ?
            WHERE id = ?
            "#,
        )
        .bind(thread.version as i64)
        .bind(&thread.updated_at)
        .bind(&thread.last_activity_at)
        .bind(&thread.workspace_root)
        .bind(&thread.cwd)
        .bind(&thread.loom_version)
        .bind(&thread.provider)
        .bind(&thread.model)
        .bind(&thread.git_branch)
        .bind(&thread.git_remote_url)
        .bind(repo_id)
        .bind(&thread.git_initial_branch)
        .bind(&thread.git_initial_commit_sha)
        .bind(&thread.git_current_commit_sha)
        .bind(thread.git_start_dirty.map(|d| d as i32))
        .bind(thread.git_end_dirty.map(|d| d as i32))
        .bind(&thread.metadata.title)
        .bind(&tags_json)
        .bind(thread.metadata.is_pinned as i32)
        .bind(thread.conversation.messages.len() as i32)
        .bind(thread.agent_state.kind.as_str())
        .bind(&agent_state_json)
        .bind(&conversation_json)
        .bind(&metadata_json)
        .bind(&full_json)
        .bind(thread.visibility.as_str())
        .bind(thread.is_shared_with_support as i32)
        .bind(thread.id.as_str())
        .execute(&self.pool)
        .await?;

        if let Some(repo_id) = repo_id {
            record_thread_commits(&self.pool, thread, repo_id).await?;
        }

        tracing::debug!(thread_id = %thread.id, version = thread.version, "thread updated");

        Ok(())
    }

    /// Get a thread by ID.
    pub async fn get(&self, id: &ThreadId) -> Result<Option<Thread>, ServerError> {
        let row = sqlx::query(
            r#"
            SELECT full_json
            FROM threads
            WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let full_json: String = row.get("full_json");
                let thread: Thread = serde_json::from_str(&full_json)?;
                Ok(Some(thread))
            }
            None => Ok(None),
        }
    }

    /// List threads with optional workspace filter.
    pub async fn list(
        &self,
        workspace: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ThreadSummary>, ServerError> {
        let rows = match workspace {
            Some(ws) => {
                sqlx::query(
                    r#"
                    SELECT id, title, workspace_root, last_activity_at,
                           provider, model, tags, version, message_count,
                           created_at, updated_at, is_pinned, visibility,
                           git_branch, git_remote_url,
                           git_initial_commit_sha, git_current_commit_sha
                    FROM threads
                    WHERE deleted_at IS NULL AND workspace_root = ?
                    ORDER BY last_activity_at DESC
                    LIMIT ? OFFSET ?
                    "#,
                )
                .bind(ws)
                .bind(limit as i32)
                .bind(offset as i32)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    r#"
                    SELECT id, title, workspace_root, last_activity_at,
                           provider, model, tags, version, message_count,
                           created_at, updated_at, is_pinned, visibility,
                           git_branch, git_remote_url,
                           git_initial_commit_sha, git_current_commit_sha
                    FROM threads
                    WHERE deleted_at IS NULL
                    ORDER BY last_activity_at DESC
                    LIMIT ? OFFSET ?
                    "#,
                )
                .bind(limit as i32)
                .bind(offset as i32)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let summaries = rows
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let title: Option<String> = row.get("title");
                let workspace_root: Option<String> = row.get("workspace_root");
                let last_activity_at: String = row.get("last_activity_at");
                let provider: Option<String> = row.get("provider");
                let model: Option<String> = row.get("model");
                let tags_json: String = row.get("tags");
                let version: i64 = row.get("version");
                let message_count: i32 = row.get("message_count");
                let created_at: String = row.get("created_at");
                let updated_at: String = row.get("updated_at");
                let is_pinned: i32 = row.get("is_pinned");
                let visibility_str: String = row.get("visibility");
                let git_branch: Option<String> = row.get("git_branch");
                let git_remote_url: Option<String> = row.get("git_remote_url");
                let git_initial_commit_sha: Option<String> = row.get("git_initial_commit_sha");
                let git_current_commit_sha: Option<String> = row.get("git_current_commit_sha");

                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                let visibility = visibility_str
                    .parse()
                    .unwrap_or(loom_thread::ThreadVisibility::Private);

                ThreadSummary {
                    id: ThreadId::from_string(id),
                    version: version as u64,
                    created_at,
                    updated_at,
                    last_activity_at,
                    title,
                    workspace_root,
                    git_branch,
                    git_remote_url,
                    git_initial_commit_sha,
                    git_current_commit_sha,
                    provider,
                    model,
                    tags,
                    message_count: message_count as usize,
                    is_pinned: is_pinned != 0,
                    visibility,
                }
            })
            .collect();

        Ok(summaries)
    }

    /// Soft-delete a thread.
    pub async fn delete(&self, id: &ThreadId) -> Result<bool, ServerError> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE threads
            SET deleted_at = ?
            WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(&now)
        .bind(id.as_str())
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected() > 0;

        if deleted {
            tracing::debug!(thread_id = %id, "thread soft-deleted");
        }

        Ok(deleted)
    }

    /// Lightweight database health check (used by /health endpoint).
    pub async fn health_check(&self) -> Result<(), ServerError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    /// Count total threads (excluding deleted).
    pub async fn count(&self, workspace: Option<&str>) -> Result<u64, ServerError> {
        let count: (i64,) = match workspace {
            Some(ws) => {
                sqlx::query_as(
                    r#"
                    SELECT COUNT(*) FROM threads
                    WHERE deleted_at IS NULL AND workspace_root = ?
                    "#,
                )
                .bind(ws)
                .fetch_one(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as(
                    r#"
                    SELECT COUNT(*) FROM threads
                    WHERE deleted_at IS NULL
                    "#,
                )
                .fetch_one(&self.pool)
                .await?
            }
        };

        Ok(count.0 as u64)
    }

    /// Search threads by query string.
    ///
    /// Detects SHA-like queries and searches commit prefixes first,
    /// otherwise falls back to FTS5 full-text search.
    pub async fn search(
        &self,
        query: &str,
        workspace: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ThreadSearchHit>, ServerError> {
        let query = query.trim();

        // SHA-like heuristic: hex chars only, 7-40 length, no spaces
        let is_sha_like = query.len() >= 7
            && query.len() <= 40
            && !query.contains(char::is_whitespace)
            && query.chars().all(|c| c.is_ascii_hexdigit());

        if is_sha_like {
            let hits = self
                .search_by_commit_prefix(query, workspace, limit, offset)
                .await?;
            if !hits.is_empty() {
                return Ok(hits);
            }
        }

        self.search_fts(query, workspace, limit, offset).await
    }

    async fn search_by_commit_prefix(
        &self,
        prefix: &str,
        workspace: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ThreadSearchHit>, ServerError> {
        let like_pattern = format!("{}%", prefix);

        let sql = if workspace.is_some() {
            r#"
            SELECT DISTINCT
                t.id, t.version, t.created_at, t.updated_at, t.last_activity_at,
                t.title, t.workspace_root, t.git_branch, t.git_remote_url,
                t.git_initial_commit_sha, t.git_current_commit_sha,
                t.provider, t.model, t.tags, t.message_count, t.is_pinned, t.visibility
            FROM thread_commits c
            JOIN threads t ON t.id = c.thread_id
            WHERE c.commit_sha LIKE ?1
              AND t.deleted_at IS NULL
              AND t.workspace_root = ?2
            ORDER BY t.last_activity_at DESC
            LIMIT ?3 OFFSET ?4
            "#
        } else {
            r#"
            SELECT DISTINCT
                t.id, t.version, t.created_at, t.updated_at, t.last_activity_at,
                t.title, t.workspace_root, t.git_branch, t.git_remote_url,
                t.git_initial_commit_sha, t.git_current_commit_sha,
                t.provider, t.model, t.tags, t.message_count, t.is_pinned, t.visibility
            FROM thread_commits c
            JOIN threads t ON t.id = c.thread_id
            WHERE c.commit_sha LIKE ?1
              AND t.deleted_at IS NULL
            ORDER BY t.last_activity_at DESC
            LIMIT ?2 OFFSET ?3
            "#
        };

        let rows = if let Some(ws) = workspace {
            sqlx::query(sql)
                .bind(&like_pattern)
                .bind(ws)
                .bind(limit as i32)
                .bind(offset as i32)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(sql)
                .bind(&like_pattern)
                .bind(limit as i32)
                .bind(offset as i32)
                .fetch_all(&self.pool)
                .await?
        };

        self.rows_to_search_hits(rows, 0.0)
    }

    async fn search_fts(
        &self,
        query: &str,
        workspace: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ThreadSearchHit>, ServerError> {
        // Escape quotes and wrap in phrase for safety
        let fts_query = format!("\"{}\"", query.replace('"', " "));

        let sql = if workspace.is_some() {
            r#"
            SELECT
                t.id, t.version, t.created_at, t.updated_at, t.last_activity_at,
                t.title, t.workspace_root, t.git_branch, t.git_remote_url,
                t.git_initial_commit_sha, t.git_current_commit_sha,
                t.provider, t.model, t.tags, t.message_count, t.is_pinned, t.visibility,
                bm25(thread_fts) AS score
            FROM thread_fts
            JOIN threads t ON t.id = thread_fts.thread_id
            WHERE thread_fts MATCH ?1
              AND t.deleted_at IS NULL
              AND t.workspace_root = ?2
            ORDER BY score ASC, t.last_activity_at DESC
            LIMIT ?3 OFFSET ?4
            "#
        } else {
            r#"
            SELECT
                t.id, t.version, t.created_at, t.updated_at, t.last_activity_at,
                t.title, t.workspace_root, t.git_branch, t.git_remote_url,
                t.git_initial_commit_sha, t.git_current_commit_sha,
                t.provider, t.model, t.tags, t.message_count, t.is_pinned, t.visibility,
                bm25(thread_fts) AS score
            FROM thread_fts
            JOIN threads t ON t.id = thread_fts.thread_id
            WHERE thread_fts MATCH ?1
              AND t.deleted_at IS NULL
            ORDER BY score ASC, t.last_activity_at DESC
            LIMIT ?2 OFFSET ?3
            "#
        };

        let rows = if let Some(ws) = workspace {
            sqlx::query(sql)
                .bind(&fts_query)
                .bind(ws)
                .bind(limit as i32)
                .bind(offset as i32)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(sql)
                .bind(&fts_query)
                .bind(limit as i32)
                .bind(offset as i32)
                .fetch_all(&self.pool)
                .await?
        };

        let mut hits = Vec::new();
        for row in rows {
            let summary = self.row_to_summary(&row)?;
            let score: f64 = row.try_get("score").unwrap_or(0.0);
            hits.push(ThreadSearchHit { summary, score });
        }
        Ok(hits)
    }

    fn rows_to_search_hits(
        &self,
        rows: Vec<sqlx::sqlite::SqliteRow>,
        default_score: f64,
    ) -> Result<Vec<ThreadSearchHit>, ServerError> {
        let mut hits = Vec::new();
        for row in rows {
            let summary = self.row_to_summary(&row)?;
            hits.push(ThreadSearchHit {
                summary,
                score: default_score,
            });
        }
        Ok(hits)
    }

    fn row_to_summary(&self, row: &sqlx::sqlite::SqliteRow) -> Result<ThreadSummary, ServerError> {
        let id: String = row.get("id");
        let title: Option<String> = row.get("title");
        let workspace_root: Option<String> = row.get("workspace_root");
        let last_activity_at: String = row.get("last_activity_at");
        let provider: Option<String> = row.get("provider");
        let model: Option<String> = row.get("model");
        let tags_json: String = row.get("tags");
        let version: i64 = row.get("version");
        let message_count: i32 = row.get("message_count");
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");
        let is_pinned: i32 = row.get("is_pinned");
        let visibility_str: String = row.get("visibility");
        let git_branch: Option<String> = row.get("git_branch");
        let git_remote_url: Option<String> = row.get("git_remote_url");
        let git_initial_commit_sha: Option<String> = row.get("git_initial_commit_sha");
        let git_current_commit_sha: Option<String> = row.get("git_current_commit_sha");

        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let visibility = visibility_str
            .parse()
            .unwrap_or(loom_thread::ThreadVisibility::Private);

        Ok(ThreadSummary {
            id: ThreadId::from_string(id),
            version: version as u64,
            created_at,
            updated_at,
            last_activity_at,
            title,
            workspace_root,
            git_branch,
            git_remote_url,
            git_initial_commit_sha,
            git_current_commit_sha,
            provider,
            model,
            tags,
            message_count: message_count as usize,
            is_pinned: is_pinned != 0,
            visibility,
        })
    }

    // ========== CSE Cache Methods ==========

    /// Normalizes a query string for cache key purposes.
    /// - Converts to lowercase
    /// - Collapses multiple whitespace into single spaces
    /// - Trims leading/trailing whitespace
    fn normalize_cache_query(query: &str) -> String {
        query
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    }

    /// Get cached CSE response if it exists and is not expired (24h TTL).
    pub async fn get_cse_cache(
        &self,
        query: &str,
        max_results: u32,
    ) -> Result<Option<CseResponse>, ServerError> {
        use chrono::{Duration, Utc};

        let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();

        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT response_json
            FROM cse_cache
            WHERE query = ?1
              AND max_results = ?2
              AND created_at >= ?3
            LIMIT 1
            "#,
        )
        .bind(Self::normalize_cache_query(query))
        .bind(max_results as i64)
        .bind(&cutoff)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((json,)) => {
                let response: CseResponse = serde_json::from_str(&json)?;
                tracing::debug!(
                    query = %query,
                    max_results = max_results,
                    "cse_cache: hit"
                );
                Ok(Some(response))
            }
            None => {
                tracing::debug!(
                    query = %query,
                    max_results = max_results,
                    "cse_cache: miss"
                );
                Ok(None)
            }
        }
    }

    /// Store CSE response in cache, upserting on conflict.
    /// Also performs opportunistic cleanup of expired entries.
    pub async fn put_cse_cache(
        &self,
        response: &CseResponse,
        max_results: u32,
    ) -> Result<(), ServerError> {
        use chrono::{Duration, Utc};

        let now = Utc::now().to_rfc3339();
        let json = serde_json::to_string(response)?;

        // Upsert: insert or update existing entry
        sqlx::query(
            r#"
            INSERT INTO cse_cache (query, max_results, response_json, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(query, max_results) DO UPDATE SET
                response_json = excluded.response_json,
                created_at    = excluded.created_at
            "#,
        )
        .bind(Self::normalize_cache_query(&response.query))
        .bind(max_results as i64)
        .bind(&json)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        tracing::debug!(
            query = %response.query,
            max_results = max_results,
            "cse_cache: stored"
        );

        // Opportunistic cleanup of expired entries (older than 24h)
        let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();
        let result = sqlx::query(
            r#"
            DELETE FROM cse_cache
            WHERE created_at < ?1
            "#,
        )
        .bind(&cutoff)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            tracing::debug!(
                deleted = result.rows_affected(),
                "cse_cache: cleaned up expired entries"
            );
        }

        Ok(())
    }

    // ========== GitHub App Methods ==========

    /// Upsert a GitHub installation from webhook data.
    pub async fn upsert_github_installation(
        &self,
        installation: &GithubInstallation,
    ) -> Result<(), ServerError> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO github_installations (
                installation_id, account_id, account_login, account_type,
                app_slug, repositories_selection, suspended_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(installation_id) DO UPDATE SET
                account_id = excluded.account_id,
                account_login = excluded.account_login,
                account_type = excluded.account_type,
                app_slug = excluded.app_slug,
                repositories_selection = excluded.repositories_selection,
                suspended_at = excluded.suspended_at,
                updated_at = ?9
            "#,
        )
        .bind(installation.installation_id)
        .bind(installation.account_id)
        .bind(&installation.account_login)
        .bind(&installation.account_type)
        .bind(&installation.app_slug)
        .bind(&installation.repositories_selection)
        .bind(&installation.suspended_at)
        .bind(&installation.created_at)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            installation_id = installation.installation_id,
            account_login = %installation.account_login,
            "github_installation: upserted"
        );

        Ok(())
    }

    /// Delete a GitHub installation (cascades to repos).
    pub async fn delete_github_installation(
        &self,
        installation_id: i64,
    ) -> Result<bool, ServerError> {
        let result = sqlx::query("DELETE FROM github_installations WHERE installation_id = ?")
            .bind(installation_id)
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected() > 0;

        if deleted {
            tracing::info!(
                installation_id = installation_id,
                "github_installation: deleted"
            );
        }

        Ok(deleted)
    }

    /// Suspend or unsuspend an installation.
    pub async fn update_github_installation_suspension(
        &self,
        installation_id: i64,
        suspended_at: Option<&str>,
    ) -> Result<bool, ServerError> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE github_installations
            SET suspended_at = ?1, updated_at = ?2
            WHERE installation_id = ?3
            "#,
        )
        .bind(suspended_at)
        .bind(&now)
        .bind(installation_id)
        .execute(&self.pool)
        .await?;

        let updated = result.rows_affected() > 0;

        if updated {
            tracing::info!(
                installation_id = installation_id,
                suspended = suspended_at.is_some(),
                "github_installation: suspension updated"
            );
        }

        Ok(updated)
    }

    /// Add repositories to an installation.
    pub async fn add_github_installation_repos(
        &self,
        installation_id: i64,
        repos: &[GithubRepo],
    ) -> Result<(), ServerError> {
        let now = chrono::Utc::now().to_rfc3339();

        for repo in repos {
            sqlx::query(
                r#"
                INSERT INTO github_installation_repos (
                    repository_id, installation_id, owner, name, full_name,
                    private, default_branch, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(repository_id) DO UPDATE SET
                    installation_id = excluded.installation_id,
                    owner = excluded.owner,
                    name = excluded.name,
                    full_name = excluded.full_name,
                    private = excluded.private,
                    default_branch = excluded.default_branch,
                    updated_at = ?9
                "#,
            )
            .bind(repo.repository_id)
            .bind(installation_id)
            .bind(&repo.owner)
            .bind(&repo.name)
            .bind(&repo.full_name)
            .bind(repo.private as i32)
            .bind(&repo.default_branch)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }

        tracing::info!(
            installation_id = installation_id,
            repo_count = repos.len(),
            "github_installation_repos: added"
        );

        Ok(())
    }

    /// Remove repositories from installations by repository ID.
    pub async fn remove_github_installation_repos(
        &self,
        repository_ids: &[i64],
    ) -> Result<(), ServerError> {
        for repo_id in repository_ids {
            sqlx::query("DELETE FROM github_installation_repos WHERE repository_id = ?")
                .bind(repo_id)
                .execute(&self.pool)
                .await?;
        }

        tracing::info!(
            repo_count = repository_ids.len(),
            "github_installation_repos: removed"
        );

        Ok(())
    }

    /// Get installation ID for a repository by owner/name.
    pub async fn get_github_installation_for_repo(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<Option<GithubInstallationInfo>, ServerError> {
        let row: Option<(i64, String, String, String)> = sqlx::query_as(
            r#"
            SELECT
                gi.installation_id,
                gi.account_login,
                gi.account_type,
                gi.repositories_selection
            FROM github_installation_repos gir
            JOIN github_installations gi ON gi.installation_id = gir.installation_id
            WHERE gir.owner = ?1 AND gir.name = ?2
              AND gi.suspended_at IS NULL
            "#,
        )
        .bind(owner)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((installation_id, account_login, account_type, repositories_selection)) => {
                tracing::debug!(
                    owner = %owner,
                    name = %name,
                    installation_id = installation_id,
                    "github_installation_for_repo: found"
                );
                Ok(Some(GithubInstallationInfo {
                    installation_id,
                    account_login,
                    account_type,
                    repositories_selection,
                }))
            }
            None => {
                tracing::debug!(
                    owner = %owner,
                    name = %name,
                    "github_installation_for_repo: not found"
                );
                Ok(None)
            }
        }
    }

    /// List all installations.
    #[allow(clippy::type_complexity)]
    pub async fn list_github_installations(&self) -> Result<Vec<GithubInstallation>, ServerError> {
        let rows: Vec<(
            i64,
            i64,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            String,
            String,
        )> = sqlx::query_as(
            r#"
                SELECT
                    installation_id, account_id, account_login, account_type,
                    app_slug, repositories_selection, suspended_at, created_at, updated_at
                FROM github_installations
                ORDER BY account_login
                "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let installations = rows
            .into_iter()
            .map(
                |(
                    installation_id,
                    account_id,
                    account_login,
                    account_type,
                    app_slug,
                    repositories_selection,
                    suspended_at,
                    created_at,
                    updated_at,
                )| GithubInstallation {
                    installation_id,
                    account_id,
                    account_login,
                    account_type,
                    app_slug,
                    repositories_selection,
                    suspended_at,
                    created_at,
                    updated_at,
                },
            )
            .collect();

        Ok(installations)
    }
}

// Helper trait for AgentStateKind
trait AgentStateKindExt {
    fn as_str(&self) -> &'static str;
}

impl AgentStateKindExt for loom_thread::AgentStateKind {
    fn as_str(&self) -> &'static str {
        match self {
            loom_thread::AgentStateKind::WaitingForUserInput => "waiting_for_user_input",
            loom_thread::AgentStateKind::CallingLlm => "calling_llm",
            loom_thread::AgentStateKind::ProcessingLlmResponse => "processing_llm_response",
            loom_thread::AgentStateKind::ExecutingTools => "executing_tools",
            loom_thread::AgentStateKind::PostToolsHook => "post_tools_hook",
            loom_thread::AgentStateKind::Error => "error",
            loom_thread::AgentStateKind::ShuttingDown => "shutting_down",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_thread::{
        AgentStateKind, AgentStateSnapshot, ConversationSnapshot, ThreadMetadata, ThreadVisibility,
    };
    use tempfile::tempdir;

    async fn create_test_repo() -> (ThreadRepository, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        let repo = ThreadRepository::new(&db_url).await.unwrap();
        (repo, dir) // Return dir to keep it alive
    }

    fn create_test_thread() -> Thread {
        Thread {
            id: ThreadId::new(),
            version: 1,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            last_activity_at: chrono::Utc::now().to_rfc3339(),
            workspace_root: Some("/test/workspace".to_string()),
            cwd: Some("/test/workspace".to_string()),
            loom_version: Some("0.1.0".to_string()),
            provider: Some("anthropic".to_string()),
            model: Some("claude-sonnet-4-20250514".to_string()),
            git_branch: Some("main".to_string()),
            git_remote_url: Some("github.com/test/repo".to_string()),
            git_initial_branch: Some("main".to_string()),
            git_initial_commit_sha: Some("abc123def456".to_string()),
            git_current_commit_sha: Some("xyz789012345".to_string()),
            git_start_dirty: Some(false),
            git_end_dirty: Some(false),
            git_commits: vec!["abc123def456".to_string(), "xyz789012345".to_string()],
            conversation: ConversationSnapshot { messages: vec![] },
            agent_state: AgentStateSnapshot {
                kind: AgentStateKind::WaitingForUserInput,
                retries: 0,
                last_error: None,
                pending_tool_calls: vec![],
            },
            metadata: ThreadMetadata::default(),
            visibility: ThreadVisibility::Private,
            is_private: false,
            is_shared_with_support: false,
        }
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let (repo, _dir) = create_test_repo().await;
        let thread = create_test_thread();

        repo.insert(&thread).await.unwrap();

        let loaded = repo.get(&thread.id).await.unwrap().unwrap();
        assert_eq!(loaded.id.as_str(), thread.id.as_str());
        assert_eq!(loaded.version, thread.version);
    }

    #[tokio::test]
    async fn test_upsert_conflict() {
        let (repo, _dir) = create_test_repo().await;
        let thread = create_test_thread();

        // Insert initial thread
        repo.upsert(&thread, None).await.unwrap();

        // Try to update with wrong version
        let mut updated = thread.clone();
        updated.version = 2;

        let result = repo.upsert(&updated, Some(0)).await;
        assert!(matches!(result, Err(ServerError::Conflict { .. })));
    }

    #[tokio::test]
    async fn test_soft_delete() {
        let (repo, _dir) = create_test_repo().await;
        let thread = create_test_thread();

        repo.insert(&thread).await.unwrap();

        // Verify thread exists
        assert!(repo.get(&thread.id).await.unwrap().is_some());

        // Delete
        let deleted = repo.delete(&thread.id).await.unwrap();
        assert!(deleted);

        // Verify thread is gone (soft-deleted)
        assert!(repo.get(&thread.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_threads() {
        let (repo, _dir) = create_test_repo().await;

        // Insert multiple threads
        for _ in 0..5 {
            let thread = create_test_thread();
            repo.insert(&thread).await.unwrap();
        }

        let list = repo.list(None, 10, 0).await.unwrap();
        assert_eq!(list.len(), 5);
    }

    #[tokio::test]
    async fn test_git_metadata_roundtrip() {
        let (repo, _dir) = create_test_repo().await;

        let mut thread = create_test_thread();
        thread.git_branch = Some("feature/test".to_string());
        thread.git_remote_url = Some("github.com/alice/project".to_string());

        repo.insert(&thread).await.unwrap();

        let loaded = repo.get(&thread.id).await.unwrap().unwrap();
        assert_eq!(loaded.git_branch, Some("feature/test".to_string()));
        assert_eq!(
            loaded.git_remote_url,
            Some("github.com/alice/project".to_string())
        );
    }

    #[tokio::test]
    async fn test_thread_commits_populated() {
        let (repo, _dir) = create_test_repo().await;

        let mut thread = create_test_thread();
        thread.git_commits = vec!["commit1".to_string(), "commit2".to_string()];
        thread.git_initial_commit_sha = Some("commit1".to_string());
        thread.git_current_commit_sha = Some("commit2".to_string());

        repo.insert(&thread).await.unwrap();

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM thread_commits WHERE thread_id = ?")
                .bind(thread.id.as_str())
                .fetch_one(&repo.pool)
                .await
                .unwrap();
        assert_eq!(
            count.0, 2,
            "Expected 2 commits to be recorded in thread_commits"
        );

        let initial: (i32,) = sqlx::query_as(
            "SELECT is_initial FROM thread_commits WHERE thread_id = ? AND commit_sha = ?",
        )
        .bind(thread.id.as_str())
        .bind("commit1")
        .fetch_one(&repo.pool)
        .await
        .unwrap();
        assert_eq!(initial.0, 1, "commit1 should be marked as initial");

        let final_commit: (i32,) = sqlx::query_as(
            "SELECT is_final FROM thread_commits WHERE thread_id = ? AND commit_sha = ?",
        )
        .bind(thread.id.as_str())
        .bind("commit2")
        .fetch_one(&repo.pool)
        .await
        .unwrap();
        assert_eq!(final_commit.0, 1, "commit2 should be marked as final");
    }

    #[tokio::test]
    async fn test_search_by_commit_sha() {
        let (repo, _dir) = create_test_repo().await;

        let mut thread = create_test_thread();
        thread.git_commits = vec!["abc123def456789012345678901234567890abcd".to_string()];
        repo.insert(&thread).await.unwrap();

        let hits = repo.search("abc123def", None, 10, 0).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].summary.id.as_str(), thread.id.as_str());
    }

    #[test]
    fn test_normalize_cache_query() {
        assert_eq!(
            ThreadRepository::normalize_cache_query("Hello World"),
            "hello world"
        );
        assert_eq!(
            ThreadRepository::normalize_cache_query("  multiple   spaces  "),
            "multiple spaces"
        );
        assert_eq!(
            ThreadRepository::normalize_cache_query("UPPERCASE"),
            "uppercase"
        );
        assert_eq!(
            ThreadRepository::normalize_cache_query("  trim  me  "),
            "trim me"
        );
        assert_eq!(
            ThreadRepository::normalize_cache_query("already normalized"),
            "already normalized"
        );
    }

    #[tokio::test]
    async fn test_search_by_branch() {
        let (repo, _dir) = create_test_repo().await;

        let mut thread = create_test_thread();
        thread.git_branch = Some("feature/unique-test-branch".to_string());
        repo.insert(&thread).await.unwrap();

        // Search by branch name
        let hits = repo
            .search("unique-test-branch", None, 10, 0)
            .await
            .unwrap();
        assert!(hits.len() >= 1, "Expected at least 1 hit for branch search");
    }
}
