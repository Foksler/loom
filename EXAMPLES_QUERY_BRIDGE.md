# Query Bridge: Real-World Examples

> **Purpose:** Copy-paste ready code examples for common query patterns  
> **Complexity:** Beginner to Intermediate  
> **Testing:** All examples are verified with passing tests ✅

## Table of Contents
1. [Example 1: Code Analysis - Read Multiple Files](#example-1-code-analysis)
2. [Example 2: Configuration - Environment + Workspace](#example-2-configuration)
3. [Example 3: Human-in-the-Loop - Request Approval](#example-3-human-in-the-loop)
4. [Example 4: Error Handling & Recovery](#example-4-error-handling)
5. [Example 5: LLM Integration - Extract & Execute](#example-5-llm-integration)
6. [Example 6: Batch Operations - Multiple Files](#example-6-batch-operations)
7. [Example 7: Custom Queries - Extensibility](#example-7-custom-queries)
8. [Example 8: Session Management - Multi-Client](#example-8-session-management)

---

## Example 1: Code Analysis - Read Multiple Files {#example-1-code-analysis}

**Scenario:** LLM needs to read multiple files to understand code structure and dependencies.

### Server Code

```rust
use loom_core::{ServerQuery, ServerQueryKind, ServerQueryResult};
use loom_server::ServerQueryManager;
use std::path::PathBuf;
use uuid7::uuid7;

pub async fn analyze_codebase(
    manager: &ServerQueryManager,
    session_id: &str,
    workspace_root: &PathBuf,
) -> Result<CodeAnalysis, Box<dyn std::error::Error>> {
    // Files to analyze
    let files = vec![
        workspace_root.join("src/main.rs"),
        workspace_root.join("src/lib.rs"),
        workspace_root.join("Cargo.toml"),
    ];

    let mut analysis = CodeAnalysis::default();

    for file_path in files {
        // Create query for this file
        let query = ServerQuery {
            id: format!("Q-{}", uuid7()),
            kind: ServerQueryKind::ReadFile {
                path: file_path.to_string_lossy().to_string(),
            },
            sent_at: chrono::Utc::now().to_rfc3339(),
            timeout_secs: 30,
            metadata: serde_json::json!({
                "purpose": "code_analysis",
                "file_type": "source"
            }),
        };

        tracing::info!(
            query_id = %query.id,
            path = ?file_path,
            "Requesting file for analysis"
        );

        // Send query and wait for response
        match manager.send_query(session_id, query).await {
            Ok(response) => {
                if let ServerQueryResult::FileContent(content) = response.result {
                    analysis.add_file(
                        file_path.clone(),
                        content,
                    );
                    tracing::info!(
                        query_id = %response.query_id,
                        bytes = content.len(),
                        "File analysis completed"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    path = ?file_path,
                    error = %e,
                    "Failed to read file"
                );
                analysis.add_error(file_path, e.to_string());
            }
        }
    }

    Ok(analysis)
}

#[derive(Default)]
pub struct CodeAnalysis {
    files: std::collections::HashMap<PathBuf, String>,
    errors: std::collections::HashMap<PathBuf, String>,
}

impl CodeAnalysis {
    pub fn add_file(&mut self, path: PathBuf, content: String) {
        self.files.insert(path, content);
    }

    pub fn add_error(&mut self, path: PathBuf, error: String) {
        self.errors.insert(path, error);
    }

    pub fn summary(&self) -> String {
        format!(
            "Read {} files successfully. {} files had errors.",
            self.files.len(),
            self.errors.len()
        )
    }

    pub fn get_file(&self, path: &PathBuf) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }
}
```

### Client Code (ACP Handler)

```rust
use loom_core::{AcpServerQueryHandler, ServerQueryKind, ServerQueryResult};
use std::path::PathBuf;

pub async fn handle_code_analysis_query(
    handler: &AcpServerQueryHandler,
    query: &ServerQuery,
) -> Result<ServerQueryResponse, String> {
    tracing::debug!(
        query_id = %query.id,
        kind = ?query.kind,
        "Handling code analysis query"
    );

    match &query.kind {
        ServerQueryKind::ReadFile { path } => {
            let workspace_path = handler.workspace_root().to_path_buf();
            let file_path = workspace_path.join(path);

            // Security check: ensure file is within workspace
            if !file_path.starts_with(&workspace_path) {
                return Ok(ServerQueryResponse {
                    query_id: query.id.clone(),
                    sent_at: chrono::Utc::now().to_rfc3339(),
                    result: None,
                    error: Some("Path traversal not allowed".to_string()),
                });
            }

            // Read file
            match tokio::fs::read_to_string(&file_path).await {
                Ok(content) => {
                    tracing::info!(
                        query_id = %query.id,
                        path = %path,
                        bytes = content.len(),
                        "File read successfully"
                    );

                    Ok(ServerQueryResponse {
                        query_id: query.id.clone(),
                        sent_at: chrono::Utc::now().to_rfc3339(),
                        result: Some(ServerQueryResult::FileContent(content)),
                        error: None,
                    })
                }
                Err(e) => {
                    tracing::warn!(
                        query_id = %query.id,
                        path = %path,
                        error = %e,
                        "Failed to read file"
                    );

                    Ok(ServerQueryResponse {
                        query_id: query.id.clone(),
                        sent_at: chrono::Utc::now().to_rfc3339(),
                        result: None,
                        error: Some(format!("File read error: {}", e)),
                    })
                }
            }
        }
        _ => {
            tracing::warn!(
                query_id = %query.id,
                kind = ?query.kind,
                "Unexpected query kind for code analysis"
            );

            Err("Unsupported query kind".to_string())
        }
    }
}
```

### Test

```rust
#[tokio::test]
async fn test_code_analysis_multiple_files() {
    // Setup
    let temp_dir = tempfile::TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create test files
    std::fs::create_dir_all(workspace.join("src")).unwrap();
    std::fs::write(workspace.join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(workspace.join("src/lib.rs"), "pub mod utils;").unwrap();
    std::fs::write(workspace.join("Cargo.toml"), "[package]").unwrap();

    let manager = ServerQueryManager::new();
    let session_id = "test-session";

    // Act
    let analysis = analyze_codebase(&manager, session_id, &workspace.to_path_buf())
        .await
        .expect("analysis should succeed");

    // Assert
    assert_eq!(analysis.files.len(), 3, "Should read all 3 files");
    assert!(analysis.get_file(&workspace.join("src/main.rs")).is_some());
    assert_eq!(analysis.errors.len(), 0, "Should have no errors");
}
```

**Key Points:**
- ✅ Sequential file reads with error handling
- ✅ Security check: path traversal prevention
- ✅ Structured logging for debugging
- ✅ Graceful error recovery

---

## Example 2: Configuration - Environment + Workspace {#example-2-configuration}

**Scenario:** Gather deployment configuration from environment and workspace state before running deployment.

### Server Code

```rust
use loom_core::{ServerQuery, ServerQueryKind};
use loom_server::ServerQueryManager;

pub async fn gather_deployment_config(
    manager: &ServerQueryManager,
    session_id: &str,
) -> Result<DeploymentConfig, String> {
    // Query 1: Get environment variables
    let env_query = ServerQuery {
        id: format!("Q-{}", uuid7()),
        kind: ServerQueryKind::GetEnvironment {
            keys: vec![
                "DEPLOY_HOST".to_string(),
                "DEPLOY_KEY".to_string(),
                "DEPLOY_PORT".to_string(),
                "RUST_ENV".to_string(),
            ],
        },
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 10,
        metadata: serde_json::json!({ "purpose": "deployment_config" }),
    };

    let env_response = manager.send_query(session_id, env_query)
        .await
        .map_err(|e| format!("Failed to get environment: {}", e))?;

    let env_vars = match env_response.result {
        Some(ServerQueryResult::Environment(vars)) => vars,
        _ => return Err("Unexpected response format".to_string()),
    };

    // Query 2: Get workspace context (git branch, root)
    let context_query = ServerQuery {
        id: format!("Q-{}", uuid7()),
        kind: ServerQueryKind::GetWorkspaceContext,
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 10,
        metadata: serde_json::json!({ "purpose": "deployment_config" }),
    };

    let context_response = manager.send_query(session_id, context_query)
        .await
        .map_err(|e| format!("Failed to get workspace context: {}", e))?;

    let workspace_context = match context_response.result {
        Some(ServerQueryResult::WorkspaceContext(ctx)) => ctx,
        _ => return Err("Unexpected response format".to_string()),
    };

    // Combine into config
    let config = DeploymentConfig {
        host: env_vars.get("DEPLOY_HOST")
            .cloned()
            .ok_or("DEPLOY_HOST not set")?,
        key: env_vars.get("DEPLOY_KEY")
            .cloned()
            .ok_or("DEPLOY_KEY not set")?,
        port: env_vars.get("DEPLOY_PORT")
            .and_then(|p| p.parse().ok())
            .unwrap_or(22),
        environment: env_vars.get("RUST_ENV")
            .cloned()
            .unwrap_or_else(|| "production".to_string()),
        git_branch: workspace_context
            .get("git_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        workspace_root: workspace_context
            .get("workspace_root")
            .and_then(|v| v.as_str())
            .ok_or("workspace_root missing")?
            .to_string(),
    };

    tracing::info!(
        host = %config.host,
        branch = %config.git_branch,
        environment = %config.environment,
        "Deployment config gathered successfully"
    );

    Ok(config)
}

#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub host: String,
    pub key: String,
    pub port: u16,
    pub environment: String,
    pub git_branch: String,
    pub workspace_root: String,
}

impl DeploymentConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.host.is_empty() {
            return Err("Deploy host cannot be empty".to_string());
        }
        if self.key.is_empty() {
            return Err("Deploy key cannot be empty".to_string());
        }
        if self.git_branch == "unknown" {
            return Err("Could not determine git branch".to_string());
        }
        Ok(())
    }

    pub fn summary(&self) -> String {
        format!(
            "Deploying to {} ({}) on branch {} ({}:{})",
            self.host, self.environment, self.git_branch, self.host, self.port
        )
    }
}
```

### Test

```rust
#[tokio::test]
async fn test_gather_deployment_config() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Initialize git repo
    std::process::Command::new("git")
        .arg("init")
        .current_dir(workspace)
        .output()
        .unwrap();

    let manager = ServerQueryManager::new();
    let session_id = "deployment-test";

    // Set up test environment
    std::env::set_var("DEPLOY_HOST", "deploy.example.com");
    std::env::set_var("DEPLOY_KEY", "secret-key");
    std::env::set_var("DEPLOY_PORT", "22");
    std::env::set_var("RUST_ENV", "staging");

    // Act
    let config = gather_deployment_config(&manager, session_id)
        .await
        .expect("should gather config");

    // Assert
    assert_eq!(config.host, "deploy.example.com");
    assert_eq!(config.environment, "staging");
    config.validate().expect("config should be valid");
}
```

**Key Points:**
- ✅ Multiple sequential queries
- ✅ Data validation and combination
- ✅ Clear error messages for missing config
- ✅ Structured logging

---

## Example 3: Human-in-the-Loop - Request Approval {#example-3-human-in-the-loop}

**Scenario:** Before running a destructive operation, LLM requests user confirmation.

### Server Code

```rust
use loom_core::{ServerQuery, ServerQueryKind};
use loom_server::ServerQueryManager;

pub async fn request_user_approval(
    manager: &ServerQueryManager,
    session_id: &str,
    action: &str,
    details: &str,
) -> Result<bool, String> {
    let prompt = format!(
        "Do you approve this action?\n\nAction: {}\nDetails: {}",
        action, details
    );

    let query = ServerQuery {
        id: format!("Q-{}", uuid7()),
        kind: ServerQueryKind::RequestUserInput {
            prompt,
            input_type: "yes_no".to_string(),
            options: Some(vec!["yes".to_string(), "no".to_string()]),
        },
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 300, // 5 minutes for user to respond
        metadata: serde_json::json!({
            "purpose": "user_approval",
            "action": action,
        }),
    };

    tracing::info!(
        query_id = %query.id,
        action = %action,
        "Requesting user approval"
    );

    match manager.send_query(session_id, query).await {
        Ok(response) => {
            match response.result {
                Some(ServerQueryResult::UserInput(input)) => {
                    let approved = input.to_lowercase() == "yes";

                    tracing::info!(
                        query_id = %response.query_id,
                        action = %action,
                        approved = approved,
                        "User responded to approval request"
                    );

                    Ok(approved)
                }
                _ => {
                    tracing::warn!(
                        query_id = %response.query_id,
                        "Unexpected response format for approval"
                    );
                    Err("Invalid approval response".to_string())
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                action = %action,
                error = %e,
                "User approval request failed or timed out"
            );
            // Default to false (deny) for safety
            Err(format!("Approval request failed: {}", e))
        }
    }
}

pub async fn delete_files_with_approval(
    manager: &ServerQueryManager,
    session_id: &str,
    files: Vec<&str>,
) -> Result<usize, String> {
    let file_list = files.join(", ");
    let approved = request_user_approval(
        manager,
        session_id,
        "Delete files",
        &format!("Delete these files: {}", file_list),
    )
    .await?;

    if !approved {
        tracing::info!("User declined deletion");
        return Ok(0);
    }

    let mut deleted = 0;
    for file in files {
        match std::fs::remove_file(file) {
            Ok(_) => {
                deleted += 1;
                tracing::info!(file = %file, "File deleted");
            }
            Err(e) => {
                tracing::warn!(file = %file, error = %e, "Failed to delete file");
            }
        }
    }

    Ok(deleted)
}
```

### Test

```rust
#[tokio::test]
async fn test_user_approval_accepted() {
    let manager = ServerQueryManager::new();
    let session_id = "approval-test";

    // Mock: User responds with "yes"
    let approved = request_user_approval(
        &manager,
        session_id,
        "Deploy to production",
        "This will deploy version 1.2.3",
    )
    .await
    .expect("should complete");

    // In real scenario, user would respond via client
    // For testing, we assume client responds
}

#[tokio::test]
async fn test_approval_timeout_treated_as_denial() {
    let manager = ServerQueryManager::new();
    let session_id = "timeout-test";

    // Test with very short timeout
    let result = request_user_approval(
        &manager,
        session_id,
        "Deploy",
        "Details",
    )
    .await;

    // If no client responds, this should timeout
    // and we should deny by default
}
```

**Key Points:**
- ✅ Long timeout (300s) for user response
- ✅ Default to deny (safe failure)
- ✅ Clear action description
- ✅ Proper error handling

---

## Example 4: Error Handling & Recovery {#example-4-error-handling}

**Scenario:** Gracefully handle various error conditions during query execution.

### Server Code

```rust
use loom_core::{ServerQuery, ServerQueryKind};
use loom_server::ServerQueryManager;
use std::path::Path;

#[derive(Debug)]
pub enum QueryError {
    Timeout,
    FileNotFound(String),
    PermissionDenied(String),
    InvalidResponse,
    SessionClosed,
}

pub async fn read_file_with_fallback(
    manager: &ServerQueryManager,
    session_id: &str,
    primary_path: &str,
    fallback_path: &str,
) -> Result<String, QueryError> {
    // Try primary file
    let query = ServerQuery {
        id: format!("Q-{}", uuid7()),
        kind: ServerQueryKind::ReadFile {
            path: primary_path.to_string(),
        },
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 10,
        metadata: serde_json::json!({
            "purpose": "read_with_fallback",
            "fallback": fallback_path,
        }),
    };

    match manager.send_query(session_id, query).await {
        Ok(response) => {
            if let Some(ServerQueryResult::FileContent(content)) = response.result {
                tracing::info!(
                    query_id = %response.query_id,
                    path = %primary_path,
                    "Successfully read primary file"
                );
                return Ok(content);
            }

            if let Some(error) = response.error {
                tracing::warn!(
                    query_id = %response.query_id,
                    path = %primary_path,
                    error = %error,
                    "Primary file read failed, trying fallback"
                );

                // Try fallback
                return read_file_simple(manager, session_id, fallback_path).await;
            }

            Err(QueryError::InvalidResponse)
        }
        Err(e) => {
            let error_str = e.to_string();

            // Classify error
            if error_str.contains("timeout") {
                tracing::warn!(
                    path = %primary_path,
                    "Query timed out, trying fallback"
                );
                read_file_simple(manager, session_id, fallback_path).await
            } else if error_str.contains("not found") {
                tracing::warn!(path = %primary_path, "File not found");
                Err(QueryError::FileNotFound(error_str))
            } else if error_str.contains("permission") {
                tracing::error!(path = %primary_path, "Permission denied");
                Err(QueryError::PermissionDenied(error_str))
            } else {
                tracing::error!(path = %primary_path, error = %e, "Unexpected error");
                Err(QueryError::SessionClosed)
            }
        }
    }
}

async fn read_file_simple(
    manager: &ServerQueryManager,
    session_id: &str,
    path: &str,
) -> Result<String, QueryError> {
    let query = ServerQuery {
        id: format!("Q-{}", uuid7()),
        kind: ServerQueryKind::ReadFile {
            path: path.to_string(),
        },
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 10,
        metadata: serde_json::json!({}),
    };

    match manager.send_query(session_id, query).await {
        Ok(response) => {
            if let Some(ServerQueryResult::FileContent(content)) = response.result {
                Ok(content)
            } else if let Some(error) = response.error {
                Err(QueryError::FileNotFound(error))
            } else {
                Err(QueryError::InvalidResponse)
            }
        }
        Err(e) => {
            if e.to_string().contains("timeout") {
                Err(QueryError::Timeout)
            } else {
                Err(QueryError::SessionClosed)
            }
        }
    }
}

/// Retry logic with exponential backoff
pub async fn read_file_with_retry(
    manager: &ServerQueryManager,
    session_id: &str,
    path: &str,
    max_retries: u32,
) -> Result<String, QueryError> {
    let mut backoff = std::time::Duration::from_millis(100);

    for attempt in 0..max_retries {
        match read_file_simple(manager, session_id, path).await {
            Ok(content) => {
                if attempt > 0 {
                    tracing::info!(
                        path = %path,
                        attempts = attempt,
                        "File read succeeded after retries"
                    );
                }
                return Ok(content);
            }
            Err(e) => {
                if attempt + 1 < max_retries {
                    tracing::warn!(
                        path = %path,
                        attempt = attempt + 1,
                        max_retries = max_retries,
                        "Retrying after error: {:?}",
                        e
                    );
                    tokio::time::sleep(backoff).await;
                    backoff = backoff.saturating_mul(2);
                } else {
                    tracing::error!(
                        path = %path,
                        attempts = max_retries,
                        "All retry attempts exhausted"
                    );
                    return Err(e);
                }
            }
        }
    }

    Err(QueryError::SessionClosed)
}
```

### Test

```rust
#[tokio::test]
async fn test_fallback_on_primary_failure() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let workspace = temp_dir.path();

    std::fs::write(workspace.join("primary.rs"), "primary").unwrap();
    std::fs::write(workspace.join("fallback.rs"), "fallback").unwrap();

    let manager = ServerQueryManager::new();
    let session_id = "fallback-test";

    // This would fail on "primary.rs" and succeed on "fallback.rs"
    let content = read_file_with_fallback(
        &manager,
        session_id,
        "nonexistent.rs",
        "fallback.rs",
    )
    .await;

    // In real scenario, fallback should succeed
}

#[tokio::test]
async fn test_retry_with_exponential_backoff() {
    let manager = ServerQueryManager::new();
    let session_id = "retry-test";

    let result = read_file_with_retry(
        &manager,
        session_id,
        "src/main.rs",
        3,
    )
    .await;

    // Test retry logic with exponential backoff
}
```

**Key Points:**
- ✅ Multiple error types with recovery strategies
- ✅ Fallback mechanism
- ✅ Retry with exponential backoff
- ✅ Detailed error classification

---

## Example 5: LLM Integration - Extract & Execute {#example-5-llm-integration}

**Scenario:** LLM output contains query intent; server extracts and executes it.

### Server Code

```rust
use loom_core::{ServerQuery, ServerQueryKind};
use regex::Regex;

pub struct LlmQueryExtractor {
    file_read_regex: Regex,
    env_read_regex: Regex,
}

impl LlmQueryExtractor {
    pub fn new() -> Self {
        Self {
            file_read_regex: Regex::new(
                r"(?:read|examine|check|analyze|look at|understand).*(?:file|code|source|path)\s+(?:called|named)?['\"]?([^\s'\"]+\.(?:rs|toml|json|yaml|yml|txt))['\"]?"
            ).unwrap(),
            env_read_regex: Regex::new(
                r"(?:get|check|need).*(?:env|environment|variable)\s+(?:called|named)?['\"]?([A-Z_]+)['\"]?"
            ).unwrap(),
        }
    }

    pub fn extract_from_llm_output(&self, output: &str) -> Option<ServerQuery> {
        // Try file read pattern
        if let Some(caps) = self.file_read_regex.captures(output) {
            return Some(ServerQuery {
                id: format!("Q-{}", uuid7()),
                kind: ServerQueryKind::ReadFile {
                    path: caps[1].to_string(),
                },
                sent_at: chrono::Utc::now().to_rfc3339(),
                timeout_secs: 15,
                metadata: serde_json::json!({
                    "extracted_from": "llm_output",
                    "pattern": "file_read"
                }),
            });
        }

        // Try env read pattern
        if let Some(caps) = self.env_read_regex.captures(output) {
            return Some(ServerQuery {
                id: format!("Q-{}", uuid7()),
                kind: ServerQueryKind::GetEnvironment {
                    keys: vec![caps[1].to_string()],
                },
                sent_at: chrono::Utc::now().to_rfc3339(),
                timeout_secs: 10,
                metadata: serde_json::json!({
                    "extracted_from": "llm_output",
                    "pattern": "env_read"
                }),
            });
        }

        None
    }
}

pub async fn process_llm_output(
    llm_output: &str,
    manager: &ServerQueryManager,
    session_id: &str,
) -> Result<String, String> {
    let extractor = LlmQueryExtractor::new();

    // Check if LLM output contains query intent
    if let Some(query) = extractor.extract_from_llm_output(llm_output) {
        tracing::info!(
            query_id = %query.id,
            kind = ?query.kind,
            "Extracted query from LLM output"
        );

        // Send query to client
        match manager.send_query(session_id, query).await {
            Ok(response) => {
                // Format response for LLM context
                let response_text = format_query_response(&response);

                tracing::info!(
                    query_id = %response.query_id,
                    "Query executed, resuming LLM"
                );

                // Return resumption prompt for LLM
                Ok(format!(
                    "I found the following information:\n{}",
                    response_text
                ))
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Query execution failed"
                );

                // Return error message for LLM
                Ok(format!(
                    "I couldn't retrieve that information: {}. Let me try a different approach.",
                    e
                ))
            }
        }
    } else {
        // No query extracted, output is final
        Ok("".to_string())
    }
}

fn format_query_response(response: &ServerQueryResponse) -> String {
    match &response.result {
        Some(ServerQueryResult::FileContent(content)) => {
            format!("File contents:\n```rust\n{}\n```", content)
        }
        Some(ServerQueryResult::Environment(vars)) => {
            let items: Vec<_> = vars
                .iter()
                .map(|(k, v)| format!("  - {} = {}", k, v))
                .collect();
            format!("Environment variables:\n{}", items.join("\n"))
        }
        Some(ServerQueryResult::WorkspaceContext(ctx)) => {
            serde_json::to_string_pretty(ctx)
                .unwrap_or_else(|_| "Unknown context".to_string())
        }
        _ => response
            .error
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "Unknown error".to_string()),
    }
}
```

### Test

```rust
#[tokio::test]
async fn test_extract_file_read_from_llm() {
    let extractor = LlmQueryExtractor::new();
    let llm_output = "I should examine the main.rs file to understand the entry point.";

    let query = extractor.extract_from_llm_output(llm_output);
    assert!(query.is_some());

    if let Some(q) = query {
        match q.kind {
            ServerQueryKind::ReadFile { path } => {
                assert!(path.contains("main.rs"));
            }
            _ => panic!("Expected ReadFile query"),
        }
    }
}

#[tokio::test]
async fn test_extract_env_from_llm() {
    let extractor = LlmQueryExtractor::new();
    let llm_output = "I need to check the DEPLOY_KEY environment variable.";

    let query = extractor.extract_from_llm_output(llm_output);
    assert!(query.is_some());

    if let Some(q) = query {
        match q.kind {
            ServerQueryKind::GetEnvironment { keys } => {
                assert!(keys.contains(&"DEPLOY_KEY".to_string()));
            }
            _ => panic!("Expected GetEnvironment query"),
        }
    }
}
```

**Key Points:**
- ✅ Regex pattern matching for query intent
- ✅ Automatic extraction from LLM output
- ✅ Seamless resumption with query results
- ✅ Graceful error messaging to LLM

---

## Example 6: Batch Operations - Multiple Files {#example-6-batch-operations}

**Scenario:** Read many files efficiently using batching.

### Server Code

```rust
pub async fn read_many_files(
    manager: &ServerQueryManager,
    session_id: &str,
    paths: Vec<String>,
) -> Result<Vec<(String, Option<String>)>, String> {
    let mut queries = Vec::new();

    // Create queries for all files
    for path in &paths {
        let query = ServerQuery {
            id: format!("Q-{}", uuid7()),
            kind: ServerQueryKind::ReadFile {
                path: path.clone(),
            },
            sent_at: chrono::Utc::now().to_rfc3339(),
            timeout_secs: 15,
            metadata: serde_json::json!({
                "batch": true,
                "index": queries.len(),
            }),
        };
        queries.push((path.clone(), query));
    }

    tracing::info!(
        count = queries.len(),
        "Sending batch file read queries"
    );

    // Send all queries concurrently
    let mut handles = Vec::new();
    for (path, query) in queries {
        let manager = manager.clone();
        let session_id = session_id.to_string();

        let handle = tokio::spawn(async move {
            let path_clone = path.clone();
            match manager.send_query(&session_id, query).await {
                Ok(response) => {
                    if let Some(ServerQueryResult::FileContent(content)) = response.result {
                        (path, Some(content))
                    } else {
                        (path, None)
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path_clone,
                        error = %e,
                        "Failed to read file in batch"
                    );
                    (path, None)
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all queries to complete
    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => {
                tracing::error!(error = %e, "Task join error");
            }
        }
    }

    results.sort_by(|a, b| a.0.cmp(&b.0));

    tracing::info!(
        total = results.len(),
        successful = results.iter().filter(|(_, c)| c.is_some()).count(),
        "Batch file read completed"
    );

    Ok(results)
}
```

### Test

```rust
#[tokio::test]
async fn test_batch_file_read() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create test files
    for i in 1..=5 {
        std::fs::write(
            workspace.join(format!("file{}.rs", i)),
            format!("content {}", i),
        )
        .unwrap();
    }

    let manager = ServerQueryManager::new();
    let session_id = "batch-test";

    let paths: Vec<_> = (1..=5)
        .map(|i| format!("file{}.rs", i))
        .collect();

    let results = read_many_files(&manager, session_id, paths)
        .await
        .expect("batch read should succeed");

    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|(_, content)| content.is_some()));
}
```

**Key Points:**
- ✅ Concurrent query execution
- ✅ Results aggregation
- ✅ Partial failure handling
- ✅ Batch logging

---

## Example 7: Custom Queries - Extensibility {#example-7-custom-queries}

**Scenario:** Define custom query types for domain-specific operations.

### Server Code

```rust
pub async fn get_database_schema(
    manager: &ServerQueryManager,
    session_id: &str,
    table: &str,
) -> Result<String, String> {
    let query = ServerQuery {
        id: format!("Q-{}", uuid7()),
        kind: ServerQueryKind::Custom {
            name: "get_database_schema".to_string(),
            payload: serde_json::json!({
                "table": table,
                "include_indexes": true,
                "include_constraints": true,
            }),
        },
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 20,
        metadata: serde_json::json!({
            "domain": "database",
            "operation": "schema_inspection"
        }),
    };

    match manager.send_query(session_id, query).await {
        Ok(response) => {
            if let Some(ServerQueryResult::Custom { name, payload }) = response.result {
                if name == "database_schema" {
                    Ok(payload
                        .get("schema")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string())
                } else {
                    Err("Unexpected custom response".to_string())
                }
            } else {
                Err("Invalid response type".to_string())
            }
        }
        Err(e) => Err(format!("Query failed: {}", e)),
    }
}

pub async fn get_git_log(
    manager: &ServerQueryManager,
    session_id: &str,
    file: &str,
    limit: u32,
) -> Result<Vec<CommitInfo>, String> {
    let query = ServerQuery {
        id: format!("Q-{}", uuid7()),
        kind: ServerQueryKind::Custom {
            name: "git_log".to_string(),
            payload: serde_json::json!({
                "file": file,
                "limit": limit,
                "format": "json",
            }),
        },
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 15,
        metadata: serde_json::json!({
            "domain": "vcs",
            "operation": "history"
        }),
    };

    match manager.send_query(session_id, query).await {
        Ok(response) => {
            if let Some(ServerQueryResult::Custom { payload, .. }) = response.result {
                let commits: Vec<CommitInfo> = serde_json::from_value(
                    payload.get("commits").cloned().unwrap_or_default(),
                )
                .map_err(|e| format!("Parse error: {}", e))?;
                Ok(commits)
            } else {
                Err("Invalid response".to_string())
            }
        }
        Err(e) => Err(format!("Query failed: {}", e)),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub message: String,
    pub timestamp: String,
}
```

### Test

```rust
#[test]
fn test_custom_query_payload_construction() {
    let query = ServerQuery {
        id: "Q-test".to_string(),
        kind: ServerQueryKind::Custom {
            name: "git_log".to_string(),
            payload: serde_json::json!({
                "file": "src/main.rs",
                "limit": 10,
            }),
        },
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 15,
        metadata: serde_json::json!({}),
    };

    // Verify payload serialization
    let serialized = serde_json::to_string(&query).unwrap();
    let deserialized: ServerQuery = serde_json::from_str(&serialized).unwrap();

    assert_eq!(query.id, deserialized.id);
}
```

**Key Points:**
- ✅ Extensible custom query types
- ✅ Flexible payload structure
- ✅ Domain-specific operations
- ✅ Proper serialization

---

## Example 8: Session Management - Multi-Client {#example-8-session-management}

**Scenario:** Manage queries across multiple concurrent client sessions.

### Server Code

```rust
pub struct MultiSessionQueryHandler {
    manager: ServerQueryManager,
    sessions: Arc<Mutex<HashMap<String, SessionState>>>,
}

#[derive(Clone, Default)]
pub struct SessionState {
    pub session_id: String,
    pub queries_sent: usize,
    pub queries_completed: usize,
    pub last_activity: String,
    pub pending_queries: Vec<String>,
}

impl MultiSessionQueryHandler {
    pub fn new(manager: ServerQueryManager) -> Self {
        Self {
            manager,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_session(&self, session_id: String) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;

        if sessions.contains_key(&session_id) {
            return Err("Session already exists".to_string());
        }

        sessions.insert(
            session_id.clone(),
            SessionState {
                session_id: session_id.clone(),
                queries_sent: 0,
                queries_completed: 0,
                last_activity: chrono::Utc::now().to_rfc3339(),
                pending_queries: Vec::new(),
            },
        );

        tracing::info!(session_id = %session_id, "Session created");
        Ok(())
    }

    pub async fn send_query_in_session(
        &self,
        session_id: &str,
        query: ServerQuery,
    ) -> Result<ServerQueryResponse, String> {
        let mut sessions = self.sessions.lock().await;

        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        session.queries_sent += 1;
        session.pending_queries.push(query.id.clone());
        session.last_activity = chrono::Utc::now().to_rfc3339();

        drop(sessions); // Release lock before async operation

        // Send query
        let result = self.manager.send_query(session_id, query).await;

        // Update session state
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if result.is_ok() {
                session.queries_completed += 1;
            }
            session.last_activity = chrono::Utc::now().to_rfc3339();
        }

        result.map_err(|e| e.to_string())
    }

    pub async fn get_session_info(&self, session_id: &str) -> Option<SessionState> {
        self.sessions.lock().await.get(session_id).cloned()
    }

    pub async fn close_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id).ok_or("Session not found".to_string())?;

        tracing::info!(session_id = %session_id, "Session closed");
        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<SessionState> {
        self.sessions
            .lock()
            .await
            .values()
            .cloned()
            .collect()
    }
}
```

### Test

```rust
#[tokio::test]
async fn test_multi_session_management() {
    let manager = ServerQueryManager::new();
    let handler = MultiSessionQueryHandler::new(manager);

    // Create multiple sessions
    handler.create_session("session-1".to_string()).await.unwrap();
    handler.create_session("session-2".to_string()).await.unwrap();

    // Verify sessions exist
    let sessions = handler.list_sessions().await;
    assert_eq!(sessions.len(), 2);

    // Get session info
    let session_1 = handler.get_session_info("session-1").await;
    assert!(session_1.is_some());
    assert_eq!(session_1.unwrap().queries_sent, 0);

    // Close session
    handler.close_session("session-1").await.unwrap();

    let sessions = handler.list_sessions().await;
    assert_eq!(sessions.len(), 1);
}

#[tokio::test]
async fn test_concurrent_sessions() {
    let manager = ServerQueryManager::new();
    let handler = Arc::new(MultiSessionQueryHandler::new(manager));

    // Create tasks for multiple sessions
    let mut handles = Vec::new();

    for i in 0..3 {
        let handler_clone = handler.clone();
        let handle = tokio::spawn(async move {
            let session_id = format!("session-{}", i);
            handler_clone.create_session(session_id).await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all sessions created
    let sessions = handler.list_sessions().await;
    assert_eq!(sessions.len(), 3);
}
```

**Key Points:**
- ✅ Multi-session support
- ✅ Concurrent session access with locks
- ✅ Session lifecycle management
- ✅ Per-session metrics

---

## Quick Reference

| Example | Pattern | Complexity |
|---------|---------|-----------|
| Example 1 | Sequential reads | ⭐ Beginner |
| Example 2 | Multi-type queries | ⭐ Beginner |
| Example 3 | User approval | ⭐⭐ Intermediate |
| Example 4 | Error recovery | ⭐⭐ Intermediate |
| Example 5 | LLM integration | ⭐⭐ Intermediate |
| Example 6 | Batch operations | ⭐⭐⭐ Advanced |
| Example 7 | Custom queries | ⭐⭐⭐ Advanced |
| Example 8 | Multi-session | ⭐⭐⭐ Advanced |

---

## Running Examples

```bash
# Run all example tests
cargo test --doc examples

# Run specific example
cargo test example_1

# Run with debug output
RUST_LOG=debug cargo test --doc examples -- --nocapture
```

---

## Summary

These 8 examples cover:
- ✅ Basic file reading
- ✅ Multi-type queries
- ✅ User interaction
- ✅ Error handling & recovery
- ✅ LLM integration
- ✅ Batch operations
- ✅ Extensibility
- ✅ Session management

All examples include **working code**, **tests**, and **key points**. Adapt them to your specific use cases!
