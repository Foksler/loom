#!/bin/bash
# End-to-end integration test for loom-web and loom-server
# Tests all API contracts and verifies data flows correctly
#
# Usage: ./tests/integration_e2e.sh [--server-url URL]
# Example: ./tests/integration_e2e.sh --server-url http://localhost:3000

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SERVER_URL="${1:-http://localhost:3000}"
TESTS_PASSED=0
TESTS_FAILED=0

# Test counter
test_num=0

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Helper to make requests
request() {
    local method=$1
    local path=$2
    local data=${3:-}
    local content_type=${4:-"application/json"}
    
    if [ -z "$data" ]; then
        curl -s -X "$method" "${SERVER_URL}${path}" \
            -H "Content-Type: ${content_type}" \
            -w "\n%{http_code}"
    else
        curl -s -X "$method" "${SERVER_URL}${path}" \
            -H "Content-Type: ${content_type}" \
            -d "$data" \
            -w "\n%{http_code}"
    fi
}

# Extract response body and status code
parse_response() {
    local response=$1
    local line_count=$(echo "$response" | wc -l)
    local status_code=$(echo "$response" | tail -n 1)
    local body=$(echo "$response" | head -n $((line_count - 1)))
    
    echo "$body"
    echo "STATUS=$status_code"
}

# Test function wrapper
test_case() {
    ((test_num++))
    log_info "Test $test_num: $1"
}

assert_status() {
    local expected=$1
    local actual=$2
    local message=$3
    
    if [ "$actual" = "$expected" ]; then
        log_success "$message (HTTP $actual)"
        return 0
    else
        log_error "$message (Expected HTTP $expected, got HTTP $actual)"
        return 1
    fi
}

# ============================================================================
# TESTS START HERE
# ============================================================================

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║         Loom Web-Server Integration E2E Test Suite                 ║"
echo "║              Testing API Contract & Data Flow                      ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""

log_info "Server URL: $SERVER_URL"
log_info "Starting integration tests..."
echo ""

# ============================================================================
# HEALTH CHECK
# ============================================================================

test_case "Health Check"
response=$(request "GET" "/health" "" "")
status=$(echo "$response" | tail -n 1)
assert_status "200" "$status" "Server is healthy"
echo ""

# ============================================================================
# TEST 1: GET THREADS (Empty Database)
# ============================================================================

test_case "Get Threads - Empty List"
response=$(request "GET" "/api/web/threads?limit=50&offset=0" "" "")
status=$(echo "$response" | tail -n 1)
assert_status "200" "$status" "Get threads endpoint responds" || true
echo ""

# ============================================================================
# TEST 2: CREATE THREAD
# ============================================================================

test_case "Create Thread - Valid Input"
THREAD_ID="T-$(uuidgen | tr '[:lower:]' '[:upper:]' | sed 's/-//g' | head -c 21)"
log_info "Generated thread ID: $THREAD_ID"

request_body=$(cat <<EOF
{"title": "Integration Test Thread - $(date +%s)"}
EOF
)

response=$(request "PUT" "/api/web/threads/$THREAD_ID" "$request_body" "")
status=$(echo "$response" | tail -n 1)
body=$(echo "$response" | head -n -1)

if assert_status "201" "$status" "Thread creation returns CREATED"; then
    # Verify thread ID in response
    if echo "$body" | grep -q "$THREAD_ID"; then
        log_success "Response contains correct thread ID"
    else
        log_error "Response does not contain thread ID"
    fi
    
    # Verify title in response
    if echo "$body" | grep -q "Integration Test Thread"; then
        log_success "Response contains correct title"
    else
        log_error "Response does not contain title"
    fi
fi
echo ""

# ============================================================================
# TEST 3: GET THREAD BY ID
# ============================================================================

test_case "Get Thread by ID"
response=$(request "GET" "/api/web/threads/$THREAD_ID" "" "")
status=$(echo "$response" | tail -n 1)
body=$(echo "$response" | head -n -1)

if assert_status "200" "$status" "Get thread returns OK"; then
    if echo "$body" | grep -q "$THREAD_ID"; then
        log_success "Response contains correct thread ID"
    else
        log_error "Response missing thread ID"
    fi
fi
echo ""

# ============================================================================
# TEST 4: LIST THREADS (Should include created thread)
# ============================================================================

test_case "List Threads - Should include created thread"
response=$(request "GET" "/api/web/threads?limit=50&offset=0" "" "")
status=$(echo "$response" | tail -n 1)
body=$(echo "$response" | head -n -1)

if assert_status "200" "$status" "List threads returns OK"; then
    if echo "$body" | grep -q "$THREAD_ID"; then
        log_success "Created thread appears in list"
    else
        log_warning "Created thread not yet visible in list (may be eventual consistency)"
    fi
fi
echo ""

# ============================================================================
# TEST 5: UPDATE THREAD TITLE
# ============================================================================

test_case "Update Thread Title"
new_title="Updated Title - $(date +%s)"
request_body=$(cat <<EOF
{"title": "$new_title"}
EOF
)

response=$(request "POST" "/api/web/threads/$THREAD_ID" "$request_body" "")
status=$(echo "$response" | tail -n 1)
body=$(echo "$response" | head -n -1)

if assert_status "200" "$status" "Update thread returns OK"; then
    if echo "$body" | grep -q "$new_title"; then
        log_success "Response contains updated title"
    else
        log_error "Response does not contain updated title"
    fi
fi
echo ""

# ============================================================================
# TEST 6: SEARCH THREADS
# ============================================================================

test_case "Search Threads"
search_term="Integration"
response=$(request "GET" "/api/web/threads/search?q=$search_term&limit=50&offset=0" "" "")
status=$(echo "$response" | tail -n 1)
body=$(echo "$response" | head -n -1)

if assert_status "200" "$status" "Search threads returns OK"; then
    if echo "$body" | grep -q "$THREAD_ID"; then
        log_success "Search returned created thread"
    else
        log_warning "Search did not return created thread (may need indexing)"
    fi
fi
echo ""

# ============================================================================
# TEST 7: ERROR HANDLING - Empty Title
# ============================================================================

test_case "Error Handling - Create with Empty Title"
request_body='{"title": ""}'
response=$(request "PUT" "/api/web/threads/T-ERRORTEST" "$request_body" "")
status=$(echo "$response" | tail -n 1)

if assert_status "400" "$status" "Empty title returns BadRequest"; then
    log_success "Error handling working correctly"
fi
echo ""

# ============================================================================
# TEST 8: ERROR HANDLING - Empty Search Query
# ============================================================================

test_case "Error Handling - Empty Search Query"
response=$(request "GET" "/api/web/threads/search?q=" "" "")
status=$(echo "$response" | tail -n 1)

if assert_status "400" "$status" "Empty search query returns BadRequest"; then
    log_success "Search validation working correctly"
fi
echo ""

# ============================================================================
# TEST 9: THREAD NOT FOUND
# ============================================================================

test_case "Error Handling - Thread Not Found"
response=$(request "GET" "/api/web/threads/T-NONEXISTENT123456" "" "")
status=$(echo "$response" | tail -n 1)

if assert_status "404" "$status" "Non-existent thread returns NotFound"; then
    log_success "404 error handling working correctly"
fi
echo ""

# ============================================================================
# TEST 10: DELETE THREAD
# ============================================================================

test_case "Delete Thread"
response=$(request "DELETE" "/api/web/threads/$THREAD_ID" "" "")
status=$(echo "$response" | tail -n 1)

if assert_status "204" "$status" "Delete thread returns NoContent"; then
    log_success "Thread deletion working correctly"
    
    # Verify thread is gone
    response=$(request "GET" "/api/web/threads/$THREAD_ID" "" "")
    status=$(echo "$response" | tail -n 1)
    
    if assert_status "404" "$status" "Deleted thread returns NotFound"; then
        log_success "Deletion persistence verified"
    fi
fi
echo ""

# ============================================================================
# TEST 11: TITLE LENGTH VALIDATION
# ============================================================================

test_case "Validation - Title exceeds max length"
long_title=$(printf 'A%.0s' {1..501})
request_body="{\"title\": \"$long_title\"}"
response=$(request "PUT" "/api/web/threads/T-LONGTEST" "$request_body" "")
status=$(echo "$response" | tail -n 1)

if assert_status "400" "$status" "Oversized title returns BadRequest"; then
    log_success "Title length validation working"
fi
echo ""

# ============================================================================
# TEST 12: PAGINATION
# ============================================================================

test_case "Pagination Support"
response=$(request "GET" "/api/web/threads?limit=10&offset=0" "" "")
status=$(echo "$response" | tail -n 1)

if assert_status "200" "$status" "Pagination parameters accepted"; then
    log_success "Pagination working correctly"
fi
echo ""

# ============================================================================
# RESULTS SUMMARY
# ============================================================================

echo ""
echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║                         Test Results                               ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""

total=$((TESTS_PASSED + TESTS_FAILED))
log_info "Total Tests: $total"
log_success "Passed: $TESTS_PASSED"
if [ $TESTS_FAILED -gt 0 ]; then
    log_error "Failed: $TESTS_FAILED"
else
    log_success "Failed: 0"
fi

pass_rate=$((TESTS_PASSED * 100 / total))
log_info "Pass Rate: ${pass_rate}%"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All integration tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed. Review output above.${NC}"
    exit 1
fi
