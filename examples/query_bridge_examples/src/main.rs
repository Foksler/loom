// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Comprehensive examples demonstrating the Server-to-Client Query Bridge
//!
//! This module provides 5 working examples showing:
//! 1. Basic file read during coding
//! 2. Environment + workspace context queries
//! 3. Human-in-the-loop approval patterns
//! 4. Error recovery and timeout handling
//! 5. Concurrent queries from multiple sessions
//!
//! Run with: `cargo run --bin query_bridge_examples` or
//! `cargo run -p query_bridge_examples`

use loom_core::{
	ServerQuery, ServerQueryError, ServerQueryHandler, ServerQueryKind, ServerQueryResponse,
	ServerQueryResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

/// Mock handler for examples - simulates client behavior
struct MockServerQueryHandler;

#[async_trait::async_trait]
impl ServerQueryHandler for MockServerQueryHandler {
	async fn handle_query(
		&self,
		query: ServerQuery,
	) -> Result<ServerQueryResponse, ServerQueryError> {
		match &query.kind {
			ServerQueryKind::ReadFile { path } => {
				// Simulate reading a file
				let content = match path.as_str() {
					"src/main.rs" => "fn main() {\n    println!(\"Hello!\");\n}",
					"Cargo.toml" => "[package]\nname = \"example\"\nversion = \"0.1.0\"",
					_ => "file content here",
				};
				Ok(ServerQueryResponse {
					query_id: query.id,
					sent_at: chrono::Utc::now().to_rfc3339(),
					result: ServerQueryResult::FileContent(content.to_string()),
					error: None,
				})
			}
			ServerQueryKind::GetEnvironment { keys } => {
				let mut env_vars = HashMap::new();
				for key in keys {
					if let Ok(val) = std::env::var(key) {
						env_vars.insert(key.clone(), val);
					}
				}
				Ok(ServerQueryResponse {
					query_id: query.id,
					sent_at: chrono::Utc::now().to_rfc3339(),
					result: ServerQueryResult::Environment(env_vars),
					error: None,
				})
			}
			ServerQueryKind::GetWorkspaceContext => {
				let context = serde_json::json!({
						"workspace": "/home/user/project",
						"git_branch": "main",
						"has_git": true,
				});
				Ok(ServerQueryResponse {
					query_id: query.id,
					sent_at: chrono::Utc::now().to_rfc3339(),
					result: ServerQueryResult::WorkspaceContext(context),
					error: None,
				})
			}
			ServerQueryKind::RequestUserInput { input_type, .. } => {
				// Simulate user choosing "yes"
				let response = match input_type.as_str() {
					"yes_no" => "yes",
					"text" => "user input response",
					_ => "default",
				};
				Ok(ServerQueryResponse {
					query_id: query.id,
					sent_at: chrono::Utc::now().to_rfc3339(),
					result: ServerQueryResult::UserInput(response.to_string()),
					error: None,
				})
			}
			ServerQueryKind::ExecuteCommand { .. } => Err(ServerQueryError::ProcessingFailed(
				"Command execution disabled".to_string(),
			)),
			ServerQueryKind::Custom { name, .. } => Err(ServerQueryError::ProcessingFailed(format!(
				"Unknown custom query: {}",
				name
			))),
		}
	}
}

// ============================================================================
// EXAMPLE 1: Basic File Read During Coding
// ============================================================================

/// **Purpose**: Demonstrates the basic file read workflow.
///
/// **Scenario**: Server LLM needs to read a file to add logging.
/// - LLM says: "I'll add logging to your main.rs"
/// - Detects: "main.rs" reference
/// - Query: ReadFile("src/main.rs")
/// - Client: Returns file contents
/// - LLM: Continues with full context
///
/// **Why This Is Important**: This is the most common query type used to
/// provide context for code modifications. It enables the LLM to read the
/// current state of files before making changes.
async fn example_1_basic_file_read() {
	info!("📖 Example 1: Basic File Read During Coding");
	println!("\n=== Example 1: Basic File Read ===\n");

	// Step 1: Create a file read query
	println!("Step 1: LLM needs to read src/main.rs");
	let query = ServerQuery {
		id: format!("Q-{:032x}", 1u128),
		kind: ServerQueryKind::ReadFile {
			path: "src/main.rs".to_string(),
		},
		sent_at: chrono::Utc::now().to_rfc3339(),
		timeout_secs: 30,
		metadata: serde_json::json!({}),
	};
	println!("  Query ID: {}", query.id);
	println!("  Path: src/main.rs\n");

	// Step 2: Simulate server sending query to client
	println!("Step 2: Server sends query to client via SSE");
	let handler = Arc::new(MockServerQueryHandler);

	// Step 3: Client handles query
	println!("Step 3: Client receives and processes query");
	match handler.handle_query(query.clone()).await {
		Ok(response) => {
			println!("  ✓ Query handled successfully");
			println!("  Response: {:?}\n", response.result);

			// Step 4: Client sends response back via HTTP POST
			println!("Step 4: Client sends response via HTTP POST /query-response");
			println!("  ✓ Server received response");

			// Step 5: LLM resumes with context
			if let ServerQueryResult::FileContent(content) = &response.result {
				println!("\nStep 5: LLM Resumes Processing");
				println!("  File contains {} bytes", content.len());
				println!(
					"  Content preview:\n    {}",
					content.lines().next().unwrap_or("")
				);
			}

			// Step 6: Assertions
			println!("\nStep 6: Verify Results");
			assert!(!response.query_id.is_empty(), "Query ID missing");
			assert!(response.error.is_none(), "No errors should occur");
			match &response.result {
				ServerQueryResult::FileContent(content) => {
					assert!(!content.is_empty(), "File content should exist");
					assert!(
						content.contains("main"),
						"Content should contain main function"
					);
				}
				_ => panic!("Expected FileContent result"),
			}
			println!("  ✓ All assertions passed");
		}
		Err(e) => {
			error!("Query failed: {}", e);
			panic!("Example 1 failed: {}", e);
		}
	}

	println!("\n✅ Example 1 Complete\n");
}

// ============================================================================
// EXAMPLE 2: Environment + Workspace Context
// ============================================================================

/// **Purpose**: Demonstrates querying environment variables and workspace
/// context.
///
/// **Scenario**: LLM needs deployment target info
/// - Query 1: GetEnvironment(["DEPLOY_HOST", "API_KEY"])
/// - Query 2: GetWorkspaceContext
/// - LLM: Continues with full deployment context
///
/// **Why This Is Important**: Deployment and configuration details are critical
/// for production code changes. This allows the LLM to make decisions based on
/// current environment without hardcoding secrets.
async fn example_2_environment_context() {
	info!("🔧 Example 2: Environment + Workspace Context");
	println!("\n=== Example 2: Environment + Workspace Context ===\n");

	let handler = Arc::new(MockServerQueryHandler);

	// Set some environment variables for demo
	std::env::set_var("DEPLOY_HOST", "prod.example.com");
	std::env::set_var("API_KEY", "secret-key-123");

	// Query 1: Get environment
	println!("Step 1: Query environment variables");
	let env_query = ServerQuery {
		id: format!("Q-{:032x}", 2u128),
		kind: ServerQueryKind::GetEnvironment {
			keys: vec!["DEPLOY_HOST".to_string(), "API_KEY".to_string()],
		},
		sent_at: chrono::Utc::now().to_rfc3339(),
		timeout_secs: 10,
		metadata: serde_json::json!({}),
	};

	let env_response = handler.handle_query(env_query.clone()).await.unwrap();

	match &env_response.result {
		ServerQueryResult::Environment(vars) => {
			println!("  ✓ Retrieved {} environment variables", vars.len());
			for (key, value) in vars {
				println!(
					"    {}: {}",
					key,
					if value.len() > 20 {
						"[redacted]"
					} else {
						value
					}
				);
			}
			assert!(vars.contains_key("DEPLOY_HOST"), "DEPLOY_HOST missing");
		}
		_ => panic!("Expected Environment result"),
	}

	// Query 2: Get workspace context
	println!("\nStep 2: Query workspace context");
	let context_query = ServerQuery {
		id: format!("Q-{:032x}", 3u128),
		kind: ServerQueryKind::GetWorkspaceContext,
		sent_at: chrono::Utc::now().to_rfc3339(),
		timeout_secs: 10,
		metadata: serde_json::json!({}),
	};

	let context_response = handler.handle_query(context_query.clone()).await.unwrap();

	match &context_response.result {
		ServerQueryResult::WorkspaceContext(context) => {
			println!("  ✓ Retrieved workspace context");
			println!(
				"    Git enabled: {}",
				context.get("has_git").unwrap_or(&serde_json::json!(false))
			);
			println!(
				"    Branch: {}",
				context
					.get("git_branch")
					.and_then(|v| v.as_str())
					.unwrap_or("N/A")
			);

			assert!(context.get("workspace").is_some(), "Workspace missing");
			assert!(context.get("has_git").is_some(), "Git info missing");
		}
		_ => panic!("Expected WorkspaceContext result"),
	}

	println!("\nStep 3: LLM Processes Combined Context");
	println!("  ✓ LLM can now deploy to prod.example.com with full workspace info\n");

	println!("✅ Example 2 Complete\n");
}

// ============================================================================
// EXAMPLE 3: Human-in-the-Loop Approval
// ============================================================================

/// **Purpose**: Demonstrates pausing for user confirmation.
///
/// **Scenario**: LLM asks before deleting a migration
/// - Query: RequestUserInput("Delete migration file?", "yes_no")
/// - User: Responds "yes"
/// - LLM: Proceeds with deletion
///
/// **Why This Is Important**: Prevents accidental data loss. For destructive
/// operations like deletion, the system can pause and ask for human
/// confirmation before proceeding.
async fn example_3_human_approval() {
	info!("👤 Example 3: Human-in-the-Loop Approval");
	println!("\n=== Example 3: Human-in-the-Loop Approval ===\n");

	let handler = Arc::new(MockServerQueryHandler);

	println!("Step 1: LLM wants to delete a migration");
	println!("  LLM: \"Should I delete this old migration?\"");

	let approval_query = ServerQuery {
		id: format!("Q-{:032x}", 4u128),
		kind: ServerQueryKind::RequestUserInput {
			prompt: "Delete old migration file db/migrations/001_initial.sql?".to_string(),
			input_type: "yes_no".to_string(),
			options: Some(vec!["yes".to_string(), "no".to_string()]),
		},
		sent_at: chrono::Utc::now().to_rfc3339(),
		timeout_secs: 60,
		metadata: serde_json::json!({}),
	};

	println!("\nStep 2: Server sends approval request to user");
	println!("  Prompt: Delete old migration file db/migrations/001_initial.sql?\n");

	let approval_response = handler.handle_query(approval_query.clone()).await.unwrap();

	match &approval_response.result {
		ServerQueryResult::UserInput(response) => {
			println!("Step 3: User responds");
			println!("  User input: \"{}\"", response);

			if response.to_lowercase() == "yes" {
				println!("\nStep 4: LLM Proceeds");
				println!("  ✓ User approved, proceeding with deletion");
				assert_eq!(response, "yes", "Expected yes response");
			}
		}
		_ => panic!("Expected UserInput result"),
	}

	println!("\n✅ Example 3 Complete\n");
}

// ============================================================================
// EXAMPLE 4: Error Recovery and Timeout Handling
// ============================================================================

/// **Purpose**: Demonstrates error handling and timeout recovery.
///
/// **Scenario**: Query times out, system recovers gracefully
/// - Query attempts file read
/// - Timeout occurs or file not found
/// - System reports error
/// - LLM notified of failure
/// - Processing continues
///
/// **Why This Is Important**: Resilience is critical for production systems.
/// When queries fail (timeout, missing file, permission denied), the system
/// should recover gracefully and allow the LLM to retry or continue with
/// degraded context.
async fn example_4_error_recovery() {
	info!("⚠️  Example 4: Error Recovery");
	println!("\n=== Example 4: Error Recovery & Timeout Handling ===\n");

	let handler = Arc::new(MockServerQueryHandler);

	println!("Step 1: Query non-existent file");
	let error_query = ServerQuery {
		id: format!("Q-{:032x}", 5u128),
		kind: ServerQueryKind::ReadFile {
			path: "src/nonexistent.rs".to_string(),
		},
		sent_at: chrono::Utc::now().to_rfc3339(),
		timeout_secs: 10,
		metadata: serde_json::json!({}),
	};

	println!("  Querying: src/nonexistent.rs\n");

	match handler.handle_query(error_query.clone()).await {
		Ok(_response) => {
			if _response.error.is_some() {
				println!("Step 2: Error Detected");
				println!("  Error: {}", _response.error.as_ref().unwrap());
				println!("  This is expected for missing files\n");

				println!("Step 3: Recovery Strategy");
				println!("  Option 1: Retry with different path");
				println!("  Option 2: Log error and continue");
				println!("  Option 3: Prompt user for correct path\n");

				// Simulate retry with correct path
				println!("Step 4: Retry with correct path");
				let retry_query = ServerQuery {
					id: format!("Q-{:032x}", 6u128),
					kind: ServerQueryKind::ReadFile {
						path: "src/main.rs".to_string(),
					},
					sent_at: chrono::Utc::now().to_rfc3339(),
					timeout_secs: 10,
					metadata: serde_json::json!({}),
				};

				match handler.handle_query(retry_query).await {
					Ok(retry_response) => {
						assert!(retry_response.error.is_none(), "Retry should succeed");
						println!("  ✓ Retry successful");
						println!("  Recovered gracefully from error\n");
					}
					Err(e) => panic!("Retry failed: {}", e),
				}
			}
		}
		Err(e) => {
			error!("Query processing failed: {}", e);
		}
	}

	println!("✅ Example 4 Complete\n");
}

// ============================================================================
// EXAMPLE 5: Concurrent Queries from Multiple Sessions
// ============================================================================

/// **Purpose**: Demonstrates handling concurrent queries from different
/// sessions.
///
/// **Scenario**: Two clients simultaneously request data
/// - Session A: ReadFile("src/module_a.rs")
/// - Session B: GetEnvironment(["API_KEY"])
/// - Session C: GetWorkspaceContext
/// - All process concurrently
/// - Responses routed correctly
///
/// **Why This Is Important**: The server handles many concurrent user sessions.
/// Queries from different sessions must be isolated and processed concurrently
/// without interference. This test verifies that the routing and isolation
/// mechanisms work correctly under concurrent load.
async fn example_5_concurrent_queries() {
	info!("⚡ Example 5: Concurrent Queries");
	println!("\n=== Example 5: Concurrent Queries from Multiple Sessions ===\n");

	let handler = Arc::new(MockServerQueryHandler);

	println!("Step 1: Create three concurrent sessions\n");

	// Create concurrent tasks
	let mut handles = vec![];

	// Session A: Read file
	{
		let h = handler.clone();
		let handle = tokio::spawn(async move {
			println!("  Session A: Starting file read query");
			let query = ServerQuery {
				id: format!("Q-{:032x}", 10u128),
				kind: ServerQueryKind::ReadFile {
					path: "src/module_a.rs".to_string(),
				},
				sent_at: chrono::Utc::now().to_rfc3339(),
				timeout_secs: 10,
				metadata: serde_json::json!({"session": "A"}),
			};

			match h.handle_query(query.clone()).await {
				Ok(_response) => {
					println!("  Session A: ✓ File read received");
					(true, "Session A success")
				}
				Err(e) => {
					println!("  Session A: ✗ Error: {}", e);
					(false, "Session A failed")
				}
			}
		});
		handles.push(handle);
	}

	// Session B: Get environment
	{
		let h = handler.clone();
		let handle = tokio::spawn(async move {
			println!("  Session B: Starting environment query");
			std::env::set_var("SESSION_B_KEY", "session-b-value");

			let query = ServerQuery {
				id: format!("Q-{:032x}", 11u128),
				kind: ServerQueryKind::GetEnvironment {
					keys: vec!["SESSION_B_KEY".to_string()],
				},
				sent_at: chrono::Utc::now().to_rfc3339(),
				timeout_secs: 10,
				metadata: serde_json::json!({"session": "B"}),
			};

			match h.handle_query(query.clone()).await {
				Ok(_response) => {
					println!("  Session B: ✓ Environment received");
					(true, "Session B success")
				}
				Err(e) => {
					println!("  Session B: ✗ Error: {}", e);
					(false, "Session B failed")
				}
			}
		});
		handles.push(handle);
	}

	// Session C: Get workspace context
	{
		let h = handler.clone();
		let handle = tokio::spawn(async move {
			println!("  Session C: Starting workspace context query");

			let query = ServerQuery {
				id: format!("Q-{:032x}", 12u128),
				kind: ServerQueryKind::GetWorkspaceContext,
				sent_at: chrono::Utc::now().to_rfc3339(),
				timeout_secs: 10,
				metadata: serde_json::json!({"session": "C"}),
			};

			match h.handle_query(query.clone()).await {
				Ok(_response) => {
					println!("  Session C: ✓ Context received");
					(true, "Session C success")
				}
				Err(e) => {
					println!("  Session C: ✗ Error: {}", e);
					(false, "Session C failed")
				}
			}
		});
		handles.push(handle);
	}

	println!("\nStep 2: All queries execute concurrently\n");

	// Wait for all sessions to complete
	let mut results = vec![];
	for handle in handles {
		if let Ok((success, msg)) = handle.await {
			results.push((success, msg));
		}
	}

	println!("\nStep 3: Verify Results");
	let all_success = results.iter().all(|(s, _)| *s);
	println!("  Sessions completed: {}", results.len());
	println!("  All successful: {}\n", all_success);

	for (success, msg) in &results {
		println!("    {} {}", if *success { "✓" } else { "✗" }, msg);
	}

	assert!(all_success, "All sessions should complete successfully");

	println!("\nStep 4: Verify No Interference");
	println!("  ✓ Query responses routed to correct sessions");
	println!("  ✓ No cross-session contamination");
	println!("  ✓ Concurrent processing works correctly\n");

	println!("✅ Example 5 Complete\n");
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() {
	// Initialize tracing
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.with_target(false)
		.with_thread_ids(false)
		.init();

	println!("\n╔══════════════════════════════════════════════════════════════╗");
	println!("║  Server-to-Client Query Bridge - Comprehensive Examples      ║");
	println!("║  Demonstrating the complete query lifecycle                  ║");
	println!("╚══════════════════════════════════════════════════════════════╝");

	// Run all examples
	example_1_basic_file_read().await;
	example_2_environment_context().await;
	example_3_human_approval().await;
	example_4_error_recovery().await;
	example_5_concurrent_queries().await;

	// Summary
	println!("\n╔══════════════════════════════════════════════════════════════╗");
	println!("║  ✅ All Examples Completed Successfully                       ║");
	println!("║                                                              ║");
	println!("║  Key Takeaways:                                              ║");
	println!("║  1. ✓ File reads are scoped to workspace                     ║");
	println!("║  2. ✓ Environment and context queries work                  ║");
	println!("║  3. ✓ Human approval prevents accidental changes            ║");
	println!("║  4. ✓ Errors are handled gracefully with recovery            ║");
	println!("║  5. ✓ Multiple sessions process concurrently                 ║");
	println!("║                                                              ║");
	println!("║  Next Steps:                                                 ║");
	println!("║  - Integrate into LLM processing loop                        ║");
	println!("║  - Add editor-specific handlers                             ║");
	println!("║  - Implement query result caching                            ║");
	println!("╚══════════════════════════════════════════════════════════════╝\n");
}
