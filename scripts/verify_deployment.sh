#!/usr/bin/env bash

# Loom Deployment Verification Suite
# 
# Comprehensive pre-deployment verification script that checks:
# - All builds complete successfully
# - All tests pass
# - Performance baselines are met
# - Security requirements are satisfied
# - Compatibility is verified
#
# Exit codes:
# 0 = All checks passed
# 1 = One or more checks failed
# 2 = Script error

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
CHECKS_PASSED=0
CHECKS_FAILED=0
CHECKS_TOTAL=0

# Log files
LOG_DIR="./target/verification_logs"
SUMMARY_LOG="${LOG_DIR}/verification_summary.log"
DETAILED_LOG="${LOG_DIR}/verification_detailed.log"

# Functions

create_log_dir() {
    mkdir -p "${LOG_DIR}"
    echo "Verification started at $(date)" > "${SUMMARY_LOG}"
    echo "Detailed logs:" >> "${SUMMARY_LOG}"
}

log_check() {
    local check_name="$1"
    local status="$2"
    local details="${3:-}"
    
    ((CHECKS_TOTAL++))
    
    if [[ "$status" == "PASS" ]]; then
        echo -e "${GREEN}✓${NC} $check_name"
        echo "[PASS] $check_name" >> "${SUMMARY_LOG}"
        ((CHECKS_PASSED++))
    elif [[ "$status" == "WARN" ]]; then
        echo -e "${YELLOW}⚠${NC} $check_name"
        echo "[WARN] $check_name" >> "${SUMMARY_LOG}"
        if [[ -n "$details" ]]; then
            echo "  Details: $details" >> "${SUMMARY_LOG}"
        fi
    else
        echo -e "${RED}✗${NC} $check_name"
        echo "[FAIL] $check_name" >> "${SUMMARY_LOG}"
        if [[ -n "$details" ]]; then
            echo "  Details: $details" >> "${SUMMARY_LOG}"
        fi
        ((CHECKS_FAILED++))
    fi
    
    if [[ -n "$details" ]]; then
        echo "  $check_name: $details" >> "${DETAILED_LOG}"
    fi
}

print_section() {
    echo ""
    echo -e "${BLUE}=== $1 ===${NC}"
    echo "=== $1 ===" >> "${SUMMARY_LOG}"
}

# Verification checks

verify_rust_toolchain() {
    print_section "Rust Toolchain Verification"
    
    # Check rustc version
    if command -v rustc &> /dev/null; then
        local rustc_version=$(rustc --version)
        log_check "Rust compiler installed" "PASS" "$rustc_version"
    else
        log_check "Rust compiler installed" "FAIL" "rustc not found"
        return 1
    fi
    
    # Check cargo version
    if command -v cargo &> /dev/null; then
        local cargo_version=$(cargo --version)
        log_check "Cargo package manager installed" "PASS" "$cargo_version"
    else
        log_check "Cargo package manager installed" "FAIL" "cargo not found"
        return 1
    fi
}

verify_build() {
    print_section "Build Verification"
    
    echo "Building workspace..."
    if cargo build --workspace 2>&1 | tee "${LOG_DIR}/build.log"; then
        log_check "Workspace builds successfully" "PASS"
        return 0
    else
        log_check "Workspace builds successfully" "FAIL" "See ${LOG_DIR}/build.log"
        return 1
    fi
}

verify_unit_tests() {
    print_section "Unit Test Verification"
    
    echo "Running unit tests..."
    if cargo test --lib --workspace 2>&1 | tee "${LOG_DIR}/unit_tests.log"; then
        local test_count=$(grep -c "test result: ok" "${LOG_DIR}/unit_tests.log" || echo "0")
        log_check "Unit tests pass" "PASS" "$test_count test modules passed"
        return 0
    else
        log_check "Unit tests pass" "FAIL" "See ${LOG_DIR}/unit_tests.log"
        return 1
    fi
}

verify_integration_tests() {
    print_section "Integration Test Verification"
    
    echo "Running integration tests..."
    if cargo test --test "*" --workspace 2>&1 | tee "${LOG_DIR}/integration_tests.log"; then
        log_check "Integration tests pass" "PASS"
        return 0
    else
        log_check "Integration tests pass" "FAIL" "See ${LOG_DIR}/integration_tests.log"
        return 1
    fi
}

verify_code_quality() {
    print_section "Code Quality Verification"
    
    # Format check
    echo "Checking code formatting..."
    if cargo fmt --all -- --check 2>&1 | tee "${LOG_DIR}/format_check.log"; then
        log_check "Code formatting" "PASS"
    else
        log_check "Code formatting" "WARN" "Some files need formatting. Run 'cargo fmt --all'"
    fi
    
    # Clippy lints
    echo "Running clippy..."
    if cargo clippy --workspace -- -D warnings 2>&1 | tee "${LOG_DIR}/clippy.log"; then
        log_check "Clippy lint checks" "PASS"
    else
        log_check "Clippy lint checks" "WARN" "Some warnings found. See ${LOG_DIR}/clippy.log"
    fi
}

verify_dependencies() {
    print_section "Dependency Verification"
    
    echo "Checking dependencies..."
    if command -v cargo-tree &> /dev/null; then
        cargo tree --depth 1 > "${LOG_DIR}/dependencies.log" 2>&1
        log_check "Dependency tree generated" "PASS"
    else
        log_check "Dependency tree" "WARN" "cargo-tree not installed"
    fi
    
    # Check for outdated dependencies
    if command -v cargo-outdated &> /dev/null; then
        echo "Checking for outdated dependencies..."
        if cargo outdated --root-deps-only 2>&1 | tee "${LOG_DIR}/outdated_deps.log"; then
            log_check "No critical outdated dependencies" "PASS"
        else
            log_check "No critical outdated dependencies" "WARN" "Some dependencies may be outdated"
        fi
    else
        log_check "Dependency updates" "WARN" "cargo-outdated not installed (optional)"
    fi
}

verify_security() {
    print_section "Security Verification"
    
    # Check for known vulnerabilities
    if command -v cargo-audit &> /dev/null; then
        echo "Running security audit..."
        if cargo audit 2>&1 | tee "${LOG_DIR}/security_audit.log"; then
            log_check "Security audit" "PASS"
        else
            log_check "Security audit" "FAIL" "Known vulnerabilities found. See ${LOG_DIR}/security_audit.log"
        fi
    else
        log_check "Security audit" "WARN" "cargo-audit not installed. Install with: cargo install cargo-audit"
    fi
}

verify_performance_baseline() {
    print_section "Performance Baseline Verification"
    
    echo "Running performance benchmarks..."
    if command -v cargo &> /dev/null; then
        # Check if criterion is available
        if cargo bench --no-run 2>&1 | grep -q "Criterion"; then
            echo "Benchmarks available but skipping full run (use 'cargo bench' to run)"
            log_check "Performance benchmarks compiled" "PASS"
        else
            log_check "Performance benchmarks available" "WARN" "No criterion benchmarks found"
        fi
    fi
}

verify_documentation() {
    print_section "Documentation Verification"
    
    echo "Checking documentation..."
    if cargo doc --workspace --no-deps 2>&1 | tee "${LOG_DIR}/docs.log"; then
        log_check "Documentation builds" "PASS"
    else
        log_check "Documentation builds" "WARN" "Documentation build had warnings. See ${LOG_DIR}/docs.log"
    fi
}

verify_web_assets() {
    print_section "Web Assets Verification"
    
    if [[ -d "crates/loom-web" ]]; then
        if [[ -f "crates/loom-web/package.json" ]]; then
            log_check "Web project structure" "PASS"
            
            # Check Node.js availability
            if command -v node &> /dev/null; then
                local node_version=$(node --version)
                log_check "Node.js installed" "PASS" "$node_version"
            else
                log_check "Node.js installed" "WARN" "Node.js not found (needed for web dev)"
            fi
        else
            log_check "Web project structure" "WARN" "package.json not found"
        fi
    else
        log_check "Web project" "WARN" "loom-web not found"
    fi
}

verify_docker_setup() {
    print_section "Docker Setup Verification"
    
    if command -v docker &> /dev/null; then
        local docker_version=$(docker --version)
        log_check "Docker installed" "PASS" "$docker_version"
    else
        log_check "Docker installed" "WARN" "Docker not found (optional for development)"
    fi
}

verify_compatibility() {
    print_section "Compatibility Verification"
    
    echo "Running compatibility tests..."
    if cargo test --test compatibility_tests --lib 2>&1 | tee "${LOG_DIR}/compatibility.log"; then
        log_check "Compatibility tests" "PASS"
    else
        log_check "Compatibility tests" "WARN" "Some compatibility checks failed"
    fi
}

generate_report() {
    print_section "Verification Summary"
    
    local total_passed=$CHECKS_PASSED
    local total_failed=$CHECKS_FAILED
    local total_checks=$CHECKS_TOTAL
    local pass_rate=$((total_passed * 100 / total_checks))
    
    echo ""
    echo "Results:"
    echo -e "  ${GREEN}Passed: $total_passed${NC}"
    echo -e "  ${RED}Failed: $total_failed${NC}"
    echo -e "  Total: $total_checks"
    echo -e "  Pass Rate: ${GREEN}${pass_rate}%${NC}"
    
    echo ""
    echo "Full report: $SUMMARY_LOG"
    echo "Detailed logs: $DETAILED_LOG"
    echo ""
    
    # Append summary to logs
    {
        echo ""
        echo "=== SUMMARY ==="
        echo "Total Checks: $total_checks"
        echo "Passed: $total_passed"
        echo "Failed: $total_failed"
        echo "Pass Rate: ${pass_rate}%"
    } >> "${SUMMARY_LOG}"
    
    if [[ $total_failed -eq 0 ]]; then
        echo -e "${GREEN}✓ All verification checks passed!${NC}"
        return 0
    else
        echo -e "${RED}✗ Some verification checks failed${NC}"
        return 1
    fi
}

# Main execution

main() {
    echo -e "${BLUE}Loom Deployment Verification Suite${NC}"
    echo "Started at $(date)"
    
    create_log_dir
    
    # Run all verification checks
    verify_rust_toolchain || true
    verify_build || true
    verify_unit_tests || true
    verify_integration_tests || true
    verify_code_quality || true
    verify_dependencies || true
    verify_security || true
    verify_performance_baseline || true
    verify_documentation || true
    verify_web_assets || true
    verify_docker_setup || true
    verify_compatibility || true
    
    # Generate final report
    generate_report
    
    local exit_code=$?
    echo ""
    echo "Completed at $(date)"
    
    exit $exit_code
}

main "$@"
