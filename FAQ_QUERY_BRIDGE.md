# FAQ: Server-to-Client Query Bridge

> **Purpose:** Answer common questions about design, performance, and features  
> **Format:** Quick Q&A  
> **Updated:** 2025-01-20

## Table of Contents
1. [Design & Architecture Q&A](#design)
2. [Performance Q&A](#performance)
3. [Security Q&A](#security)
4. [Implementation Q&A](#implementation)
5. [Future Features Q&A](#future)
6. [Troubleshooting Q&A](#troubleshooting)

---

## Design & Architecture Q&A {#design}

### Q: Why do we need a query bridge?

**A:** LLMs are running on the server, but need information from the client (workspace files, environment, user decisions). The query bridge enables the server to request this information and incorporate it into LLM processing—creating a tighter feedback loop.

**Without bridge:**
- LLM generates output, server sends it
- Client reads it, runs commands, reports back
- Very slow, many round-trips

**With bridge:**
- LLM says "I need to check src/main.rs"
- Server sends query, gets file content
- LLM continues with actual code context
- One round-trip instead of three

---

### Q: Why SSE + HTTP instead of WebSocket?

**A:** Phase 1 uses SSE (Server-Sent Events) for server→client queries, and HTTP POST for client→server responses.

**SSE advantages:**
- ✅ Simple, HTTP-based (no special protocol)
- ✅ Works through most firewalls/proxies
- ✅ Built-in reconnection handling
- ✅ Easy to debug (just HTTP)
- ✅ Less resource overhead initially

**SSE disadvantages:**
- ❌ Half-duplex (server → client only)
- ❌ Separate HTTP channel for responses
- ❌ Higher latency than WebSocket

**Phase 3 plan:** Upgrade to WebSocket for lower latency and true bidirectional communication.

---

### Q: What's the difference between queries and tools?

**A:** 
- **Tools** = Functions the LLM can call (calculate, format, etc.)
- **Queries** = Information requests from server to client

Tools are inside the model context. Queries are outside—they cross the server-client boundary.

Example:
```
Tool:  LLM says "calculate 2+2" → Tool execution → Returns "4"
Query: LLM says "I need src/main.rs" → Query to client → Returns file content
```

---

### Q: Can queries be nested?

**A:** Currently, no. A query waits for a response before the LLM continues. Nested queries would require:
- State machine complexity
- Conversation history tracking
- Potential deadlock scenarios

**Future consideration:** Async queries that don't block LLM processing.

---

### Q: How is conversation state preserved?

**A:** The `LlmContextRestorer` (Phase 2) maintains conversation history:

```rust
[LLM] "I'll read main.rs"
[System] *Checkpoint taken*
→ *Query sent to client*
[Client] *File read*
→ *Response received*
[System] "I found: <file content>"
[LLM] <continues with new context>
```

Each checkpoint preserves:
- Original LLM output
- Query ID
- Response received
- Time taken

---

### Q: What happens if a query fails?

**A:** Multiple recovery strategies:

1. **Timeout** (most common): Query waits 30s for response
   - Server resumes: "User didn't respond. Proceeding with defaults..."

2. **File not found**: Client returns error
   - Server resumes: "File missing. Trying alternative..."

3. **Permission denied**: Security check failed
   - Server resumes: "Access denied. Different approach needed..."

4. **Network error**: Connection lost
   - Server: Timeout after 30s, resumes with error message

---

## Performance Q&A {#performance}

### Q: How fast are queries?

**A:** Typical latencies:

| Operation | Latency | Notes |
|-----------|---------|-------|
| ReadFile (small) | 10-50ms | < 10KB file |
| ReadFile (large) | 50-500ms | 1-10MB file |
| GetEnvironment | 5-15ms | Simple lookup |
| GetWorkspaceContext | 20-100ms | Spawns git process |
| User input | 1000-300000ms | Depends on user response |

**Theoretical minimum:** ~5ms (network + serialization)
**Practical target:** <100ms for file reads

---

### Q: How do we optimize query performance?

**A:** Several strategies:

1. **Batching queries** (multiple files at once):
```rust
// Instead of:
file1 = query(path1)
file2 = query(path2)
// → 2 queries, 100ms each = 200ms

// Use:
[file1, file2] = query_batch([path1, path2])
// → 1 round-trip, ~100ms
```

2. **Caching query responses** (Phase 2):
```rust
// First query: 100ms
let file1 = query(path, session)

// Second query: 1ms (cached)
let file1_again = query(path, session)
```

3. **Reducing file sizes** (application-specific):
```rust
// Instead of:
file = query("/huge_log.txt")  // 10MB, 500ms

// Use:
last_100_lines = query("/huge_log.txt", range=(lines=-100))  // 10KB, 50ms
```

4. **Increasing timeout** for slow files:
```rust
let query = ServerQuery {
    timeout_secs: 60,  // Give client 60 seconds
    ..default
};
```

---

### Q: Can queries handle large files?

**A:** Yes, but with caveats:

```
File Size | Read Time | Recommendation
1MB       | ~100ms    | No problem
10MB      | ~1s       | OK, maybe cache result
100MB     | ~10s      | Increase timeout, consider streaming
1GB+      | ~100s+    | Use streaming (Phase 3)
```

**For large files, consider:**
- Reading only needed sections
- Caching results
- Streaming transfer (future feature)

---

### Q: How many concurrent queries are supported?

**A:** Theoretically unlimited, practically limited by:

| Factor | Limit | Bottleneck |
|--------|-------|-----------|
| Memory | ~1000 queries | Server memory |
| File descriptor | ~1024 | OS limit |
| Network bandwidth | ~1 Gbps | Connection speed |
| Client processing | Variable | Client CPU/disk |

**Recommendation:** <100 concurrent queries for stability.

If you need more: use batching or implement queuing.

---

### Q: What's the memory overhead per query?

**A:** Approximately **2-5 KB** per pending query:

```rust
ServerQuery {
    id: "Q-..." (36 bytes)
    kind: {...} (variable, ~200 bytes typical)
    sent_at: "2025-01-20T..." (30 bytes)
    timeout_secs: 30 (4 bytes)
    metadata: {...} (variable, ~200 bytes)
}

Total: ~500 bytes typical
With overhead: ~2-5 KB allocation
```

**1000 pending queries:** ~5MB memory

---

## Security Q&A {#security}

### Q: Can clients read files outside the workspace?

**A:** **No.** The query handler enforces workspace boundaries:

```rust
// Client tries:
ReadFile { path: "../../etc/passwd" }

// Handler checks:
if !requested_path.starts_with(workspace_root) {
    return Err("Path traversal not allowed")
}

// Result: ❌ Access denied
```

---

### Q: Can LLMs execute arbitrary commands?

**A:** **No.** ExecuteCommand is:
1. **Explicitly disabled** in current implementation
2. Would require opt-in at configuration
3. Even if enabled, only whitelisted commands allowed

```rust
// Current:
ServerQueryKind::ExecuteCommand { ... } 
→ return Err("Command execution disabled for security")

// Future (if enabled):
ServerQueryKind::ExecuteCommand { command: "ls", args: [...] }
→ Check against whitelist
→ Run in isolated sandbox
```

---

### Q: What information is logged?

**A:** Detailed logging (for debugging) includes:

```
[DEBUG] Query sent
    query_id: "Q-..."
    session_id: "session-123"
    kind: ReadFile { path: "src/main.rs" }
    metadata: {...}
```

**Sensitive data handling:**
- ✅ File paths logged (not contents)
- ✅ Environment variable names logged (not values)
- ⚠️ Query responses may contain file contents (be careful with logs)
- ❌ Never log passwords, secrets, tokens

**Recommendation:** Strip sensitive data from logs in production.

---

### Q: Can a malicious client exploit the query system?

**A:** Limited attack surface:

| Attack | Defense |
|--------|---------|
| **Path traversal** | Workspace boundary check |
| **Reading private files** | File permission check |
| **Slow loris (timeout)** | Query timeout enforced |
| **Memory exhaustion** | Rate limiting (future) |
| **Fake responses** | Session ID validation |
| **Man-in-the-middle** | HTTPS in production required |

**Production recommendations:**
- Use HTTPS for all connections
- Validate client certificates
- Rate limit queries per session
- Monitor for suspicious patterns
- Audit logs regularly

---

## Implementation Q&A {#implementation}

### Q: How do I add a new query type?

**A:** 

1. **Add variant to ServerQueryKind** (crates/loom-core/src/server_query.rs):
```rust
pub enum ServerQueryKind {
    // ... existing ...
    MyCustomQuery { param: String },
}
```

2. **Add variant to ServerQueryResult** (same file):
```rust
pub enum ServerQueryResult {
    // ... existing ...
    MyCustomResult { data: String },
}
```

3. **Implement handler** (crates/loom-acp/src/agent.rs):
```rust
ServerQueryKind::MyCustomQuery { param } => {
    // Process query
    Ok(ServerQueryResponse {
        query_id: query.id.clone(),
        sent_at: now_rfc3339(),
        result: Some(ServerQueryResult::MyCustomResult { 
            data: "result".to_string() 
        }),
        error: None,
    })
}
```

4. **Add test**:
```rust
#[tokio::test]
async fn test_custom_query() {
    // Test implementation
}
```

5. **Update documentation** with usage example.

---

### Q: How do I extend the query system?

**A:** The `Custom` variant supports extensible queries:

```rust
ServerQueryKind::Custom {
    name: "my_operation".to_string(),
    payload: json!({
        "param1": "value",
        "param2": 123,
    }),
}

// Response:
ServerQueryResult::Custom {
    name: "my_operation".to_string(),
    payload: json!({
        "status": "success",
        "result": {...}
    }),
}
```

**Advantages:**
- ✅ No code change required
- ✅ Flexible schema
- ✅ Forward compatible

**Disadvantages:**
- ❌ No type safety
- ❌ Manual JSON handling
- ❌ Validation burden

**Recommendation:** Use Custom for prototyping, create proper variant for production.

---

### Q: Can I use queries outside of LLM context?

**A:** **Yes.** Queries work independently:

```rust
// Standalone usage (no LLM involved)
let manager = ServerQueryManager::new();
let query = ServerQuery::read_file("src/main.rs");
let response = manager.send_query("session-123", query).await?;

// Just a client-server communication mechanism
// LLM is optional
```

---

### Q: How do I test query functionality?

**A:** 

```rust
// Unit test (mock)
#[test]
fn test_query_serialization() {
    let query = ServerQuery::read_file("test.rs");
    let json = serde_json::to_string(&query).unwrap();
    let deserialized = serde_json::from_str(&json).unwrap();
    assert_eq!(query.id, deserialized.id);
}

// Integration test (real manager)
#[tokio::test]
async fn test_query_dispatch() {
    let manager = ServerQueryManager::new();
    let query = ServerQuery::read_file("Cargo.toml");
    let response = manager.send_query("session-123", query).await?;
    assert!(response.result.is_some());
}

// End-to-end test (server + client)
#[tokio::test]
async fn test_full_flow() {
    let server = start_test_server().await;
    let client = start_test_client().await;
    // Test complete flow
}
```

---

## Future Features Q&A {#future}

### Q: What's the Phase 2 plan?

**A:** **LLM Integration** (4-6 hours)

- ✅ Query extraction from LLM output
- ✅ Automatic query dispatch
- ✅ Context restoration for LLM continuation
- ✅ Error recovery
- ✅ Testing & documentation

Enables LLMs to transparently read files during reasoning.

---

### Q: What's the Phase 3 plan?

**A:** **WebSocket Upgrade** (8 hours)

**Changes:**
- Replace SSE + HTTP with WebSocket
- True bidirectional communication
- Lower latency (~5-10ms vs ~50-100ms)
- Persistent connection

**Impact:**
- ✅ Faster query response
- ✅ Simpler code (single connection)
- ✅ Better for real-time features
- ❌ Less compatible with certain networks
- ❌ Requires client upgrade

---

### Q: What about streaming large files?

**A:** **Future enhancement** for Phase 4+

```rust
// Currently:
ReadFile { path: "huge.log" }
→ Entire file (~100MB) loaded into memory
→ Sent over HTTP
→ Deserialized on client
→ Risk of OOM

// Future (streaming):
ReadFileStream { path: "huge.log", chunk_size: 1MB }
→ File streamed in chunks
→ Client processes incrementally
→ Memory usage constant
→ Can cancel mid-stream
```

**Timeline:** Estimated 6-8 hours

---

### Q: Can we cache query responses?

**A:** **Yes, in Phase 2+**

```rust
pub struct CachingDispatcher {
    cache: HashMap<(PathBuf, String), CacheEntry>,
}

impl CachingDispatcher {
    pub async fn dispatch(&self, query: ServerQuery) -> Result<...> {
        let key = (path.clone(), session_id.clone());
        
        // Check cache
        if let Some(entry) = self.cache.get(&key) {
            if !entry.is_stale() {
                return Ok(entry.response.clone());
            }
        }

        // Fetch fresh
        let response = manager.send_query(...).await?;
        self.cache.insert(key, CacheEntry::new(response.clone()));
        Ok(response)
    }
}
```

**Considerations:**
- Cache invalidation (how long?)
- Memory consumption
- Stale data issues

---

### Q: Can we batch queries?

**A:** **Yes, in Phase 2+**

```rust
// Read multiple files efficiently
let paths = vec!["src/main.rs", "src/lib.rs", "Cargo.toml"];
let results = dispatcher.batch_read(session_id, paths).await?;

// Sends all queries concurrently
// Waits for all responses
// ~100ms instead of 300ms (3 × 100ms)
```

---

### Q: Will queries work with offline clients?

**A:** **Not in current design.** Queries require client connection.

**Workarounds:**
1. **Cache query responses** during online time
2. **Pre-fetch common files** before going offline
3. **Fallback handler** that returns cached data

**Future:** Offline-first mode (Phase 4+) with sync when client reconnects.

---

## Troubleshooting Q&A {#troubleshooting}

### Q: Query times out but client is running?

**A:** Check these in order:

1. **Is client subscribed to SSE?**
   ```bash
   curl -i http://localhost:8080/v1/sessions/test/listen
   # Should show "200 OK" with streaming response
   ```

2. **Is handler implemented for query type?**
   ```rust
   match query.kind {
       ServerQueryKind::ReadFile { .. } => { /* handler */ }
       _ => Err("unsupported query type")
   }
   ```

3. **Is response being sent?**
   ```bash
   RUST_LOG=debug cargo run 2>&1 | grep "query-response"
   # Should see POST request from client
   ```

4. **Increase timeout**:
   ```rust
   let query = ServerQuery {
       timeout_secs: 60,  // Was 30
       ..default
   };
   ```

---

### Q: Why am I getting "path traversal" errors?

**A:** Client sent path like `../../etc/passwd`.

**Check:**
- Is path normalized? Use `Path::canonicalize()`
- Is workspace correctly set?
- Are symlinks involved?

**Fix:**
```rust
let workspace = env::current_dir()?.canonicalize()?;
let requested = workspace.join(&path).canonicalize()?;

if !requested.starts_with(&workspace) {
    return Err("path traversal not allowed");
}
```

---

### Q: Can I query the same file twice?

**A:** **Yes,** but consider caching for performance:

```rust
// Without caching: 2 queries, ~200ms
let file1 = query("src/main.rs").await?;  // ~100ms
let file1_again = query("src/main.rs").await?;  // ~100ms

// With caching: 2 queries, ~100ms
let file1 = query("src/main.rs").await?;  // ~100ms (fetch)
let file1_again = query("src/main.rs").await?;  // ~0ms (cached)
```

See Phase 2+ caching feature.

---

### Q: How do I debug a stuck query?

**A:**

```bash
# 1. Check pending queries
curl http://localhost:8080/v1/sessions/session-123/queries

# 2. Enable debug logs
RUST_LOG=debug cargo run

# 3. Look for:
# [INFO] Query sent: Q-...
# [WARN] Query timeout: Q-... (if timeout occurs)

# 4. Check client status
# Is client connected?
# Is handler running?
# Check client logs

# 5. If still stuck, kill query
# (implementation-specific, may need manual session cleanup)
```

---

### Q: Why can't my client send a response?

**A:** Check these:

1. **Content-Type header:**
   ```bash
   curl -X POST ... \
     -H "Content-Type: application/json"  # ← Required!
   ```

2. **Response format:**
   ```json
   {
     "query_id": "Q-...",
     "sent_at": "2025-01-20T12:00:00Z",
     "result": { ... },
     "error": null
   }
   ```

3. **Endpoint path:**
   ```
   POST /v1/sessions/{session_id}/query-response  ← Exact path
   ```

4. **Session ID matches:**
   ```
   Sent query in: session-123
   Response to: session-123  ← Must match!
   ```

---

## Summary

**Key Takeaways:**
- ✅ Queries bridge server-client gap
- ✅ SSE + HTTP design is simple & reliable
- ✅ Multiple safety checks prevent misuse
- ✅ Performance good for typical workloads
- ✅ Extensible for custom query types
- ✅ Phase 2+ adds LLM integration
- ✅ Phase 3+ adds WebSocket & streaming

**Next Steps:**
1. Read [PHASE_2_IMPLEMENTATION_GUIDE.md](PHASE_2_IMPLEMENTATION_GUIDE.md) for integration details
2. See [EXAMPLES_QUERY_BRIDGE.md](EXAMPLES_QUERY_BRIDGE.md) for code examples
3. Check [TROUBLESHOOTING_QUERY_BRIDGE.md](TROUBLESHOOTING_QUERY_BRIDGE.md) if issues arise

---

**Questions?** Check the relevant guide or open an issue.
