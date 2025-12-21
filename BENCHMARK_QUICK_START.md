# Query Bridge Benchmarks - Quick Start

## TL;DR

```bash
# Run all benchmarks
cargo bench -p loom-server --bench query_bridge_benchmarks

# View HTML report
open target/criterion/report/index.html
```

## Files

- **Benchmark Code**: [crates/loom-server/benches/query_bridge_benchmarks.rs](file:///home/ghuntley/loom/crates/loom-server/benches/query_bridge_benchmarks.rs)
- **Detailed Results**: [QUERY_BRIDGE_BENCHMARK_RESULTS.md](file:///home/ghuntley/loom/QUERY_BRIDGE_BENCHMARK_RESULTS.md)
- **HTML Report**: `target/criterion/report/index.html`

## What's Benchmarked

### ✓ Latency (Roundtrip time)
- Single query: **2.4ms**
- Large payload (100KB): **3.5ms**
- 10 concurrent: **2.8ms**

### ✓ Throughput (Queries per second)
- Single session: **418 QPS**
- 10 sessions: **1.5K QPS**
- 100 burst: **18.3K QPS**

### ✓ Serialization (JSON overhead)
- Small (<1µs)
- Large (67µs for 100KB)

### ✓ Manager Operations
- HashMap lookup: **2µs** (empty list)
- Response retrieval: **7µs** (with clone)

### ✓ Query Types
- ReadFile, ExecuteCommand, RequestUserInput, GetEnvironment all ~2.4ms

## Common Commands

### Run specific benchmark group
```bash
cargo bench -p loom-server --bench query_bridge_benchmarks -- latency
cargo bench -p loom-server --bench query_bridge_benchmarks -- throughput
cargo bench -p loom-server --bench query_bridge_benchmarks -- serialization
```

### Save baseline for regression testing
```bash
cargo bench -p loom-server --bench query_bridge_benchmarks -- --save-baseline main
```

### Check for regressions
```bash
cargo bench -p loom-server --bench query_bridge_benchmarks -- --baseline main
```

### Verbose output
```bash
cargo bench -p loom-server --bench query_bridge_benchmarks -- --verbose
```

## Interpreting Results

- **time: [2.40 ms]** = median latency
- **thrpt: [418 elem/s]** = operations per second  
- **change: NEUTRAL** = no regression vs baseline
- Outliers are normal if <20%

## Key Metrics

| Metric | Target | Result | Status |
|--------|--------|--------|--------|
| Single query latency | <200ms | 2.4ms | ✓ |
| Concurrent (10x) | <200ms | 2.8ms | ✓ |
| Throughput (sustained) | >1K QPS | 1.5K QPS | ✓ |
| Serialization | <10µs | <1µs | ✓ |

## Test Details

**Why important**: 
- Validates sub-200ms SLA for server↔client queries
- Measures throughput for concurrent clients
- Ensures JSON serialization isn't a bottleneck
- Detects regressions in async/locking performance

**What's measured**:
- End-to-end roundtrip time (send + receive)
- Different payload sizes (small, medium, large)
- Various concurrency levels (1, 10, 100)
- All ServerQueryKind variants

**Test setup**:
- Tokio async runtime
- Mock response handling (simulated delays)
- Arc/Mutex-based manager
- Broadcast channel for responses

## Hypothesis Testing

Criterion uses statistical analysis to detect regressions:

```
time: [2.3993 ms]
change: [-1.2288% +0.3074% +1.9798%] (p = 0.74 > 0.05)
No change in performance detected.
```

- **p-value > 0.05**: No regression (95% confidence)
- **p-value < 0.05**: Likely regression, investigate
- **Outliers**: Normal if <20%, concerning if >20%

## CI Integration

Add to GitHub Actions:

```yaml
- name: Run Benchmarks
  run: cargo bench -p loom-server --bench query_bridge_benchmarks
```

For regression detection:

```yaml
- name: Check Regressions
  run: |
    cargo bench -p loom-server --bench query_bridge_benchmarks \
      -- --baseline main --output-format json > benchmark.json
```

## Performance Targets

All passing ✓

| Category | Metric | Target | Actual | Headroom |
|----------|--------|--------|--------|----------|
| Latency | Single query | <200ms | 2.4ms | 98% |
| Latency | 10 concurrent | <200ms | 2.8ms | 98% |
| Throughput | Sustained QPS | >1K | 1.49K | 49% |
| Serialization | JSON overhead | <10µs | <1µs | 99% |

## Troubleshooting

### "Unable to complete 10 samples in time"
- Increase measurement_time or reduce sample_size
- Already configured - if persists, may indicate system load

### "assertion failed: n >= 10"
- Need minimum 10 samples
- Check sample_size and measurement_time configuration
- Already fixed in current version

### High variance in results
- Run on quiet system (no background processes)
- Check CPU governor settings
- Expected variance: <5% for latency tests

### Regression detected
1. Check recent changes to ServerQueryManager
2. Profile with `cargo flamegraph`
3. Compare against baseline with `--baseline`

## Next Steps

- [ ] Run baseline comparison with `--baseline main`
- [ ] Integrate into CI/CD pipeline
- [ ] Set up automated regression detection
- [ ] Consider memory profiling if latency increases
- [ ] Monitor throughput trends over time

## References

- [Full Results](file:///home/ghuntley/loom/QUERY_BRIDGE_BENCHMARK_RESULTS.md)
- [Criterion Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Benchmark Source](file:///home/ghuntley/loom/crates/loom-server/benches/query_bridge_benchmarks.rs)

---

**Generated**: Dec 20, 2025  
**Status**: ✓ All benchmarks passing
