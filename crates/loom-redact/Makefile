# loom-redact Makefile
#
# This Makefile automates updating the vendored gitleaks rules.

GITLEAKS_REPO := https://raw.githubusercontent.com/gitleaks/gitleaks/master
THIRD_PARTY_DIR := third_party/gitleaks

.PHONY: update-gitleaks clean

# Update gitleaks.toml and LICENSE from upstream
update-gitleaks:
	@echo "Fetching gitleaks.toml from upstream..."
	@curl -fsSL "$(GITLEAKS_REPO)/config/gitleaks.toml" -o "$(THIRD_PARTY_DIR)/gitleaks.toml"
	@echo "Fetching LICENSE from upstream..."
	@curl -fsSL "$(GITLEAKS_REPO)/LICENSE" -o "$(THIRD_PARTY_DIR)/LICENSE"
	@echo "Updating README.md with source info..."
	@echo "# Vendored gitleaks rules" > "$(THIRD_PARTY_DIR)/README.md"
	@echo "" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "Source: https://github.com/gitleaks/gitleaks" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "Branch: master" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "Updated: $$(date -I)" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "## License" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "The gitleaks project is licensed under the MIT License. See LICENSE file." >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "## Usage" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "" >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "Run \`make update-gitleaks\` to refresh from upstream." >> "$(THIRD_PARTY_DIR)/README.md"
	@echo "Done! Updated gitleaks rules in $(THIRD_PARTY_DIR)/"

clean:
	rm -f "$(THIRD_PARTY_DIR)/gitleaks.toml"
	rm -f "$(THIRD_PARTY_DIR)/LICENSE"
	rm -f "$(THIRD_PARTY_DIR)/README.md"
