# Query Tracing and Debugging Infrastructure

## Overview

The query tracing infrastructure provides comprehensive, low-overhead tracing of server-to-client queries for debugging and performance analysis. Each query is assigned a unique trace ID and events are recorded chronologically, enabling complete reconstruction of query lifecycles.

## Architecture

### Core Components

#### 1. **QueryTracer** (`crates/loom-server/src/query_tracing.rs`)
Main tracer struct that records events for a single query.

```rust
pub struct QueryTracer {
    pub trace_id: TraceId,
    pub query_id: String,
    pub session_id: Option<String>,
    pub events: Vec<TraceEvent>,
    next_sequence: u32,
}
```

**Key Methods:**
- `new(query_id, session_id)` - Create new tracer with auto-generated trace_id
- `record_sent(timeout_secs)` - Record query sent event
- `record_response_received(status)` - Record response receipt
- `record_error(error, details)` - Record error event
- `record_timeout(timeout_secs)` - Record timeout event
- `record_event(event_type, details)` - Generic event recording
- `total_duration()` - Get total time from creation to last event
- `event_duration(from_idx, to_idx)` - Get duration between specific events
- `has_error()` - Check if trace contains error or timeout
- `is_slow(threshold)` - Check if query exceeded duration threshold

#### 2. **TraceEvent** (in same module)
Individual event in a trace timeline.

```rust
pub struct TraceEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub sequence: u32,
    pub details: serde_json::Value,
}
```

Events track:
- Precise timestamp (UTC)
- Event type (created, sent, response_received, processed, injected, error, timeout)
- Sequence number for ordering
- Arbitrary JSON details for event-specific metadata

#### 3. **QueryTraceStore** (in same module)
Thread-safe store for managing multiple traces with capacity limits.

```rust
pub struct QueryTraceStore {
    traces: Arc<Mutex<HashMap<String, QueryTracer>>>,
    max_traces: usize,
}
```

**Key Methods:**
- `new(max_traces)` - Create store with capacity limit
- `store(tracer)` - Store a completed tracer (with LRU eviction)
- `get(trace_id)` - Retrieve trace by ID
- `list_trace_ids()` - Get all trace IDs
- `get_session_traces(session_id)` - Filter by session
- `get_slow_traces(threshold)` - Get traces exceeding duration threshold
- `get_stats()` - Get aggregated statistics
- `clear()` - Clear all traces

#### 4. **TraceTimeline** (in same module)
Human-readable timeline view of a trace.

```rust
pub struct TraceTimeline {
    pub trace_id: String,
    pub query_id: String,
    pub session_id: Option<String>,
    pub total_duration_ms: u64,
    pub events: Vec<TimelineEvent>,
    pub has_error: bool,
    pub is_slow: bool,
}
```

Converts raw trace data into formatted output for HTTP responses.

## Integration Points

### 1. **Application State** (`crates/loom-server/src/api.rs`)
TraceStore is included in AppState:

```rust
pub struct AppState {
    // ... other fields ...
    pub trace_store: QueryTraceStore,
}

// In create_app_state():
trace_store: QueryTraceStore::default(), // 10,000 max traces
```

### 2. **Debug Endpoints** (in same api.rs)

#### GET /v1/debug/query-traces/{trace_id}
Retrieve full timeline for a specific query.

**Response Example:**
```json
{
  "trace_id": "TRACE-123e4567-e89b-12d3-a456-426614174000",
  "query_id": "Q-abc123",
  "session_id": "session-user1",
  "total_duration_ms": 245,
  "has_error": false,
  "is_slow": false,
  "events": [
    {
      "sequence": 0,
      "event_type": "created",
      "timestamp": "2025-12-20T21:00:00Z",
      "elapsed_ms": 0,
      "details": { "query_id": "Q-abc123", "session_id": "session-user1" }
    },
    {
      "sequence": 1,
      "event_type": "sent",
      "timestamp": "2025-12-20T21:00:00.100Z",
      "elapsed_ms": 100,
      "details": { "timeout_secs": 10 }
    },
    {
      "sequence": 2,
      "event_type": "response_received",
      "timestamp": "2025-12-20T21:00:00.245Z",
      "elapsed_ms": 245,
      "details": { "status": "ok" }
    }
  ]
}
```

#### GET /v1/debug/query-traces
List all traces with optional session filtering.

**Query Parameters:**
- `session_id` (optional) - Filter by session

**Response Example:**
```json
{
  "traces": [
    {
      "trace_id": "TRACE-123e4567-e89b-12d3-a456-426614174000",
      "query_id": "Q-abc123",
      "session_id": "session-user1",
      "event_count": 3,
      "total_duration_ms": 245,
      "has_error": false
    }
  ],
  "count": 1
}
```

#### GET /v1/debug/query-traces/stats
Get aggregated statistics about all traces.

**Response Example:**
```json
{
  "total_traces": 1523,
  "traces_with_errors": 42,
  "slow_traces": 18,
  "avg_events_per_trace": 4,
  "slow_trace_details": [
    {
      "trace_id": "TRACE-slow-123",
      "query_id": "Q-slow",
      "session_id": "session-user2",
      "total_duration_ms": 5234,
      "event_count": 4,
      "has_error": false
    }
  ]
}
```

## Event Types

Events recorded in query lifecycle:

| Event | When | Details | Usage |
|-------|------|---------|-------|
| `created` | Tracer initialization | query_id, session_id | Timeline start |
| `sent` | Query sent to client | timeout_secs | Request initiation |
| `received` | Response from client | status | Response receipt |
| `response_received` | Processing response | status | Same as received |
| `processed` | Query processing done | - | Processing completion |
| `injected` | Result injected to LLM | - | LLM injection |
| `error` | Error occurred | error message | Failure tracking |
| `timeout` | Timeout occurred | timeout_secs | Timeout detection |
| Custom | User-defined | arbitrary JSON | Application-specific |

## Performance Characteristics

### Memory Overhead
- Per-trace: ~1KB base + event size (~100 bytes each)
- Default capacity: 10,000 traces = ~10-15MB
- Configurable via `QueryTraceStore::new(capacity)`

### Time Overhead
- Tracer creation: <0.1ms
- Event recording: <0.01ms
- Store operation: <0.1ms (amortized with capacity-based eviction)

### Latency Impact
Negligible (<1ms per query) due to async design and optional integration.

## Usage Patterns

### Basic Usage

```rust
use loom_server::query_tracing::QueryTracer;

// Create tracer for a query
let mut tracer = QueryTracer::new("Q-123", Some("session-1".to_string()));

// Record events
tracer.record_sent(10); // 10 second timeout
// ... do work ...
tracer.record_response_received("ok");
```

### With Store

```rust
use loom_server::query_tracing::QueryTraceStore;

let store = QueryTraceStore::default(); // 10k capacity
let trace_id = tracer.trace_id.as_str().to_string();

// Store completed trace
store.store(tracer).await;

// Retrieve later
if let Some(retrieved) = store.get(&trace_id).await {
    println!("Query duration: {}ms", retrieved.total_duration().as_millis());
}
```

### Session Analysis

```rust
// Get all traces for a specific session
let session_traces = store.get_session_traces("session-123").await;

for tracer in session_traces {
    if tracer.has_error() {
        println!("Error in query {}: {:?}", tracer.query_id, tracer.events);
    }
}
```

### Performance Monitoring

```rust
// Identify slow queries
let slow_threshold = Duration::from_secs(5);
let slow_traces = store.get_slow_traces(slow_threshold).await;

for info in slow_traces {
    eprintln!(
        "SLOW: {} took {}ms",
        info.query_id, info.total_duration_ms
    );
}

// Get statistics
let stats = store.get_stats().await;
println!("Total traces: {}", stats.total_traces);
println!("Error rate: {:.2}%", 
    (stats.traces_with_errors as f64 / stats.total_traces as f64) * 100.0
);
```

## Testing

### Unit Tests (`crates/loom-server/src/query_tracing.rs`)
Located in module with comprehensive tests:
- Trace ID uniqueness
- Event sequence ordering
- Duration calculations
- Error/timeout detection
- Store capacity management
- Session filtering
- Performance overhead validation

### Integration Tests
- `crates/loom-server/src/tests/query_tracing_tests.rs` - Core tracing tests
- `crates/loom-server/src/tests/tracing_integration_tests.rs` - Full lifecycle tests

### Debug Endpoint Tests (`crates/loom-server/src/api.rs`)
- `test_get_query_trace_endpoint()` - Single trace retrieval
- `test_list_query_traces_endpoint()` - List all traces
- `test_list_query_traces_with_session_filter()` - Session filtering
- `test_get_trace_stats_endpoint()` - Statistics endpoint
- `test_debug_endpoints_integration()` - Full integration test
- `test_get_query_trace_not_found()` - Error handling

## Logging Integration

Tracers use structured logging with `tracing` crate:

```rust
tracing::debug!(
    trace_id = %tracer.trace_id.0,
    query_id = %tracer.query_id,
    event_type = %event_type_str,
    "trace event recorded"
);
```

All debug endpoint handlers include comprehensive logging:
- Request reception
- Trace retrieval
- Statistics computation
- Error conditions

## Recommended Monitoring

### Metrics to Watch

1. **Total Traces in Store**
   - Alert if approaching capacity (>90%)
   - Indicates high query volume or slow processing

2. **Error Rate**
   - Alert if >5% of traces have errors
   - Indicates reliability issues

3. **Slow Trace Count**
   - Alert if >10% of recent traces exceed 5s
   - Indicates performance degradation

4. **Session Isolation**
   - Monitor traces per session
   - Identify stuck or misbehaving sessions

### Recommended Logging

Enable debug logging for tracing module in production:
```
RUST_LOG=loom_server::query_tracing=debug
```

## Future Enhancements

### Planned Features

1. **Trace Export**
   - Export to OpenTelemetry format
   - Integration with distributed tracing systems (Jaeger, Zipkin)

2. **Advanced Filtering**
   - Filter by query type
   - Filter by error message
   - Time range filtering

3. **Persistence**
   - Optional SQLite storage for long-term analysis
   - Trace archival for compliance

4. **Real-time Streaming**
   - WebSocket endpoint for live trace streaming
   - Alert webhooks for error traces

5. **Machine Learning Integration**
   - Anomaly detection for slow queries
   - Automatic performance regression detection

## Troubleshooting

### High Memory Usage
**Problem:** Trace store consuming excessive memory

**Solution:**
1. Reduce `max_traces` capacity in `create_app_state()`
2. Enable more frequent eviction by monitoring slow traces
3. Clear old traces periodically (implement retention policy)

### Missing Traces
**Problem:** Expected traces not in store

**Causes:**
- Trace ID typo (must start with "TRACE-")
- Trace evicted due to capacity limit
- Tracer never stored (must call `store.store(tracer)`)

**Solution:**
- Check trace listing endpoint first
- Verify store capacity is sufficient
- Monitor eviction in logs

### Performance Bottlenecks
**Problem:** Query tracing causing latency

**Solution:**
1. Profile with: `RUST_LOG=loom_server::query_tracing=trace`
2. Check store operation times in logs
3. Reduce event recording frequency
4. Consider async store operations

## Related Documentation

- [Query Manager Implementation](./IMPLEMENTATION_SERVER_CLIENT_QUERY_BRIDGE.md)
- [Query Security](./QUERY_SECURITY_IMPLEMENTATION.md)
- [Metrics and Observability](./METRICS_IMPLEMENTATION_SUMMARY.md)
- [Performance Tuning](./PERFORMANCE_TUNING.md)

---

**Module:** `loom-server`  
**Location:** `crates/loom-server/src/query_tracing.rs`  
**Status:** Stable  
**Last Updated:** 2025-12-20
