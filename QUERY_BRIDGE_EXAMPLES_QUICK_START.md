# Query Bridge Examples - Quick Start

## Run the Examples

```bash
cd /home/ghuntley/loom
cargo run --example query_bridge_scenarios
```

Or from the examples crate:
```bash
cargo run -p query_bridge_examples --example query_bridge_scenarios
```

## What You'll See

All 5 examples running sequentially with clear output showing:

1. **Example 1**: Basic file read (44 bytes)
2. **Example 2**: 3 environment variables + workspace context  
3. **Example 3**: Error handling with retry
4. **Example 4**: 3 concurrent sessions processing simultaneously
5. **Example 5**: Custom query with metadata response

## File Location

**Main Example File**: `examples/query_bridge_examples/examples/query_bridge_scenarios.rs`

**Full Documentation**: See `EXAMPLES_QUERY_BRIDGE_SCENARIOS.md`

## Key Patterns Demonstrated

### Pattern 1: Basic File Read
```rust
let query = ServerQuery {
    id: "Q-{32 hex}",
    kind: ServerQueryKind::ReadFile { path: "src/main.rs" },
    timeout_secs: 30,
    // ...
};
handler.handle_query(query).await.unwrap()
```
**Use Case**: LLM needs file context before modifying

### Pattern 2: Environment Query
```rust
ServerQueryKind::GetEnvironment {
    keys: vec!["DEPLOY_HOST", "API_KEY"]
}
```
**Use Case**: Deployment decisions without hardcoded secrets

### Pattern 3: Concurrent Queries
```rust
let (env, ctx) = tokio::join!(
    handler.handle_query(env_query),
    handler.handle_query(ctx_query)
);
```
**Use Case**: Gather multiple data sources in parallel

### Pattern 4: Concurrent Sessions
```rust
let handle = tokio::spawn(async move {
    handler.handle_query(query).await
});
```
**Use Case**: Multiple users simultaneously

### Pattern 5: Custom Queries
```rust
ServerQueryKind::Custom {
    name: "GetFileMetadata",
    payload: json!({ "fields": ["size", "modified"] })
}
```
**Use Case**: Application-specific queries

## Output Example

```
======================================================================
  Server-to-Client Query Bridge - Comprehensive Examples
  5 Complete Working Scenarios
======================================================================

======================================================================
EXAMPLE 1: Basic File Read During Coding
======================================================================

STEP 1: LLM is about to modify src/main.rs
  LLM Output: "I'll add better error handling to your main.rs"

STEP 2: Pattern Detector identifies 'main.rs'
  Created query: Q-00000000000000000000000000000001
  Type: ReadFile
  Path: src/main.rs

STEP 3: Client processes query
  ✓ Query handled successfully
  Content: 44 bytes
  Preview: fn main() {

✅ Example 1 PASSED

[... Examples 2-5 continue ...]

======================================================================
✅ ALL EXAMPLES COMPLETED SUCCESSFULLY
======================================================================
```

## Key Takeaways

1. ✓ File reads enable code context
2. ✓ Environment queries support deployment  
3. ✓ Error handling recovers gracefully
4. ✓ Concurrent sessions remain isolated
5. ✓ Custom queries extend functionality

## Testing Coverage

- ✅ Query ID correlation (Q-format validation)
- ✅ JSON serialization/deserialization
- ✅ Async/await patterns
- ✅ Concurrent processing
- ✅ Error recovery
- ✅ Custom handlers

## Common Use Cases

### Reading a File
1. LLM mentions "main.rs"
2. Server detects pattern
3. Creates ReadFile query
4. Returns file content
5. LLM resumes with context

### Deployment
1. LLM asks "deploy to prod?"
2. Server queries environment
3. Server queries workspace context
4. LLM gets deployment info
5. LLM proceeds safely

### Error Recovery
1. Query times out or fails
2. System reports error
3. LLM decides on retry strategy
4. Retry with different parameters
5. Continue or escalate

### Custom Tools
1. Tool needs specific data
2. Create custom query type
3. Implement custom handler
4. Process and return result
5. Tool uses metadata

## Implementation Checklist

To integrate query bridge into your system:

- [ ] Review Example 1 for basic file reading
- [ ] Review Example 2 for multi-query patterns
- [ ] Review Example 3 for error handling
- [ ] Review Example 4 for session isolation
- [ ] Review Example 5 for custom extension
- [ ] Implement ServerQueryHandler for your client
- [ ] Hook into LLM response processing
- [ ] Test with multiple concurrent sessions
- [ ] Add observability/metrics
- [ ] Hardening (rate limits, validation)

## Debugging Tips

### Enable Debug Logging
```bash
RUST_LOG=debug cargo run --example query_bridge_scenarios
```

### Check Query IDs
Look for Q- format: `Q-{32 hex digits}`

### Verify Query Correlation
- Query sends with ID: `Q-abc123...`
- Response returns same: `query_id: "Q-abc123..."`

### Session Isolation
Each session should receive only its own responses

### Timeout Handling
Check timeout_secs field (default 30 seconds)

## Performance Notes

- Mock delays simulate real I/O: 5-50ms
- Concurrent sessions use tokio async
- No blocking operations
- ~500ms total runtime for all 5 examples

## Next: Full Integration

1. See `IMPLEMENTATION_SERVER_CLIENT_QUERY_BRIDGE.md` for architecture
2. See `INTEGRATION_GUIDE.md` for step-by-step setup
3. See `QUERY_SECURITY_IMPLEMENTATION.md` for security hardening
4. See `PERFORMANCE_TUNING.md` for optimization
