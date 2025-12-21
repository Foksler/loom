#!/usr/bin/env bash
# deploy.sh - Deploys Loom to target environment
#
# Usage: ./deploy.sh [OPTIONS]
#   --env ENVIRONMENT      Target environment (dev, staging, prod) [default: staging]
#   --version VERSION      Version to deploy [default: latest]
#   --host HOST            Target host/server
#   --user USER            SSH user [default: deploy]
#   --method METHOD        Deployment method (docker, binary, systemd) [default: docker]
#   --dry-run              Show what would happen without making changes
#   --no-backup            Skip backup creation
#   --help                 Show this help message

set -euo pipefail

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration defaults
ENVIRONMENT="${ENVIRONMENT:-staging}"
VERSION="${VERSION:-latest}"
HOST="${HOST:-}"
USER="${USER:-deploy}"
METHOD="${METHOD:-docker}"
DRY_RUN=0
NO_BACKUP=0

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
        --version) VERSION="$2"; shift 2 ;;
        --host) HOST="$2"; shift 2 ;;
        --user) USER="$2"; shift 2 ;;
        --method) METHOD="$2"; shift 2 ;;
        --dry-run) DRY_RUN=1; shift ;;
        --no-backup) NO_BACKUP=1; shift ;;
        --help) show_help; exit 0 ;;
        *) log_error "Unknown option: $1"; show_help; exit 1 ;;
    esac
done

show_help() {
    head -n 18 "$0" | tail -n +2
}

# Validate inputs
if [ -z "$HOST" ]; then
    log_error "Target host is required (--host)"
    exit 1
fi

log_info "=== Loom Deployment ==="
log_info "Environment: $ENVIRONMENT"
log_info "Version: $VERSION"
log_info "Target: $USER@$HOST"
log_info "Method: $METHOD"
log_info "Dry-run: $([ $DRY_RUN -eq 1 ] && echo 'YES' || echo 'NO')"
log_info ""

# SSH configuration
SSH_OPTS=(-o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new)
DEPLOY_USER="$USER@$HOST"

# Pre-deployment checks
log_info "Running pre-deployment checks..."

# Check connectivity
if ! ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" "true" &>/dev/null; then
    log_error "Cannot reach $DEPLOY_USER"
    exit 1
fi
log_success "Host connectivity verified"

# Verify deployment directory
DEPLOY_DIR="/opt/loom"
if ! ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" "test -d $DEPLOY_DIR"; then
    log_warn "Deployment directory does not exist: $DEPLOY_DIR"
    if [ $DRY_RUN -eq 0 ]; then
        log_info "Creating deployment directory..."
        ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" "sudo mkdir -p $DEPLOY_DIR && sudo chown $USER:$USER $DEPLOY_DIR"
    fi
fi

# Create backup
if [ $NO_BACKUP -eq 0 ]; then
    log_info "Creating deployment backup..."
    BACKUP_NAME="loom-backup-$(date +%Y%m%d-%H%M%S)"
    
    if [ $DRY_RUN -eq 0 ]; then
        if ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" "test -d $DEPLOY_DIR/current"; then
            ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
                "cd $DEPLOY_DIR && cp -r current $BACKUP_NAME && echo 'Backup created: $BACKUP_NAME'"
            log_success "Backup created: $BACKUP_NAME"
        fi
    else
        log_info "[DRY-RUN] Would create backup: $BACKUP_NAME"
    fi
fi

# Deploy based on method
case "$METHOD" in
    docker)
        log_info "Deploying via Docker..."
        if [ $DRY_RUN -eq 0 ]; then
            deploy_docker
        else
            log_info "[DRY-RUN] Would deploy via Docker (version: $VERSION)"
        fi
        ;;
    binary)
        log_info "Deploying via binary..."
        if [ $DRY_RUN -eq 0 ]; then
            deploy_binary
        else
            log_info "[DRY-RUN] Would deploy via binary (version: $VERSION)"
        fi
        ;;
    systemd)
        log_info "Deploying via systemd..."
        if [ $DRY_RUN -eq 0 ]; then
            deploy_systemd
        else
            log_info "[DRY-RUN] Would deploy via systemd (version: $VERSION)"
        fi
        ;;
    *)
        log_error "Unknown deployment method: $METHOD"
        exit 1
        ;;
esac

log_success "Deployment completed successfully!"
exit 0

# Docker deployment
deploy_docker() {
    local image="loom-server:$VERSION"
    
    log_info "Pulling Docker image: $image"
    ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" "docker pull $image || docker load < /tmp/loom-server.tar"
    
    log_info "Stopping current container..."
    ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" "docker-compose -f $DEPLOY_DIR/docker-compose.yml down || true"
    
    log_info "Starting new container..."
    ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
        "cd $DEPLOY_DIR && docker-compose -f docker-compose.yml up -d"
    
    log_success "Docker deployment completed"
}

# Binary deployment
deploy_binary() {
    local binary_path="$PROJECT_ROOT/target/release/loom-server"
    
    if [ ! -f "$binary_path" ]; then
        log_error "Binary not found: $binary_path"
        exit 1
    fi
    
    log_info "Uploading binary..."
    scp "${SSH_OPTS[@]}" "$binary_path" "$DEPLOY_USER:$DEPLOY_DIR/loom-server-new"
    
    log_info "Installing new binary..."
    ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
        "cd $DEPLOY_DIR && mv loom-server-new loom-server && chmod +x loom-server"
    
    log_info "Restarting service..."
    ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" "systemctl restart loom || sudo systemctl restart loom"
    
    log_success "Binary deployment completed"
}

# Systemd deployment
deploy_systemd() {
    log_info "Deploying via systemd..."
    
    ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
        "sudo systemctl pull-image loom-server:$VERSION || true"
    
    ssh "${SSH_OPTS[@]}" "$DEPLOY_USER" \
        "sudo systemctl restart loom || sudo systemctl start loom"
    
    log_success "Systemd deployment completed"
}
