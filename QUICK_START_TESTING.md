# Quick Start: Loom Testing

## 5-Minute Setup

```bash
# Make scripts executable (if needed)
chmod +x scripts/verify_deployment.sh
chmod +x scripts/verify_security.sh
chmod +x scripts/establish_performance_baseline.sh
```

## Essential Commands

### Run Tests
```bash
make test              # All tests
make check            # Full CI check (format + lint + build + test)
```

### Run Specific Tests
```bash
cargo test --test integration_tests           # Integration tests
cargo test --test compatibility_tests         # Compatibility tests
cargo test --lib -p loom-server               # Unit tests (loom-server)
```

### Performance
```bash
cargo bench --bench performance_benchmarks    # Run benchmarks
```

### Pre-Deployment
```bash
./scripts/verify_deployment.sh                # Full verification
./scripts/verify_security.sh                  # Security audit
./scripts/establish_performance_baseline.sh compare  # Performance check
```

## Test Files at a Glance

| File | Tests | Status |
|------|-------|--------|
| `crates/loom-server/tests/integration_tests.rs` | 8+2 | ✅ |
| `crates/loom-server/benches/performance_benchmarks.rs` | 6+ | ✅ |
| `tests/compatibility_tests.rs` | 11 | ✅ |

## Documentation

- **Quick Reference**: [TESTING_INDEX.md](TESTING_INDEX.md)
- **Complete Guide**: [TESTING_COMPREHENSIVE.md](TESTING_COMPREHENSIVE.md)
- **Suite Details**: [TEST_SUITE_FINAL.md](TEST_SUITE_FINAL.md)
- **Full Report**: [INTEGRATION_TEST_SUITE_COMPLETE.md](INTEGRATION_TEST_SUITE_COMPLETE.md)
- **Summary**: [DELIVERABLES_SUMMARY.txt](DELIVERABLES_SUMMARY.txt)

## Development Loop

```bash
# After making changes
make test                # Quick test
make check              # Full check before commit
./scripts/verify_deployment.sh  # Full verification before push
```

## Performance Baseline

```bash
# First time: Establish baseline
./scripts/establish_performance_baseline.sh save

# Later: Compare against baseline
./scripts/establish_performance_baseline.sh compare
```

## Security Checks

```bash
# Run security audit
./scripts/verify_security.sh

# Results in: target/security_reports/
```

## What Was Added

- ✅ 8 Integration tests + 2 property tests
- ✅ 6 Performance benchmarks + variants
- ✅ 11 Compatibility tests
- ✅ 3 Verification scripts
- ✅ 1400+ lines of documentation

## Test Coverage

| Category | Count | Status |
|----------|-------|--------|
| Integration | 8 | ✅ |
| Property Tests | 2 | ✅ |
| Compatibility | 11 | ✅ |
| Benchmarks | 6+ | ✅ |
| **Total** | **27+** | **✅** |

## Troubleshooting

**Tests hang?**
```bash
timeout 60 cargo test test_name
```

**Need verbose output?**
```bash
cargo test test_name -- --nocapture
```

**Run single-threaded?**
```bash
cargo test -- --test-threads=1
```

## Next: Full Deployment

```bash
# Complete pre-deployment check
./scripts/verify_deployment.sh

# Check logs
cat target/verification_logs/verification_summary.log
```

---

For more information, see [TESTING_COMPREHENSIVE.md](TESTING_COMPREHENSIVE.md)
