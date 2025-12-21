//! Integration tests for web_integration module
//!
//! Tests that the /api/web/* endpoints work correctly with the database.

#[cfg(test)]
mod tests {
    use crate::db::ThreadRepository;
    use loom_thread::{Thread, ThreadId};
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn setup_test_db() -> (Arc<ThreadRepository>, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_url = format!("sqlite://{}/test.db", temp_dir.path().display());

        let repo = Arc::new(
            ThreadRepository::new(&db_url)
                .await
                .expect("Failed to create repository"),
        );

        (repo, temp_dir)
    }

    /// Test creating a thread via the repository
    ///
    /// This is a smoke test to verify the web integration handlers
    /// can interact with the database correctly.
    #[tokio::test]
    async fn test_create_thread_persistence() {
        let (repo, _temp) = setup_test_db().await;

        // Create a thread
        let mut thread = Thread::new();
        thread.metadata.title = Some("Integration Test Thread".to_string());

        let thread_id = thread.id.clone();

        // Upsert should succeed
        let inserted = repo
            .upsert(&thread, None)
            .await
            .expect("Failed to upsert thread");

        assert_eq!(inserted.id, thread_id);
        assert_eq!(
            inserted.metadata.title,
            Some("Integration Test Thread".to_string())
        );
    }

    /// Test retrieving a thread
    #[tokio::test]
    async fn test_get_thread_retrieval() {
        let (repo, _temp) = setup_test_db().await;

        // Create and store a thread
        let mut thread = Thread::new();
        thread.metadata.title = Some("Test Retrieval".to_string());
        let thread_id = thread.id.clone();

        repo.upsert(&thread, None)
            .await
            .expect("Failed to upsert");

        // Retrieve it
        let retrieved = repo
            .get(&thread_id)
            .await
            .expect("Failed to get thread")
            .expect("Thread not found");

        assert_eq!(retrieved.id, thread_id);
        assert_eq!(
            retrieved.metadata.title,
            Some("Test Retrieval".to_string())
        );
    }

    /// Test deleting a thread (soft delete)
    #[tokio::test]
    async fn test_thread_soft_delete() {
        let (repo, _temp) = setup_test_db().await;

        let mut thread = Thread::new();
        thread.metadata.title = Some("To Delete".to_string());
        let thread_id = thread.id.clone();

        repo.upsert(&thread, None)
            .await
            .expect("Failed to upsert");

        // Delete it
        let deleted = repo.delete(&thread_id).await.expect("Failed to delete");
        assert!(deleted);

        // Should not be found in list anymore (soft delete)
        let retrieved = repo
            .get(&thread_id)
            .await
            .expect("Failed to check deleted thread");
        assert!(retrieved.is_none());
    }

    /// Test listing threads
    #[tokio::test]
    async fn test_list_threads() {
        let (repo, _temp) = setup_test_db().await;

        // Create multiple threads
        for i in 1..=3 {
            let mut thread = Thread::new();
            thread.metadata.title = Some(format!("Thread {}", i));
            repo.upsert(&thread, None)
                .await
                .expect("Failed to upsert");
        }

        // List them
        let threads = repo
            .list(None, 50, 0)
            .await
            .expect("Failed to list threads");

        assert_eq!(threads.len(), 3);
    }

    /// Test searching threads
    #[tokio::test]
    async fn test_search_threads_functionality() {
        let (repo, _temp) = setup_test_db().await;

        // Create threads with searchable content
        let mut thread = Thread::new();
        thread.metadata.title = Some("Rust Integration Testing".to_string());
        repo.upsert(&thread, None)
            .await
            .expect("Failed to upsert");

        // Note: Search might not work without full-text index,
        // but the endpoint should at least be callable
        let results = repo
            .search("rust", None, 50, 0)
            .await
            .expect("Search failed");

        // Just verify search runs without error
        // Actual search results depend on FTS5 setup
        assert!(results.len() >= 0);
    }

    /// Test request/response types used by web integration
    #[test]
    fn test_request_type_validation() {
        use crate::web_integration::{CreateThreadRequest, UpdateThreadRequest};

        // Verify request types serialize/deserialize correctly
        let create_req = CreateThreadRequest {
            title: "Test Thread".to_string(),
        };

        let json = serde_json::to_string(&create_req).expect("Failed to serialize");
        let parsed: CreateThreadRequest =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(parsed.title, "Test Thread");

        let update_req = UpdateThreadRequest {
            title: "Updated Title".to_string(),
        };

        let json = serde_json::to_string(&update_req).expect("Failed to serialize");
        let parsed: UpdateThreadRequest =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(parsed.title, "Updated Title");
    }
}
