# Integration Guide: Query Bridge into Existing LLM Loop

**Purpose:** Step-by-step guide to integrate the Query Bridge into your existing LLM processing system  
**Audience:** Backend engineers, system architects  
**Duration:** 2-4 hours  
**Prerequisites:** Phase 1 & 2 specs complete, Rust knowledge

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Integration Points](#integration-points)
3. [Step-by-Step Integration](#step-by-step-integration)
4. [Code Walkthrough](#code-walkthrough)
5. [Configuration](#configuration)
6. [Testing Checklist](#testing-checklist)
7. [Deployment Guide](#deployment-guide)
8. [Rollback Strategy](#rollback-strategy)

---

## Architecture Overview

### Query Bridge in Your LLM Loop

```
┌─────────────────┐
│  User Input     │
└────────┬────────┘
         │
    ┌────▼─────────────────────────────┐
    │  LLM Processing Loop              │
    │  ┌─────────────────────────────┐  │
    │  │ 1. Generate LLM Output      │  │
    │  └────────────┬────────────────┘  │
    │               │                    │
    │  ┌────────────▼────────────────┐  │
    │  │ 2. Extract Query (NEW)      │  │
    │  └────────────┬────────────────┘  │
    │               │                    │
    │  ┌────────────▼────────────────┐  │
    │  │ 3. Dispatch Query (NEW)     │  │
    │  └────────────┬────────────────┘  │
    │               │                    │
    │  ┌────────────▼────────────────┐  │
    │  │ 4. Restore Context (NEW)    │  │
    │  └────────────┬────────────────┘  │
    │               │                    │
    │  ┌────────────▼────────────────┐  │
    │  │ 5. Continue LLM             │  │
    │  └────────────┬────────────────┘  │
    │               │                    │
    └───────────────┼────────────────────┘
                    │
         ┌──────────▼──────────┐
         │  Final Response     │
         └─────────────────────┘
```

### Component Responsibilities

| Component | Owned By | Responsibility |
|-----------|----------|-----------------|
| `LlmQueryExtractor` | Agent | Detect queries in LLM output |
| `QueryDispatcher` | Server | Send/receive queries with timeout |
| `LlmContextRestorer` | Agent | Maintain conversation history |
| `ServerQueryManager` | Server | Manage query state & responses |
| `AcpServerQueryHandler` | Client | Handle queries from server |

---

## Integration Points

### 1. LLM Client Integration

**File:** `crates/loom-acp/src/agent.rs`

The agent receives LLM responses and needs to:
- Check if response contains a query
- If yes: dispatch it and resume
- If no: return final response

### 2. Server Query Manager

**File:** `crates/loom-server/src/server_query.rs`

Manages:
- Pending queries waiting for response
- Response correlation by query_id
- Timeout enforcement
- Per-session rate limiting

### 3. HTTP Response Handler

**File:** `crates/loom-server/src/routes.rs` (new)

Provides endpoint:
- `POST /v1/sessions/{id}/query-response` - Client sends response

### 4. Client Handler

**File:** `crates/loom-acp/src/agent.rs`

Implements:
- `ServerQueryHandler` trait
- File reading with path validation
- Environment access control
- Workspace context gathering

---

## Step-by-Step Integration

### Step 1: Initialize Query Components (30 min)

**File:** `crates/loom-server/src/state.rs` or equivalent server initialization

```rust
use loom_server::ServerQueryManager;
use loom_acp::{LlmQueryExtractor, QueryDispatcher};

pub struct AppState {
    pub query_manager: Arc<ServerQueryManager>,
    pub query_dispatcher: Arc<QueryDispatcher>,
}

pub fn create_app_state() -> AppState {
    let query_manager = Arc::new(ServerQueryManager::new());
    let query_dispatcher = Arc::new(QueryDispatcher::new(
        (*query_manager).clone()
    ));

    AppState {
        query_manager,
        query_dispatcher,
    }
}
```

**Configuration:**
```toml
[query_bridge]
enabled = true
default_timeout_secs = 30
max_queries_per_session = 20
extraction_cache_size = 1000
```

### Step 2: Add Response Handler Endpoint (30 min)

**File:** `crates/loom-server/src/routes/query_response.rs` (new)

```rust
use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use loom_core::ServerQueryResponse;

#[derive(serde::Deserialize)]
pub struct QueryResponsePayload {
    pub query_id: String,
    pub sent_at: String,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

pub async fn handle_query_response(
    State(state): State<Arc<AppState>>,
    Path((session_id,)): Path<(String,)>,
    Json(payload): Json<QueryResponsePayload>,
) -> impl IntoResponse {
    tracing::debug!(
        session_id = %session_id,
        query_id = %payload.query_id,
        "Received query response"
    );

    let response = ServerQueryResponse {
        query_id: payload.query_id,
        sent_at: payload.sent_at,
        result: payload.result,
        error: payload.error,
    };

    match state.query_manager.receive_response(&session_id, response).await {
        Ok(_) => {
            tracing::info!("Query response recorded");
            (StatusCode::OK, "Response received")
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to record response");
            (StatusCode::BAD_REQUEST, "Failed to record response")
        }
    }
}

// Register route
pub fn register_routes(router: Router) -> Router {
    router.post(
        "/v1/sessions/:session_id/query-response",
        handle_query_response
    )
}
```

### Step 3: Integrate into Agent Processing (1 hour)

**File:** `crates/loom-acp/src/agent.rs`

Modify the main agent loop:

```rust
pub struct Agent {
    // ... existing fields
    query_extractor: LlmQueryExtractor,
    context_restorer: LlmContextRestorer,
}

impl Agent {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            // ... existing init
            query_extractor: LlmQueryExtractor::new(),
            context_restorer: LlmContextRestorer::new(100),
        }
    }

    /// Main processing loop with query support
    pub async fn process_llm_output(
        &mut self,
        llm_output: &str,
        session_id: &str,
    ) -> Result<ProcessingResult, AgentError> {
        // Step 1: Try to extract query
        if let Some(query) = self.query_extractor.extract(llm_output) {
            tracing::info!(
                query_id = %query.id,
                kind = ?query.kind,
                "Extracted query from LLM output"
            );

            // Step 2: Checkpoint state
            self.context_restorer.checkpoint(llm_output);

            // Step 3: Dispatch query
            match self.dispatch_query(session_id, query).await {
                Ok(response) => {
                    // Step 4: Resume with response
                    let resumption = self.context_restorer
                        .resume_with_response(&response);
                    
                    Ok(ProcessingResult::QueryProcessed {
                        resumption,
                        should_continue: true,
                    })
                }
                Err(e) => {
                    // Step 5: Resume with error
                    let error_msg = self.context_restorer
                        .resume_with_error(&e.to_string());
                    
                    Ok(ProcessingResult::QueryFailed {
                        error_msg,
                        should_continue: true,
                    })
                }
            }
        } else {
            // No query extracted - LLM output is final
            Ok(ProcessingResult::Final {
                output: llm_output.to_string(),
            })
        }
    }

    async fn dispatch_query(
        &self,
        session_id: &str,
        query: ServerQuery,
    ) -> Result<ServerQueryResponse, DispatchError> {
        // Use dispatcher to send query and wait for response
        self.query_dispatcher.dispatch(session_id, query).await
    }
}

#[derive(Debug)]
pub enum ProcessingResult {
    Final { output: String },
    QueryProcessed { resumption: String, should_continue: bool },
    QueryFailed { error_msg: String, should_continue: bool },
}
```

### Step 4: Update Main Processing Loop (30 min)

**File:** `crates/loom-acp/src/main.rs` or equivalent

```rust
pub async fn run_conversation(
    mut agent: Agent,
    mut llm: LlmClient,
    session_id: &str,
) -> Result<ConversationOutput> {
    let mut iteration = 0;
    const MAX_ITERATIONS: usize = 10; // Prevent infinite loops

    loop {
        iteration += 1;
        if iteration > MAX_ITERATIONS {
            tracing::warn!("Max iterations reached");
            return Err(AgentError::MaxIterationsExceeded);
        }

        // Generate LLM output
        let llm_output = llm.generate().await?;
        tracing::debug!(
            iteration = iteration,
            output_len = llm_output.len(),
            "LLM generated output"
        );

        // Process output (including query extraction)
        match agent.process_llm_output(&llm_output, session_id).await? {
            ProcessingResult::Final { output } => {
                tracing::info!("Conversation complete");
                return Ok(ConversationOutput {
                    final_response: output,
                    total_iterations: iteration,
                });
            }
            ProcessingResult::QueryProcessed { resumption, should_continue } => {
                if !should_continue {
                    return Ok(ConversationOutput {
                        final_response: resumption,
                        total_iterations: iteration,
                    });
                }

                // Add resumption text back to LLM context
                llm.add_system_message(&resumption);
                tracing::debug!(
                    iteration = iteration,
                    "Resuming LLM with query response"
                );
            }
            ProcessingResult::QueryFailed { error_msg, should_continue } => {
                if !should_continue {
                    return Err(AgentError::QueryProcessingFailed(error_msg));
                }

                // Continue with error message
                llm.add_system_message(&error_msg);
                tracing::warn!(
                    iteration = iteration,
                    error = %error_msg,
                    "Query failed, continuing with error message"
                );
            }
        }
    }
}
```

### Step 5: Update Client Handler (20 min)

**File:** `crates/loom-acp/src/agent.rs` - Query Handler Implementation

```rust
use loom_core::ServerQueryHandler;
use std::path::PathBuf;

pub struct AcpServerQueryHandler {
    workspace_root: PathBuf,
}

#[async_trait::async_trait]
impl ServerQueryHandler for AcpServerQueryHandler {
    async fn handle_query(
        &self,
        query: loom_core::ServerQuery,
    ) -> Result<loom_core::ServerQueryResponse, loom_core::ServerQueryError> {
        tracing::debug!(query_id = %query.id, "Handling query");

        let result = match query.kind {
            ServerQueryKind::ReadFile { path } => {
                self.handle_read_file(&path).await
            }
            ServerQueryKind::GetEnvironment { keys } => {
                self.handle_get_environment(&keys).await
            }
            ServerQueryKind::GetWorkspaceContext => {
                self.handle_get_workspace_context().await
            }
            ServerQueryKind::RequestUserInput { .. } => {
                // CLI doesn't support user input
                Err(ServerQueryError::Unsupported(
                    "RequestUserInput not supported in CLI mode".into()
                ))
            }
            _ => {
                Err(ServerQueryError::Unsupported(
                    format!("Unsupported query type: {:?}", query.kind)
                ))
            }
        };

        Ok(ServerQueryResponse {
            query_id: query.id,
            sent_at: now_rfc3339(),
            result: result.ok(),
            error: result.err().map(|e| e.to_string()),
        })
    }
}
```

---

## Code Walkthrough

### Complete Integration Example

```rust
// In your LLM handler endpoint
pub async fn handle_llm_request(
    State(app_state): State<Arc<AppState>>,
    session_id: String,
    user_message: String,
) -> Result<Response, Error> {
    // 1. Initialize components
    let agent = Agent::new(app_state.workspace_root.clone());
    let mut llm_client = app_state.create_llm_client(&session_id).await?;

    // 2. Add user message
    llm_client.add_user_message(&user_message);

    // 3. Run conversation with query support
    let result = run_conversation(
        agent,
        llm_client,
        &session_id,
    ).await?;

    Ok(Response {
        output: result.final_response,
        iterations: result.total_iterations,
    })
}
```

### Query Extraction Pattern Matching

```rust
pub fn extract_common_patterns(llm_output: &str) -> Vec<ServerQuery> {
    let mut queries = Vec::new();

    // Pattern 1: File reads
    if let Some(file_path) = extract_file_pattern(llm_output) {
        queries.push(ServerQuery::read_file(&file_path));
    }

    // Pattern 2: Environment access
    if let Some(env_keys) = extract_env_pattern(llm_output) {
        queries.push(ServerQuery::get_environment(env_keys));
    }

    // Pattern 3: Workspace context
    if extract_workspace_pattern(llm_output) {
        queries.push(ServerQuery::workspace_context());
    }

    queries
}

fn extract_file_pattern(text: &str) -> Option<String> {
    let regex = regex::Regex::new(
        r"(?:read|check|examine|review).*['\"]?([^\s'\"]+\.(?:rs|toml|json|md))['\"]?"
    ).ok()?;
    
    regex.captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}
```

---

## Configuration

### Server Configuration

**File:** `config.toml` or environment variables

```toml
[query_bridge]
enabled = true
default_timeout_secs = 30
max_queries_per_session = 20
max_query_size_bytes = 10485760  # 10MB
response_timeout_secs = 60

[query_bridge.extraction]
enabled = true
cache_size = 1000
pattern_timeout_ms = 100

[query_bridge.rate_limiting]
enabled = true
per_session_per_min = 100
per_type = { read_file = 10, get_environment = 50 }
```

### Client Configuration

```toml
[acp.query_handler]
enabled = true
workspace_root = "/home/user/project"
max_file_size_bytes = 10485760
allowed_file_extensions = ["rs", "toml", "json", "md"]
blocked_paths = [".git", ".env", "target"]
```

---

## Testing Checklist

### Unit Tests

- [ ] Query extraction from various LLM outputs
- [ ] Context restoration preserves history
- [ ] Timeout enforcement works correctly
- [ ] Error messages format correctly
- [ ] Path validation prevents escape attacks

### Integration Tests

- [ ] Full query→response→resume flow works
- [ ] Multiple sequential queries work
- [ ] Query timeout triggers error resume
- [ ] File not found handled gracefully
- [ ] Environment variables returned correctly

### End-to-End Tests

- [ ] Full LLM conversation with file reads
- [ ] Error recovery and retry logic
- [ ] Multi-session concurrent queries
- [ ] Memory doesn't leak over time
- [ ] Performance meets SLA

### Test Commands

```bash
# Run query-specific tests
cargo test -p loom-acp query_extractor
cargo test -p loom-server query_dispatcher
cargo test -p loom-acp agent::integration

# Run with logging
RUST_LOG=debug cargo test -- --nocapture

# Run benchmarks
cargo bench --bench query_dispatch
```

---

## Deployment Guide

### Pre-Deployment Checklist

- [ ] All tests passing
- [ ] Load testing completed
- [ ] Security audit done
- [ ] Documentation updated
- [ ] Rollback plan documented

### Deployment Steps

1. **Stage 1: Feature Flag (Day 1)**
   ```rust
   if config.query_bridge.enabled {
       // Enable query processing
   } else {
       // Disable - all queries return NotImplemented
   }
   ```

2. **Stage 2: Canary Deployment (Days 2-3)**
   - Deploy to 5% of sessions
   - Monitor error rate
   - Monitor latency
   - Check for memory leaks

3. **Stage 3: Gradual Rollout (Days 4-7)**
   - Increase to 25% → 50% → 100%
   - Monitor at each step
   - Be ready to roll back

4. **Stage 4: Full Production (Day 8+)**
   - All sessions enabled
   - Monitor metrics continuously

### Monitoring During Deployment

```rust
// Track these metrics
metrics::counter!("query.extracted", 1);
metrics::histogram!("query.latency_ms", duration);
metrics::gauge!("queries.pending", count);
metrics::counter!("query.timeout", 1);
metrics::counter!("query.error", 1);
```

---

## Rollback Strategy

### If Issues Arise

**Fast Rollback (< 5 minutes):**
```toml
[query_bridge]
enabled = false  # Disables all query processing
```

**Graceful Degradation:**
```rust
// If query dispatcher fails, continue without queries
match agent.process_llm_output(output, session_id).await {
    Ok(result) => handle_result(result),
    Err(e) if config.query_bridge.graceful_degradation => {
        // Return final output without processing queries
        handle_error_gracefully(&output)
    }
    Err(e) => return Err(e),
}
```

### Monitoring Alerts

Set up alerts for:
- Error rate > 5%
- P99 latency > 1000ms
- Timeout rate > 10%
- Memory usage spike > 20%

---

## Summary

Query Bridge integration transforms your LLM loop from a simple request→response model into an interactive, context-aware system. Key accomplishments:

- ✅ Automatic query extraction
- ✅ Transparent dispatch without breaking LLM flow
- ✅ Conversation state preservation
- ✅ Robust error handling
- ✅ Production-ready monitoring

**Next Steps:**
1. Run integration tests locally
2. Stage deployment to canary
3. Monitor metrics continuously
4. Gather feedback from users
