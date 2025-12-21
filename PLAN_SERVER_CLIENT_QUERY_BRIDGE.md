# Plan: Server-to-Client Query Bridge via ACP

**Status:** Planning (Not Implemented)  
**Date:** 2025-01-01  
**Scope:** Bi-directional communication bridge enabling server to send queries to client via ACP protocol

---

## Problem Statement

Currently, Loom's communication is **unidirectional**: client sends prompts to server/LLM, receives responses via SSE. 

We need a **server-initiated query mechanism** where:
- Server can send a query/request to the client
- Client processes the query asynchronously
- Client sends back a response
- Communication happens over the existing ACP stdio connection (or ACP-like WebSocket upgrade for always-on scenarios)

### Use Cases

1. **Polling tool results from local filesystem** - Server asks client to read a file after tool execution
2. **Querying client environment** - Server needs to know about local workspace/config/context
3. **Requesting human input** - Server pauses execution, asks user for confirmation/input, resumes
4. **Multi-agent coordination** - Multiple instances need to query each other
5. **Editor integration** - VSCode/Zed extensions initiate Loom calls, Loom queries back for context

---

## Architecture Overview

### Transport Layer Decision: SSE vs WebSocket

| Feature | SSE | WebSocket |
|---------|-----|-----------|
| **Server→Client** | Native (built-in) | Yes |
| **Client→Server** | Separate HTTP | Built-in |
| **Bi-directional** | Requires 2 connections | Native |
| **Complexity** | Lower | Higher |
| **Persistence** | Per-request | Persistent |
| **Browser support** | Yes | Yes |
| **Polling latency** | Higher (if synchronous) | Lower |

**Recommendation for v1:** Keep SSE for current flows, add **request-response pattern on top** using:
- Existing SSE for responses (server→client)
- HTTP POST for queries (server→client indirect) + server polling for results (client→server)

**Recommendation for v2:** WebSocket for persistent bi-directional when ACP sessions become always-on.

---

## Detailed Design

### 1. Data Model: ServerQuery

```rust
/// Sent from server to client (via SSE or HTTP response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerQuery {
    /// Unique query ID for correlation (UUID7)
    pub id: String,  // "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0"
    
    /// Query type (discriminator)
    pub kind: ServerQueryKind,
    
    /// Timestamp when query was sent
    pub sent_at: String,  // RFC3339
    
    /// Timeout in seconds (client should abandon if exceeded)
    pub timeout_secs: u32,
    
    /// Optional metadata for debugging
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerQueryKind {
    /// Server requests client to read a file from filesystem
    ReadFile {
        path: String,
    },
    
    /// Server requests client to execute a local command
    ExecuteCommand {
        command: String,
        args: Vec<String>,
        timeout_secs: u32,
    },
    
    /// Server requests human input (pause & ask user)
    RequestUserInput {
        prompt: String,
        input_type: String,  // "text", "yes_no", "selection"
        options: Option<Vec<String>>,  // For selection
    },
    
    /// Server requests client environment information
    GetEnvironment {
        keys: Vec<String>,  // ENV var names to retrieve
    },
    
    /// Server requests workspace context
    GetWorkspaceContext {
        // What workspace info to return
    },
    
    /// Extensible: custom queries
    Custom {
        name: String,
        payload: serde_json::Value,
    },
}
```

### 2. Response Model: ServerQueryResponse

```rust
/// Sent from client back to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerQueryResponse {
    /// Correlate back to ServerQuery::id
    pub query_id: String,
    
    /// Timestamp when response was sent
    pub sent_at: String,  // RFC3339
    
    /// Result data (type depends on query kind)
    pub result: ServerQueryResult,
    
    /// Optional error message
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerQueryResult {
    FileContent(String),
    CommandOutput {
        exit_code: i32,
        stdout: String,
        stderr: String,
    },
    UserInput(String),
    Environment(std::collections::HashMap<String, String>),
    WorkspaceContext(serde_json::Value),
    Custom {
        name: String,
        payload: serde_json::Value,
    },
}
```

### 3. Client-Side Handling (in ACP agent)

The `LoomAcpAgent` receives server queries and responds:

```rust
/// Handler for incoming server queries
pub trait ServerQueryHandler: Send + Sync {
    /// Process a query, return response
    async fn handle_query(&self, query: ServerQuery) -> Result<ServerQueryResponse>;
}

/// Default implementation for ACP agent
pub struct AcpServerQueryHandler {
    workspace_root: PathBuf,
    // Can query filesystem, environment, user via editor callbacks
}

impl ServerQueryHandler for AcpServerQueryHandler {
    async fn handle_query(&self, query: ServerQuery) -> Result<ServerQueryResponse> {
        match query.kind {
            ServerQueryKind::ReadFile { path } => {
                let full_path = self.workspace_root.join(&path);
                let content = tokio::fs::read_to_string(&full_path).await?;
                Ok(ServerQueryResponse {
                    query_id: query.id,
                    sent_at: now_rfc3339(),
                    result: ServerQueryResult::FileContent(content),
                    error: None,
                })
            }
            ServerQueryKind::RequestUserInput { prompt, .. } => {
                // Delegate to editor via ACP callback
                let user_response = self.request_user_input(&prompt).await?;
                Ok(ServerQueryResponse {
                    query_id: query.id,
                    sent_at: now_rfc3339(),
                    result: ServerQueryResult::UserInput(user_response),
                    error: None,
                })
            }
            // ... handle other types ...
        }
    }
}
```

### 4. Server-Side Query Manager

```rust
/// Manages pending queries and responses
pub struct ServerQueryManager {
    /// Pending queries waiting for response
    pending: Arc<Mutex<HashMap<String, ServerQuery>>>,
    
    /// Responses received from client
    responses: Arc<Mutex<HashMap<String, ServerQueryResponse>>>,
    
    /// Notification channel for query responses
    response_tx: broadcast::Sender<ServerQueryResponse>,
}

impl ServerQueryManager {
    /// Send query to client (via SSE or other transport)
    pub async fn send_query(
        &self,
        session_id: &str,
        query: ServerQuery,
    ) -> Result<ServerQueryResponse> {
        let query_id = query.id.clone();
        let timeout = Duration::from_secs(query.timeout_secs as u64);
        
        // Store query as pending
        self.pending.lock().await.insert(query_id.clone(), query.clone());
        
        // Send to client (via SSE channel, WebSocket, etc.)
        self.send_to_client(&session_id, &query).await?;
        
        // Wait for response with timeout
        let response = tokio::time::timeout(
            timeout,
            self.wait_for_response(&query_id),
        ).await
        .map_err(|_| ServerQueryError::Timeout)?
        ?;
        
        Ok(response)
    }
    
    /// Called when client sends response back
    pub async fn receive_response(&self, response: ServerQueryResponse) {
        let query_id = response.query_id.clone();
        self.responses.lock().await.insert(query_id.clone(), response.clone());
        let _ = self.response_tx.send(response);
    }
    
    async fn wait_for_response(&self, query_id: &str) -> Result<ServerQueryResponse> {
        let mut rx = self.response_tx.subscribe();
        while let Ok(response) = rx.recv().await {
            if response.query_id == query_id {
                return Ok(response);
            }
        }
        Err(ServerQueryError::NoResponse)
    }
}
```

### 5. SSE Event Format Extension

Add new SSE event type for server queries in the existing streaming:

```json
{
  "event": "server_query",
  "data": {
    "type": "server_query",
    "query": {
      "id": "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0",
      "kind": {
        "type": "read_file",
        "path": "src/main.rs"
      },
      "sent_at": "2025-01-01T12:00:00Z",
      "timeout_secs": 30,
      "metadata": {}
    }
  }
}
```

Client response (HTTP POST to new endpoint):

```bash
POST /v1/sessions/{session_id}/query-response
Content-Type: application/json

{
  "query_id": "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0",
  "sent_at": "2025-01-01T12:00:05Z",
  "result": {
    "type": "file_content",
    "content": "fn main() { ... }"
  },
  "error": null
}
```

---

## Integration Points

### In ACP Agent Flow

```
Client sends prompt
  ↓
Server processes via LLM
  ↓
Server decides to query client (e.g., read a file)
  ↓
Server sends ServerQuery via SSE event
  ↓
Client receives query via ACP notification pump
  ↓
Client executes query (read file, ask user, etc.)
  ↓
Client sends response via HTTP POST to /query-response
  ↓
Server receives response, resumes LLM processing
  ↓
Server sends next response chunk to client
```

### In Threading/Session Management

1. **SessionId** uniquely identifies an ACP session
2. **ServerQuery::id** is separate from ThreadId (UUID7 with "Q-" prefix)
3. Queries don't persist in Thread (they're ephemeral during a turn)
4. Responses logged in Thread metadata for audit trail (optional)

### New HTTP Endpoints

```
POST /v1/sessions/{session_id}/query-response
├─ Input: ServerQueryResponse
├─ Validates query_id is pending
├─ Stores response
├─ Notifies ServerQueryManager
└─ Response: { status: "ok" }

GET /v1/sessions/{session_id}/queries
├─ Debugging endpoint: list pending queries
└─ Response: Vec<ServerQuery>
```

---

## Error Handling

### Client Errors
- **File not found** → error in ServerQueryResponse
- **Permission denied** → error in ServerQueryResponse
- **Timeout** → client abandoned query after `timeout_secs`

### Server Errors
- **Query timeout** → return error to LLM as tool failure
- **Client disconnected** → query cannot be sent, treat as network error
- **Malformed response** → log warning, continue with null/default result

### Retry Strategy
- Use existing `loom-http-retry` for /query-response POST
- Queries are fire-and-forget from server perspective (no auto-retry)
- Client can re-query if server missed response

---

## Testing Strategy

### Property-Based Tests

```rust
proptest! {
    /// ServerQuery ID format is always valid
    #[test]
    fn server_query_id_format(id in "[a-zA-Z0-9-]{36}") {
        let query = ServerQuery {
            id: format!("Q-{}", id),
            // ...
        };
        prop_assert!(query.id.starts_with("Q-"));
    }

    /// ServerQueryResponse always correlates to a query
    #[test]
    fn response_correlates_to_query(
        query in arb_server_query(),
        response_delay_ms in 0u32..5000,
    ) {
        // Send query, wait, receive response
        // Assert query_id matches
    }
    
    /// Timeout prevents hanging indefinitely
    #[test]
    fn query_timeout_enforced(query in arb_server_query()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = ServerQueryManager::new();
            manager.send_query("session-1", query).timeout(Duration::from_secs(31)).await.is_err()
        });
    }
}
```

### Unit Tests
- ServerQuery serialization roundtrip
- Response deserialization
- Query ID generation
- Timeout calculation

### Integration Tests
- Full roundtrip: send query → client processes → server receives response
- Multiple concurrent queries in same session
- Query timeout behavior
- ACP notification pump integration

---

## Phase Planning

### Phase 1 (v1): Core Query Framework
- ✓ Define ServerQuery/ServerQueryResponse types
- ✓ Implement ServerQueryManager in loom-server
- ✓ Add /query-response HTTP endpoint
- ✓ Extend SSE format with server_query event
- ✓ Add AcpServerQueryHandler to loom-acp
- ✓ Tests: property-based + unit tests

### Phase 2 (v2): WebSocket Upgrade
- Migrate from request-response HTTP to persistent WebSocket
- Reduce latency for rapid query sequences
- Support streaming results for large responses

### Phase 3 (v3): Advanced Features
- Query priorities/queueing
- Structured logging of all queries
- Query analytics (latency, errors, types)
- Client-side query caching

---

## Migration Path from Current Architecture

### Current State
```
CLI ←→ Server (HTTP/SSE) ←→ LLM
       (one-way streaming)
```

### After Implementation
```
ACP Client ←→ Server (HTTP/SSE + query-response POST)
               ↓
              LLM
              (can query client during processing)
```

### Backward Compatibility
- Existing SSE streaming endpoints unchanged
- New event type `server_query` in SSE stream
- New HTTP POST endpoint `/query-response` (opt-in)
- Old CLI still works without query support

---

## Dependencies & Crates

### New Types (loom-core or loom-thread)
- `ServerQuery`, `ServerQueryKind`, `ServerQueryResponse`, `ServerQueryResult`
- `ServerQueryError`
- `ServerQueryHandler` trait

### New Crate (loom-server-query, or fold into loom-server)
- `ServerQueryManager`
- HTTP handlers for endpoints
- SSE event serialization

### Modified Crates
- `loom-acp` - Add `AcpServerQueryHandler`
- `loom-server` - Add query endpoints and manager
- `loom-core` - Add trait for pluggable handlers

---

## Configuration & Deployment

### Server Config
```toml
[server.query]
# Enable server-to-client queries
enabled = true

# Default query timeout (seconds)
default_timeout_secs = 30

# Max concurrent queries per session
max_queries_per_session = 10
```

### Client Config
```toml
[acp]
# Enable query handling
handle_queries = true

# Query types to block (security)
blocked_query_types = ["execute_command"]
```

---

## Open Questions & Decisions

1. **Should queries be logged in thread?**
   - Pro: Audit trail, debugging
   - Con: Pollutes thread data
   - Decision: Optional metadata only

2. **Should client-side queries be persistent across restarts?**
   - Pro: Resume in-flight queries
   - Con: Complexity
   - Decision: Queries are session-scoped, not persisted

3. **Should there be query priority levels?**
   - Pro: Critical queries processed first
   - Con: Over-engineering for v1
   - Decision: All queries same priority; phase 2 if needed

4. **What happens if server disconnects mid-query?**
   - Client abandons query after timeout
   - No persistent queue
   - Server can retry by sending same query again

5. **Should responses be signed/authenticated?**
   - Pro: Prevent forgery
   - Con: Adds latency
   - Decision: Session authentication sufficient for v1

---

## Example Usage Scenarios

### Scenario 1: File Context During Coding
```
Editor: "Add logging to this file"
  ↓ ACP sends prompt to Loom
  ↓ Loom → LLM: "Please add logging to..."
  ↓ LLM: "I'll add logging. Let me read the file first."
  ↓ Loom → Client: ServerQuery(ReadFile("src/main.rs"))
  ↓ Client → Loom: ServerQueryResponse(file contents)
  ↓ Loom → LLM: "File contents are: ... now I'll add logging"
  ↓ Loom outputs edits to editor
```

### Scenario 2: Human-in-the-Loop Review
```
Loom → LLM: "Should I delete this function?"
LLM: "Here's my recommendation..."
Loom → Client: ServerQuery(RequestUserInput("Approve deletion?"))
Client: User clicks "Yes" in editor popup
Client → Loom: ServerQueryResponse("yes")
Loom → LLM: "User approved, proceeding with deletion"
```

### Scenario 3: Environment Querying
```
Loom → LLM: "Deploy this app"
LLM: "I need to check environment variables"
Loom → Client: ServerQuery(GetEnvironment(["DEPLOY_KEY", "DEPLOY_HOST"]))
Client → Loom: ServerQueryResponse({ "DEPLOY_KEY": "***", "DEPLOY_HOST": "prod.example.com" })
Loom → LLM: "Environment is configured, proceeding..."
```

---

## Summary

This design provides a **flexible, extensible framework** for server-to-client communication that:

- ✅ Leverages existing SSE infrastructure
- ✅ Maintains backward compatibility
- ✅ Supports diverse query types (file, command, user input, environment)
- ✅ Includes timeout and error handling
- ✅ Scales from simple to complex scenarios
- ✅ Has a clear migration path to WebSocket v2
- ✅ Integrates seamlessly with ACP agent mode

The core insight is treating queries as **first-class RPC calls** with correlation IDs, timeouts, and structured results, rather than ad-hoc polling or side-channel communication.
