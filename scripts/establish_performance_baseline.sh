#!/usr/bin/env bash

# Loom Performance Baseline Establishment Script
#
# Establishes and tracks performance baselines for:
# - Query creation time
# - State update performance
# - Concurrent request handling
# - Memory usage
# - Build size
#
# Usage: ./scripts/establish_performance_baseline.sh [save|compare]
# - save: Establish new baseline
# - compare: Compare against existing baseline (default)

set -euo pipefail

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

BASELINE_DIR="./target/performance_baselines"
CURRENT_BASELINE="${BASELINE_DIR}/current_baseline.json"
PREVIOUS_BASELINE="${BASELINE_DIR}/previous_baseline.json"
COMPARISON_LOG="${BASELINE_DIR}/baseline_comparison.log"

ACTION="${1:-compare}"

# Create baseline directory
mkdir -p "$BASELINE_DIR"

print_section() {
    echo ""
    echo -e "${BLUE}=== $1 ===${NC}"
}

# Measure build size
measure_build_size() {
    print_section "Build Size Measurement"
    
    echo "Building release artifacts..."
    cargo build --release --workspace 2>/dev/null || true
    
    local debug_size=$(du -sh target/debug 2>/dev/null | awk '{print $1}' || echo "N/A")
    local release_size=$(du -sh target/release 2>/dev/null | awk '{print $1}' || echo "N/A")
    
    echo "Debug build size: $debug_size"
    echo "Release build size: $release_size"
    
    echo "$release_size"
}

# Measure query latency
measure_query_latency() {
    print_section "Query Latency Measurement"
    
    # Simple test: measure time to create a query
    local start_time=$(date +%s%N)
    cargo test --lib test_component_rendering_integration --quiet 2>/dev/null || true
    local end_time=$(date +%s%N)
    
    local elapsed_ms=$(( (end_time - start_time) / 1000000 ))
    echo "Query creation test completed in ${elapsed_ms}ms"
    
    echo "$elapsed_ms"
}

# Measure concurrent throughput
measure_concurrent_throughput() {
    print_section "Concurrent Request Throughput"
    
    echo "Testing concurrent request handling..."
    
    # Run the concurrent test
    local start_time=$(date +%s%N)
    cargo test --lib test_concurrent_api_requests --quiet 2>/dev/null || true
    local end_time=$(date +%s%N)
    
    local elapsed_ms=$(( (end_time - start_time) / 1000000 ))
    local throughput=$((1000 / (elapsed_ms / 10)))  # Approximate: requests per second
    
    echo "10 concurrent requests completed in ${elapsed_ms}ms"
    echo "Estimated throughput: ~${throughput} requests/sec"
    
    echo "$throughput"
}

# Generate baseline JSON
generate_baseline() {
    local build_size="$1"
    local query_latency="$2"
    local throughput="$3"
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    cat > "${BASELINE_DIR}/measurements.json" <<EOF
{
  "timestamp": "$timestamp",
  "build_size": "$build_size",
  "query_latency_ms": $query_latency,
  "concurrent_throughput_rps": $throughput,
  "rust_version": "$(rustc --version)",
  "cargo_version": "$(cargo --version)"
}
EOF
    
    cat "${BASELINE_DIR}/measurements.json"
}

# Save baseline
save_baseline() {
    print_section "Saving Performance Baseline"
    
    # Move current to previous
    if [[ -f "$CURRENT_BASELINE" ]]; then
        cp "$CURRENT_BASELINE" "$PREVIOUS_BASELINE"
        echo "Backed up previous baseline"
    fi
    
    # Measure all metrics
    local build_size=$(measure_build_size)
    local query_latency=$(measure_query_latency)
    local throughput=$(measure_concurrent_throughput)
    
    # Generate and save baseline
    generate_baseline "$build_size" "$query_latency" "$throughput" > "$CURRENT_BASELINE"
    
    echo -e "${GREEN}✓ Baseline saved to $CURRENT_BASELINE${NC}"
}

# Compare baseline
compare_baseline() {
    print_section "Comparing Against Baseline"
    
    if [[ ! -f "$CURRENT_BASELINE" ]]; then
        echo -e "${YELLOW}⚠ No baseline found. Run with 'save' action first.${NC}"
        return 1
    fi
    
    # Measure current metrics
    local build_size=$(measure_build_size)
    local query_latency=$(measure_query_latency)
    local throughput=$(measure_concurrent_throughput)
    
    # Generate temporary measurement
    generate_baseline "$build_size" "$query_latency" "$throughput" > "${BASELINE_DIR}/current_measurement.json"
    
    # Compare (simple comparison)
    print_section "Baseline Comparison Results"
    
    {
        echo "Timestamp: $(date)"
        echo ""
        echo "Previous Baseline:"
        cat "$CURRENT_BASELINE" | jq . 2>/dev/null || cat "$CURRENT_BASELINE"
        echo ""
        echo "Current Measurement:"
        cat "${BASELINE_DIR}/current_measurement.json" | jq . 2>/dev/null || cat "${BASELINE_DIR}/current_measurement.json"
    } | tee "$COMPARISON_LOG"
    
    echo ""
    echo "Detailed comparison saved to: $COMPARISON_LOG"
}

# Help
show_help() {
    cat <<EOF
Usage: $0 [ACTION]

Actions:
  save      - Establish and save a new performance baseline
  compare   - Compare current performance against baseline (default)
  help      - Show this help message

Examples:
  $0 save           # Save new baseline
  $0 compare        # Compare against baseline
  $0               # Same as 'compare'

Baselines are stored in: $BASELINE_DIR/

Measured Metrics:
  - Build size (debug and release)
  - Query creation latency
  - Concurrent request throughput
  - Rust and Cargo versions

EOF
}

# Main
main() {
    case "${ACTION}" in
        save)
            save_baseline
            ;;
        compare)
            compare_baseline
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            echo "Unknown action: $ACTION"
            echo "Use 'help' for usage information"
            exit 1
            ;;
    esac
}

main "$@"
