# Loom Makefile
#
# Development tasks for the loom workspace.

GITLEAKS_REPO := https://raw.githubusercontent.com/gitleaks/gitleaks/master
GITLEAKS_VENDOR_DIR := crates/loom-redact/third_party/gitleaks

.PHONY: all build test lint format check clean update-gitleaks sbom sbom-spdx sbom-cyclonedx release docker-build docker-run

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

# Docker container targets (using devenv + Nix)

# Build loom-server Docker container via devenv/Nix
# Output: OCI/Docker image (devenv handles image format)
docker-build:
	@echo "Building loom-server container via devenv/Nix..."
	@(set -e; \
	  devenv container build loom-server; \
	  echo ""; \
	  echo "✓ Docker container built successfully"; \
	  echo "  Image name: loom-server:latest"; \
	  echo "  To load into Docker: docker load < result"; \
	  echo "  To run: docker run --rm -p 8080:8080 loom-server:latest")

# Run loom-server container locally
# Requires docker-build target to have run first
docker-run: docker-build
	@echo "Loading container into Docker and running..."
	@(set -e; \
	  docker load < result; \
	  echo ""; \
	  echo "Starting loom-server container (Ctrl+C to stop)..."; \
	  docker run --rm -p 8080:8080 loom-server:latest)
