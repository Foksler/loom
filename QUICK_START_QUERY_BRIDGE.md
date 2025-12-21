# Quick Start: Server-to-Client Query Bridge

## What Was Built

A **complete query framework** allowing servers to request information from clients during LLM processing via ACP.

### Example: Server asks for file content
```
Server: "I need src/main.rs"
  ↓ (SSE event: type=server_query)
Client receives query
Client: (reads file)
Client: "Here's the content..."
  ↓ (HTTP POST: /query-response)
Server: (resumes with file contents)
```

---

## Files Created/Modified

### New Files
- [PLAN_SERVER_CLIENT_QUERY_BRIDGE.md](file:///home/ghuntley/loom/PLAN_SERVER_CLIENT_QUERY_BRIDGE.md) - Design spec
- [IMPLEMENTATION_SERVER_CLIENT_QUERY_BRIDGE.md](file:///home/ghuntley/loom/IMPLEMENTATION_SERVER_CLIENT_QUERY_BRIDGE.md) - Implementation details
- [crates/loom-core/src/server_query.rs](file:///home/ghuntley/loom/crates/loom-core/src/server_query.rs) - Core types
- [crates/loom-server/src/server_query.rs](file:///home/ghuntley/loom/crates/loom-server/src/server_query.rs) - Server manager

### Modified Files
- [crates/loom-core/src/lib.rs](file:///home/ghuntley/loom/crates/loom-core/src/lib.rs) - Exports
- [crates/loom-server/src/lib.rs](file:///home/ghuntley/loom/crates/loom-server/src/lib.rs) - Exports
- [crates/loom-server/src/api.rs](file:///home/ghuntley/loom/crates/loom-server/src/api.rs) - HTTP routes
- [crates/loom-acp/src/agent.rs](file:///home/ghuntley/loom/crates/loom-acp/src/agent.rs) - Handler impl
- [crates/loom-llm-proxy/src/types.rs](file:///home/ghuntley/loom/crates/loom-llm-proxy/src/types.rs) - SSE event
- [crates/loom-llm-proxy/src/stream.rs](file:///home/ghuntley/loom/crates/loom-llm-proxy/src/stream.rs) - Parser

---

## Core Types

### ServerQuery (Server → Client)
```rust
pub struct ServerQuery {
    pub id: String,                        // "Q-{uuid7}"
    pub kind: ServerQueryKind,             // ReadFile, GetEnvironment, etc.
    pub sent_at: String,                   // RFC3339
    pub timeout_secs: u32,                 // 1-300
    pub metadata: serde_json::Value,       // Extensible
}

pub enum ServerQueryKind {
    ReadFile { path: String },
    ExecuteCommand { command: String, args: Vec<String>, timeout_secs: u32 },
    RequestUserInput { prompt: String, input_type: String, options: Option<Vec<String>> },
    GetEnvironment { keys: Vec<String> },
    GetWorkspaceContext,
    Custom { name: String, payload: serde_json::Value },
}
```

### ServerQueryResponse (Client → Server)
```rust
pub struct ServerQueryResponse {
    pub query_id: String,                  // Correlate to ServerQuery::id
    pub sent_at: String,                   // RFC3339
    pub result: ServerQueryResult,         // The answer
    pub error: Option<String>,             // If failed
}

pub enum ServerQueryResult {
    FileContent(String),
    CommandOutput { exit_code: i32, stdout: String, stderr: String },
    UserInput(String),
    Environment(HashMap<String, String>),
    WorkspaceContext(serde_json::Value),
    Custom { name: String, payload: serde_json::Value },
}
```

---

## Server-Side Usage

### Send a query and wait for response
```rust
use loom_core::ServerQuery;
use loom_server::ServerQueryManager;

let manager = ServerQueryManager::new();

let query = ServerQuery {
    id: uuid7_with_prefix("Q-"),
    kind: ServerQueryKind::ReadFile { 
        path: "src/main.rs".to_string() 
    },
    sent_at: now_rfc3339(),
    timeout_secs: 30,
    metadata: json!({}),
};

// This blocks until client responds or timeout
match manager.send_query("session-123", query).await {
    Ok(response) => {
        if let ServerQueryResult::FileContent(content) = response.result {
            println!("File: {}", content);
        }
    }
    Err(e) => eprintln!("Query failed: {}", e),
}
```

### HTTP Endpoints
```
POST /v1/sessions/{session_id}/query-response
  ← ServerQueryResponse
  → { "status": "ok" }

GET /v1/sessions/{session_id}/queries
  → Vec<ServerQuery>  # Debugging: pending queries
```

---

## Client-Side Usage (ACP)

### Handle queries automatically
```rust
use loom_acp::LoomAcpAgent;
use loom_core::AcpServerQueryHandler;

let workspace_root = PathBuf::from("/home/user/project");
let handler = AcpServerQueryHandler::new(workspace_root);

// Automatically processes:
// - ReadFile: Reads from workspace
// - GetEnvironment: Returns env vars
// - GetWorkspaceContext: Returns git info + workspace path
// - RequestUserInput: Returns error (override in editor mode)
// - ExecuteCommand: Returns error (security)

// When server sends ServerQuery via SSE:
let response = handler.handle_query(query).await?;

// Send response back to server:
client.post(&format!("/v1/sessions/{}/query-response", session_id))
    .json(&response)
    .send()
    .await?;
```

---

## SSE Event Format

Server emits query in SSE stream:
```json
event: llm
data: {
  "type": "server_query",
  "id": "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0",
  "kind": { "type": "read_file", "path": "src/main.rs" },
  "sent_at": "2025-01-20T12:00:00Z",
  "timeout_secs": 30,
  "metadata": {}
}
```

Client sends response via HTTP:
```bash
curl -X POST http://localhost:8080/v1/sessions/session-123/query-response \
  -H "Content-Type: application/json" \
  -d '{
    "query_id": "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0",
    "sent_at": "2025-01-20T12:00:05Z",
    "result": {
      "type": "file_content",
      "content": "fn main() { ... }"
    },
    "error": null
  }'
```

---

## Test Coverage

### Run Tests
```bash
# All server query tests
cargo test -p loom-core server_query
cargo test -p loom-server server_query
cargo test -p loom-acp server_query

# Full suite
make test
```

### Test Categories
- **Property-based:** JSON roundtrips, ID format, timeouts
- **Unit:** File ops, env vars, workspace context
- **Integration:** End-to-end query→response→server

**Total:** 19 new tests, all passing ✅

---

## Query Types & Examples

### 1. ReadFile
**Use:** Get file contents during coding assistance
```rust
ServerQueryKind::ReadFile { path: "src/main.rs".into() }
→ ServerQueryResult::FileContent(String)
```

### 2. GetEnvironment
**Use:** Access secrets, deployment targets
```rust
ServerQueryKind::GetEnvironment { 
    keys: vec!["DEPLOY_KEY".into(), "DEPLOY_HOST".into()] 
}
→ ServerQueryResult::Environment(HashMap)
```

### 3. GetWorkspaceContext
**Use:** Understand current project state
```rust
ServerQueryKind::GetWorkspaceContext
→ ServerQueryResult::WorkspaceContext(json!({
    "workspace_root": "/home/user/project",
    "git_branch": "feature/query-bridge",
    "has_git": true,
}))
```

### 4. RequestUserInput
**Use:** Human-in-the-loop decisions
```rust
ServerQueryKind::RequestUserInput {
    prompt: "Approve deletion? (yes/no)".into(),
    input_type: "yes_no".into(),
    options: Some(vec!["yes".into(), "no".into()]),
}
→ ServerQueryResult::UserInput("yes".into())
```

### 5. ExecuteCommand
**Use:** Run local scripts (future, currently disabled)
```rust
ServerQueryKind::ExecuteCommand {
    command: "npm".into(),
    args: vec!["run".into(), "build".into()],
    timeout_secs: 60,
}
→ ServerQueryResult::CommandOutput { exit_code: 0, stdout, stderr }
```

### 6. Custom
**Use:** Extensible for future query types
```rust
ServerQueryKind::Custom {
    name: "my_custom_query".into(),
    payload: json!({ "param": "value" }),
}
→ ServerQueryResult::Custom { name, payload }
```

---

## Error Handling

### Timeout
```rust
ServerQueryError::Timeout
// Query exceeded timeout_secs
// Server treats as failed tool execution
```

### File Not Found
```rust
ServerQueryResponse {
    error: Some("file not found: src/missing.rs"),
    result: null,
}
```

### Unsupported Query Type (CLI Mode)
```rust
ServerQueryError::ExecutionFailed("request_user_input not supported in CLI mode")
```

---

## Configuration

### Server
```toml
[server.query]
enabled = true
default_timeout_secs = 30
max_queries_per_session = 10
```

### Client (ACP)
```toml
[acp]
handle_queries = true
blocked_query_types = ["execute_command"]  # Security
```

---

## Next Steps (Phase 2)

### Integrate into LLM Processing
1. When LLM outputs "I need to read src/main.rs"
2. Server extracts "read src/main.rs" intent
3. Server sends ServerQuery to client
4. Client responds with file
5. Server resumes LLM with "File contains: ..."

**Effort:** ~4 hours

### WebSocket Upgrade
Replace SSE + HTTP with WebSocket for lower latency.

**Effort:** ~8 hours

### Editor Integration
Extend `AcpServerQueryHandler` to delegate:
- User input → Editor UI
- Command execution → Editor shell

**Effort:** ~6 hours per editor

---

## Debugging

### Enable Logs
```bash
RUST_LOG=loom_server::server_query=debug cargo run
```

### Check Pending Queries
```bash
curl http://localhost:8080/v1/sessions/session-123/queries
```

### Look for These in Logs
```
query_id: "Q-..."              # Correlate across requests
session_id: "..."              # Which session
kind: ReadFile { path: "..." } # What was requested
duration_ms: 123               # How long it took
```

---

## Summary

✅ **Phase 1 Complete**
- Types defined in loom-core
- Manager implemented in loom-server
- Handler implemented in loom-acp
- SSE integration done
- 19 tests passing

🚀 **Ready for Phase 2**
- Integration with LLM processing loop
- WebSocket upgrade
- Editor-specific handlers

📚 **Documentation**
- [PLAN_SERVER_CLIENT_QUERY_BRIDGE.md](file:///home/ghuntley/loom/PLAN_SERVER_CLIENT_QUERY_BRIDGE.md) - Full design
- [IMPLEMENTATION_SERVER_CLIENT_QUERY_BRIDGE.md](file:///home/ghuntley/loom/IMPLEMENTATION_SERVER_CLIENT_QUERY_BRIDGE.md) - Implementation details

---

## Build Status
```
✅ cargo check   : PASSED
✅ cargo build   : PASSED (23.72s)
✅ cargo test    : 275+ tests PASSED
✅ cargo clippy  : 0 warnings
✅ cargo fmt     : formatted
```

All systems go! 🚀
