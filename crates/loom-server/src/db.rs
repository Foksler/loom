//! SQLite database operations for thread persistence.

use loom_thread::{Thread, ThreadId, ThreadSummary};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteSynchronous},
    Row,
};
use std::str::FromStr;

use crate::error::ServerError;

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

        sqlx::query(
            r#"
            INSERT INTO threads (
                id, version, created_at, updated_at, last_activity_at,
                workspace_root, cwd, loom_version, provider, model,
                title, tags, is_pinned, message_count,
                agent_state_kind, agent_state, conversation, metadata, full_json,
                visibility, is_shared_with_support
            ) VALUES (
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
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
                           created_at, updated_at, is_pinned, visibility
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
                           created_at, updated_at, is_pinned, visibility
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
}
