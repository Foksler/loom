#!/usr/bin/env bash
# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Check format for entire project with dprint and Terraform

# shellcheck source=../lib.sh
source "$(dirname "$0")/../lib.sh"

main() {
    script_header "Checking format for entire project with dprint and Terraform"
    
    validate_devenv
    require_command dprint
    
    step_start "Checking format for entire project with dprint"
    run_cmd_verbose "dprint check"
    step_complete "dprint format check passed"
    
    step_start "Checking Terraform file formatting"
    if [[ -d "infra/dns" ]]; then
        require_command tofu
        log_info "Found infra/dns directory, checking Terraform formatting"
        cd infra/dns || exit 1
        run_cmd_verbose "tofu fmt -check"
        cd ../.. || exit 1
        step_complete "Terraform format check passed"
    else
        log_debug "No infra/dns directory found, skipping Terraform format check"
    fi
    
    script_footer
}

main "$@"