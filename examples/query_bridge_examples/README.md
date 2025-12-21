# Server-to-Client Query Bridge - Comprehensive Examples

This example suite demonstrates the complete Server-to-Client Query Bridge implementation with 5 working scenarios.

## Quick Start

Run all examples:

```bash
cargo run -p query_bridge_examples
```

Or from the workspace root:

```bash
cd /home/ghuntley/loom
cargo run -p query_bridge_examples
```

## Examples Included

### Example 1: Basic File Read During Coding
**File:** `src/main.rs` - Lines 110-183  
**Purpose:** Demonstrates the most common query type - reading files from the client's workspace

**Scenario:**
- LLM: "I'll add logging to your main.rs"
- System detects "main.rs" reference
- Sends `ReadFile("src/main.rs")` query
- Client returns file contents
- LLM resumes with full context

**Why Important:**
File context is essential for any code modification task. The LLM needs to read the current state before making changes.

**Key Assertions:**
- Query ID is properly formatted (`Q-{32 hex digits}`)
- Response contains file content
- No errors occur for existing files

---

### Example 2: Environment + Workspace Context
**File:** `src/main.rs` - Lines 192-250  
**Purpose:** Shows how to query environment variables and workspace state

**Scenario:**
- Query 1: `GetEnvironment(["DEPLOY_HOST", "API_KEY"])`
- Query 2: `GetWorkspaceContext`
- LLM uses both for deployment decisions

**Why Important:**
Production code changes require knowledge of:
- Deployment targets and secrets
- Git branch and workspace state
- Build configuration

This enables the LLM to make context-aware decisions without hardcoding env vars.

**Key Assertions:**
- Environment variables are retrieved correctly
- Workspace context includes git information
- Multiple sequential queries work

---

### Example 3: Human-in-the-Loop Approval
**File:** `src/main.rs` - Lines 259-308  
**Purpose:** Demonstrates pausing for user confirmation on destructive operations

**Scenario:**
- LLM: "Should I delete this old migration?"
- System sends: `RequestUserInput(prompt="Delete?", type="yes_no")`
- User responds: "yes"
- LLM proceeds with deletion

**Why Important:**
Prevents accidental data loss. For destructive operations (deletes, database changes), the system can pause and request explicit human approval.

**Key Assertions:**
- User input query works
- Response correctly matches user choice
- LLM decision logic can use the response

---

### Example 4: Error Recovery and Timeout Handling
**File:** `src/main.rs` - Lines 317-390  
**Purpose:** Shows graceful error handling and recovery patterns

**Scenario:**
- Query attempts to read nonexistent file
- System detects error
- Offers recovery strategies (retry, log, prompt)
- Retries with correct path
- Succeeds on retry

**Why Important:**
Network timeouts, permission errors, and missing files will happen. The system must:
1. Report errors clearly
2. Offer recovery options
3. Allow LLM to retry or continue with degraded context

**Key Assertions:**
- Errors are handled without panicking
- Recovery strategies are available
- Retry succeeds after path correction

---

### Example 5: Concurrent Queries from Multiple Sessions
**File:** `src/main.rs` - Lines 399-540  
**Purpose:** Verifies that multiple sessions can query concurrently without interference

**Scenario:**
- Session A: `ReadFile("src/module_a.rs")`
- Session B: `GetEnvironment(["SESSION_B_KEY"])`
- Session C: `GetWorkspaceContext`
- All process concurrently
- Responses routed to correct sessions

**Why Important:**
The server handles many concurrent user sessions. This test verifies:
- Query isolation between sessions
- Concurrent processing works
- Responses route to correct clients
- No cross-session data leakage

**Key Assertions:**
- All 3 sessions complete successfully
- No interference between sessions
- All assertions pass independently

---

## Architecture

Each example follows this flow:

```
┌──────────────────────────────────────────┐
│ Setup                                    │
│ - Create MockServerQueryHandler          │
│ - Prepare query with test data           │
└──────────────────────────────────────────┘
                    ↓
┌──────────────────────────────────────────┐
│ Execution                                │
│ - Handler processes query                │
│ - Returns ServerQueryResponse            │
│ - May include result or error            │
└──────────────────────────────────────────┘
                    ↓
┌──────────────────────────────────────────┐
│ Verification                             │
│ - Assert response structure              │
│ - Assert result correctness              │
│ - Assert no unexpected errors            │
└──────────────────────────────────────────┘
```

## Mock Handler

The `MockServerQueryHandler` simulates client behavior without requiring actual filesystem access:

```rust
impl ServerQueryHandler for MockServerQueryHandler {
    async fn handle_query(&self, query: ServerQuery) -> Result<ServerQueryResponse, ServerQueryError> {
        match &query.kind {
            ServerQueryKind::ReadFile { path } => { /* return mock content */ }
            ServerQueryKind::GetEnvironment { keys } => { /* return env vars */ }
            ServerQueryKind::GetWorkspaceContext => { /* return mock context */ }
            ServerQueryKind::RequestUserInput { .. } => { /* simulate user input */ }
            _ => { /* error for unsupported types */ }
        }
    }
}
```

## Running Individual Examples

To run only one example, modify `main.rs` and comment out the others:

```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    
    // Run only Example 1:
    example_1_basic_file_read().await;
    
    // Comment out:
    // example_2_environment_context().await;
    // example_3_human_approval().await;
    // etc.
}
```

## Extending the Examples

To add a new example:

1. Create a new async function:
```rust
async fn example_6_my_new_scenario() {
    println!("\n=== Example 6: My Scenario ===\n");
    // Your test code
}
```

2. Add to `main()`:
```rust
example_6_my_new_scenario().await;
```

3. Follow the pattern:
   - Step 1-3: Setup
   - Step 4-5: Execution
   - Step 6: Verification
   - Assertions at the end

## Testing

Run tests:
```bash
cargo test -p query_bridge_examples
```

Build without running:
```bash
cargo build -p query_bridge_examples
```

Check for warnings:
```bash
cargo clippy -p query_bridge_examples
```

## Output Example

```
╔══════════════════════════════════════════════════════════════╗
║  Server-to-Client Query Bridge - Comprehensive Examples      ║
║  Demonstrating the complete query lifecycle                  ║
╚══════════════════════════════════════════════════════════════╝

=== Example 1: Basic File Read ===

Step 1: LLM needs to read src/main.rs
  Query ID: Q-00000000000000000000000000000001
  ...

✅ Example 1 Complete

=== Example 2: Environment + Workspace Context ===
...

✅ All Examples Completed Successfully
```

## Key Features Demonstrated

✅ **Type Safety**: Strongly-typed query kinds and results  
✅ **Error Handling**: Graceful error recovery patterns  
✅ **Async/Await**: Tokio-based concurrent processing  
✅ **Session Isolation**: Multiple concurrent sessions work independently  
✅ **Structured Logging**: Tracing spans for debugging  
✅ **Assertions**: Property-based verification of results  

## Dependencies

- `loom-core`: Core types (ServerQuery, ServerQueryResponse, etc.)
- `tokio`: Async runtime
- `serde_json`: JSON serialization
- `tracing`: Structured logging
- `chrono`: Timestamp generation

## Integration with Loom

These examples form the foundation for:
1. **LLM Processing Integration**: Embedding query handling in the inference loop
2. **Editor Handlers**: Custom implementations for VSCode, Zed, etc.
3. **Persistent Storage**: Adding query result caching
4. **Analytics**: Tracking query patterns and performance

## Next Steps

After understanding these examples:

1. **Phase 2**: Integrate into LLM processing loop
   - Detect context requests in LLM output
   - Send ServerQueries automatically
   - Inject results back into context

2. **Phase 3**: WebSocket upgrade
   - Replace HTTP/SSE with persistent WebSocket
   - Lower latency for rapid back-and-forth queries
   - Bidirectional flow

3. **Phase 4**: Advanced handlers
   - Editor-specific UI (VSCode quick pick, etc.)
   - Command execution with safety policies
   - Database query support

## References

- [README_QUERY_BRIDGE.md](../../README_QUERY_BRIDGE.md) - Architecture overview
- [QUICK_START_QUERY_BRIDGE.md](../../QUICK_START_QUERY_BRIDGE.md) - Quick reference
- [IMPLEMENTATION_SERVER_CLIENT_QUERY_BRIDGE.md](../../IMPLEMENTATION_SERVER_CLIENT_QUERY_BRIDGE.md) - Detailed implementation

## License

Same as Loom (MIT)
