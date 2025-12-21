# Loom Makefile
#
# Development tasks for the loom workspace.

GITLEAKS_REPO := https://raw.githubusercontent.com/gitleaks/gitleaks/master
GITLEAKS_VENDOR_DIR := crates/loom-redact/third_party/gitleaks

.PHONY: all build test lint format check clean help dev update-gitleaks sbom sbom-spdx sbom-cyclonedx release docker-build docker-run test-e2e test-e2e-ui test-e2e-debug

# Help target
help:
	@echo "Loom Makefile Targets"
	@echo "===================="
	@echo ""
	@echo "Core Development:"
	@echo "  make build              - Build entire workspace"
	@echo "  make test               - Run all tests"
	@echo "  make lint               - Run clippy linter"
	@echo "  make format             - Format code with rustfmt"
	@echo "  make check-format       - Check formatting without modifying"
	@echo "  make fix                - Auto-fix clippy issues and format"
	@echo "  make check              - Full CI checks (format + lint + build + test)"
	@echo "  make dev                - Watch mode for development"
	@echo ""
	@echo "Code Quality:"
	@echo "  make sbom               - Generate SBOM (SPDX and CycloneDX)"
	@echo "  make sbom-spdx          - Generate SPDX SBOM"
	@echo "  make sbom-cyclonedx     - Generate CycloneDX SBOM"
	@echo ""
	@echo "Testing:"
	@echo "  make test-e2e           - Run E2E tests"
	@echo "  make test-e2e-ui        - Run E2E tests in UI mode"
	@echo "  make test-e2e-debug     - Run E2E tests in debug mode"
	@echo ""
	@echo "Docker:"
	@echo "  make docker-build       - Build Docker image"
	@echo "  make docker-run         - Build and run Docker container"
	@echo ""
	@echo "Release:"
	@echo "  make release            - Build release (build + test + SBOM)"
	@echo "  make update-gitleaks    - Update gitleaks rules from upstream"
	@echo "  make clean              - Clean build artifacts"

# Default target
all: format lint build test

# Build the entire workspace
build:
	cargo build --workspace

# Run all tests in workspace
test:
	cargo test --workspace

# Run clippy on entire workspace
lint:
	cargo clippy --workspace -- -D warnings

# Auto-fix clippy warnings and format code
fix:
	cargo clippy --workspace --fix --allow-dirty --allow-staged
	cargo fmt --all

# Format all code in workspace
format:
	cargo fmt --all

# Check formatting without modifying files
check-format:
	cargo fmt --all -- --check

# Development watch mode
dev:
	@echo "Starting development watch mode..."
	@echo "Watching for file changes and rebuilding..."
	@cargo watch -c -q -w src -w crates -x "build --all" -x "test --lib" 2>/dev/null || \
		(echo "Note: cargo-watch not installed. Install with: cargo install cargo-watch" && \
		 echo "For now, run 'cargo build' manually after changes")

# Run all checks (format check + lint + build + test)
check: check-format lint build test

# Update gitleaks.toml and LICENSE from upstream
update-gitleaks:
	@echo "Fetching gitleaks.toml from upstream..."
	@curl -fsSL "$(GITLEAKS_REPO)/config/gitleaks.toml" -o "$(GITLEAKS_VENDOR_DIR)/gitleaks.toml"
	@echo "Fetching LICENSE from upstream..."
	@curl -fsSL "$(GITLEAKS_REPO)/LICENSE" -o "$(GITLEAKS_VENDOR_DIR)/LICENSE"
	@echo "Updating README.md with source info..."
	@echo "# Vendored gitleaks rules" > "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "Source: https://github.com/gitleaks/gitleaks" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "Branch: master" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "Updated: $$(date -I)" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "## License" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "The gitleaks project is licensed under the MIT License. See LICENSE file." >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "## Usage" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "" >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "Run \`make update-gitleaks\` to refresh from upstream." >> "$(GITLEAKS_VENDOR_DIR)/README.md"
	@echo "Done! Updated gitleaks rules in $(GITLEAKS_VENDOR_DIR)/"

clean:
	rm -f "$(GITLEAKS_VENDOR_DIR)/gitleaks.toml"
	rm -f "$(GITLEAKS_VENDOR_DIR)/LICENSE"
	rm -f "$(GITLEAKS_VENDOR_DIR)/README.md"

# SBOM configuration
SBOM_DIR ?= target/sbom

# Generate SBOM in SPDX JSON 2.3 format (default)
sbom-spdx:
	@command -v cargo-sbom >/dev/null 2>&1 || \
		(echo "Error: cargo-sbom not found. Install with: cargo install cargo-sbom --version 0.10.0" && exit 1)
	mkdir -p "$(SBOM_DIR)"
	cargo sbom --output-format=spdx_json_2_3 > "$(SBOM_DIR)/loom.spdx.json"
	@echo "Generated SBOM: $(SBOM_DIR)/loom.spdx.json"

# Generate SBOM in CycloneDX JSON 1.4 format
sbom-cyclonedx:
	@command -v cargo-sbom >/dev/null 2>&1 || \
		(echo "Error: cargo-sbom not found. Install with: cargo install cargo-sbom --version 0.10.0" && exit 1)
	mkdir -p "$(SBOM_DIR)"
	cargo sbom --output-format=cyclone_dx_json_1_4 > "$(SBOM_DIR)/loom.cyclonedx.json"
	@echo "Generated SBOM: $(SBOM_DIR)/loom.cyclonedx.json"

# Generate both SPDX and CycloneDX SBOMs
sbom: sbom-spdx sbom-cyclonedx

# Release build: full build + test + SBOM
release: build test sbom
	@echo "Release build complete. Artifacts in target/debug/ and $(SBOM_DIR)/"

# Docker container targets (using Nix flake.nix + dockerTools)

# Build loom-server Docker image via Nix flake
# Output: OCI/Docker image tarball (./result)
docker-build:
	@echo "Building loom-server Docker image via Nix..."
	@(set -e; \
	  nix --extra-experimental-features nix-command --extra-experimental-features flakes build .#loom-server-image -L --impure; \
	  echo ""; \
	  echo "✓ Docker image built successfully"; \
	  echo "  Output: ./result (OCI/Docker image tarball)"; \
	  echo "  Image name: loom-server:latest"; \
	  echo ""; \
	  echo "To load into Docker:"; \
	  echo "  docker load < ./result"; \
	  echo ""; \
	  echo "To run:"; \
	  echo "  docker run --rm -p 8080:8080 loom-server:latest")

# Run loom-server container locally
# Builds image, loads into Docker, and runs it
docker-run: docker-build
	@echo "Loading image into Docker and running..."
	@(set -e; \
	  docker load < ./result; \
	  echo ""; \
	  echo "✓ Starting loom-server container (Ctrl+C to stop)"; \
	  echo ""; \
	  docker run --rm -p 8080:8080 loom-server:latest)

# E2E Testing targets

# Run E2E tests
test-e2e:
	@echo "Running E2E tests..."
	cd crates/loom-web && npm run test:e2e

# Run E2E tests in UI mode (interactive)
test-e2e-ui:
	@echo "Running E2E tests in UI mode..."
	cd crates/loom-web && npm run test:e2e:ui

# Run E2E tests in debug mode
test-e2e-debug:
	@echo "Running E2E tests in debug mode..."
	cd crates/loom-web && npm run test:e2e:debug
