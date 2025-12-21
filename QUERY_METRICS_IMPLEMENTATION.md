# Query Metrics Implementation

## Overview

This document describes the complete implementation of query metrics and monitoring for the Loom server. The system uses Prometheus metrics to track query operations throughout their lifecycle.

## Architecture

### Components

1. **QueryMetrics** (`crates/loom-server/src/query_metrics.rs`)
   - Core metrics collection using Prometheus library
   - Prometheus registry for metric storage
   - Methods to record query lifecycle events

2. **ServerQueryManager** (`crates/loom-server/src/server_query.rs`)
   - Integrated metrics recording at key points
   - Automatic latency measurement
   - Error tracking with categorization

3. **API Handler** (`crates/loom-server/src/api.rs`)
   - GET `/metrics` endpoint returning Prometheus format
   - Integration of QueryMetrics into AppState

## Metrics Collected

### Counters

| Metric | Labels | Purpose |
|--------|--------|---------|
| `loom_queries_sent_total` | None | Total queries sent to client |
| `loom_queries_succeeded_total` | None | Total successful query responses |
| `loom_queries_failed_total` | None | Total failed/timed out queries |
| `loom_query_timeouts_total` | `query_type` | Timeout count by query type |
| `loom_queries_success_by_type` | `query_type`, `session_id` | Success by type and session |
| `loom_queries_failure_by_type` | `query_type`, `error_type` | Failure by type and error |

### Gauges

| Metric | Labels | Purpose |
|--------|--------|---------|
| `loom_queries_pending` | None | Current pending query count |
| `loom_queries_pending_by_type` | `query_type` | Pending count by type |

### Histograms

| Metric | Labels | Purpose |
|--------|--------|---------|
| `loom_query_latency_seconds` | None | Overall query latency |
| `loom_query_latency_by_type_seconds` | `query_type` | Latency by query type |

**Latency Buckets**: 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0 seconds

## Query Types

The system recognizes three query types:
- `read_file`: File reading queries
- `env`: Environment variable queries
- `workspace`: Workspace information queries

## Query Lifecycle Tracking

### 1. Query Sent
```rust
metrics.record_sent(query_type, session_id)
```
- Increments `queries_sent_total`
- Increments `queries_pending` gauge
- Increments `pending_by_type` for the query type
- Starts latency timer

### 2. Query Success
```rust
metrics.record_success(query_type, session_id, latency_secs)
```
- Increments `queries_succeeded_total`
- Decrements `queries_pending` gauge
- Records latency in histograms
- Increments `queries_success_by_type` counter

### 3. Query Failure
```rust
metrics.record_failure(query_type, error_type, session_id)
```
- Increments `queries_failed_total`
- Decrements `queries_pending` gauge
- Increments `queries_failure_by_type` counter
- Auto-increments `query_timeouts_total` if error_type is "timeout"

## Integration Points

### ServerQueryManager Creation

In `api.rs`, the manager is created with metrics:

```rust
let query_metrics = Arc::new(QueryMetrics::default());
let query_manager = Arc::new(ServerQueryManager::with_metrics(query_metrics.clone()));
```

### Query Sending

In `server_query.rs`, `send_query` method:
1. Records query sent with `record_sent()`
2. Measures elapsed time during wait
3. Records success with `record_success()` and latency
4. On timeout, records failure with error_type="timeout"

### Metrics Export

GET `/metrics` endpoint returns Prometheus text format with all metrics.

## Usage Examples

### Monitoring Queries per Second
```promql
rate(loom_queries_sent_total[1m])
```

### Query Success Rate
```promql
(rate(loom_queries_succeeded_total[5m]) / rate(loom_queries_sent_total[5m])) * 100
```

### Pending Queries Alert
```promql
loom_queries_pending > 100
```

### P95 Query Latency
```promql
histogram_quantile(0.95, loom_query_latency_seconds)
```

### Timeout Rate by Type
```promql
rate(loom_query_timeouts_total[5m]) > 0.01
```

## Testing

### Unit Tests
- `query_metrics.rs`: Metrics creation, counters, labels, gauges, latency
- Concurrent metrics updates
- Prometheus format encoding

### Integration Tests
- `query_metrics_integration_tests.rs`:
  - Metrics recording at each lifecycle stage
  - Label correctness
  - Pending gauge tracking
  - Session-level metrics
  - Latency histogram buckets
  - Prometheus format output

## Best Practices

1. **Always use query_type labels** for filtering and aggregation
2. **Monitor pending queries** to detect stuck connections
3. **Track error_type** to identify error patterns
4. **Use session_id** in success metrics for per-user analysis
5. **Alert on timeout rate** increases which indicate client issues
6. **Dashboard histograms** for latency percentiles

## Future Enhancements

1. **Custom error types**: Extend error_type labels for more granular categorization
2. **Query content size metrics**: Track bytes sent/received
3. **Client version tracking**: Add client version label
4. **Performance optimization tracking**: Latency improvements over time
5. **Rate limiting metrics**: Track rate limit hits per session

## Files Modified

- `crates/loom-server/src/query_metrics.rs` - Core metrics implementation (no changes needed, was complete)
- `crates/loom-server/src/server_query.rs` - Added metrics recording to send_query()
- `crates/loom-server/src/api.rs` - Integrated metrics with ServerQueryManager
- `crates/loom-server/src/tests/query_metrics_integration_tests.rs` - New integration tests
- `crates/loom-server/src/tests/mod.rs` - Registered new test module

## API Endpoint

```
GET /metrics
Content-Type: text/plain; version=0.0.4; charset=utf-8

# HELP loom_queries_sent_total Total number of queries sent from server to client
# TYPE loom_queries_sent_total counter
loom_queries_sent_total 1234

# HELP loom_queries_succeeded_total Total number of queries that received successful response
# TYPE loom_queries_succeeded_total counter
loom_queries_succeeded_total 1220

...
```

## Verification

The implementation has been verified to:
1. ✅ Compile successfully with Rust toolchain
2. ✅ Record metrics at each query lifecycle stage
3. ✅ Set labels correctly for filtering
4. ✅ Track pending queries accurately
5. ✅ Export Prometheus-compatible format
6. ✅ Support concurrent metric updates

## Configuration

No additional configuration is required. Metrics are enabled by default when the server starts.

To scrape metrics with Prometheus:
```yaml
scrape_configs:
  - job_name: 'loom-server'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```
