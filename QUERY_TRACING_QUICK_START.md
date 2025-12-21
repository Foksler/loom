# Query Tracing Quick Start Guide

## Using Query Tracing in Your Code

### Create and Use a Tracer

```rust
use loom_server::query_tracing::QueryTracer;

// Create tracer for a query
let mut tracer = QueryTracer::new("Q-123", Some("session-1".to_string()));

// Record query sent
tracer.record_sent(10); // 10 second timeout

// ... do work ...

// Record response
tracer.record_response_received("ok");

// Check if there were errors
if tracer.has_error() {
    println!("Query failed!");
}

// Get total duration
println!("Query took: {}ms", tracer.total_duration().as_millis());
```

### Store and Retrieve Traces

```rust
use loom_server::query_tracing::QueryTraceStore;

// Get store from AppState (already available)
// let store = &state.trace_store;

// Store completed trace
let trace_id = tracer.trace_id.as_str().to_string();
store.store(tracer).await;

// Retrieve later
if let Some(retrieved) = store.get(&trace_id).await {
    println!("Query duration: {}ms", retrieved.total_duration().as_millis());
}

// List all traces
let all_ids = store.list_trace_ids().await;

// Filter by session
let session_traces = store.get_session_traces("session-1").await;

// Find slow queries
let slow = store.get_slow_traces(Duration::from_secs(5)).await;

// Get statistics
let stats = store.get_stats().await;
println!("Total: {}, Errors: {}, Slow: {}", 
    stats.total_traces, stats.traces_with_errors, stats.slow_traces);
```

### Record Custom Events

```rust
// Record custom event with arbitrary JSON
tracer.record_event(
    "custom_processing",
    serde_json::json!({
        "stage": "validation",
        "result": "passed",
        "duration_ms": 123,
    }),
);

// Record error with custom details
tracer.record_error(
    "Connection timeout",
    Some(serde_json::json!({
        "host": "client.example.com",
        "port": 8080,
        "retry_count": 3,
    })),
);
```

## HTTP Endpoints

### Get Single Trace Timeline
```bash
curl http://localhost:8080/v1/debug/query-traces/TRACE-123
```

Response includes all events with elapsed_ms, durations, and metadata.

### List Traces
```bash
# All traces
curl http://localhost:8080/v1/debug/query-traces

# Filter by session
curl http://localhost:8080/v1/debug/query-traces?session_id=user123
```

### Get Statistics
```bash
curl http://localhost:8080/v1/debug/query-traces/stats
```

Returns aggregate metrics: total, errors, slow count, event averages.

## Event Recording Pattern

Typical query lifecycle:

```rust
let mut tracer = QueryTracer::new("Q-abc", Some("session-xyz".to_string()));

// Event 1: Query sent (automatic in record_sent)
tracer.record_sent(30); // 30-second timeout

// Event 2: Processing
tracer.record_event("processing", json!({"stage": "validation"}));

// Event 3: Response received
tracer.record_response_received("ok");

// Event 4: Completion
tracer.record_event("completed", json!({"status": "success"}));

// Store for debugging
store.store(tracer).await;
```

## Performance Tips

1. **Trace overhead is negligible** (<1ms per query)
2. **Default capacity is 10,000 traces** - change if needed
3. **Events are stored in memory** - use for recent debugging, not archival
4. **Store automatically evicts old traces** via LRU when capacity exceeded

## Debugging Slow Queries

```rust
// Find all slow queries
let slow_traces = store.get_slow_traces(Duration::from_secs(5)).await;

for info in slow_traces {
    eprintln!(
        "SLOW QUERY: {} took {}ms in session {}",
        info.query_id, info.total_duration_ms, 
        info.session_id.unwrap_or_default()
    );
}

// Get detailed timeline
if let Some(tracer) = store.get("TRACE-123").await {
    for (i, event) in tracer.events.iter().enumerate() {
        println!(
            "[{}] {} at {:?}",
            i, event.event_type, event.timestamp
        );
    }
    
    // Show duration between events
    for (i, _) in tracer.events.iter().enumerate().take(tracer.events.len() - 1) {
        if let Some(duration) = tracer.event_duration(i, i + 1) {
            println!(
                "  {} -> {}: {}ms",
                tracer.events[i].event_type,
                tracer.events[i + 1].event_type,
                duration.as_millis()
            );
        }
    }
}
```

## Monitoring Recommendations

### Set up alerts for:
- **Store capacity > 90%** - May be evicting important traces
- **Error rate > 5%** - Check if queries are failing
- **Slow trace count > 10%** - Performance degradation
- **Missing session traces** - Client sessions stuck or disconnected

### Recommended log level:
```bash
RUST_LOG=loom_server::query_tracing=debug
```

## Error Handling

```rust
// Check for errors
if tracer.has_error() {
    // Look for error events
    for event in &tracer.events {
        if event.event_type == "error" || event.event_type == "timeout" {
            println!("Error: {:?}", event.details);
        }
    }
}
```

## Session Management

```rust
// Get all traces for a user's session
let session_id = "user-123-session-456";
let traces = store.get_session_traces(session_id).await;

let mut error_count = 0;
let mut total_duration = 0u128;

for tracer in traces {
    if tracer.has_error() {
        error_count += 1;
    }
    total_duration += tracer.total_duration().as_millis();
}

println!("Session stats: {} queries, {} errors, {}ms total",
    traces.len(), error_count, total_duration);
```

## Timeline View

Convert raw trace to human-readable format:

```rust
use loom_server::query_tracing::TraceTimeline;

let timeline = TraceTimeline::from_tracer(&tracer);

// JSON response structure (as used by debug endpoints)
println!("Query: {}", timeline.query_id);
println!("Total duration: {}ms", timeline.total_duration_ms);
println!("Status: {}", if timeline.has_error { "ERROR" } else { "OK" });
println!("Performance: {}", if timeline.is_slow { "SLOW" } else { "OK" });

for event in &timeline.events {
    println!("[{}ms] {}: {:?}", 
        event.elapsed_ms, event.event_type, event.details);
}
```

## Common Patterns

### Profile a query operation
```rust
let mut tracer = QueryTracer::new(query_id, session_id);
let start = Instant::now();

tracer.record_sent(timeout);
// ... send query to client ...

// Client responds
tracer.record_response_received("ok");
let send_to_receive = start.elapsed();
println!("Round trip: {}ms", send_to_receive.as_millis());

// Process result
tracer.record_event("processing", json!({}));
// ... do processing ...

tracer.record_event("completed", json!({}));
store.store(tracer).await;
```

### Correlate with logging
```rust
let trace_id = tracer.trace_id.as_str().to_string();

tracing::info!(
    trace_id = %trace_id,
    query_id = %tracer.query_id,
    "Query completed"
);

// Later, correlate logs with detailed trace:
// GET /v1/debug/query-traces/{trace_id}
```

### Monitor session health
```rust
let stats = store.get_stats().await;
let error_rate = (stats.traces_with_errors as f64 / stats.total_traces as f64) * 100.0;

tracing::warn!(
    total_traces = stats.total_traces,
    error_rate = format!("{:.2}%", error_rate),
    slow_count = stats.slow_traces,
    "Trace store health check"
);
```

## Files to Review

1. **Implementation Details**
   - [Query Tracing Module](file:///home/ghuntley/loom/crates/loom-server/src/query_tracing.rs)
   - [Debug Endpoints](file:///home/ghuntley/loom/crates/loom-server/src/api.rs#L1200-L1315)

2. **Tests**
   - [Module Tests](file:///home/ghuntley/loom/crates/loom-server/src/query_tracing.rs#L442-L658)
   - [Integration Tests](file:///home/ghuntley/loom/crates/loom-server/src/tests/tracing_integration_tests.rs)
   - [Endpoint Tests](file:///home/ghuntley/loom/crates/loom-server/src/api.rs#L1692-L1949)

3. **Documentation**
   - [Full Documentation](./QUERY_TRACING_DOCUMENTATION.md)
   - [Implementation Summary](./QUERY_TRACING_IMPLEMENTATION_SUMMARY.md)

---

**Quick Start Complete!**

For detailed documentation, see [QUERY_TRACING_DOCUMENTATION.md](./QUERY_TRACING_DOCUMENTATION.md)
