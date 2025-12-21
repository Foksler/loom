# Troubleshooting Query Bridge

> **Purpose:** Solve common issues and debug Phase 1/2 problems  
> **Audience:** Developers, DevOps, debuggers  
> **Updated:** 2025-01-20

## Table of Contents
1. [Quick Diagnosis](#quick-diagnosis)
2. [Common Issues & Solutions](#common-issues)
3. [Log Interpretation Guide](#log-interpretation)
4. [Debugging Commands](#debugging-commands)
5. [Performance Profiling](#performance-profiling)
6. [Testing Checklist](#testing-checklist)
7. [Advanced Diagnostics](#advanced-diagnostics)

---

## Quick Diagnosis

### Symptom Flowchart

```
Problem: Query not responding?
├─ Check: Is client connected?
│  └─ No → [ISSUE: Client Connection Lost](#issue-client-connection)
│  └─ Yes → [ISSUE: Client Not Handling Query](#issue-client-handler)
│
Problem: Getting timeout errors?
├─ Check: Query timeout_secs value
│  └─ Too low (< 5s) → Increase to 30s
│  └─ Too high (> 300s) → Decrease to 60s
│
Problem: File access denied?
├─ Check: Path traversal protection
│  └─ Path outside workspace → [ISSUE: Security Check](#issue-security)
│  └─ Permission issue → [ISSUE: File Permissions](#issue-permissions)
│
Problem: Wrong response format?
├─ Check: Response serialization
│  └─ [ISSUE: Deserialization Error](#issue-deser)
│
Problem: Performance degradation?
├─ Check: Memory usage
│  └─ [PERFORMANCE: Memory Profiling](#perf-memory)
```

---

## Common Issues & Solutions {#common-issues}

### ISSUE: Client Connection Lost {#issue-client-connection}

**Symptoms:**
```
ERROR: session not found: session-123
ERROR: query timeout: no client response
Query sent but no SSE event received
```

**Root Causes:**
1. Client disconnected before responding
2. Network interruption
3. Session ID mismatch
4. Client not subscribed to SSE stream

**Diagnosis:**

```bash
# Check if session exists
curl http://localhost:8080/v1/sessions/session-123/queries

# Expected: [] or list of pending queries
# Actual: 404 or empty

# Check client subscription status
RUST_LOG=loom_server::server_query=debug cargo run

# Look for logs:
# [INFO] Session created: session-123
# [ERROR] Client not found for session-123 (before query sent)
```

**Solutions:**

1. **Verify client SSE subscription** before sending query:
```rust
// Server: Wait for client to connect before sending queries
pub async fn wait_for_client(
    manager: &ServerQueryManager,
    session_id: &str,
    timeout_secs: u32,
) -> Result<(), String> {
    let start = std::time::Instant::now();
    loop {
        if manager.has_client(session_id) {
            return Ok(());
        }
        if start.elapsed().as_secs() > timeout_secs as u64 {
            return Err("Client not connected".to_string());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

// Usage
wait_for_client(&manager, "session-123", 5).await?;
```

2. **Increase timeout** for slow connections:
```rust
let query = ServerQuery {
    timeout_secs: 60,  // Increased from 30
    ..default
};
```

3. **Implement connection heartbeat**:
```rust
pub async fn send_keepalive(
    manager: &ServerQueryManager,
    session_id: &str,
) -> Result<(), String> {
    let query = ServerQuery {
        id: format!("Q-keepalive-{}", uuid7()),
        kind: ServerQueryKind::Custom {
            name: "keepalive".to_string(),
            payload: json!({}),
        },
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 5,
        metadata: json!({ "internal": true }),
    };
    
    manager.send_query(session_id, query).await?;
    Ok(())
}
```

---

### ISSUE: Client Not Handling Query {#issue-client-handler}

**Symptoms:**
```
Query sent successfully but no response received
Client receives SSE event but doesn't respond
Query timeout after waiting for response
```

**Root Causes:**
1. Handler not implemented for query kind
2. Client-side error in handler
3. Response serialization error

**Diagnosis:**

```bash
# Enable client-side debug logging
RUST_LOG=loom_acp::agent=debug cargo run

# Look for:
# [DEBUG] Received server query
# [ERROR] Handler failed for query_kind: ...
# [WARN] Failed to send response
```

**Solutions:**

1. **Check handler implementation**:
```rust
// Verify handler supports query kind
pub async fn handle_query(
    handler: &AcpServerQueryHandler,
    query: &ServerQuery,
) -> Result<ServerQueryResponse, String> {
    match &query.kind {
        ServerQueryKind::ReadFile { path } => {
            // Implementation here
        }
        ServerQueryKind::GetEnvironment { keys } => {
            // Implementation here
        }
        ServerQueryKind::RequestUserInput { .. } => {
            // CLI mode: return error
            return Err("user input not supported in CLI mode".to_string());
        }
        ServerQueryKind::Custom { name, .. } => {
            tracing::warn!("Unknown custom query: {}", name);
            return Err(format!("custom query not supported: {}", name));
        }
        _ => return Err("unsupported query kind".to_string()),
    }
}
```

2. **Add error context to handler**:
```rust
pub async fn handle_query_with_context(
    handler: &AcpServerQueryHandler,
    query: &ServerQuery,
) -> Result<ServerQueryResponse, String> {
    tracing::debug!(
        query_id = %query.id,
        kind = ?query.kind,
        "Processing query"
    );

    let result = handle_query(handler, query).await;

    match &result {
        Ok(_) => tracing::info!(query_id = %query.id, "Query handled successfully"),
        Err(e) => tracing::error!(
            query_id = %query.id,
            error = %e,
            "Query handler error"
        ),
    }

    result
}
```

3. **Test handler in isolation**:
```rust
#[tokio::test]
async fn test_handler_read_file() {
    let workspace = PathBuf::from("/tmp/test");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(workspace.join("test.rs"), "fn main() {}").unwrap();

    let handler = AcpServerQueryHandler::new(workspace);
    let query = ServerQuery {
        id: "Q-test".to_string(),
        kind: ServerQueryKind::ReadFile {
            path: "test.rs".to_string(),
        },
        sent_at: chrono::Utc::now().to_rfc3339(),
        timeout_secs: 10,
        metadata: json!({}),
    };

    let response = handler.handle_query(&query).await.expect("should handle");
    assert!(response.error.is_none());
}
```

---

### ISSUE: Security Check Failed {#issue-security}

**Symptoms:**
```
ERROR: path traversal not allowed: ../../etc/passwd
ERROR: path outside workspace: /etc/hosts
Query returns permission denied
```

**Root Causes:**
1. Client trying to read files outside workspace
2. Path traversal attack attempt
3. Legitimate file outside workspace

**Diagnosis:**

```bash
# Check workspace configuration
echo "Workspace root: $(pwd)"

# Verify path in query
RUST_LOG=loom_acp::agent=debug cargo run

# Look for:
# [DEBUG] Path check: requested=/etc/passwd, workspace=/home/user/project
# [WARN] Path traversal detected
```

**Solutions:**

1. **Verify workspace root configuration**:
```rust
pub fn new(workspace_root: PathBuf) -> Self {
    let workspace_root = workspace_root.canonicalize()
        .expect("workspace root must exist");
    
    tracing::info!(
        path = %workspace_root.display(),
        "Handler initialized with workspace root"
    );

    Self {
        workspace_root,
        ..default
    }
}
```

2. **Use secure path normalization**:
```rust
pub fn is_path_allowed(workspace: &Path, requested: &str) -> bool {
    // Normalize both paths
    let workspace = match workspace.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let requested_path = workspace.join(requested);
    let requested = match requested_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false, // File doesn't exist (OK for some cases)
    };

    // Check if requested is under workspace
    requested.starts_with(&workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_file_in_workspace() {
        let workspace = Path::new("/home/user/project");
        assert!(is_path_allowed(workspace, "src/main.rs"));
    }

    #[test]
    fn test_blocks_traversal() {
        let workspace = Path::new("/home/user/project");
        assert!(!is_path_allowed(workspace, "../../etc/passwd"));
    }

    #[test]
    fn test_blocks_absolute_path() {
        let workspace = Path::new("/home/user/project");
        assert!(!is_path_allowed(workspace, "/etc/hosts"));
    }
}
```

3. **Return proper error messages**:
```rust
pub fn read_file_safely(workspace: &Path, path: &str) -> Result<String, String> {
    if !is_path_allowed(workspace, path) {
        return Err(format!(
            "Access denied: path outside workspace ({})",
            workspace.display()
        ));
    }

    std::fs::read_to_string(workspace.join(path))
        .map_err(|e| format!("File read error: {}", e))
}
```

---

### ISSUE: Deserialization Error {#issue-deser}

**Symptoms:**
```
serde_json::error: expected value at line 1 column 0
ServerQueryResponse deserialization failed
JSON parse error in event stream
```

**Root Causes:**
1. Malformed JSON in response
2. Field name mismatch
3. Type mismatch (string vs number)
4. Empty/null response

**Diagnosis:**

```bash
# Capture the actual response
curl -X POST http://localhost:8080/v1/sessions/test/query-response \
  -H "Content-Type: application/json" \
  -d '{...}' \
  -v

# Check Content-Type header
# Should be: application/json

# Enable JSON debugging
RUST_LOG=serde_json=debug cargo run
```

**Solutions:**

1. **Validate JSON before sending**:
```rust
pub fn validate_response_json(json_str: &str) -> Result<(), String> {
    match serde_json::from_str::<ServerQueryResponse>(json_str) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Invalid response JSON: {}", e)),
    }
}

// Usage
let json = r#"{"query_id":"Q-123",...}"#;
validate_response_json(json)?;
```

2. **Use strict deserialization**:
```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerQueryResponse {
    pub query_id: String,
    pub sent_at: String,
    pub result: Option<ServerQueryResult>,
    pub error: Option<String>,
}
```

3. **Add response debugging endpoint**:
```rust
#[post("/debug/validate-response")]
async fn debug_validate_response(body: String) -> impl Responder {
    match serde_json::from_str::<ServerQueryResponse>(&body) {
        Ok(r) => HttpResponse::Ok().json(json!({
            "valid": true,
            "response": r
        })),
        Err(e) => HttpResponse::BadRequest().json(json!({
            "valid": false,
            "error": e.to_string(),
            "body_preview": &body[..std::cmp::min(100, body.len())]
        })),
    }
}

// Test it:
// curl -X POST http://localhost:8080/debug/validate-response \
//   -d '{"query_id":"Q-123",...}'
```

---

### ISSUE: File Permissions {#issue-permissions}

**Symptoms:**
```
ERROR: permission denied (os error 13)
Cannot read file: Permission denied
Access denied for src/secret.key
```

**Root Causes:**
1. File not readable by process
2. Directory permissions issue
3. SELinux/AppArmor restrictions

**Diagnosis:**

```bash
# Check file permissions
ls -la src/secret.key

# Check process user
whoami

# Check process permissions
ps aux | grep loom

# Try to read as process user
sudo -u loom_user cat src/secret.key
```

**Solutions:**

1. **Fix file permissions**:
```bash
# Make file readable by current user
chmod 644 src/secret.key

# Make directory readable
chmod 755 src/
```

2. **Run process with correct permissions**:
```bash
# As root
sudo -u www-data cargo run

# Or fix in code
pub fn ensure_readable(path: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.permissions().readonly() {
        Ok(())
    } else {
        Err("File not readable".to_string())
    }
}
```

3. **Add permission error context**:
```rust
pub async fn read_file_with_perms(path: &Path) -> Result<String, String> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                let metadata = std::fs::metadata(path)
                    .map(|m| format!("mode={:o}", m.permissions().mode()))
                    .unwrap_or_default();
                Err(format!(
                    "Permission denied ({}). Make sure file is readable.",
                    metadata
                ))
            }
            _ => Err(format!("Read error: {}", e)),
        },
    }
}
```

---

## Log Interpretation Guide {#log-interpretation}

### Log Levels

```
[TRACE] Ultra-detailed internal operations (rarely needed)
[DEBUG] Detailed diagnostic info for troubleshooting
[INFO]  Important events (normal operation)
[WARN]  Warning conditions (issues to investigate)
[ERROR] Error conditions (failures)
```

### Enabling Logs

```bash
# Single module
RUST_LOG=loom_server::server_query=debug cargo run

# Multiple modules
RUST_LOG=loom_server::server_query=debug,loom_acp::agent=debug cargo run

# Everything
RUST_LOG=debug cargo run

# JSON structured logging (if enabled)
RUST_LOG=debug RUST_LOG_FORMAT=json cargo run
```

### Key Log Fields

| Field | Meaning | Example |
|-------|---------|---------|
| `query_id` | Query identifier | `Q-019b2b97-fddf-...` |
| `session_id` | Client session | `session-123` |
| `kind` | Query type | `ReadFile { path: "..." }` |
| `duration_ms` | Time taken | `1234` |
| `bytes` | Data size | `4096` |
| `error` | Error message | `file not found` |

### Log Examples

**Successful Query:**
```
[INFO] loom_server::server_query: Query sent to client
    query_id: "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0"
    session_id: "session-123"
    kind: ReadFile { path: "src/main.rs" }

[INFO] loom_acp::agent: Query handled
    query_id: "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0"
    bytes: 1024
    duration_ms: 45
```

**Failed Query:**
```
[WARN] loom_server::server_query: Query dispatch failed
    query_id: "Q-019b2b97-fddf-7602-a3e4-1c4a295110c0"
    session_id: "session-123"
    error: "timeout"
    duration_ms: 30000
```

---

## Debugging Commands {#debugging-commands}

### Check Server Status

```bash
# Server running?
curl -i http://localhost:8080/health

# Expected: 200 OK

# List sessions
curl http://localhost:8080/v1/sessions

# List queries for session
curl http://localhost:8080/v1/sessions/session-123/queries

# Expected response:
# [
#   {
#     "id": "Q-...",
#     "kind": {...},
#     "sent_at": "2025-01-20T...",
#     "timeout_secs": 30
#   }
# ]
```

### Simulate Query Flow

```bash
# 1. Start server
RUST_LOG=debug cargo run &

# 2. Get session ID
SESSION_ID="test-$(date +%s)"

# 3. Send query (simulates client need)
# Would be done by client in SSE stream
curl -X POST http://localhost:8080/v1/sessions/$SESSION_ID/query \
  -H "Content-Type: application/json" \
  -d '{
    "kind": {
      "type": "read_file",
      "path": "Cargo.toml"
    }
  }'

# 4. Simulate response
curl -X POST http://localhost:8080/v1/sessions/$SESSION_ID/query-response \
  -H "Content-Type: application/json" \
  -d '{
    "query_id": "Q-...",
    "sent_at": "2025-01-20T12:00:00Z",
    "result": {
      "type": "file_content",
      "content": "[package]\nname = \"loom\""
    }
  }'
```

### Check Timeouts

```bash
# Query with 2 second timeout
curl -X POST ... \
  -d '{
    "timeout_secs": 2,
    ...
  }'

# Watch server logs for timeout
RUST_LOG=debug cargo run 2>&1 | grep -i timeout

# Expected:
# [ERROR] query timeout: no response within 2 seconds
```

### Test Path Security

```bash
# Try path traversal (should fail)
curl -X POST /v1/sessions/test/query-response \
  -d '{
    "result": {
      "type": "file_content",
      "path": "../../etc/passwd"
    }
  }'

# Expected error:
# "path traversal not allowed"

# Try absolute path (should fail)
curl -X POST /v1/sessions/test/query-response \
  -d '{
    "result": {
      "type": "file_content",
      "path": "/etc/hosts"
    }
  }'

# Expected error:
# "path outside workspace"
```

---

## Performance Profiling {#performance-profiling}

### Measure Query Latency

```bash
# Simple timing
time cargo test test_query_end_to_end

# With instrumentation
RUST_LOG=loom_server=debug cargo test test_query_end_to_end -- --nocapture | \
  grep -E "duration_ms|Query (sent|handled)"

# Detailed profiling with perf
perf record -F 99 cargo test --release test_query_end_to_end
perf report
```

### Memory Usage

```bash
# Check RSS (resident set size)
ps aux | grep cargo

# Monitor over time
watch -n 1 'ps aux | grep cargo | grep -v grep'

# Valgrind check
valgrind --leak-check=full cargo test --release test_query_end_to_end

# Flamegraph
cargo install flamegraph
cargo flamegraph --bin loom-server --test test_query_end_to_end
```

### Query Processing Speed

```rust
#[bench]
fn bench_query_dispatch(b: &mut Bencher) {
    let manager = ServerQueryManager::new();
    let query = ServerQuery::read_file("src/main.rs");

    b.iter(|| {
        black_box(manager.send_query("session-123", query.clone()))
    });
}

// Run with:
// cargo bench test_query_dispatch
```

---

## Testing Checklist {#testing-checklist}

### Unit Tests
- [ ] ServerQuery serialization roundtrip
- [ ] ServerQueryResponse deserialization
- [ ] Query ID format (`Q-` prefix + uuid7)
- [ ] Timeout validation (1-300 seconds)
- [ ] Error message formatting

### Integration Tests
- [ ] Server → Client query transmission
- [ ] Client response processing
- [ ] Session creation/cleanup
- [ ] Concurrent session handling
- [ ] Query timeout mechanism

### Security Tests
- [ ] Path traversal prevention
- [ ] Workspace boundary enforcement
- [ ] File permission checks
- [ ] Sensitive data handling
- [ ] Command execution disabled

### Performance Tests
- [ ] Single query latency < 100ms
- [ ] 10 concurrent queries < 500ms
- [ ] 100 file reads < 1s
- [ ] Memory stable under load
- [ ] No resource leaks

### Run Full Test Suite

```bash
# Run all tests
make test

# With coverage
cargo tarpaulin --out Html

# With logging
RUST_LOG=debug cargo test -- --nocapture

# Performance tests
cargo test --release -- --nocapture
```

---

## Advanced Diagnostics

### Network Tracing

```bash
# Capture HTTP traffic
tcpdump -i lo port 8080 -A -s0

# Analyze with Wireshark
tcpdump -i lo port 8080 -w query_traffic.pcap
wireshark query_traffic.pcap

# Using curl verbose
curl -v -X POST ... 2>&1 | grep -E "^>|^<|{.*}"
```

### Database Query Log

```bash
# If using SQLite backend
SQLITE_LOG=debug cargo run

# Check query log
sqlite3 loom.db "SELECT * FROM server_queries ORDER BY sent_at DESC LIMIT 10;"
```

### Panic Analysis

```bash
# Get full backtrace
RUST_BACKTRACE=full cargo run

# Save backtrace
RUST_BACKTRACE=1 cargo run 2>&1 | tee backtrace.log

# Analyze with addr2line
addr2line -e target/debug/loom <address>
```

---

## Escalation Path

If issue persists:

1. **Gather diagnostics:**
```bash
RUST_LOG=debug cargo test -- --nocapture 2>&1 | tee debug.log
cargo test --release -- --nocapture 2>&1 | tee release.log
```

2. **Create minimal reproduction:**
```rust
#[tokio::test]
async fn reproduce_issue() {
    // Minimal code that reproduces the problem
}
```

3. **Open issue with:**
- [ ] Debug logs (sanitized)
- [ ] Minimal reproduction test
- [ ] Expected vs actual behavior
- [ ] Environment (OS, Rust version, etc.)

---

## Summary

**Quick Fix Strategy:**
1. Check logs: `RUST_LOG=debug cargo run`
2. Identify component: Server/Client/Network
3. Match symptom to issue above
4. Apply solution
5. Test with provided commands

**Prevention:**
- Always run: `make check` before committing
- Monitor logs in production
- Test timeout scenarios
- Validate path security

---

**Last Updated:** 2025-01-20  
**Next Review:** When Phase 2 is integrated
