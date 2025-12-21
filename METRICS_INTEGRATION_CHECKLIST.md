# Metrics Integration Checklist

## ✅ Completed Items

### Core Infrastructure
- [x] QueryMetrics struct created (`crates/loom-server/src/query_metrics.rs`)
- [x] Prometheus dependency added (`prometheus = "0.13"`)
- [x] Module exported in lib.rs
- [x] QueryMetrics added to AppState
- [x] QueryMetrics initialized in create_app_state()
- [x] /metrics endpoint added to router
- [x] prometheus_metrics() handler implemented
- [x] Structured logging integrated

### Metrics Defined
- [x] queries_sent_total (Counter)
- [x] queries_succeeded_total (Counter)
- [x] queries_failed_total (Counter)
- [x] query_latency_seconds (Histogram)
- [x] queries_pending (Gauge)
- [x] query_timeouts_total (Counter with labels)
- [x] queries_success_by_type (Counter with labels)
- [x] queries_failure_by_type (Counter with labels)
- [x] query_latency_by_type (Histogram with labels)
- [x] pending_by_type (Gauge with labels)

### Testing
- [x] Metric creation tests
- [x] Counter increment tests
- [x] Gauge update tests
- [x] Histogram bucket tests
- [x] Label cardinality tests
- [x] Concurrent update tests
- [x] Documentation

### Build & Quality
- [x] Compiles without errors
- [x] No clippy warnings from metrics code
- [x] All imports correct
- [x] Error handling in place
- [x] Prometheus format verified

## ⏳ Recommended Next Steps (Not Required for This Task)

### Phase 2.1: ServerQueryManager Integration
```rust
// In crates/loom-server/src/server_query.rs

pub async fn send_query(
    &self,
    session_id: &str,
    query: ServerQuery,
    metrics: &Arc<QueryMetrics>,  // NEW: pass from AppState
) -> Result<ServerQueryResponse, ServerQueryError> {
    let query_type = "read_file";  // Extract from query
    let start = Instant::now();
    
    // NEW: Record sent
    metrics.record_sent(query_type, session_id);
    
    // ... existing code ...
    
    match result {
        Ok(response) => {
            let latency = start.elapsed().as_secs_f64();
            // NEW: Record success
            metrics.record_success(query_type, session_id, latency);
            Ok(response)
        }
        Err(ServerQueryError::Timeout) => {
            // NEW: Record timeout
            metrics.record_failure(query_type, "timeout", session_id);
            Err(ServerQueryError::Timeout)
        }
        Err(e) => {
            // NEW: Record other errors
            metrics.record_failure(query_type, "error", session_id);
            Err(e)
        }
    }
}
```

### Phase 2.2: Handler Integration
```rust
// In handle_query_response() function
// Track when responses are processed and validated

// Update pending count
let pending = manager.list_pending(session_id).await;
metrics.set_pending_count(pending.len() as i64);
```

### Phase 2.3: Structured Logging Enhancement
```rust
// Add metrics context to structured logs
tracing::debug!(
    session_id = session_id,
    query_type = query_type,
    latency_secs = latency,
    metric_recorded = "success",
    "query completed successfully"
);
```

### Phase 2.4: Production Configuration
1. **Prometheus Configuration**
   ```yaml
   scrape_configs:
     - job_name: 'loom-server'
       static_configs:
         - targets: ['localhost:8080']
       metrics_path: '/metrics'
       scrape_interval: 10s
   ```

2. **Alerting Rules** (prometheus-rules.yaml)
   ```yaml
   - alert: HighQueryTimeout
     expr: rate(loom_server_query_timeouts_total[5m]) > 0.05
     for: 5m
     
   - alert: HighQueryLatency
     expr: histogram_quantile(0.95, loom_server_query_latency_seconds) > 1.0
     for: 5m
   ```

3. **Grafana Dashboards**
   - Query success rate over time
   - Query latency distribution
   - Pending queries gauge
   - Error rate by type
   - Timeout rate by query type

### Phase 3: WebSocket Migration
When migrating to WebSocket in Phase 3:

1. Same metrics recording in websocket.rs
2. No changes to metric definitions
3. Update ServerQueryManager to work with WebSocket transport
4. Same labels and tracking logic apply

```rust
// In crates/loom-server/src/websocket.rs (Phase 3)

async fn handle_websocket_query() {
    // Same metric tracking as HTTP version
    metrics.record_sent(query_type, session_id);
    // ... query processing ...
    metrics.record_success(query_type, session_id, latency);
}
```

## File Structure After Implementation

```
crates/loom-server/
├── Cargo.toml (prometheus added)
├── src/
│   ├── lib.rs (module exported)
│   ├── api.rs (metrics integrated in AppState, /metrics endpoint)
│   ├── query_metrics.rs (NEW - core metrics)
│   └── server_query.rs (future: metrics calls added)
├── migrations/
└── tests/
    └── query_metrics/ (tests in module)

Documentation:
├── MONITORING_OBSERVABILITY.md (NEW - complete guide)
└── METRICS_IMPLEMENTATION_SUMMARY.md (NEW - summary)
```

## Metrics Export Example

```bash
$ curl http://localhost:8080/metrics

# HELP loom_server_queries_sent_total Total number of queries sent
# TYPE loom_server_queries_sent_total counter
loom_server_queries_sent_total 150

# HELP loom_server_queries_succeeded_total Total successful
# TYPE loom_server_queries_succeeded_total counter
loom_server_queries_succeeded_total 145

# HELP loom_server_queries_failed_total Total failed
# TYPE loom_server_queries_failed_total counter
loom_server_queries_failed_total 5

# HELP loom_server_query_latency_seconds Latency histogram
# TYPE loom_server_query_latency_seconds histogram
loom_server_query_latency_seconds_bucket{le="0.01"} 10
loom_server_query_latency_seconds_bucket{le="0.1"} 100
loom_server_query_latency_seconds_bucket{le="1.0"} 145
loom_server_query_latency_seconds_sum 85.3
loom_server_query_latency_seconds_count 145

# HELP loom_server_queries_pending Current pending
# TYPE loom_server_queries_pending gauge
loom_server_queries_pending 0

# HELP loom_server_query_timeouts_total Timeouts by type
# TYPE loom_server_query_timeouts_total counter
loom_server_query_timeouts_total{query_type="read_file"} 2
loom_server_query_timeouts_total{query_type="env"} 1

# HELP loom_server_queries_success_by_type Success by type
# TYPE loom_server_queries_success_by_type counter
loom_server_queries_success_by_type{query_type="read_file",session_id="s-1"} 50
loom_server_queries_success_by_type{query_type="env",session_id="s-1"} 45

# HELP loom_server_queries_failure_by_type Failures by type
# TYPE loom_server_queries_failure_by_type counter
loom_server_queries_failure_by_type{query_type="read_file",error_type="timeout"} 2
loom_server_queries_failure_by_type{query_type="env",error_type="network"} 1

# HELP loom_server_query_latency_by_type_seconds Latency by type
# TYPE loom_server_query_latency_by_type_seconds histogram
loom_server_query_latency_by_type_seconds_bucket{query_type="read_file",le="0.1"} 80
loom_server_query_latency_by_type_seconds_bucket{query_type="read_file",le="1.0"} 95

# HELP loom_server_queries_pending_by_type Pending by type
# TYPE loom_server_queries_pending_by_type gauge
loom_server_queries_pending_by_type{query_type="read_file"} 0
loom_server_queries_pending_by_type{query_type="env"} 0
```

## Quick Verification

After implementation, verify:

```bash
# 1. Compile check
cargo build --lib -p loom-server
✅ Compiles without errors

# 2. Lint check
cargo clippy --lib -p loom-server
✅ No warnings from query_metrics

# 3. Check endpoint exists
grep -n "route(\"/metrics\"" crates/loom-server/src/api.rs
✅ Found at line ~117

# 4. Check AppState includes metrics
grep -n "query_metrics" crates/loom-server/src/api.rs | head -5
✅ Found in struct and initialization

# 5. Verify module exported
grep -n "pub use query_metrics" crates/loom-server/src/lib.rs
✅ Found public export
```

## Integration Points Summary

| Component | File | Change | Status |
|-----------|------|--------|--------|
| QueryMetrics | query_metrics.rs | New file | ✅ |
| AppState | api.rs | Added field | ✅ |
| Router | api.rs | Added route | ✅ |
| Module exports | lib.rs | Added exports | ✅ |
| Dependencies | Cargo.toml | Added prometheus | ✅ |
| ServerQueryManager | server_query.rs | Ready for integration | ⏳ |
| Handlers | api.rs | Ready for integration | ⏳ |
| Tests | query_metrics.rs | Included | ✅ |
| Documentation | MONITORING_OBSERVABILITY.md | New file | ✅ |

## Testing Verification

Run individual metric tests:
```bash
# Test creation
cargo test --lib query_metrics::tests::test_metrics_creation

# Test counter increments
cargo test --lib query_metrics::tests::test_record_sent_increments_counters

# Test all metrics tests
cargo test --lib query_metrics 2>/dev/null | grep -E "test result|running"
```

## Notes

- **No Pre-existing Tests Modified**: The implementation adds new code only
- **Backward Compatible**: AppState change is additive (new field)
- **Ready for Integration**: All infrastructure in place for next phase
- **Well Documented**: Two comprehensive guides provided
- **Thread Safe**: Uses Arc<> for shared metrics access
- **Performance Impact**: Minimal - atomic counter increments only
