#!/usr/bin/env bash
set -euo pipefail

# Build loom CLI for multiple platforms
# Usage: ./scripts/build-cli-binaries.sh [platform...]
# If no platforms specified, builds all

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${LOOM_SERVER_BIN_DIR:-${ROOT}/bin}"
mkdir -p "${BIN_DIR}"

# Platform to Rust target mapping
declare -A TARGETS=(
    ["linux-x86_64"]="x86_64-unknown-linux-gnu"
    ["linux-aarch64"]="aarch64-unknown-linux-gnu"
    ["macos-x86_64"]="x86_64-apple-darwin"
    ["macos-aarch64"]="aarch64-apple-darwin"
    ["windows-x86_64"]="x86_64-pc-windows-msvc"
)

build_one() {
    local platform="$1"
    local target="${TARGETS[$platform]}"
    local exe_suffix=""
    
    if [[ -z "${target}" ]]; then
        echo "Unknown platform: ${platform}"
        exit 1
    fi
    
    [[ "${platform}" == windows-* ]] && exe_suffix=".exe"
    
    echo "Building loom for ${platform} (${target})..."
    
    rustup target add "${target}" 2>/dev/null || true
    cargo build --bin loom --release --target "${target}"
    
    cp "${ROOT}/target/${target}/release/loom${exe_suffix}" "${BIN_DIR}/${platform}"
    chmod +x "${BIN_DIR}/${platform}"
    
    echo "Built: ${BIN_DIR}/${platform}"
}

# If specific platforms provided, build those; otherwise build all
if [[ $# -gt 0 ]]; then
    for platform in "$@"; do
        build_one "$platform"
    done
else
    for platform in "${!TARGETS[@]}"; do
        build_one "$platform"
    done
fi

echo "All binaries built in: ${BIN_DIR}"
ls -la "${BIN_DIR}"
