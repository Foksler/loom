# Query Metrics - Quick Reference

## TL;DR

Loom server now exports Prometheus metrics for all query operations. Metrics are available at `GET /metrics`.

## Metrics Endpoint

```
GET http://localhost:8080/metrics
```

Returns Prometheus text format with all query metrics.

## Key Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `loom_queries_sent_total` | Counter | none | Total queries sent |
| `loom_queries_succeeded_total` | Counter | none | Successful queries |
| `loom_queries_failed_total` | Counter | none | Failed queries |
| `loom_queries_pending` | Gauge | none | Current pending count |
| `loom_query_latency_seconds` | Histogram | none | Query latency |
| `loom_query_timeouts_total` | Counter | query_type | Timeouts by type |
| `loom_queries_success_by_type` | Counter | query_type, session_id | Success by type+session |
| `loom_queries_failure_by_type` | Counter | query_type, error_type | Failure by type+error |

## Query Types

- `read_file` - File reading
- `env` - Environment variables
- `workspace` - Workspace info

## Error Types

- `timeout` - Query timed out
- `network` - Network error
- `invalid_response` - Invalid response format
- (extensible for custom errors)

## Prometheus Queries

```promql
# Query rate (queries/second)
rate(loom_queries_sent_total[1m])

# Success rate (percentage)
(rate(loom_queries_succeeded_total[5m]) / rate(loom_queries_sent_total[5m])) * 100

# Average latency (seconds)
rate(loom_query_latency_seconds_sum[5m]) / rate(loom_query_latency_seconds_count[5m])

# P95 latency (seconds)
histogram_quantile(0.95, loom_query_latency_seconds)

# Pending queries
loom_queries_pending

# Timeout rate (timeouts/second)
rate(loom_query_timeouts_total[5m])

# Queries by type
rate(loom_queries_sent_total{query_type="read_file"}[1m])

# Queries by session
loom_queries_success_by_type{session_id="alice"}
```

## Alerting Examples

```yaml
groups:
  - name: loom-queries
    rules:
      # Alert if query success rate drops below 95%
      - alert: LowQuerySuccessRate
        expr: |
          (rate(loom_queries_succeeded_total[5m]) / rate(loom_queries_sent_total[5m])) < 0.95
        for: 5m
        annotations:
          summary: "Query success rate below 95%"

      # Alert if timeout rate is high
      - alert: HighTimeoutRate
        expr: rate(loom_query_timeouts_total[5m]) > 0.01
        for: 5m
        annotations:
          summary: "Query timeout rate above 1%"

      # Alert if too many pending queries
      - alert: TooManyPendingQueries
        expr: loom_queries_pending > 100
        for: 2m
        annotations:
          summary: "More than 100 pending queries"

      # Alert if query latency is high
      - alert: HighQueryLatency
        expr: histogram_quantile(0.95, loom_query_latency_seconds) > 2
        for: 5m
        annotations:
          summary: "P95 query latency exceeds 2 seconds"
```

## Grafana Dashboard

### Query Volume Panel
```
rate(loom_queries_sent_total[1m])
```

### Success Rate Panel
```
(rate(loom_queries_succeeded_total[5m]) / rate(loom_queries_sent_total[5m])) * 100
```

### Latency Panel (Graph)
```
histogram_quantile(0.95, loom_query_latency_seconds)
histogram_quantile(0.5, loom_query_latency_seconds)
histogram_quantile(0.99, loom_query_latency_seconds)
```

### Pending Queries Panel (Gauge)
```
loom_queries_pending
```

### Query Types Panel (Pie Chart)
```
increase(loom_queries_sent_total{query_type=~".*"}[5m])
```

### Error Distribution Panel (Pie Chart)
```
increase(loom_queries_failure_by_type{error_type=~".*"}[5m])
```

## Common Issues

### No metrics appearing
1. Ensure server is running
2. Check `/metrics` endpoint returns data
3. Verify Prometheus can reach the endpoint

### Wrong labels in metrics
- Labels are set automatically by the system
- Check query_type values: read_file, env, workspace
- Check session_id values: format from client

### Metrics stuck at same value
- Normal if no queries are being sent
- Counters don't reset, only increase
- Gauges change with active queries

## Performance Impact

- Negligible: ~1-2 microseconds per query
- Metrics gathering: <1ms per request
- No blocking operations

## Configuration

No configuration needed. Metrics enabled by default.

To configure Prometheus scraping:
```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'loom-server'
    metrics_path: '/metrics'
    static_configs:
      - targets: ['localhost:8080']
```

## Integration

### Prometheus
1. Add scrape config pointing to `/metrics`
2. Start Prometheus
3. Query metrics in Prometheus UI

### Grafana
1. Add Prometheus as datasource
2. Create dashboard with panels using PromQL
3. Import sample dashboards

### Alertmanager
1. Set up alert rules (see examples above)
2. Configure Alertmanager receivers
3. Connect Prometheus to Alertmanager

## Support

See `QUERY_METRICS_IMPLEMENTATION.md` for detailed documentation.
