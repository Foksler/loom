#!/usr/bin/env bash
# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Format entire project with dprint and Terraform

# shellcheck source=../lib.sh
source "$(dirname "$0")/../lib.sh"

main() {
    script_header "Formatting entire project with dprint and Terraform"
    
    validate_devenv
    require_command dprint
    
    step_start "Formatting entire project with dprint"
    run_cmd_verbose "dprint fmt"
    step_complete "dprint formatting completed"
    
    step_start "Formatting Terraform files"
    if [[ -d "infra/dns" ]]; then
        require_command tofu
        log_info "Found infra/dns directory, formatting Terraform files"
        cd infra/dns || exit 1
        run_cmd_verbose "tofu fmt"
        cd ../.. || exit 1
        step_complete "Terraform formatting completed"
    else
        log_debug "No infra/dns directory found, skipping Terraform formatting"
    fi
    
    script_footer
}

main "$@"