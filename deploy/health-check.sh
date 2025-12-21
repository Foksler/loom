#!/usr/bin/env bash
# health-check.sh - Verifies deployment health and service status
#
# Usage: ./health-check.sh [OPTIONS]
#   --url URL              Service URL to check [default: http://localhost:8080]
#   --timeout SECONDS      Health check timeout [default: 30]
#   --retries N            Number of retry attempts [default: 5]
#   --interval SECONDS     Wait between retries [default: 5]
#   --verbose              Enable verbose output

set -euo pipefail

# Configuration
SERVICE_URL="${SERVICE_URL:-http://localhost:8080}"
TIMEOUT="${TIMEOUT:-30}"
RETRIES="${RETRIES:-5}"
INTERVAL="${INTERVAL:-5}"
VERBOSE="${VERBOSE:-0}"

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
log_verbose() { [ $VERBOSE -eq 1 ] && echo -e "${BLUE}[DEBUG]${NC} $*" || true; }

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --url) SERVICE_URL="$2"; shift 2 ;;
        --timeout) TIMEOUT="$2"; shift 2 ;;
        --retries) RETRIES="$2"; shift 2 ;;
        --interval) INTERVAL="$2"; shift 2 ;;
        --verbose) VERBOSE=1; shift ;;
        *) log_error "Unknown option: $1"; exit 1 ;;
    esac
done

log_info "=== Loom Health Check ==="
log_info "Service URL: $SERVICE_URL"
log_info "Timeout: ${TIMEOUT}s per request"
log_info "Retries: $RETRIES"
log_info "Interval: ${INTERVAL}s"
log_info ""

# Health check metrics
TOTAL_CHECKS=0
FAILED_CHECKS=0
RESPONSE_TIMES=()

# Check if curl is available
if ! command -v curl &> /dev/null; then
    log_error "curl is required but not installed"
    exit 1
fi

# Function to check service health
check_health() {
    local attempt=$1
    local url="$SERVICE_URL/health"
    
    log_verbose "Health check attempt $attempt/$RETRIES to $url"
    
    local start_time=$(date +%s%N)
    local http_code
    local response
    
    # Make health check request
    response=$(curl -s -w "\n%{http_code}" \
        --max-time "$TIMEOUT" \
        --connect-timeout 5 \
        -H "User-Agent: loom-health-check/1.0" \
        "$url" 2>/dev/null || echo -e "\nERROR")
    
    local end_time=$(date +%s%N)
    local response_time=$(( (end_time - start_time) / 1000000 )) # Convert to ms
    
    http_code=$(echo "$response" | tail -n1)
    local body=$(echo "$response" | head -n-1)
    
    RESPONSE_TIMES+=("$response_time")
    ((TOTAL_CHECKS++))
    
    if [ "$http_code" = "200" ]; then
        log_success "Health check passed (HTTP 200) - ${response_time}ms"
        log_verbose "Response: $body"
        return 0
    else
        log_warn "Health check failed (HTTP $http_code) - attempt $attempt/$RETRIES"
        ((FAILED_CHECKS++))
        return 1
    fi
}

# Function to check system metrics
check_metrics() {
    log_info "Checking service metrics..."
    
    local metrics_url="$SERVICE_URL/metrics"
    local metrics
    
    metrics=$(curl -s --max-time "$TIMEOUT" \
        -H "User-Agent: loom-health-check/1.0" \
        "$metrics_url" 2>/dev/null || echo "ERROR")
    
    if [[ "$metrics" == "ERROR" ]]; then
        log_warn "Could not retrieve metrics from $metrics_url"
        return 1
    fi
    
    log_success "Metrics endpoint responding"
    
    # Parse metrics (if available)
    if command -v jq &> /dev/null && [[ "$metrics" == "{"* ]]; then
        log_verbose "Metrics (JSON):"
        log_verbose "$(echo "$metrics" | jq . 2>/dev/null || echo "$metrics")"
    fi
    
    return 0
}

# Function to check system resources
check_resources() {
    log_info "Checking system resources..."
    
    # CPU usage
    if command -v top &> /dev/null; then
        local cpu_usage=$(top -bn1 | grep "Cpu(s)" | cut -d',' -f1 | awk '{print $2}' || echo "N/A")
        log_info "CPU usage: $cpu_usage"
    fi
    
    # Memory usage
    if command -v free &> /dev/null; then
        local mem_usage=$(free | grep Mem | awk '{printf("%.1f%%", $3/$2 * 100.0)}')
        log_info "Memory usage: $mem_usage"
    fi
    
    # Disk usage
    if command -v df &> /dev/null; then
        local disk_usage=$(df / | awk 'NR==2 {print $5}')
        log_info "Disk usage: $disk_usage"
    fi
}

# Perform health checks with retries
attempt=0
while [ $attempt -lt $RETRIES ]; do
    ((attempt++))
    
    if check_health $attempt; then
        log_success "Service is healthy!"
        break
    fi
    
    if [ $attempt -lt $RETRIES ]; then
        log_info "Retrying in ${INTERVAL}s..."
        sleep "$INTERVAL"
    fi
done

log_info ""
log_info "=== Health Check Summary ==="
log_info "Total checks: $TOTAL_CHECKS"
log_info "Failed checks: $FAILED_CHECKS"

if [ ${#RESPONSE_TIMES[@]} -gt 0 ]; then
    local min_time=${RESPONSE_TIMES[0]}
    local max_time=${RESPONSE_TIMES[0]}
    local total_time=0
    
    for time in "${RESPONSE_TIMES[@]}"; do
        total_time=$((total_time + time))
        [ $time -lt $min_time ] && min_time=$time
        [ $time -gt $max_time ] && max_time=$time
    done
    
    local avg_time=$((total_time / ${#RESPONSE_TIMES[@]}))
    
    log_info "Response times:"
    log_info "  Min: ${min_time}ms"
    log_info "  Max: ${max_time}ms"
    log_info "  Avg: ${avg_time}ms"
fi

# Check metrics if service is healthy
if [ $FAILED_CHECKS -eq 0 ]; then
    log_info ""
    check_metrics || true
    log_info ""
    check_resources || true
fi

log_info ""

# Exit status
if [ $FAILED_CHECKS -eq 0 ]; then
    log_success "All health checks passed!"
    exit 0
else
    log_error "Health checks failed ($FAILED_CHECKS/$TOTAL_CHECKS)"
    exit 1
fi
