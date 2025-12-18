# Loom Makefile
#
# Development tasks for the loom workspace.

GITLEAKS_REPO := https://raw.githubusercontent.com/gitleaks/gitleaks/master
GITLEAKS_VENDOR_DIR := crates/loom-redact/third_party/gitleaks

.PHONY: all build test lint format check clean update-gitleaks

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
