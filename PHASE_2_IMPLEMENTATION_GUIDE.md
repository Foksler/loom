# Phase 2 Implementation Guide: LLM Query Integration

> **Status:** Phase 2 Planning & Implementation  
> **Target Duration:** 4-6 hours  
> **Complexity:** Medium  
> **Dependencies:** Phase 1 Complete ✅

## Table of Contents
1. [Overview](#overview)
2. [Architecture Changes](#architecture-changes)
3. [Integration Flow](#integration-flow)
4. [Implementation Steps](#implementation-steps)
5. [Code Examples](#code-examples)
6. [Migration Guide](#migration-guide)
7. [Debugging Phase 2](#debugging-phase-2)
8. [Performance Tuning](#performance-tuning)
9. [Testing Strategy](#testing-strategy)
10. [Troubleshooting](#troubleshooting)

---

## Overview

### What's New in Phase 2

Phase 2 integrates the **Server-to-Client Query Bridge** into the **LLM processing loop**. This allows:

- **LLM reads files** during reasoning: "I'll check src/main.rs to understand the structure"
- **LLM gets environment** to access deployment config
- **LLM requests user input** for critical decisions
- **LLM context aware** of workspace state and git branch

### Phase 1 vs Phase 2

| Aspect | Phase 1 | Phase 2 |
|--------|---------|---------|
| **Framework** | ✅ Core types, manager, handler | ✅ Integrated into LLM processing |
| **Testing** | 19 unit/integration tests | + property-based tests for LLM flows |
| **Trigger** | Manual queries | LLM output analysis → automatic extraction |
| **Error Recovery** | Basic timeout handling | Retry logic, context restoration |
| **Performance** | Single query | Pipeline optimization, batching |
| **Documentation** | 4 guides | + Real-world examples, troubleshooting |

### Key Goals
1. ✅ Extract query intent from LLM output
2. ✅ Send queries transparently without breaking LLM flow
3. ✅ Resume LLM with query results
4. ✅ Handle failures gracefully
5. ✅ Maintain conversation continuity

---

## Architecture Changes

### New Components

#### 1. **LLM Query Extractor**
Analyzes LLM output to detect queries:
```
LLM Output: "I'll read the config file to understand deployment..."
              ↓
         [QueryExtractor]
              ↓
         Query: ReadFile { path: "deploy.conf" }
```

#### 2. **Query Dispatcher**
Routes queries through the bridge:
```
Query → [SessionManager] → [ClientHandler] → Client Response
         ↓
    [AcpServerQueryHandler]
```

#### 3. **Context Restorer**
Maintains LLM conversation state:
```
LLM State → [Checkpoint] → Query Interrupt → [Restore] → Resume
```

### Modified Components

```
┌─────────────┐
│   LLM       │  Phase 1: Direct output
│  Processing │  Phase 2: Query-aware processing
└──────┬──────┘
       │
       ├─→ [QueryExtractor] → Detect intent
       │        ↓
       │   Is Query? → Yes → [QueryDispatcher]
       │        ↓                ↓
       │       No          [SessionManager]
       │        ↓                ↓
       └──→ [Continue LLM]  [ClientHandler]
                                ↓
                           [Response]
                                ↓
                         [Context Restorer]
                                ↓
                          [Resume LLM]
```

---

## Integration Flow

### Happy Path: File Read Query

```
1. LLM generates: "To fix the bug, I need to examine src/lib.rs"
2. Server extracts: ReadFile { path: "src/lib.rs" }
3. Server sends query via SSE to client
4. Client reads file → responds via HTTP
5. Server captures response
6. Server resumes LLM: "I found the code:\n<file_content>"
7. LLM continues analysis with new context
```

### Error Path: File Not Found

```
1. LLM generates: "Let me check missing_file.rs"
2. Server extracts: ReadFile { path: "missing_file.rs" }
3. Server sends query → client responds with error
4. Server resumes LLM: "File not found. Let's try another approach..."
5. LLM adapts strategy
```

### Timeout Path: User Input

```
1. LLM generates: "I should ask for approval"
2. Server extracts: RequestUserInput { ... }
3. Server sends query → client times out (60s)
4. Server catches timeout
5. Server resumes LLM: "User didn't respond. Proceeding with defaults..."
6. LLM continues with fallback
```

---

## Implementation Steps

### Step 1: Add Query Extraction Logic (1 hour)

**File:** `crates/loom-acp/src/query_extractor.rs`

```rust
use regex::Regex;
use serde_json::json;

pub struct LlmQueryExtractor {
    patterns: Vec<QueryPattern>,
}

impl LlmQueryExtractor {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                QueryPattern::file_read(),
                QueryPattern::env_vars(),
                QueryPattern::user_input(),
                QueryPattern::workspace_context(),
            ],
        }
    }

    /// Try to extract a query from LLM output
    /// Returns None if no query pattern matched
    pub fn extract(&self, llm_output: &str) -> Option<ServerQuery> {
        for pattern in &self.patterns {
            if let Some(query) = pattern.match_query(llm_output) {
                return Some(query);
            }
        }
        None
    }
}

pub struct QueryPattern {
    regex: Regex,
    kind_fn: fn(captures: &regex::Captures) -> ServerQueryKind,
}

impl QueryPattern {
    fn file_read() -> Self {
        Self {
            regex: Regex::new(
                r"(?:read|check|examine|review).*(?:file|path).*['\"]?([^\s'\"]+)['\"]?"
            ).unwrap(),
            kind_fn: |caps| ServerQueryKind::ReadFile {
                path: caps[1].to_string(),
            },
        }
    }

    fn env_vars() -> Self {
        Self {
            regex: Regex::new(
                r"(?:get|check|fetch).*(?:env|environment|variable).*['\"]?([A-Z_]+)['\"]?"
            ).unwrap(),
            kind_fn: |caps| ServerQueryKind::GetEnvironment {
                keys: vec![caps[1].to_string()],
            },
        }
    }

    fn user_input() -> Self {
        Self {
            regex: Regex::new(
                r"(?:ask|request|prompt).*user.*(?:approval|confirm|input)"
            ).unwrap(),
            kind_fn: |_| ServerQueryKind::RequestUserInput {
                prompt: "Proceed?".to_string(),
                input_type: "yes_no".to_string(),
                options: Some(vec!["yes".into(), "no".into()]),
            },
        }
    }

    fn workspace_context() -> Self {
        Self {
            regex: Regex::new(r"(?:check|see).*workspace.*(?:state|context|branch)").unwrap(),
            kind_fn: |_| ServerQueryKind::GetWorkspaceContext,
        }
    }

    pub fn match_query(&self, text: &str) -> Option<ServerQuery> {
        if let Some(caps) = self.regex.find(text) {
            let kind = (self.kind_fn)(&self.regex.captures(text).unwrap());
            Some(ServerQuery {
                id: generate_query_id(),
                kind,
                sent_at: now_rfc3339(),
                timeout_secs: 30,
                metadata: json!({}),
            })
        } else {
            None
        }
    }
}
```

**Tests to add:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_extract_file_read_query() {
        let extractor = LlmQueryExtractor::new();
        let output = "I need to read src/main.rs to understand the entry point.";
        
        let query = extractor.extract(output);
        assert!(query.is_some());
        
        if let Some(q) = query {
            matches!(q.kind, ServerQueryKind::ReadFile { path } if path == "src/main.rs");
        }
    }

    proptest! {
        #[test]
        fn prop_valid_query_id_format(query in any::<String>()) {
            let extractor = LlmQueryExtractor::new();
            if let Some(q) = extractor.extract(&query) {
                prop_assert!(q.id.starts_with("Q-"));
            }
        }
    }
}
```

### Step 2: Add Query Dispatcher (1.5 hours)

**File:** `crates/loom-server/src/query_dispatcher.rs`

```rust
use crate::ServerQueryManager;
use loom_core::{ServerQuery, ServerQueryResponse};
use std::time::Duration;
use tokio::time::timeout;

pub struct QueryDispatcher {
    manager: ServerQueryManager,
    default_timeout: Duration,
}

impl QueryDispatcher {
    pub fn new(manager: ServerQueryManager) -> Self {
        Self {
            manager,
            default_timeout: Duration::from_secs(30),
        }
    }

    /// Send query and wait for response
    /// Returns response or error if timeout
    pub async fn dispatch(
        &self,
        session_id: &str,
        query: ServerQuery,
    ) -> Result<ServerQueryResponse, DispatchError> {
        let timeout_duration = Duration::from_secs(query.timeout_secs as u64);

        let result = timeout(
            timeout_duration,
            self.manager.send_query(session_id, query.clone()),
        )
        .await;

        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => Err(DispatchError::QueryFailed(e.to_string())),
            Err(_) => Err(DispatchError::Timeout),
        }
    }

    /// Dispatch multiple queries in sequence
    pub async fn dispatch_batch(
        &self,
        session_id: &str,
        queries: Vec<ServerQuery>,
    ) -> Vec<Result<ServerQueryResponse, DispatchError>> {
        let mut results = Vec::new();
        for query in queries {
            let result = self.dispatch(session_id, query).await;
            results.push(result);
        }
        results
    }
}

#[derive(Debug)]
pub enum DispatchError {
    Timeout,
    QueryFailed(String),
    SessionNotFound(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "query timed out"),
            Self::QueryFailed(e) => write!(f, "query failed: {}", e),
            Self::SessionNotFound(id) => write!(f, "session not found: {}", id),
        }
    }
}
```

### Step 3: Add Context Restoration (1.5 hours)

**File:** `crates/loom-acp/src/context_restorer.rs`

```rust
use loom_core::ServerQueryResponse;
use serde_json::{json, Value};

pub struct LlmContextRestorer {
    conversation_history: Vec<ConversationTurn>,
    max_history: usize,
}

pub struct ConversationTurn {
    pub role: String,
    pub content: String,
    pub metadata: Option<Value>,
}

impl LlmContextRestorer {
    pub fn new(max_history: usize) -> Self {
        Self {
            conversation_history: Vec::new(),
            max_history,
        }
    }

    /// Checkpoint current LLM state before query
    pub fn checkpoint(&mut self, llm_state: &str) {
        self.conversation_history.push(ConversationTurn {
            role: "assistant".to_string(),
            content: llm_state.to_string(),
            metadata: Some(json!({ "type": "checkpoint" })),
        });
    }

    /// Resume LLM with query response
    pub fn resume_with_response(
        &mut self,
        response: &ServerQueryResponse,
    ) -> String {
        let response_text = self.format_response(response);
        
        self.conversation_history.push(ConversationTurn {
            role: "system".to_string(),
            content: format!("Query Response:\n{}", response_text),
            metadata: Some(json!({
                "type": "query_response",
                "query_id": response.query_id,
            })),
        });

        response_text
    }

    /// Resume with error message
    pub fn resume_with_error(&mut self, error: &str) -> String {
        let error_text = format!("Query failed: {}", error);
        
        self.conversation_history.push(ConversationTurn {
            role: "system".to_string(),
            content: error_text.clone(),
            metadata: Some(json!({ "type": "query_error" })),
        });

        error_text
    }

    fn format_response(&self, response: &ServerQueryResponse) -> String {
        match &response.result {
            ServerQueryResult::FileContent(content) => {
                format!("File content:\n```\n{}\n```", content)
            }
            ServerQueryResult::Environment(vars) => {
                let items: Vec<_> = vars
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                format!("Environment variables:\n{}", items.join("\n"))
            }
            ServerQueryResult::UserInput(input) => {
                format!("User input: {}", input)
            }
            ServerQueryResult::WorkspaceContext(ctx) => {
                format!("Workspace context:\n{}", serde_json::to_string_pretty(ctx).unwrap_or_default())
            }
            _ => "Query response received".to_string(),
        }
    }

    /// Get full conversation history for context
    pub fn get_history(&self) -> &[ConversationTurn] {
        &self.conversation_history
    }

    /// Clear history (useful for session cleanup)
    pub fn clear(&mut self) {
        self.conversation_history.clear();
    }
}
```

### Step 4: Integrate into LLM Processing Loop (1 hour)

**File:** `crates/loom-acp/src/agent.rs` (modifications)

```rust
// Add these to the agent's process method
pub async fn process_llm_output(
    &mut self,
    llm_output: &str,
    session_id: &str,
) -> Result<String, AgentError> {
    // Step 1: Try to extract query from LLM output
    if let Some(query) = self.query_extractor.extract(llm_output) {
        tracing::info!(
            query_id = %query.id,
            kind = ?query.kind,
            "Extracted query from LLM output"
        );

        // Step 2: Checkpoint current state
        self.context_restorer.checkpoint(llm_output);

        // Step 3: Dispatch query to client
        match self.query_dispatcher.dispatch(session_id, query.clone()).await {
            Ok(response) => {
                // Step 4: Resume with response
                let resumption_text = self.context_restorer.resume_with_response(&response);
                tracing::info!(
                    query_id = %response.query_id,
                    "Query completed, resuming LLM"
                );
                Ok(resumption_text)
            }
            Err(e) => {
                // Step 5: Handle error and resume
                let error_text = self.context_restorer.resume_with_error(&e.to_string());
                tracing::warn!(
                    error = %e,
                    "Query dispatch failed"
                );
                Ok(error_text)
            }
        }
    } else {
        // No query extracted, LLM output is final
        Ok(llm_output.to_string())
    }
}
```

### Step 5: Add Integration Tests (1 hour)

**File:** `crates/loom-acp/tests/integration_llm_queries.rs`

```rust
#[tokio::test]
async fn test_llm_file_read_query_flow() {
    // Setup
    let workspace = TempDir::new().unwrap();
    let session_id = "test-session";
    
    // Create test file
    let test_file = workspace.path().join("test.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    let mut agent = create_test_agent(workspace.path());
    let llm_output = "I should read test.rs to understand the code.";

    // Act
    let result = agent.process_llm_output(llm_output, session_id).await;

    // Assert
    assert!(result.is_ok());
    let resumption = result.unwrap();
    assert!(resumption.contains("fn main()"));
}

#[tokio::test]
async fn test_llm_query_with_error() {
    let workspace = TempDir::new().unwrap();
    let session_id = "test-session";
    
    let mut agent = create_test_agent(workspace.path());
    let llm_output = "Let me check missing.rs for the code.";

    let result = agent.process_llm_output(llm_output, session_id).await;

    assert!(result.is_ok());
    let resumption = result.unwrap();
    assert!(resumption.contains("Query failed") || resumption.contains("not found"));
}

#[tokio::test]
async fn test_conversation_history_preserved() {
    let workspace = TempDir::new().unwrap();
    let session_id = "test-session";
    let test_file = workspace.path().join("test.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    let mut agent = create_test_agent(workspace.path());
    
    // First query
    let output1 = "I should read test.rs";
    agent.process_llm_output(output1, session_id).await.unwrap();

    // Verify history has checkpoint
    let history = agent.context_restorer.get_history();
    assert!(history.iter().any(|t| {
        t.metadata
            .as_ref()
            .and_then(|m| m.get("type"))
            .and_then(|t| t.as_str())
            == Some("checkpoint")
    }));
}
```

---

## Code Examples

### Example 1: Extract & Dispatch Query

```rust
// In your LLM processing handler
let extractor = LlmQueryExtractor::new();
let dispatcher = QueryDispatcher::new(manager);

let llm_output = "I'll examine src/main.rs to find the entry point.";

if let Some(query) = extractor.extract(llm_output) {
    match dispatcher.dispatch("session-123", query).await {
        Ok(response) => {
            println!("Got response: {:?}", response.result);
            // Resume LLM with response
        }
        Err(e) => {
            eprintln!("Query failed: {}", e);
            // Resume LLM with error message
        }
    }
}
```

### Example 2: Batch Queries

```rust
let queries = vec![
    ServerQuery::read_file("src/main.rs"),
    ServerQuery::read_file("src/lib.rs"),
    ServerQuery::get_environment(vec!["RUST_LOG"]),
];

let results = dispatcher.dispatch_batch("session-123", queries).await;

for (query, result) in queries.iter().zip(results.iter()) {
    match result {
        Ok(response) => println!("✓ {:?}", query),
        Err(e) => println!("✗ {:?}: {}", query, e),
    }
}
```

### Example 3: Error Handling in LLM Loop

```rust
pub async fn llm_processing_with_queries(
    llm: &mut LlmClient,
    agent: &mut Agent,
    session_id: &str,
) -> Result<String, Error> {
    loop {
        let llm_output = llm.generate().await?;

        match agent.process_llm_output(&llm_output, session_id).await {
            Ok(resumption) => {
                if resumption.is_empty() {
                    // LLM finished (no query)
                    return Ok(llm_output);
                } else {
                    // Query was processed, feed resumption back to LLM
                    llm.add_system_message(&resumption);
                }
            }
            Err(e) => {
                tracing::error!("Query processing failed: {}", e);
                // Optionally retry or abort
                return Err(e);
            }
        }
    }
}
```

---

## Migration Guide

### From Phase 1 to Phase 2

#### 1. Update Dependencies

```toml
[dependencies]
loom-core = { path = "../loom-core", features = ["query-bridge"] }
loom-server = { path = "../loom-server", features = ["query-bridge"] }
```

#### 2. Initialize Components

```rust
// Before: Just manager
let manager = ServerQueryManager::new();

// After: Full pipeline
let manager = ServerQueryManager::new();
let dispatcher = QueryDispatcher::new(manager);
let extractor = LlmQueryExtractor::new();
let restorer = LlmContextRestorer::new(100);
```

#### 3. Update LLM Processing

```rust
// Before: Direct LLM output
let output = llm.generate().await?;
return Ok(output);

// After: Query-aware processing
let output = llm.generate().await?;
let resumption = agent.process_llm_output(&output, session_id).await?;

if resumption.is_empty() {
    // LLM finished
    return Ok(output);
} else {
    // Query was processed, continue LLM
    llm.add_message(&resumption);
}
```

#### 4. Update Tests

```rust
// Before: Manual query creation
let query = ServerQuery { ... };
let response = manager.send_query(session_id, query).await?;

// After: Extracted from LLM output
let output = "I'll read main.rs";
let response = agent.process_llm_output(output, session_id).await?;
```

---

## Debugging Phase 2

### Enable Debug Logging

```bash
RUST_LOG=loom_acp::query_extractor=debug,\
loom_server::query_dispatcher=debug,\
loom_acp::context_restorer=debug \
cargo run
```

### Log Examples

```
[DEBUG] loom_acp::query_extractor: Extracted query from LLM output
    query_id: "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0"
    kind: ReadFile { path: "src/main.rs" }

[DEBUG] loom_server::query_dispatcher: Dispatching query
    session_id: "session-123"
    timeout_secs: 30

[DEBUG] loom_acp::context_restorer: Resuming LLM with response
    query_id: "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0"
    result: FileContent(String) (size: 1024 bytes)
```

### Common Issues

| Issue | Root Cause | Fix |
|-------|-----------|-----|
| **No queries extracted** | Regex patterns too strict | Loosen patterns, add more variations |
| **Timeouts** | Client not responding | Check client connection, increase timeout |
| **LLM loops** | No termination condition | Add max iteration count |
| **Lost context** | History truncation | Increase `max_history` in restorer |

---

## Performance Tuning

### Query Batching

```rust
// Instead of sequential queries
for file in files {
    let q = ServerQuery::read_file(file);
    dispatcher.dispatch(session_id, q).await?;
}

// Use batching
let queries = files.iter()
    .map(|f| ServerQuery::read_file(f))
    .collect();
let results = dispatcher.dispatch_batch(session_id, queries).await;
```

### Timeout Configuration

```rust
// Aggressive timeout for fast responses
let query = ServerQuery {
    timeout_secs: 5,
    ..default
};

// Relaxed timeout for slow operations
let query = ServerQuery {
    timeout_secs: 60,
    ..default
};
```

### Response Caching

```rust
pub struct CachingDispatcher {
    dispatcher: QueryDispatcher,
    cache: Arc<Mutex<HashMap<String, ServerQueryResponse>>>,
}

impl CachingDispatcher {
    pub async fn dispatch(
        &self,
        session_id: &str,
        query: ServerQuery,
    ) -> Result<ServerQueryResponse, DispatchError> {
        let cache_key = format!("{}:{:?}", session_id, query.kind);
        
        if let Some(cached) = self.cache.lock().await.get(&cache_key) {
            return Ok(cached.clone());
        }

        let response = self.dispatcher.dispatch(session_id, query).await?;
        self.cache.lock().await.insert(cache_key, response.clone());
        Ok(response)
    }
}
```

---

## Testing Strategy

### Unit Tests
- Query extraction from various LLM outputs
- Query dispatcher error handling
- Context restorer history management

### Integration Tests
- Full LLM → query → response → resume flow
- Multi-query sequences
- Error recovery scenarios

### Property-Based Tests
- Query ID format validity
- Timeout behavior correctness
- Response serialization roundtrips

### Load Tests
- Handle N concurrent queries
- Batch dispatch with large file sets
- Memory usage under sustained load

---

## Troubleshooting

### Query Not Extracted
1. Check regex patterns: `RUST_LOG=query_extractor=debug`
2. Add pattern for your LLM output format
3. Test pattern independently: `cargo test prop_extract`

### Timeout Errors
1. Verify client is connected: `curl /v1/sessions/{id}/queries`
2. Increase timeout_secs in query
3. Check network latency between server/client

### LLM Loops Forever
1. Add max iteration counter
2. Set stricter extraction patterns
3. Log query extraction: `RUST_LOG=query_extractor=debug`

### Lost Conversation History
1. Increase `LlmContextRestorer::max_history`
2. Check history size: `agent.context_restorer.get_history().len()`
3. Add history checkpoint logging

---

## Summary

Phase 2 transforms the query bridge from a **framework** into an **integrated feature** of the LLM processing pipeline. Key accomplishments:

- ✅ Automatic query extraction from LLM output
- ✅ Transparent query dispatch without breaking LLM flow
- ✅ Conversation state restoration for seamless continuation
- ✅ Robust error handling and recovery
- ✅ Comprehensive testing and debugging support

**Ready for implementation!** 🚀
