#!/usr/bin/env bash
# build.sh - Builds release binary for Loom
# 
# Usage: ./build.sh [TARGET]
#   TARGET - optional build target (default: debug, use 'release' for optimized builds)
#
# Environment variables:
#   CARGO_PROFILE - Cargo profile (dev, release, custom)
#   RUST_LOG - Logging level (info, debug, warn, error)
#   BUILD_DIR - Output directory for artifacts

set -euo pipefail

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
TARGET="${1:-debug}"
CARGO_PROFILE="${CARGO_PROFILE:-$TARGET}"
RUST_LOG="${RUST_LOG:-info}"
BUILD_DIR="${BUILD_DIR:-$PROJECT_ROOT/target/$CARGO_PROFILE}"
TIMESTAMP=$(date -u +%Y-%m-%d-%H:%M:%S)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

log_info "=== Loom Release Build ==="
log_info "Profile: $CARGO_PROFILE"
log_info "Build directory: $BUILD_DIR"
log_info "Timestamp: $TIMESTAMP"
log_info ""

# Pre-build checks
log_info "Running pre-build checks..."
if ! command -v cargo &> /dev/null; then
    log_error "Rust toolchain not found. Install from https://rustup.rs"
    exit 1
fi

cd "$PROJECT_ROOT"

# Show Rust version
RUST_VERSION=$(rustc --version)
log_info "Using: $RUST_VERSION"

# Check workspace
log_info "Validating workspace..."
if ! cargo metadata --format-version 1 > /dev/null 2>&1; then
    log_error "Invalid Cargo workspace"
    exit 1
fi

# Run linting
log_info "Running clippy checks..."
if ! cargo clippy --workspace --all-targets -- -D warnings; then
    log_error "Clippy validation failed"
    exit 1
fi

# Format check
log_info "Checking code formatting..."
if ! cargo fmt --all -- --check; then
    log_warn "Code formatting issues detected. Run 'cargo fmt --all' to fix."
    # Don't fail on format, just warn
fi

# Build tests
log_info "Building tests..."
if ! cargo test --workspace --no-run --profile "$CARGO_PROFILE"; then
    log_error "Test build failed"
    exit 1
fi

# Run tests
log_info "Running test suite..."
if ! cargo test --workspace --profile "$CARGO_PROFILE"; then
    log_error "Test suite failed"
    exit 1
fi

# Build release binaries
log_info "Building binaries (profile: $CARGO_PROFILE)..."

# Main server binary
if ! cargo build --workspace --profile "$CARGO_PROFILE"; then
    log_error "Build failed"
    exit 1
fi

# Create build metadata
log_info "Creating build metadata..."
BUILD_INFO_FILE="$BUILD_DIR/build-info.json"
mkdir -p "$BUILD_DIR"

cat > "$BUILD_INFO_FILE" << EOF
{
  "timestamp": "$TIMESTAMP",
  "profile": "$CARGO_PROFILE",
  "rust_version": "$(rustc --version)",
  "cargo_version": "$(cargo --version)",
  "git_commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "git_branch": "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')",
  "build_host": "$(hostname)",
  "build_user": "$(whoami)"
}
EOF

log_success "Build completed successfully!"
log_info "Build artifacts:"
log_info "  Location: $BUILD_DIR"
log_info "  Metadata: $BUILD_INFO_FILE"

# List binaries
log_info "Binaries built:"
if [ -d "$BUILD_DIR" ]; then
    find "$BUILD_DIR" -maxdepth 1 -type f -executable ! -name ".*" -o -name "*.exe" | while read -r binary; do
        if [ -x "$binary" ]; then
            size=$(du -h "$binary" | cut -f1)
            log_info "  $(basename "$binary") - $size"
        fi
    done
fi

log_success "Ready for deployment!"
exit 0
