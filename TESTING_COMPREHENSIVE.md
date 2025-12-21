# Comprehensive Testing Guide

This guide covers all testing frameworks, tools, and procedures for the Loom system.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Unit Tests](#unit-tests)
3. [Integration Tests](#integration-tests)
4. [Performance Benchmarks](#performance-benchmarks)
5. [Compatibility Tests](#compatibility-tests)
6. [E2E Tests](#e2e-tests)
7. [Test Results Interpretation](#test-results-interpretation)
8. [Continuous Integration](#continuous-integration)

## Quick Start

### Run All Tests
```bash
make test
```

### Run Specific Test Suite
```bash
# Unit tests only
cargo test --lib --workspace

# Integration tests only
cargo test --test integration_tests

# Compatibility tests
cargo test --test compatibility_tests

# E2E tests
make test-e2e
```

### Run With Output
```bash
# Show println! output
cargo test -- --nocapture

# Show test names
cargo test -- --nocapture --test-threads=1
```

## Unit Tests

Unit tests verify individual components in isolation.

### Location
- **Rust tests**: `src/**/*.rs` files with `#[cfg(test)]` modules
- **Web tests**: `crates/loom-web/src/**/*.rs` with WASM test support

### Running Unit Tests

```bash
# All unit tests
cargo test --lib --workspace

# Single crate
cargo test --lib -p loom-server

# Specific test
cargo test --lib query_creation_simple
```

### Writing Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Test description explaining what's being tested.
    ///
    /// Purpose: Explain why this test is important and what it validates.
    #[test]
    fn test_function_behavior() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }
}
```

## Integration Tests

Integration tests verify components working together.

### Test Files
- `crates/loom-server/tests/integration_tests.rs` - Query bridge integration
- Other integration scenarios in `crates/*/tests/`

### Running Integration Tests

```bash
# All integration tests
cargo test --test '*'

# Specific test
cargo test --test integration_tests test_component_rendering_integration

# With output
cargo test --test integration_tests -- --nocapture
```

### Integration Test Scenarios

#### 1. Component Rendering Integration
**File**: `tests/integration_tests.rs::test_component_rendering_integration`

Tests that API endpoints correctly handle component rendering requests.

**What it tests**:
- Request validation
- Response formatting
- Status codes

**Run**:
```bash
cargo test --test integration_tests test_component_rendering_integration
```

#### 2. Concurrent API Requests
**File**: `tests/integration_tests.rs::test_concurrent_api_requests`

Verifies the system handles concurrent requests without race conditions.

**What it tests**:
- Concurrent request handling (10 simultaneous)
- Data isolation
- No data corruption

**Run**:
```bash
cargo test --test integration_tests test_concurrent_api_requests
```

#### 3. State Management Isolation
**File**: `tests/integration_tests.rs::test_state_management_isolation`

Ensures state doesn't leak between requests.

**What it tests**:
- Independent query storage
- State isolation
- Query count accuracy

**Run**:
```bash
cargo test --test integration_tests test_state_management_isolation
```

#### 4. Error Handling
**File**: `tests/integration_tests.rs::test_error_handling_invalid_input`

Validates proper error handling for invalid inputs.

**What it tests**:
- Input validation
- Error response format
- Graceful failure

**Run**:
```bash
cargo test --test integration_tests test_error_handling
```

### Property-Based Tests

Property tests use `proptest` to verify properties hold across many inputs.

**File**: `tests/integration_tests.rs::property_tests`

```bash
cargo test --test integration_tests prop_valid_query_produces_valid_response
```

## Performance Benchmarks

Performance benchmarks measure and track system performance.

### Benchmark Tool
Uses `criterion` crate for statistical benchmarking with HTML reports.

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --bench performance_benchmarks

# Run specific benchmark
cargo bench --bench performance_benchmarks query_creation

# Generate HTML report (in target/criterion/)
cargo bench --bench performance_benchmarks -- --verbose
```

### Benchmark Scenarios

#### 1. Query Creation Time
**Name**: `query_creation_simple`

Simple query creation baseline measurement.

**Expected**: < 1ms per query
**Measured**: Creation + ID generation overhead

#### 2. Query with Context
**Name**: `query_creation_with_context`

Query creation with additional context data.

**Expected**: < 2ms per query
**Measured**: Context serialization overhead

#### 3. Concurrent Operations (10-500 queries)
**Name**: `concurrent_queries`

Throughput under concurrent load.

**Expected**: Linear scaling up to system limits
**Parameters**: 10, 50, 100, 500 concurrent queries

#### 4. State Updates by Size
**Name**: `state_updates_by_size`

Performance with varying data sizes.

**Tested sizes**: 10B, 100B, 1KB, 10KB
**Expected**: Linear growth with data size

#### 5. Query Manager Operations
**Name**: `query_manager_create_and_lock`

Synchronization overhead measurement.

**Expected**: < 100µs per operation
**Measured**: Lock acquisition and release

#### 6. UUID Generation
**Name**: `uuid_generation`

ID generation baseline.

**Expected**: < 10µs per UUID
**Measured**: System random generation

### Interpreting Benchmark Results

```
test query_creation_simple
  time:   [123.45 us 124.32 us 125.28 us]
           ├─┬─ lower bound: 123.45 µs
           │ ├─ estimate:    124.32 µs
           │ └─ upper bound: 125.28 µs
           ...
  change: [-2.34% -1.45% -0.61%]
           (likely improvement in this run)
```

- **time range**: Statistical confidence interval
- **change**: Performance change from last run
- **likely improvement/regression**: Statistical assessment

## Compatibility Tests

Tests verify compatibility across versions and platforms.

### Running Compatibility Tests

```bash
cargo test --test compatibility_tests
```

### Test Coverage

#### 1. Rust Edition Compatibility
Verifies Rust 1.70+ compatibility.

```bash
cargo test --test compatibility_tests test_rust_edition_compatibility
```

#### 2. Dependency Compatibility
Checks all workspace dependencies.

```bash
cargo test --test compatibility_tests test_dependency_compatibility
```

#### 3. Feature Combinations
Tests various feature flag combinations.

```bash
cargo test --test compatibility_tests test_feature_combinations
```

#### 4. Code Formatting
Verifies consistent formatting.

```bash
cargo test --test compatibility_tests test_code_formatting_consistency
```

#### 5. Clippy Lints
Checks code quality lints.

```bash
cargo test --test compatibility_tests test_clippy_lints
```

#### 6. Browser Compatibility
Tests web UI in different browsers (E2E tests).

Tested browsers:
- Chrome/Chromium
- Firefox
- Safari
- Edge

## E2E Tests

End-to-end tests verify complete user workflows.

### Framework
Uses Playwright with TypeScript.

### Location
- `crates/loom-web/tests/e2e/` - E2E test files
- `playwright.config.ts` - Configuration

### Running E2E Tests

```bash
# Run all E2E tests
make test-e2e

# Interactive UI mode
make test-e2e-ui

# Debug mode
make test-e2e-debug

# Or from loom-web directory
cd crates/loom-web
npm run test:e2e
npm run test:e2e:ui    # Interactive
npm run test:e2e:debug # Debugging
```

### E2E Test Scenarios

E2E tests cover:
1. **Component rendering** - Visual correctness
2. **User interactions** - Click, type, submit
3. **Navigation** - Route transitions
4. **API integration** - Backend communication
5. **Error scenarios** - Error handling
6. **Accessibility** - WCAG compliance

## Test Results Interpretation

### Success Indicators
```bash
$ cargo test --workspace
   Compiling loom v0.1.0
    Finished test [unoptimized + debuginfo] target(s) in 2.34s
     Running unittests src/lib.rs
...
test result: ok. 42 passed; 0 failed; 0 ignored; 12 measured

     Running tests/integration_tests.rs
...
test result: ok. 8 passed; 0 failed; 0 ignored
```

✓ **All tests passed**

### Failure Indicators

#### Test Failure Example
```rust
thread 'test_concurrent_api_requests' panicked at 'assertion failed: 
result.is_ok()', tests/integration_tests.rs:42:5
```

**Action**:
1. Read the panic message carefully
2. Check the assertion condition
3. Review test logs with `--nocapture`
4. Fix the code or test as appropriate

#### Benchmark Regression Example
```
test query_creation_simple
  time:   [123.45 us 124.32 us 125.28 us]
  change: [+15.34% +18.45% +21.23%]
           (likely regression detected)
```

**Action**:
1. Investigate code changes since last baseline
2. Profile to identify bottleneck
3. Optimize or accept with documentation
4. Update baseline: `./scripts/establish_performance_baseline.sh save`

## Continuous Integration

### CI Pipeline

Tests run automatically on:
- Pull requests
- Commits to main branch
- Scheduled daily runs

### CI Checks (in order)

1. **Format check** - `cargo fmt --all -- --check`
2. **Lint check** - `cargo clippy --workspace -- -D warnings`
3. **Unit tests** - `cargo test --lib --workspace`
4. **Integration tests** - `cargo test --test '*'`
5. **Build release** - `cargo build --release --workspace`
6. **Documentation** - `cargo doc --workspace --no-deps`
7. **E2E tests** - `npm run test:e2e` (web only)

### Local CI Check

Run full CI pipeline locally:
```bash
make check
```

This runs:
1. Format check
2. Lint check (clippy)
3. Build
4. Tests

## Performance Baseline Tracking

### Establish Baseline
```bash
./scripts/establish_performance_baseline.sh save
```

Saves current performance measurements to `target/performance_baselines/`.

### Compare Against Baseline
```bash
./scripts/establish_performance_baseline.sh compare
```

Compares current performance against saved baseline.

### Baseline Location
- Current: `target/performance_baselines/current_baseline.json`
- Previous: `target/performance_baselines/previous_baseline.json`
- Measurements: `target/performance_baselines/measurements.json`

## Deployment Verification

### Pre-Deployment Checklist

Run comprehensive verification:
```bash
./scripts/verify_deployment.sh
```

This performs:
- ✓ Rust toolchain verification
- ✓ Build verification
- ✓ Unit test verification
- ✓ Integration test verification
- ✓ Code quality checks
- ✓ Dependency checks
- ✓ Security audit
- ✓ Performance baseline verification
- ✓ Documentation verification
- ✓ Compatibility verification

### Verification Results
Results saved to: `target/verification_logs/`
- `verification_summary.log` - Summary of all checks
- `verification_detailed.log` - Detailed results
- Various test logs - Specific test output

## Troubleshooting

### Tests Hang or Timeout

```bash
# Run with timeout
timeout 60 cargo test test_name

# Run single-threaded for better diagnostics
cargo test test_name -- --test-threads=1
```

### Memory Issues During Testing

```bash
# Reduce test parallelism
cargo test -- --test-threads=2

# Run only unit tests (lighter weight)
cargo test --lib
```

### Benchmark Noise

For stable benchmarks:
```bash
# Close other applications
# Disable CPU frequency scaling
# Run multiple times:
cargo bench --bench performance_benchmarks -- --verbose --save-baseline baseline_name
```

### CI Failures Don't Match Local

```bash
# Run exact CI check locally
make check

# Or individually:
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo build --workspace
cargo test --workspace
```

## Best Practices

### Writing Tests

1. **Use clear names**: `test_creates_query_with_valid_input`
2. **Test one thing**: One assertion per test focus
3. **Document purpose**: Add doc comment explaining why
4. **Arrange-Act-Assert**: Clear structure
5. **Use helpers**: Extract common setup

### Running Tests

1. **Before commit**: `make check`
2. **Before push**: `./scripts/verify_deployment.sh`
3. **Regular baseline**: Run `establish_performance_baseline.sh` periodically
4. **Monitor CI**: Check GitHub Actions results

### Performance Testing

1. **Establish baseline early**
2. **Test in release mode** for real-world numbers
3. **Measure on stable hardware**
4. **Track trends, not absolute numbers**
5. **Document regressions** with investigation results

## Additional Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Playwright Testing](https://playwright.dev/)
- [Proptest Documentation](https://docs.rs/proptest/)

## Support

For test-related questions or issues:
1. Check test output with `--nocapture` flag
2. Review test documentation in code
3. Check CI logs for environment-specific issues
4. Consult project documentation
