#!/usr/bin/env bash

###############################################################################
# Loom CI/CD Pipeline Verification Script
# 
# This script verifies all CI/CD pipeline stages are properly configured
# and working correctly.
#
# Usage: ./scripts/verify-ci-pipeline.sh
###############################################################################

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# State tracking
PASS=0
FAIL=0
WARN=0

# Functions
check_file() {
    local file="$1"
    local name="$2"
    
    if [[ -f "$file" ]]; then
        echo -e "${GREEN}✓${NC} $name exists"
        ((PASS++))
        return 0
    else
        echo -e "${RED}✗${NC} $name NOT FOUND: $file"
        ((FAIL++))
        return 1
    fi
}

check_command() {
    local cmd="$1"
    local name="${2:-$cmd}"
    
    if command -v "$cmd" &> /dev/null; then
        local version
        version=$("$cmd" --version 2>&1 | head -1 || echo "installed")
        echo -e "${GREEN}✓${NC} $name: $version"
        ((PASS++))
        return 0
    else
        echo -e "${RED}✗${NC} $name NOT FOUND - Install with: cargo install $cmd"
        ((FAIL++))
        return 1
    fi
}

check_workflow() {
    local file="$1"
    local name="$2"
    
    if [[ ! -f "$file" ]]; then
        echo -e "${RED}✗${NC} Workflow not found: $file"
        ((FAIL++))
        return 1
    fi
    
    # Check for key patterns
    if grep -q "name:" "$file" && grep -q "jobs:" "$file"; then
        echo -e "${GREEN}✓${NC} Workflow $name is valid"
        ((PASS++))
        return 0
    else
        echo -e "${RED}✗${NC} Workflow $name is malformed"
        ((FAIL++))
        return 1
    fi
}

header() {
    echo ""
    echo -e "${BLUE}===================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}===================================================${NC}"
}

section() {
    echo ""
    echo -e "${YELLOW}→ $1${NC}"
}

result() {
    echo ""
    echo -e "${BLUE}===================================================${NC}"
    echo -e "${BLUE}Pipeline Verification Results${NC}"
    echo -e "${BLUE}===================================================${NC}"
    echo -e "${GREEN}Passed:${NC} $PASS"
    echo -e "${RED}Failed:${NC} $FAIL"
    echo -e "${YELLOW}Warnings:${NC} $WARN"
    echo ""
    
    if [[ $FAIL -eq 0 ]]; then
        echo -e "${GREEN}✓ All checks passed!${NC}"
        return 0
    else
        echo -e "${RED}✗ Some checks failed. See above for details.${NC}"
        return 1
    fi
}

###############################################################################
# Main Verification
###############################################################################

header "Loom CI/CD Pipeline Verification"

# 1. Check repository setup
section "1. Repository Configuration"
check_file ".git/config" ".git configuration"
check_file ".github/workflows" "GitHub Actions workflows directory"

# 2. Check GitHub Actions workflows
section "2. GitHub Actions Workflows"
check_workflow ".github/workflows/ci.yml" "Main CI workflow"
check_workflow ".github/workflows/build.yml" "Build workflow"
check_workflow ".github/workflows/e2e-tests.yml" "E2E tests workflow"

# 3. Check local build tools
section "3. Local Build Tools"
check_command "rustc" "Rust compiler"
check_command "cargo" "Cargo package manager"

# Optional tools
section "4. Optional Development Tools"
if command -v cargo-fmt &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-fmt (code formatter)"
    ((PASS++))
else
    echo -e "${YELLOW}!${NC} cargo-fmt not installed - Install with: rustup component add rustfmt"
    ((WARN++))
fi

if command -v cargo-clippy &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-clippy (linter)"
    ((PASS++))
else
    echo -e "${YELLOW}!${NC} cargo-clippy not installed - Install with: rustup component add clippy"
    ((WARN++))
fi

if command -v cargo-deny &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-deny (dependency checker)"
    ((PASS++))
else
    echo -e "${YELLOW}!${NC} cargo-deny not installed - Install with: cargo install cargo-deny"
    ((WARN++))
fi

# 4. Check Makefile
section "5. Build Configuration"
check_file "Makefile" "Build Makefile"

# Verify Makefile targets
if [[ -f "Makefile" ]]; then
    echo -e "${YELLOW}→ Makefile targets:${NC}"
    for target in build test lint format check check-format fix dev; do
        if grep -q "^$target:" Makefile; then
            echo -e "  ${GREEN}✓${NC} make $target"
            ((PASS++))
        else
            echo -e "  ${RED}✗${NC} make $target not found"
            ((FAIL++))
        fi
    done
fi

# 5. Check source structure
section "6. Source Code Structure"
check_file "Cargo.toml" "Workspace Cargo.toml"
check_file "crates" "Crates directory"

if [[ -d "crates" ]]; then
    local crate_count
    crate_count=$(find crates -maxdepth 1 -name "Cargo.toml" | wc -l)
    echo -e "${GREEN}✓${NC} Found $crate_count crates"
    ((PASS++))
fi

# 6. Check documentation
section "7. Documentation"
check_file "README.md" "README.md"
check_file "CONTRIBUTING.md" "CONTRIBUTING.md"
check_file "CI_CD_PIPELINE.md" "CI/CD documentation"
check_file "DEPLOYMENT_GUIDE.md" "Deployment guide"

# 7. Verify Rust toolchain
section "8. Rust Toolchain"
echo -e "${YELLOW}→ Rust versions:${NC}"

rustc_version=$(rustc --version)
echo -e "  ${GREEN}✓${NC} $rustc_version"
((PASS++))

cargo_version=$(cargo --version)
echo -e "  ${GREEN}✓${NC} $cargo_version"
((PASS++))

# Check for stable toolchain
if rustup toolchain list | grep -q stable; then
    echo -e "  ${GREEN}✓${NC} Stable toolchain installed"
    ((PASS++))
else
    echo -e "  ${RED}✗${NC} Stable toolchain missing - Install with: rustup toolchain install stable"
    ((FAIL++))
fi

# Check for nightly (optional but recommended)
if rustup toolchain list | grep -q nightly; then
    echo -e "  ${GREEN}✓${NC} Nightly toolchain installed"
    ((PASS++))
else
    echo -e "  ${YELLOW}!${NC} Nightly toolchain not installed (optional)"
    ((WARN++))
fi

# 8. Test local build
section "9. Local Build Test"
echo -e "${YELLOW}→ Testing cargo check...${NC}"
if cargo check --workspace --quiet 2>/dev/null; then
    echo -e "  ${GREEN}✓${NC} Workspace compiles successfully"
    ((PASS++))
else
    echo -e "  ${RED}✗${NC} Workspace compilation failed"
    echo -e "    ${YELLOW}Run:${NC} cargo check --workspace"
    ((FAIL++))
fi

# 9. Check for tests
section "10. Test Suite"
local test_count
test_count=$(find . -name "*.rs" -type f -exec grep -l "#\[test\]" {} \; | wc -l)
if [[ $test_count -gt 0 ]]; then
    echo -e "${GREEN}✓${NC} Found test files: $test_count"
    ((PASS++))
else
    echo -e "${YELLOW}!${NC} No test files found"
    ((WARN++))
fi

# Quick test run
echo -e "${YELLOW}→ Testing cargo test...${NC}"
if cargo test --lib --quiet 2>/dev/null; then
    echo -e "  ${GREEN}✓${NC} Tests pass"
    ((PASS++))
else
    echo -e "  ${RED}✗${NC} Tests fail"
    echo -e "    ${YELLOW}Run:${NC} cargo test"
    ((FAIL++))
fi

# 10. Check Node.js (for web UI)
section "11. Web UI Tools (Optional)"
if command -v node &> /dev/null; then
    node_version=$(node --version)
    echo -e "${GREEN}✓${NC} Node.js: $node_version"
    ((PASS++))
    
    if command -v npm &> /dev/null; then
        npm_version=$(npm --version)
        echo -e "${GREEN}✓${NC} npm: $npm_version"
        ((PASS++))
    fi
else
    echo -e "${YELLOW}!${NC} Node.js not installed (optional for web UI development)"
    ((WARN++))
fi

# 11. Check Docker (optional)
section "12. Docker (Optional)"
if command -v docker &> /dev/null; then
    docker_version=$(docker --version)
    echo -e "${GREEN}✓${NC} $docker_version"
    ((PASS++))
else
    echo -e "${YELLOW}!${NC} Docker not installed (optional for containerization)"
    ((WARN++))
fi

# 12. Print summary and recommendations
section "13. Recommendations"
if [[ $FAIL -eq 0 ]]; then
    echo -e "${GREEN}✓ Your environment is properly configured!${NC}"
    echo ""
    echo "You can now:"
    echo "  • Run: make check         (full CI checks)"
    echo "  • Run: make build         (build workspace)"
    echo "  • Run: make test          (run tests)"
    echo "  • Run: make lint          (run clippy)"
    echo "  • Run: make dev           (development mode)"
else
    echo -e "${RED}Please fix the issues above before proceeding.${NC}"
fi

if [[ $WARN -gt 0 ]]; then
    echo ""
    echo -e "${YELLOW}Note: $WARN optional dependencies missing.${NC}"
    echo "Consider installing them for a complete development experience."
fi

echo ""
echo "For detailed information, see:"
echo "  • CI_CD_PIPELINE.md"
echo "  • DEV_ENVIRONMENT_SETUP.md"
echo "  • CONTRIBUTING.md"

# Final result
result
exit $?
