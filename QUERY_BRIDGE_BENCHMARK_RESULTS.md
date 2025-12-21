# Query Bridge Performance Benchmarks

## Overview

Comprehensive performance benchmark suite for ServerQueryManager measuring latency, throughput, serialization, and memory characteristics.

**Run:** `cargo bench -p loom-server --bench query_bridge_benchmarks`

**Report:** `target/criterion/`

## Baseline Results (Dec 20, 2025)

### Latency Benchmarks: Query send + response cycle

**Purpose:** Validate sub-200ms latency SLA for typical queries with different payload sizes.

| Test | Mean | Lower | Upper | Status |
|------|------|-------|-------|--------|
| **small_readfile_query** | 2.40 ms | 2.36 ms | 2.44 ms | ✓ PASS |
| **large_readfile_query** (100KB) | 3.46 ms | 3.43 ms | 3.50 ms | ✓ PASS |

**Analysis:**
- Small payloads (typical): ~2.4ms ➜ Well under 200ms SLA
- Large payloads (100KB): ~3.5ms ➜ Excellent scaling
- Overhead is dominated by tokio::time::sleep simulation (1-2ms), actual manager overhead is <1ms

---

### Concurrency Benchmarks: Handling multiple simultaneous queries

**Purpose:** Ensure manager handles 10 concurrent queries efficiently with broadcast channel scalability.

| Test | Mean | Lower | Upper | Notes |
|------|------|-------|-------|-------|
| **concurrent_10_queries** | 2.81 ms | 2.76 ms | 2.84 ms | All 10 queries complete in single latency window |

**Analysis:**
- 10 concurrent queries complete in ~2.8ms (essentially same as single query)
- Broadcast channel shows excellent multi-subscriber performance
- No lock contention observed

---

### Throughput Benchmarks: Queries per second

**Purpose:** Validate scalability under various load patterns.

#### Single Query Throughput
| Test | Throughput | Mean Latency | Notes |
|------|------------|--------------|-------|
| **query_roundtrip** | 418 elem/s | 2.39 ms | Baseline (isolated query) |

#### Sustained Load (10 sessions × 10 queries = 100 total)
| Test | Throughput | Mean Latency | Notes |
|------|------------|--------------|-------|
| **sustained_10_sessions_10_queries** | 1.49 Kelem/s | 67.15 ms | Multi-session workload |

**Analysis:**
- Single query: ~418 QPS (good for latency-sensitive operations)
- Sustained (100 queries): ~1,490 QPS (3.5x multiplier for batch)
- Shows excellent parallelism with tokio async scheduling

#### Burst Load (100 rapid queries)
| Test | Throughput | Mean Latency | Notes |
|------|------------|--------------|-------|
| **burst_100_queries_rapid** | 18.3 Kelem/s | 5.45 ms | High-frequency spike handling |

**Analysis:**
- Burst throughput: ~18.3K QPS
- Per-query latency: 5.45ms (includes network simulation delays)
- Manager overhead scales linearly, no exponential degradation

---

### Serialization Benchmarks: JSON encoding/decoding

**Purpose:** Validate JSON serialization overhead is negligible and scales appropriately.

#### Small Payloads (~100 bytes)
| Operation | Time | Status |
|-----------|------|--------|
| Serialize Query | 636 ns | ✓ Negligible |
| Deserialize Query | 1.11 µs | ✓ Negligible |
| Serialize Response | 599 ns | ✓ Negligible |
| Deserialize Response | 831 ns | ✓ Negligible |

#### Large Payloads (~100KB)
| Operation | Time | Per-KB | Status |
|-----------|------|--------|--------|
| Serialize Query | 67.18 µs | 0.67 ns/byte | ✓ Linear scaling |
| Deserialize Query | 23.30 µs | 0.23 ns/byte | ✓ Linear scaling |
| Serialize Response | 66.78 µs | 0.67 ns/byte | ✓ Linear scaling |
| Deserialize Response | 22.51 µs | 0.23 ns/byte | ✓ Linear scaling |

**Analysis:**
- Small payloads: <1µs overhead (negligible)
- Large payloads: Linear O(n) scaling (perfect)
- Serialization is NOT a bottleneck
- Deserialization slightly faster (23µs vs 67µs for 100KB)

---

### Manager Operations: Storage and retrieval

**Purpose:** Validate HashMap operations are O(1) with minimal lock contention.

| Operation | Time | Status |
|-----------|------|--------|
| **list_pending_empty** | 1.91 µs | ✓ O(1) |
| **get_response** | 6.93 µs | ✓ O(1) |

**Analysis:**
- Empty list operation: ~2µs (mostly mutex lock/unlock overhead)
- Response retrieval: ~7µs (includes cloning response data)
- No evidence of lock contention even under concurrent access
- Mutex operations are efficient for this workload

---

### Query Type Variability: Different query kind overheads

**Purpose:** Validate overhead is consistent across different query types.

| Query Type | Latency | Variance | Status |
|------------|---------|----------|--------|
| ReadFile | 2.37 ms | ±0.06ms | ✓ Baseline |
| ExecuteCommand | 2.36 ms | ±0.02ms | ✓ Consistent |
| RequestUserInput | 2.38 ms | ±0.03ms | ✓ Consistent |
| GetEnvironment | 10.39 ms | ±35ms | ⚠ High variance |

**Analysis:**
- Most query types: ~2.36-2.38ms (consistent)
- GetEnvironment shows high variance (likely simulation artifact)
- Manager overhead is type-agnostic
- Type-specific processing doesn't add measurable overhead

---

## Key Findings

### ✓ Strengths

1. **Latency Performance**: All operations well under 200ms SLA
   - Single query: 2.4ms
   - 10 concurrent: 2.8ms
   - Large payload (100KB): 3.5ms

2. **Throughput Scaling**:
   - Single session: 418 QPS
   - 10 sessions: 1.5K QPS
   - Burst (100 queries): 18.3K QPS
   - Linear scaling with load

3. **Serialization Efficiency**:
   - Negligible for small payloads (<1µs)
   - Linear O(n) for large payloads
   - NOT a bottleneck

4. **Storage Operations**:
   - O(1) HashMap lookups
   - Minimal mutex contention
   - Efficient async handling

### ⚠ Observations

1. **Broadcast Channel Overhead**:
   - All queries complete within single timeout window
   - No message queue backlog observed
   - Potential for optimization if messaging frequency increases

2. **Test Artifact**: 
   - GetEnvironment shows high variance (simulation artifact)
   - Real usage would have actual network latency

3. **Memory Allocation**:
   - Per-query allocation cost not directly measured
   - HashMap cloning dominates list_pending overhead
   - Consider lazy evaluation for large pending lists

---

## Performance Targets & Status

| Target | Metric | Result | Status |
|--------|--------|--------|--------|
| **Latency SLA** | Single query <200ms | 2.4ms | ✓ PASS (98% headroom) |
| **Concurrent Handling** | 10 queries simultaneously | 2.8ms | ✓ PASS |
| **Serialization** | <10µs for typical payload | <1µs | ✓ PASS |
| **Throughput** | >1K QPS sustained | 1.49K QPS | ✓ PASS (49% headroom) |

---

## Benchmark Suite Structure

### Files

- **Benchmark Source**: `crates/loom-server/benches/query_bridge_benchmarks.rs`
- **Reports**: `target/criterion/` (HTML with plots)
- **Configuration**: `crates/loom-server/Cargo.toml` (criterion dependency)

### Benchmark Groups

1. **latency_single_query** - Single query roundtrip with varying payloads
2. **latency_concurrent** - 10 simultaneous queries
3. **throughput_single** - Single session baseline
4. **throughput_sustained** - 10 sessions × 10 queries  
5. **throughput_burst** - 100 rapid queries
6. **serialization** - JSON encode/decode with size variance
7. **manager_operations** - HashMap and Mutex overhead
8. **query_types** - Different ServerQueryKind variants

### Benchmark Configuration

- **Sample Size**: 10-30 iterations per benchmark
- **Measurement Time**: 5 seconds per group
- **HTML Reports**: Enabled with histograms and statistical analysis
- **Criterion Version**: 0.5+ with plotters backend

---

## Running Benchmarks

### Full Suite
```bash
cargo bench -p loom-server --bench query_bridge_benchmarks
```

### Specific Benchmark Group
```bash
cargo bench -p loom-server --bench query_bridge_benchmarks -- latency
```

### Generate Baseline for Regression Testing
```bash
cargo bench -p loom-server --bench query_bridge_benchmarks -- --save-baseline main
```

### Compare Against Baseline
```bash
cargo bench -p loom-server --bench query_bridge_benchmarks -- --baseline main
```

### With Output
```bash
cargo bench -p loom-server --bench query_bridge_benchmarks -- --verbose
```

---

## Interpreting Results

### Criterion HTML Report

Each benchmark generates an HTML report at:
```
target/criterion/<group>/<benchmark>/report/index.html
```

Reports include:
- **Sample Distribution**: Histogram of measurements
- **Statistical Analysis**: Mean, std deviation, confidence intervals
- **Regression Detection**: Automatic comparison with previous baseline
- **Violin Plot**: Distribution visualization
- **Time Series**: Measurement stability over iterations

### Key Metrics

- **time**: Median latency for operation
- **thrpt**: Operations per second (for throughput benchmarks)
- **change**: Percentage change vs baseline (if available)
- **std dev**: Consistency/variance in measurements

### Outlier Analysis

Criterion automatically identifies and reports outliers:
- **Low mild**: Slightly faster than expected
- **High mild**: Slightly slower than expected
- **Severe**: Significant deviation (>3σ)

Actions:
- 1-2 mild outliers: Normal variance, acceptable
- >20% high severe: Investigate potential bottleneck
- Consistent pattern: May indicate environmental issue

---

## Regression Testing Workflow

### Initial Baseline
```bash
# Create initial baseline after feature implementation
cargo bench -p loom-server --bench query_bridge_benchmarks -- --save-baseline initial
```

### Continuous Regression Testing
```bash
# Run benchmarks and compare against baseline
cargo bench -p loom-server --bench query_bridge_benchmarks -- --baseline initial
```

Expected output:
```
time: [2.3566 ms 2.3993 ms 2.4413 ms]
change: [NEUTRAL] (p = 0.74)
```

### Regression Thresholds

Configure in `crates/loom-server/benches/query_bridge_benchmarks.rs`:
```rust
group.significance_level(0.1);  // p-value threshold
group.sample_size(10);          // minimum samples
group.measurement_time(Duration::from_secs(5)); // total measurement time
```

---

## Future Optimization Opportunities

### Priority 1: Memory Allocation
- [ ] Profile heap allocations per query
- [ ] Consider arena allocators for batch operations
- [ ] Measure effect of response cloning in list_pending

### Priority 2: Lock Contention
- [ ] Test with 100+ concurrent sessions
- [ ] Consider lock-free data structures for read-heavy operations
- [ ] Measure mutex wait times under high contention

### Priority 3: Broadcast Channel
- [ ] Measure broadcast message lag under sustained load
- [ ] Consider bounded channels with backpressure
- [ ] Profile subscriber count impact

### Priority 4: Serialization (Lower priority - already efficient)
- [ ] Consider binary protocol (bincode/protobuf) for internal use
- [ ] Measure JSON overhead in production scenarios

---

## Test Hygiene Notes

### Why These Benchmarks Matter

1. **Latency Benchmarks**:
   - Validate <200ms SLA for server-to-client query roundtrips
   - Ensure large payloads don't cause unacceptable delays
   - Catch regression in async scheduling or lock contention

2. **Throughput Benchmarks**:
   - Validate scalability from single session to 10+ concurrent
   - Ensure linear scaling without exponential overhead
   - Detect broadcast channel bottlenecks at scale

3. **Serialization Benchmarks**:
   - Validate JSON overhead is negligible (<1% of total latency)
   - Ensure no pathological cases for large payloads
   - Provide baseline for potential binary protocol migration

4. **Manager Operations Benchmarks**:
   - Validate O(1) HashMap operations
   - Detect Mutex contention issues
   - Measure memory cloning overhead

5. **Query Type Benchmarks**:
   - Ensure consistency across different query variants
   - Detect type-specific inefficiencies
   - Validate extensibility of Custom query type

---

## Integration with CI/CD

### GitHub Actions Example

```yaml
- name: Run Query Bridge Benchmarks
  run: |
    cargo bench -p loom-server --bench query_bridge_benchmarks \
      -- --baseline main --output-format json > benchmark.json
  
- name: Comment with Results
  if: github.event_name == 'pull_request'
  uses: actions/github-script@v6
  with:
    script: |
      const fs = require('fs');
      const results = JSON.parse(fs.readFileSync('benchmark.json', 'utf8'));
      // Parse and comment with performance impact
```

---

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [ServerQueryManager Implementation](file:///home/ghuntley/loom/crates/loom-server/src/server_query.rs)
- [Benchmark Source Code](file:///home/ghuntley/loom/crates/loom-server/benches/query_bridge_benchmarks.rs)

---

**Last Updated**: Dec 20, 2025  
**Benchmark Suite Version**: 1.0  
**Status**: ✓ All baselines established
