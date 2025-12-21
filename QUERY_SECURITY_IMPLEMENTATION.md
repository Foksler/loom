# Query Security Implementation

## Overview

The `query_security.rs` module provides comprehensive security hardening and validation for server queries in Loom. It implements multiple layers of protection against common security vulnerabilities and abuse patterns.

## Module Location

[crates/loom-server/src/query_security.rs](file:///home/ghuntley/loom/crates/loom-server/src/query_security.rs)

## Components

### 1. SecurityError Enum

Custom error type for all security validation failures with structured error messages:

- **QueryTooLarge**: Enforces maximum query size (default: 10 KB)
- **QueryLimitExceeded**: Tracks per-session query quota
- **InvalidTimeout**: Validates timeout bounds (1-300 seconds)
- **PathEscapeAttempt**: Detects directory traversal attacks
- **BlockedPath**: Enforces restricted path patterns
- **RateLimitExceeded**: Rate limiting violations
- **InvalidJson**: JSON parsing errors
- **ResultTooLarge**: Response size limits (default: 100 MB)
- **MalformedResult**: Invalid result structures
- **InvalidPath**: Path validation failures

All errors implement structured logging via `tracing` crate.

### 2. QueryValidator Struct

Core validation component for incoming queries.

#### Configuration

```rust
pub struct QueryValidator {
    max_query_size_bytes: usize,      // Default: 10 KB
    max_queries_per_session: u32,     // Default: 10
    timeout_range: (u32, u32),        // Default: (1, 300) seconds
    blocked_paths: Vec<String>,       // Default: /etc, /root, /sys
}
```

#### Key Methods

- **`validate_size()`** - Enforces query size limits
  - Default: 10 KB maximum
  - Prevents DoS from oversized payloads
  
- **`validate_timeout()`** - Enforces timeout bounds
  - Range: 1-300 seconds
  - Prevents infinite timeouts and invalid values
  
- **`validate_path()`** - Multi-layer path security
  - Rejects absolute paths
  - Blocks configured dangerous paths
  - Detects `../` traversal attempts
  
- **`validate_json()`** - JSON payload validation
  - Parses and validates JSON structure
  - Returns parsed Value on success

#### Example Usage

```rust
let validator = QueryValidator::new();

// Validate query size
validator.validate_size(&query_string)?;

// Validate timeout
validator.validate_timeout(timeout_seconds)?;

// Validate path
validator.validate_path("data/analysis.txt")?;

// Validate JSON payload
let payload = validator.validate_json(json_str)?;
```

### 3. PathSanitizer Struct

Dedicated path validation and normalization component.

#### Features

- **Workspace containment**: Ensures paths stay within workspace root
- **Canonical resolution**: Uses system path canonicalization
- **Null byte detection**: Rejects paths with embedded null bytes
- **Absolute path rejection**: Only allows relative paths
- **Escape detection**: Prevents `../` based directory traversal

#### Example Usage

```rust
let sanitizer = PathSanitizer::new("/workspace/root");

// Returns canonical path if valid
let safe_path = sanitizer.sanitize("data/queries/analysis.json")?;

// Rejects dangerous patterns
assert!(sanitizer.sanitize("../../../etc/passwd").is_err());
assert!(sanitizer.sanitize("/etc/passwd").is_err());
```

### 4. RateLimiter Struct

Token bucket algorithm for per-session query rate limiting.

#### Configuration

- **Default rate**: 100 queries per minute (≈1.67 tokens/second)
- **Max tokens**: Configurable bucket size
- **Per-session**: Maintains separate buckets for each session ID

#### Key Methods

- **`check_rate_limit(session_id)`** - Async rate limit check
  - Returns `Ok(())` if token available
  - Deducts token on success
  - Returns `RateLimitExceeded` on limit
  
- **`get_remaining_tokens(session_id)`** - Query remaining quota
  
- **`reset(session_id)`** - Clear session's rate limit state

#### Algorithm Details

- Tokens refill based on elapsed time
- Refill rate: `elapsed_seconds * refill_rate`
- Bucket cap: never exceeds `max_tokens`
- Thread-safe: uses `Arc<RwLock<HashMap>>`

#### Example Usage

```rust
let limiter = RateLimiter::new(); // 100 queries/minute

// Check before each query
match limiter.check_rate_limit("session-123").await {
    Ok(()) => {
        // Process query
    }
    Err(SecurityError::RateLimitExceeded) => {
        // Return 429 Too Many Requests
    }
}

// Optional: Query remaining tokens
let remaining = limiter.get_remaining_tokens("session-123").await;
info!("Remaining queries: {}", remaining);
```

### 5. ResultValidator Struct

Response validation and error message sanitization.

#### Configuration

- **Default max size**: 100 MB
- **Configurable**: Use `with_max_size()` for custom limits

#### Key Methods

- **`validate(result: &Value)`** - Validates response structure
  - Checks JSON validity
  - Enforces size limits
  - Verifies object/array structure
  
- **`sanitize_error(error_msg: &str)`** - Removes sensitive information
  - Filters out file paths (`/home/user/...`)
  - Removes stack traces
  - Filters "at" and "stack" keywords
  - Returns generic message if fully filtered

#### Example Usage

```rust
let validator = ResultValidator::new();

// Validate response
let result = json!({"data": query_result});
validator.validate(&result)?;

// Sanitize error messages before sending to client
let safe_error = validator.sanitize_error(
    "thread panicked at 'error' src/main.rs:42\nstack backtrace:..."
);
// Returns: "thread panicked at 'error'\n"
```

## Test Coverage

The module includes comprehensive property-based and unit tests:

### Unit Tests

| Test | Purpose |
|------|---------|
| `test_query_size_validation_passes_within_limit` | Verify small queries pass |
| `test_query_size_validation_fails_exceeds_limit` | Verify oversized queries rejected |
| `test_timeout_validation_passes_within_range` | Verify valid timeouts accepted |
| `test_timeout_validation_fails_outside_range` | Verify invalid timeouts rejected |
| `test_path_escape_rejected` | Verify `../` patterns blocked |
| `test_blocked_paths_rejected` | Verify dangerous paths blocked |
| `test_valid_paths_accepted` | Verify legitimate paths allowed |
| `test_json_validation_valid` | Verify valid JSON parsed |
| `test_json_validation_invalid` | Verify invalid JSON rejected |
| `test_path_sanitizer_absolute_path_rejected` | Verify absolute paths rejected |
| `test_path_sanitizer_null_bytes_rejected` | Verify null bytes detected |
| `test_rate_limiter_allows_within_rate` | Verify rate limiting works |
| `test_rate_limiter_blocks_exceeding_rate` | Verify limits enforced |
| `test_rate_limiter_reset` | Verify rate limit reset |
| `test_result_validator_accepts_valid_json` | Verify valid results accepted |
| `test_result_validator_rejects_oversized_result` | Verify size limits enforced |
| `test_result_sanitizer_removes_file_paths` | Verify path sanitization |
| `test_result_sanitizer_removes_stack_traces` | Verify trace removal |

### Property-Based Tests

Using `proptest` for property testing (ensures invariants hold):

1. **`prop_small_queries_always_pass`**
   - **Property**: Any query < 5 KB should pass validation
   - **Purpose**: Ensure legitimate queries are never blocked
   - **Range**: 0-5000 bytes
   
2. **`prop_paths_with_traversal_always_rejected`**
   - **Property**: Any path containing `..` should fail validation
   - **Purpose**: Prevent directory traversal attacks reliably
   - **Range**: `..` + alphanumeric suffix
   
3. **`prop_invalid_timeouts_always_rejected`**
   - **Property**: Timeouts outside 1-300s range always rejected
   - **Purpose**: Enforce timeout bounds consistently
   - **Range**: Very large timeout values (near u32::MAX)

## Security Hardening Features

### 1. Defense in Depth

- Multiple validation layers (size, timeout, path, JSON)
- Each component is independent and reusable
- Fail-closed design (reject unless explicitly allowed)

### 2. Path Security

- **Workspace containment**: Paths verified within workspace root
- **Canonical resolution**: Resolves all symlinks and `..` components
- **Null byte injection**: Detects null byte attacks
- **Absolute path blocking**: Rejects `/etc/passwd` style attacks

### 3. Rate Limiting

- **Token bucket algorithm**: Proven, fair rate limiting
- **Per-session isolation**: Users can't consume each other's quota
- **Async-safe**: No blocking operations, safe for Tokio runtime
- **Configurable**: Deploy-time rate limit customization

### 4. Result Sanitization

- **Error message filtering**: Removes sensitive paths/traces
- **Size limits**: Prevents memory exhaustion
- **Structure validation**: Ensures valid JSON responses

### 5. Structured Logging

All security events logged with structured data:
- Validation failures with context
- Rate limit violations with session ID
- Path security violations with attempted path

## Integration Guide

### Adding to Handlers

```rust
use loom_server::{QueryValidator, RateLimiter, PathSanitizer, ResultValidator};

async fn handle_query(
    session_id: String,
    query: String,
    timeout_seconds: u32,
) -> Result<Value, SecurityError> {
    let validator = QueryValidator::new();
    let sanitizer = PathSanitizer::new("/workspace");
    let rate_limiter = RateLimiter::new();
    let result_validator = ResultValidator::new();

    // 1. Validate query size
    validator.validate_size(&query)?;

    // 2. Check rate limit
    rate_limiter.check_rate_limit(&session_id).await?;

    // 3. Validate timeout
    validator.validate_timeout(timeout_seconds)?;

    // 4. Process query
    let query_path = extract_path(&query);
    let safe_path = sanitizer.sanitize(&query_path)?;

    // 5. Execute query (stub)
    let result = execute_query(&safe_path).await?;

    // 6. Validate result
    result_validator.validate(&result)?;

    Ok(result)
}
```

### Custom Configuration

```rust
let validator = QueryValidator::with_limits(
    5 * 1024,           // 5 KB max query
    100,                // 100 queries per session
    (5, 600),           // 5-600 second timeout range
    vec![
        "/etc".to_string(),
        "/root".to_string(),
        "/sys".to_string(),
        "/proc".to_string(),
    ],
);

let limiter = RateLimiter::with_rate(
    200.0 / 60.0,  // 200 queries per minute
    200,           // 200 token bucket
);

let result_validator = ResultValidator::with_max_size(50 * 1024 * 1024); // 50 MB
```

## Performance Characteristics

| Component | Time Complexity | Space Complexity | Notes |
|-----------|-----------------|------------------|-------|
| `QueryValidator::validate_size()` | O(1) | O(1) | String length check |
| `QueryValidator::validate_timeout()` | O(1) | O(1) | Range check |
| `QueryValidator::validate_path()` | O(n) | O(1) | n = blocked_paths count |
| `QueryValidator::validate_json()` | O(n) | O(n) | n = JSON size |
| `PathSanitizer::sanitize()` | O(n) | O(n) | n = path length + canonicalize |
| `RateLimiter::check_rate_limit()` | O(1) amortized | O(m) | m = active sessions |
| `ResultValidator::validate()` | O(n) | O(n) | n = result size |
| `ResultValidator::sanitize_error()` | O(n) | O(n) | n = error message length |

## Thread Safety

- **QueryValidator**: Immutable, safe to share across threads
- **PathSanitizer**: Immutable, safe to share across threads
- **RateLimiter**: Async-safe using `Arc<RwLock<...>>`
- **ResultValidator**: Immutable, safe to share across threads

## Future Enhancements

Potential additions for future versions:

1. **Custom validators**: Plugin architecture for domain-specific validation
2. **Audit logging**: Detailed security event auditing
3. **Threat response**: Automatic rate limit escalation for suspicious patterns
4. **Blocklist management**: Dynamic path/pattern blacklist updates
5. **Metrics integration**: Prometheus metrics for security events
6. **Pattern-based detection**: Regex patterns for query injection detection

## Examples

### Complete Query Handler with Security

```rust
#[derive(Clone)]
pub struct SecureQueryHandler {
    validator: QueryValidator,
    sanitizer: PathSanitizer,
    limiter: RateLimiter,
    result_validator: ResultValidator,
}

impl SecureQueryHandler {
    pub fn new(workspace_root: &str) -> Self {
        Self {
            validator: QueryValidator::new(),
            sanitizer: PathSanitizer::new(workspace_root),
            limiter: RateLimiter::new(),
            result_validator: ResultValidator::new(),
        }
    }

    pub async fn handle_query(
        &self,
        session_id: String,
        query: String,
        timeout_seconds: u32,
    ) -> Result<Value, SecurityError> {
        // Validate inputs
        self.validator.validate_size(&query)?;
        self.validator.validate_timeout(timeout_seconds)?;
        self.limiter.check_rate_limit(&session_id).await?;

        // Extract and sanitize path
        let path = extract_path_from_query(&query);
        let safe_path = self.sanitizer.sanitize(path)?;

        // Execute query (stub)
        let result = execute_query(safe_path, timeout_seconds).await?;

        // Validate result
        self.result_validator.validate(&result)?;

        Ok(result)
    }
}
```

## References

- [OWASP Path Traversal](https://owasp.org/www-community/attacks/Path_Traversal)
- [Token Bucket Algorithm](https://en.wikipedia.org/wiki/Token_bucket)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
