#!/usr/bin/env bash
# rollback.sh - Rolls back to previous deployment on failure
#
# Usage: ./rollback.sh [OPTIONS]
#   --env ENVIRONMENT      Target environment (dev, staging, prod)
#   --host HOST            Target host/server
#   --user USER            SSH user [default: deploy]
#   --backup-name NAME     Specific backup to restore
#   --method METHOD        Deployment method (docker, binary, systemd)
#   --verify               Run health checks after rollback
#   --dry-run              Show what would happen without making changes

set -euo pipefail

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Configuration
ENVIRONMENT="${ENVIRONMENT:-staging}"
HOST="${HOST:-}"
USER="${USER:-deploy}"
BACKUP_NAME="${BACKUP_NAME:-}"
METHOD="${METHOD:-docker}"
VERIFY=0
DRY_RUN=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --env) ENVIRONMENT="$2"; shift 2 ;;
        --host) HOST="$2"; shift 2 ;;
        --user) USER="$2"; shift 2 ;;
        --backup-name) BACKUP_NAME="$2"; shift 2 ;;
        --method) METHOD="$2"; shift 2 ;;
        --verify) VERIFY=1; shift ;;
        --dry-run) DRY_RUN=1; shift ;;
        *) log_error "Unknown option: $1"; exit 1 ;;
    esac
done

# Validate inputs
if [ -z "$HOST" ]; then
    log_error "Target host is required (--host)"
    exit 1
fi

log_info "=== Loom Rollback ==="
log_info "Environment: $ENVIRONMENT"
log_info "Target: $USER@$HOST"
log_info "Method: $METHOD"
log_info "Dry-run: $([ $DRY_RUN -eq 1 ] && echo 'YES' || echo 'NO')"
log_info ""

# SSH configuration
SSH_OPTS=(-o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new)
DEPLOY_USER="$USER@$HOST"
DEPLOY_DIR="/opt/loom"

# Check connectivity
log_info "Verifying host connectivity..."
if ! ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" "true" &>/dev/null; then
    log_error "Cannot reach $DEPLOY_USER"
    exit 1
fi
log_success "Host connectivity verified"

# List available backups
log_info "Available backups:"
backups=$(ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
    "ls -dt $DEPLOY_DIR/loom-backup-* 2>/dev/null | head -10" || echo "")

if [ -z "$backups" ]; then
    log_error "No backups found in $DEPLOY_DIR"
    exit 1
fi

echo "$backups" | while read -r backup; do
    echo "  $(basename "$backup")"
done

log_info ""

# Determine backup to restore
if [ -z "$BACKUP_NAME" ]; then
    BACKUP_NAME=$(ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
        "ls -dt $DEPLOY_DIR/loom-backup-* 2>/dev/null | head -1 | xargs basename")
fi

if [ -z "$BACKUP_NAME" ]; then
    log_error "No backup available to restore"
    exit 1
fi

log_info "Restoring backup: $BACKUP_NAME"
log_warn "This will restore the previous deployment!"
log_info ""

# Confirmation prompt (unless dry-run)
if [ $DRY_RUN -eq 0 ]; then
    read -p "Are you sure you want to rollback? (yes/no): " -r confirm
    if [ "$confirm" != "yes" ]; then
        log_warn "Rollback cancelled"
        exit 0
    fi
fi

log_info ""
log_info "Starting rollback..."

# Perform rollback
case "$METHOD" in
    docker)
        rollback_docker "$BACKUP_NAME"
        ;;
    binary)
        rollback_binary "$BACKUP_NAME"
        ;;
    systemd)
        rollback_systemd "$BACKUP_NAME"
        ;;
    *)
        log_error "Unknown deployment method: $METHOD"
        exit 1
        ;;
esac

log_success "Rollback completed!"

# Verify deployment if requested
if [ $VERIFY -eq 1 ]; then
    log_info ""
    log_info "Running health checks..."
    sleep 5 # Give service time to start
    
    if "$SCRIPT_DIR/health-check.sh" --url "http://$HOST:8080"; then
        log_success "Service is healthy after rollback!"
    else
        log_error "Service health check failed after rollback"
        exit 1
    fi
fi

exit 0

# Docker rollback
rollback_docker() {
    local backup="$1"
    
    log_info "Stopping current container..."
    if [ $DRY_RUN -eq 0 ]; then
        ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
            "cd $DEPLOY_DIR && docker-compose down || true"
    else
        log_info "[DRY-RUN] Would stop Docker containers"
    fi
    
    log_info "Restoring backup: $backup"
    if [ $DRY_RUN -eq 0 ]; then
        ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
            "cd $DEPLOY_DIR && rm -rf current && cp -r $backup current"
    else
        log_info "[DRY-RUN] Would restore $backup to current"
    fi
    
    log_info "Starting containers from backup..."
    if [ $DRY_RUN -eq 0 ]; then
        ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
            "cd $DEPLOY_DIR/current && docker-compose up -d"
    else
        log_info "[DRY-RUN] Would start Docker containers"
    fi
}

# Binary rollback
rollback_binary() {
    local backup="$1"
    
    log_info "Stopping service..."
    if [ $DRY_RUN -eq 0 ]; then
        ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
            "systemctl stop loom || sudo systemctl stop loom || true"
    else
        log_info "[DRY-RUN] Would stop loom service"
    fi
    
    log_info "Restoring binary from backup: $backup"
    if [ $DRY_RUN -eq 0 ]; then
        ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
            "cd $DEPLOY_DIR && cp $backup/loom-server . && chmod +x loom-server"
    else
        log_info "[DRY-RUN] Would restore binary from $backup"
    fi
    
    log_info "Restoring configuration from backup..."
    if [ $DRY_RUN -eq 0 ]; then
        ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
            "cd $DEPLOY_DIR && cp -r $backup/config/* config/ 2>/dev/null || true"
    else
        log_info "[DRY-RUN] Would restore configuration"
    fi
    
    log_info "Starting service..."
    if [ $DRY_RUN -eq 0 ]; then
        ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
            "systemctl start loom || sudo systemctl start loom"
    else
        log_info "[DRY-RUN] Would start loom service"
    fi
}

# Systemd rollback
rollback_systemd() {
    local backup="$1"
    
    log_info "Restoring from backup: $backup"
    if [ $DRY_RUN -eq 0 ]; then
        ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
            "sudo systemctl stop loom || true && \
             sudo cp -r $backup/* /opt/loom/ && \
             sudo systemctl start loom"
    else
        log_info "[DRY-RUN] Would restore systemd service from $backup"
    fi
}
