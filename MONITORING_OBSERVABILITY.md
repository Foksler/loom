# Monitoring & Observability Infrastructure

## Overview

The Loom Server includes comprehensive Prometheus metrics for monitoring and observability of query operations. This document describes the metrics infrastructure, how to use it, and how to extend it.

## Architecture

### Components

1. **QueryMetrics** (`crates/loom-server/src/query_metrics.rs`)
   - Core metrics collection system
   - Prometheus metric registration and management
   - Structured metrics export

2. **Integration Points**
   - `AppState` - Contains `query_metrics: Arc<QueryMetrics>`
   - `/metrics` endpoint - HTTP GET handler for Prometheus scraping
   - Server query lifecycle tracking

3. **Metrics Endpoint**
   - **URL**: `GET /metrics`
   - **Format**: Prometheus text format (0.0.4)
   - **Content-Type**: `text/plain; version=0.0.4; charset=utf-8`

## Available Metrics

### Counters

#### `loom_server_queries_sent_total`
**Purpose**: Total number of queries sent from server to client.

**Type**: Counter
**Labels**: None
**Example**: `loom_server_queries_sent_total 150`

#### `loom_server_queries_succeeded_total`
**Purpose**: Total number of queries that received successful response from client.

**Type**: Counter
**Labels**: None
**Example**: `loom_server_queries_succeeded_total 145`

#### `loom_server_queries_failed_total`
**Purpose**: Total number of queries that failed or timed out.

**Type**: Counter
**Labels**: None
**Example**: `loom_server_queries_failed_total 5`

#### `loom_server_query_timeouts_total`
**Purpose**: Total number of query timeouts, broken down by query type.

**Type**: Counter with labels
**Labels**: `query_type` (read_file, env, workspace, etc.)
**Example**:
```
loom_server_query_timeouts_total{query_type="read_file"} 2
loom_server_query_timeouts_total{query_type="env"} 1
```

#### `loom_server_queries_success_by_type`
**Purpose**: Successful queries broken down by type and session.

**Type**: Counter with labels
**Labels**: `query_type`, `session_id`
**Example**:
```
loom_server_queries_success_by_type{query_type="read_file",session_id="session-123"} 50
```

#### `loom_server_queries_failure_by_type`
**Purpose**: Failed queries broken down by type and error classification.

**Type**: Counter with labels
**Labels**: `query_type`, `error_type` (timeout, network, invalid_response)
**Example**:
```
loom_server_queries_failure_by_type{query_type="read_file",error_type="network"} 2
loom_server_queries_failure_by_type{query_type="env",error_type="timeout"} 1
```

### Gauges

#### `loom_server_queries_pending`
**Purpose**: Number of queries currently pending response from client.

**Type**: Gauge
**Labels**: None
**Example**: `loom_server_queries_pending 12`

#### `loom_server_queries_pending_by_type`
**Purpose**: Number of pending queries broken down by type.

**Type**: Gauge with labels
**Labels**: `query_type`
**Example**:
```
loom_server_queries_pending_by_type{query_type="read_file"} 8
loom_server_queries_pending_by_type{query_type="env"} 4
```

### Histograms

#### `loom_server_query_latency_seconds`
**Purpose**: Query latency in seconds (from send to response).

**Type**: Histogram
**Labels**: None
**Buckets**: [0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
**Example**:
```
loom_server_query_latency_seconds_bucket{le="0.1"} 50
loom_server_query_latency_seconds_bucket{le="1.0"} 145
loom_server_query_latency_seconds_sum 85.5
loom_server_query_latency_seconds_count 145
```

#### `loom_server_query_latency_by_type_seconds`
**Purpose**: Query latency broken down by query type.

**Type**: Histogram with labels
**Labels**: `query_type`
**Buckets**: [0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
**Example**:
```
loom_server_query_latency_by_type_seconds_bucket{query_type="read_file",le="0.1"} 40
loom_server_query_latency_by_type_seconds_bucket{query_type="read_file",le="1.0"} 85
```

## Recording Metrics

### API

The `QueryMetrics` struct provides methods to record metric observations:

```rust
pub struct QueryMetrics {
    // ...
}

impl QueryMetrics {
    /// Record a query being sent
    pub fn record_sent(&self, query_type: &str, session_id: &str)
    
    /// Record successful query response
    pub fn record_success(&self, query_type: &str, session_id: &str, latency_secs: f64)
    
    /// Record query failure (error or timeout)
    pub fn record_failure(&self, query_type: &str, error_type: &str, session_id: &str)
    
    /// Record latency for a query
    pub fn record_latency(&self, query_type: &str, latency_secs: f64)
    
    /// Set the number of pending queries
    pub fn set_pending_count(&self, count: i64)
    
    /// Export metrics in Prometheus format
    pub fn gather_metrics(&self) -> Result<String, prometheus::Error>
}
```

### Query Types

Standard query types:
- `read_file` - Reading file contents
- `env` - Reading environment variables
- `workspace` - Getting workspace information
- `command` - Executing commands
- Custom types as needed

### Error Types

Standard error types for failure tracking:
- `timeout` - Query exceeded timeout
- `network` - Network error contacting client
- `invalid_response` - Client returned invalid response
- `error` - Generic client error
- Custom types as needed

## Integration Examples

### 1. Basic Query Tracking

```rust
use loom_server::QueryMetrics;
use std::sync::Arc;
use std::time::Instant;

let metrics = Arc::new(QueryMetrics::default());
let start = Instant::now();

// Record query sent
metrics.record_sent("read_file", "session-123");

// ... query processing ...

match result {
    Ok(_) => {
        let latency = start.elapsed().as_secs_f64();
        metrics.record_success("read_file", "session-123", latency);
    }
    Err(e) => {
        metrics.record_failure("read_file", "network", "session-123");
    }
}
```

### 2. ServerQueryManager Integration

```rust
// In server_query.rs send_query() method
pub async fn send_query(
    &self,
    session_id: &str,
    query: ServerQuery,
    metrics: &Arc<QueryMetrics>,  // passed from AppState
) -> Result<ServerQueryResponse, ServerQueryError> {
    let query_id = query.id.clone();
    let query_type = &query.query_type;
    let start = Instant::now();
    
    metrics.record_sent(query_type, session_id);
    
    // ... existing logic ...
    
    match result {
        Ok(response) => {
            let latency = start.elapsed().as_secs_f64();
            metrics.record_success(query_type, session_id, latency);
            Ok(response)
        }
        Err(ServerQueryError::Timeout) => {
            metrics.record_failure(query_type, "timeout", session_id);
            Err(ServerQueryError::Timeout)
        }
        Err(e) => {
            metrics.record_failure(query_type, "error", session_id);
            Err(e)
        }
    }
}
```

### 3. Accessing Metrics in Handlers

The metrics are available via `AppState`:

```rust
async fn prometheus_metrics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ServerError> {
    match state.query_metrics.gather_metrics() {
        Ok(metrics) => {
            Ok((StatusCode::OK, metrics))
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to gather metrics");
            Err(ServerError::Internal("Failed to gather metrics".into()))
        }
    }
}
```

## Prometheus Configuration

### Scrape Configuration

Add to Prometheus `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'loom-server'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
```

### PromQL Queries

#### Success Rate
```promql
rate(loom_server_queries_succeeded_total[5m]) / (rate(loom_server_queries_sent_total[5m]))
```

#### Pending Queries
```promql
loom_server_queries_pending
```

#### P95 Latency
```promql
histogram_quantile(0.95, rate(loom_server_query_latency_seconds_bucket[5m]))
```

#### Timeout Rate by Type
```promql
rate(loom_server_query_timeouts_total[5m])
```

#### Average Latency by Type
```promql
rate(loom_server_query_latency_by_type_seconds_sum[5m]) / rate(loom_server_query_latency_by_type_seconds_count[5m])
```

## Testing

### Unit Tests

The `QueryMetrics` module includes comprehensive tests:

```bash
cargo test --lib query_metrics
```

Tests cover:
- Metric creation and initialization
- Counter increments on success/failure
- Latency recording in histograms
- Label setting and cardinality
- Gauge value updates
- Concurrent metric updates

### Integration Testing

To test the `/metrics` endpoint:

```bash
# Start server
cargo run --bin loom-server

# Scrape metrics
curl http://localhost:8080/metrics

# Verify format
curl http://localhost:8080/metrics | head -20
```

### Example Response

```
# HELP loom_server_queries_sent_total Total number of queries sent from server to client
# TYPE loom_server_queries_sent_total counter
loom_server_queries_sent_total 150

# HELP loom_server_queries_succeeded_total Total number of queries that received successful response
# TYPE loom_server_queries_succeeded_total counter
loom_server_queries_succeeded_total 145

# HELP loom_server_queries_failed_total Total number of queries that failed or timed out
# TYPE loom_server_queries_failed_total counter
loom_server_queries_failed_total 5

# HELP loom_server_query_latency_seconds Query latency in seconds from send to response
# TYPE loom_server_query_latency_seconds histogram
loom_server_query_latency_seconds_bucket{le="0.01"} 5
loom_server_query_latency_seconds_bucket{le="0.1"} 50
loom_server_query_latency_seconds_bucket{le="1.0"} 145
loom_server_query_latency_seconds_bucket{le="+Inf"} 145
loom_server_query_latency_seconds_sum 85.5
loom_server_query_latency_seconds_count 145

# HELP loom_server_queries_pending Number of queries currently pending response from client
# TYPE loom_server_queries_pending gauge
loom_server_queries_pending 0
```

## Future Enhancements

### Phase 2 Extensions

1. **System Metrics**
   - Memory usage (RSS, heap)
   - CPU usage percentage
   - Open connections count
   - Thread count

2. **Database Metrics**
   - Query execution times
   - Connection pool stats
   - Cache hit rates

3. **Custom Labels**
   - Query priority level
   - Client version
   - Operation context

### Phase 3 (WebSocket) Updates

When migrating to WebSocket transport in Phase 3:
1. Metrics collection logic remains unchanged
2. Only the call sites (in websocket.rs) will change
3. Same labels and counters apply to WebSocket queries
4. Latency measurement still from query send to response

## Structured Logging

All metrics are integrated with structured logging:

```rust
tracing::debug!(
    query_type = "read_file",
    session_id = "session-123",
    latency_secs = 0.5,
    "recorded query success"
);
```

This provides correlation between logs and metrics via:
- Trace IDs (from `query_tracing.rs`)
- Session IDs
- Query types

## Troubleshooting

### Metrics Endpoint Returns 500

1. Check logs: `journalctl -u loom-server -f`
2. Verify metrics initialization in `AppState::create_app_state()`
3. Check Prometheus encoder compatibility

### Missing Metrics

1. Verify `record_*()` methods are called in query lifecycle
2. Check label values are not empty strings
3. Ensure metrics are registered before use

### High Cardinality Issues

If metric cardinality grows too high:
1. Limit session_id variations (use session pool)
2. Aggregate similar error types
3. Use metric relabeling in Prometheus

## References

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Rust Prometheus Client](https://github.com/prometheus/client_rust)
- [Metric Types](https://prometheus.io/docs/concepts/metric_types/)
- [Best Practices](https://prometheus.io/docs/practices/naming/)
