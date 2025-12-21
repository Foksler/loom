# Query Bridge Comprehensive Examples

Complete working examples demonstrating the server-to-client query bridge functionality with 5 realistic scenarios.

**Location**: [examples/query_bridge_examples/examples/query_bridge_scenarios.rs](file:///home/ghuntley/loom/examples/query_bridge_examples/examples/query_bridge_scenarios.rs)

**Run**: `cargo run --example query_bridge_scenarios`

---

## Overview

This comprehensive example file demonstrates all core query bridge patterns:

1. **Basic File Read** - Most common pattern for LLM code context
2. **Environment + Workspace Context** - Multi-query deployment scenarios
3. **Error Handling** - Graceful recovery and retry mechanisms
4. **Concurrent Sessions** - Multiple clients with isolation
5. **Custom Queries** - Extensible query types

Each example includes:
- ✅ Full setup and teardown
- ✅ Clear step-by-step flow
- ✅ Detailed comments explaining purpose and importance
- ✅ Assertions verifying correct behavior
- ✅ Logging at INFO level for observability
- ✅ Realistic mock data

---

## Example 1: Basic File Read During Coding

**File**: [query_bridge_scenarios.rs#L207-L282](file:///home/ghuntley/loom/examples/query_bridge_examples/examples/query_bridge_scenarios.rs#L207-L282)

### Purpose
Demonstrates the most common query pattern: reading a file to provide context for code modifications.

### Why Important
When an LLM mentions a specific file, the server detects the pattern and reads the file to provide full context. This enables intelligent code modifications based on current file state.

### Flow
1. LLM output mentions "main.rs"
2. Pattern detector identifies file reference
3. Creates `ReadFile("src/main.rs")` query
4. Client reads file asynchronously
5. Returns content via `ServerQueryResponse`
6. LLM resumes with full context

### Key Assertions
- ✓ Query ID correlates correctly (Q-{32 hex})
- ✓ File content returned without errors
- ✓ Content is non-empty and relevant
- ✓ Query processing is fast (10ms mock delay)

### Test Output
```
STEP 1: LLM is about to modify src/main.rs
STEP 2: Pattern Detector identifies 'main.rs'
STEP 3: Client processes query
  ✓ Query handled successfully
  Content: 44 bytes
  Preview: fn main() {
STEP 4: Verify Results
  ✓ Query ID correlates correctly
  ✓ File content returned
  ✓ No errors occurred
✅ Example 1 PASSED
```

---

## Example 2: Environment + Workspace Context

**File**: [query_bridge_scenarios.rs#L294-L361](file:///home/ghuntley/loom/examples/query_bridge_examples/examples/query_bridge_scenarios.rs#L294-L361)

### Purpose
Demonstrates querying multiple pieces of context simultaneously for deployment decisions.

### Why Important
Deployment and infrastructure decisions require multiple data sources. The system can issue parallel queries to gather environment variables, workspace configuration, and version information without hardcoding secrets or infrastructure details.

### Flow
1. LLM needs deployment information
2. Query 1: `GetEnvironment([DEPLOY_HOST, API_KEY, DATABASE_URL])`
3. Query 2: `GetWorkspaceContext`
4. Both execute in parallel via `tokio::join!`
5. Results injected back into LLM context

### Key Assertions
- ✓ Both queries processed successfully
- ✓ Environment variables returned (3 variables)
- ✓ Workspace context includes git branch, languages
- ✓ Deployment info available for LLM decision

### Test Output
```
STEP 4: Execute queries in parallel
  ✓ Both queries completed

STEP 5: Process Query 1 Response
  Retrieved 3 environment variables:
    • DATABASE_URL: [redacted]
    • DEPLOY_HOST: prod.example.com
    • API_KEY: sk-demo-key-123

STEP 6: Process Query 2 Response
  Retrieved workspace context:
    • Workspace: "/home/user/project"
    • Git Branch: "main"
    • Languages: ["rust","typescript"]

✅ Example 2 PASSED
```

---

## Example 3: Error Handling & Graceful Recovery

**File**: [query_bridge_scenarios.rs#L406-L477](file:///home/ghuntley/loom/examples/query_bridge_examples/examples/query_bridge_scenarios.rs#L406-L477)

### Purpose
Demonstrates graceful error handling and recovery strategies.

### Why Important
In production, queries can fail due to:
- Missing files
- Permission denied
- Network timeouts
- Client disconnections

The system must handle these gracefully, report errors clearly, and allow retries with different parameters or fallback strategies.

### Flow
1. Query for file that might not exist
2. System detects error condition
3. Error reported to LLM
4. LLM retries with correct/alternate path
5. Retry succeeds with fallback

### Recovery Strategies
- Check alternate paths
- List available files
- Ask user for correct path
- Continue with degraded context

### Key Assertions
- ✓ Error detected and reported properly
- ✓ Retry with different path succeeds
- ✓ System remains stable after error
- ✓ No cascading failures

### Test Output
```
STEP 5: Retry with Correct Path
  ✓ Retry successful
  ✓ Retrieved fallback file

STEP 6: Resume Processing
  ✓ Error recovered from gracefully
  ✓ LLM can continue with alternative file
  ✓ System remains stable

✅ Example 3 PASSED
```

---

## Example 4: Concurrent Sessions

**File**: [query_bridge_scenarios.rs#L495-L642](file:///home/ghuntley/loom/examples/query_bridge_examples/examples/query_bridge_scenarios.rs#L495-L642)

### Purpose
Demonstrates multiple concurrent client sessions processing queries independently.

### Why Important
The server handles many concurrent user sessions simultaneously. Each session can issue queries independently. The system must:
- Route responses to correct sessions
- Maintain session isolation
- Process queries concurrently
- Prevent cross-session contamination

### Flow
1. Create 3 concurrent session tasks (A, B, C)
2. Session A: File read query
3. Session B: Environment query
4. Session C: Workspace context query
5. All execute in parallel via `tokio::spawn`
6. Responses routed correctly to each session

### Parallel Queries
- **Session A**: ReadFile("src/module_a.rs")
- **Session B**: GetEnvironment(["DEPLOY_HOST"])
- **Session C**: GetWorkspaceContext

### Key Assertions
- ✓ All 3 sessions complete successfully
- ✓ Each receives correct response (not mixed)
- ✓ No cross-session contamination
- ✓ Concurrent processing verified

### Test Output
```
STEP 2: All queries executing concurrently

  Session A: Starting file read query
  Session B: Starting environment query
  Session C: Starting workspace context query

STEP 3: Collect Results
  ✓ Session A: File (12 bytes)
  ✓ Session B: Environment (1 vars)
  ✓ Session C: Context retrieved

STEP 4: Verify Results
  ✓ All 3 sessions completed successfully
  ✓ Each session received correct response
  ✓ No cross-session contamination

✅ Example 4 PASSED
```

---

## Example 5: Custom Query Type

**File**: [query_bridge_scenarios.rs#L646-L712](file:///home/ghuntley/loom/examples/query_bridge_examples/examples/query_bridge_scenarios.rs#L646-L712)

### Purpose
Demonstrates extensible custom query types for application-specific needs.

### Why Important
The standard query types (ReadFile, GetEnvironment, etc.) cover common cases, but different tools and use cases may need custom queries. The system supports arbitrary custom queries with:
- Pattern detection for custom types
- Custom handlers that process requests
- Extensibility without modifying core types
- Type-safe custom result payloads

### Flow
1. LLM mentions file metadata need
2. Pattern detector recognizes "GetFileMetadata" pattern
3. Create `Custom { name: "GetFileMetadata", payload: {...} }`
4. Client processes via custom handler
5. Returns `ServerQueryResult::Custom` with metadata
6. LLM uses metadata for intelligent decisions

### Custom Query Structure
```rust
ServerQueryKind::Custom {
    name: "GetFileMetadata",
    payload: {
        "file_path": "src/main.rs",
        "fields": ["size", "modified", "permissions"]
    }
}
```

### Key Assertions
- ✓ Custom query processed by correct handler
- ✓ Result includes requested metadata
- ✓ Payload structure matches expectations
- ✓ LLM can make decisions based on metadata

### Test Output
```
STEP 3: Client processes custom query
  ✓ Custom query processed

STEP 4: Parse custom result
  Query type: GetFileMetadata
  Result metadata:
    • Size: 1024 bytes
    • Modified: "2025-01-15T10:30:00Z"
    • Permissions: "rw-r--r--"

STEP 5: LLM Uses Custom Metadata
  ✓ LLM can now make intelligent decisions
  ✓ Has full file metadata available
  ✓ Custom query mechanism works

✅ Example 5 PASSED
```

---

## Architecture

### Mock Client Handler

The `MockClientHandler` simulates client-side processing:

```rust
struct MockClientHandler {
    processed_count: Arc<AtomicU32>,
}

#[async_trait::async_trait]
impl ServerQueryHandler for MockClientHandler {
    async fn handle_query(&self, query: ServerQuery) 
        -> Result<ServerQueryResponse, ServerQueryError> {
        // Simulate processing with realistic delays
        match &query.kind {
            ServerQueryKind::ReadFile { path } => { ... }
            ServerQueryKind::GetEnvironment { keys } => { ... }
            ServerQueryKind::GetWorkspaceContext => { ... }
            // etc.
        }
    }
}
```

### Key Features

1. **Structured Logging**: Uses `tracing` for observability
2. **Async/Await**: Demonstrates async patterns with `tokio`
3. **Concurrent Processing**: Examples 2 and 4 use `tokio::join!` and `tokio::spawn`
4. **Realistic Delays**: Mock handlers include small delays (5-50ms) to simulate real I/O
5. **Error Cases**: Example 3 demonstrates both success and error paths
6. **Custom Query Support**: Example 5 shows extensibility pattern

---

## Running the Examples

### Single Example Run
```bash
cargo run --example query_bridge_scenarios
```

### With Specific Log Level
```bash
RUST_LOG=debug cargo run --example query_bridge_scenarios
```

### From Project Root
```bash
cd /home/ghuntley/loom
cargo run -p query_bridge_examples --example query_bridge_scenarios
```

---

## Expected Output

All 5 examples execute sequentially and complete with:
- ✅ Clear step-by-step output
- ✅ Query processing logged at INFO level
- ✅ All assertions passed
- ✅ Summary of key takeaways
- ✅ Suggestions for next steps

Total runtime: ~500ms (with simulated I/O delays)

---

## Integration Points

### For LLM Integration
1. Hook into LLM response processing
2. Detect file references using patterns
3. Create ReadFile queries before feeding to LLM
4. Inject responses back into context

### For Server Implementation
1. Implement `ServerQueryManager` to route queries
2. Track pending queries per session
3. Match responses via query ID correlation
4. Handle timeouts and retries

### For Client Implementation
1. Implement `ServerQueryHandler` trait
2. Listen for queries via SSE/WebSocket
3. Process query kinds appropriately
4. Send responses via HTTP POST

---

## Testing & Verification

✅ All examples compile without warnings
✅ All 5 examples execute successfully
✅ All assertions pass
✅ Concurrent sessions demonstrate isolation
✅ Error handling shows recovery
✅ Custom queries demonstrate extensibility

---

## What's Demonstrated

| Aspect | Example | Details |
|--------|---------|---------|
| Basic Query | #1 | File read with pattern detection |
| Parallel Queries | #2 | Multiple concurrent queries |
| Error Recovery | #3 | Timeout, retry, graceful degradation |
| Session Isolation | #4 | 3 concurrent clients without interference |
| Extensibility | #5 | Custom query types and handlers |

---

## Next Steps

1. **Integrate into LLM Processing Loop**
   - Detect file references in LLM output
   - Issue queries before resuming

2. **Add Editor-Specific Handlers**
   - VS Code extension
   - JetBrains IDEs
   - Custom tools

3. **Implement Query Result Caching**
   - Cache file contents
   - Invalidate on file changes
   - Reduce round-trip latency

4. **Add Metrics & Observability**
   - Query latency histograms
   - Error rate tracking
   - Session statistics

5. **Production Hardening**
   - Rate limiting per session
   - Resource limits
   - Security validation
