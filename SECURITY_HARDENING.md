# Security Hardening Guide: Query Bridge Defense Layers

**Purpose:** Comprehensive security configuration for production Query Bridge deployments  
**Audience:** Security engineers, DevOps engineers  
**Duration:** 3-4 hours implementation  
**Prerequisites:** Integration Guide complete

---

## Table of Contents

1. [Security Architecture](#security-architecture)
2. [Defense Layers](#defense-layers)
3. [Configuration for Strict Mode](#configuration-for-strict-mode)
4. [Audit Logging Setup](#audit-logging-setup)
5. [Rate Limiting Configuration](#rate-limiting-configuration)
6. [Path Whitelisting](#path-whitelisting)
7. [Secret Management](#secret-management)
8. [Security Checklist](#security-checklist)

---

## Security Architecture

### Threat Model

Query Bridge is susceptible to:

1. **Path Traversal Attacks** - Reading files outside workspace
2. **Environment Exfiltration** - Stealing secrets from env vars
3. **DoS via Queries** - Overwhelming server with queries
4. **Query Tampering** - Modifying queries in transit
5. **Response Leakage** - Exposing sensitive data in responses

### Defense Strategy

```
┌─────────────────────────────────────────────────┐
│  Query arrives (untrusted)                      │
└────────────────┬────────────────────────────────┘
                 │
        ┌────────▼────────┐
        │ Rate Limiting   │  ← Layer 1: Volume control
        └────────┬────────┘
                 │
        ┌────────▼──────────────┐
        │ Query Validation      │  ← Layer 2: Format/bounds
        └────────┬──────────────┘
                 │
        ┌────────▼──────────────┐
        │ Authorization         │  ← Layer 3: Access control
        └────────┬──────────────┘
                 │
        ┌────────▼──────────────┐
        │ Path Normalization    │  ← Layer 4: Escape prevention
        └────────┬──────────────┘
                 │
        ┌────────▼──────────────┐
        │ Audit Logging         │  ← Layer 5: Accountability
        └────────┬──────────────┘
                 │
        ┌────────▼──────────────┐
        │ Execute Operation     │  ← Layer 6: Safe execution
        └────────┬──────────────┘
                 │
        ┌────────▼──────────────┐
        │ Redact Response       │  ← Layer 7: Output sanitization
        └─────────────────────────┘
```

---

## Defense Layers

### Layer 1: Rate Limiting

Prevent query floods and DoS attacks.

**Configuration:**

```toml
[security.rate_limiting]
# Global limits
enabled = true
max_queries_per_second = 1000
max_queries_per_minute = 10000

# Per-session limits
max_queries_per_session_per_minute = 100
max_concurrent_queries_per_session = 5

# Per-type limits
read_file_per_minute = 20
get_environment_per_minute = 50
request_user_input_per_minute = 5

# Sliding window strategy
window_type = "sliding"  # or "fixed"
allow_burst = true
burst_multiplier = 2.0
```

**Implementation:**

```rust
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

pub struct RateLimiter {
    per_session_limits: HashMap<String, QueryCounter>,
    global_counter: QueryCounter,
    config: RateLimiterConfig,
}

pub struct QueryCounter {
    queries: Vec<SystemTime>,
    max_per_minute: usize,
}

impl RateLimiter {
    pub fn check_rate_limit(
        &mut self,
        session_id: &str,
        query_kind: &ServerQueryKind,
    ) -> Result<(), SecurityError> {
        // Check global rate
        self.global_counter.add_query()?;

        // Check per-session rate
        let session_counter = self.per_session_limits
            .entry(session_id.to_string())
            .or_insert_with(|| QueryCounter::new(
                self.config.max_per_session
            ));
        session_counter.add_query()?;

        // Check per-type rate
        match query_kind {
            ServerQueryKind::ReadFile { .. } => {
                if session_counter.read_file_count_per_min()
                    > self.config.read_file_per_minute {
                    return Err(SecurityError::RateLimitExceeded {
                        query_type: "ReadFile".into(),
                        limit: self.config.read_file_per_minute,
                    });
                }
            }
            ServerQueryKind::GetEnvironment { .. } => {
                if session_counter.env_count_per_min()
                    > self.config.get_environment_per_minute {
                    return Err(SecurityError::RateLimitExceeded {
                        query_type: "GetEnvironment".into(),
                        limit: self.config.get_environment_per_minute,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }
}
```

### Layer 2: Query Validation

Ensure queries conform to expected format and bounds.

**Configuration:**

```toml
[security.validation]
enabled = true

# Query size limits
max_query_size_bytes = 10000
max_path_length_bytes = 500

# Timeout bounds
min_timeout_secs = 1
max_timeout_secs = 300

# ID format
require_uuid_prefix = "Q-"
require_uuid_version = 7

# Allowed query types
allowed_kinds = [
    "ReadFile",
    "GetEnvironment",
    "GetWorkspaceContext",
]
blocked_kinds = [
    "ExecuteCommand",
]
```

**Implementation:**

```rust
pub fn validate_query(
    query: &ServerQuery,
    config: &SecurityConfig,
) -> Result<(), ValidationError> {
    // 1. Size checks
    let query_size = serde_json::to_string(query)?.len();
    if query_size > config.max_query_size_bytes {
        return Err(ValidationError::TooLarge {
            size: query_size,
            max: config.max_query_size_bytes,
        });
    }

    // 2. ID format check
    if !query.id.starts_with(&config.require_uuid_prefix) {
        return Err(ValidationError::InvalidIdFormat(
            query.id.clone()
        ));
    }

    // 3. Timeout bounds
    if query.timeout_secs < config.min_timeout_secs {
        return Err(ValidationError::TimeoutTooShort(query.timeout_secs));
    }
    if query.timeout_secs > config.max_timeout_secs {
        return Err(ValidationError::TimeoutTooLong(query.timeout_secs));
    }

    // 4. Query kind allowed
    match &query.kind {
        ServerQueryKind::ReadFile { path } => {
            if path.len() > config.max_path_length_bytes {
                return Err(ValidationError::PathTooLong(path.clone()));
            }
        }
        ServerQueryKind::ExecuteCommand { .. } if config.blocked_kinds
            .contains(&"ExecuteCommand".to_string()) => {
            return Err(ValidationError::QueryTypeBlocked(
                "ExecuteCommand".into()
            ));
        }
        _ => {}
    }

    Ok(())
}
```

### Layer 3: Authorization

Check if requester can perform this operation.

**Configuration:**

```toml
[security.authorization]
enabled = true

[security.authorization.per_session]
# Session must have valid token
require_session_token = true
token_expiry_secs = 3600

# Session rate limits
max_queries_per_session = 100

[security.authorization.roles]
admin = { read_file = true, get_env = true, execute_cmd = true }
user = { read_file = true, get_env = true, execute_cmd = false }
viewer = { read_file = false, get_env = false, execute_cmd = false }

[security.authorization.query_type_limits]
# Who can read files
read_file_roles = ["admin", "user"]

# Who can access environment
get_environment_roles = ["admin", "user"]

# Who can request user input
request_user_input_roles = ["admin", "user"]
```

**Implementation:**

```rust
pub async fn authorize_query(
    session: &Session,
    query: &ServerQuery,
    config: &SecurityConfig,
) -> Result<(), AuthorizationError> {
    // 1. Check session validity
    if !session.is_valid() {
        return Err(AuthorizationError::SessionExpired);
    }

    // 2. Check session role for query type
    let role = &session.role;
    match &query.kind {
        ServerQueryKind::ReadFile { .. } => {
            if !config.read_file_roles.contains(role) {
                return Err(AuthorizationError::Forbidden {
                    query_type: "ReadFile".into(),
                    role: role.clone(),
                });
            }
        }
        ServerQueryKind::GetEnvironment { .. } => {
            if !config.get_environment_roles.contains(role) {
                return Err(AuthorizationError::Forbidden {
                    query_type: "GetEnvironment".into(),
                    role: role.clone(),
                });
            }
        }
        ServerQueryKind::ExecuteCommand { .. } => {
            return Err(AuthorizationError::OperationBlocked(
                "ExecuteCommand not allowed".into()
            ));
        }
        _ => {}
    }

    // 3. Check query against allowlist (if enabled)
    if config.enable_query_allowlist {
        if !config.is_allowed(query) {
            return Err(AuthorizationError::NotOnAllowlist);
        }
    }

    Ok(())
}
```

### Layer 4: Path Normalization

Prevent `../` and symlink escape attacks.

**Configuration:**

```toml
[security.path_handling]
enabled = true

# Normalization strategy
normalize = true
resolve_symlinks = true

# Whitelisting (see Path Whitelisting section)
use_whitelist = true
whitelist_mode = "allowlist"  # or "blocklist"

# Workspace boundary
workspace_root = "/home/user/project"
allow_outside_workspace = false

# Symbolic links
allow_symlinks = false
max_symlink_depth = 0
```

**Implementation:**

```rust
pub fn normalize_and_validate_path(
    requested_path: &str,
    workspace_root: &Path,
    config: &SecurityConfig,
) -> Result<PathBuf, PathError> {
    // 1. Parse path
    let path = Path::new(requested_path);

    // 2. Normalize (remove .., ., etc)
    let normalized = path
        .components()
        .fold(PathBuf::new(), |mut acc, comp| {
            match comp {
                std::path::Component::Normal(c) => acc.push(c),
                std::path::Component::ParentDir => {
                    // Reject .. attempts
                    return acc;  // Ignore parent traversal
                }
                std::path::Component::RootDir => {
                    // Reject absolute paths
                    return acc;
                }
                _ => {}
            }
            acc
        });

    // 3. Resolve symlinks
    let mut final_path = workspace_root.join(&normalized);
    if config.resolve_symlinks {
        final_path = std::fs::canonicalize(&final_path)
            .map_err(|_| PathError::CanonicalizeError)?;
    }

    // 4. Check workspace boundary
    if !final_path.starts_with(workspace_root) {
        return Err(PathError::EscapeAttempt {
            requested: requested_path.to_string(),
            canonical: final_path.display().to_string(),
        });
    }

    // 5. Check whitelist
    if config.use_whitelist {
        if !config.is_path_allowed(&final_path) {
            return Err(PathError::NotWhitelisted(
                final_path.display().to_string()
            ));
        }
    }

    Ok(final_path)
}
```

### Layer 5: Audit Logging

Log all access for compliance and forensics.

See [Audit Logging Setup](#audit-logging-setup) below.

### Layer 6: Safe Execution

Execute queries with proper error handling and timeouts.

**Configuration:**

```toml
[security.execution]
enabled = true

# Execution limits
max_execution_time_secs = 30
max_output_size_bytes = 10000000

# Error handling
catch_panics = true
catch_timeouts = true
```

**Implementation:**

```rust
pub async fn execute_query_safely(
    query: &ServerQuery,
    handler: &impl ServerQueryHandler,
    config: &SecurityConfig,
) -> Result<ServerQueryResponse, ServerQueryError> {
    // 1. Timeout protection
    let timeout_duration = Duration::from_secs(query.timeout_secs as u64);

    // 2. Execute with timeout
    let result = tokio::time::timeout(timeout_duration, async {
        // Panic safety
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || handler.handle_query(query)
        )).map_err(|_| ServerQueryError::ExecutionPanic)
    })
    .await
    .map_err(|_| ServerQueryError::Timeout);

    // 3. Handle results
    match result {
        Ok(Ok(response)) => {
            // Size check
            let response_size = serde_json::to_string(&response)?.len();
            if response_size > config.max_output_size_bytes {
                return Err(ServerQueryError::ResponseTooLarge {
                    size: response_size,
                    max: config.max_output_size_bytes,
                });
            }
            Ok(response)
        }
        Ok(Err(e)) => Err(e),
        Err(e) => Err(e),
    }
}
```

### Layer 7: Response Redaction

Remove sensitive data from responses.

**Configuration:**

```toml
[security.redaction]
enabled = true

# Sensitive patterns
redact_patterns = [
    "password.*=.*",
    "[A-Za-z0-9_-]{20,}",  # Long random strings (API keys)
    "Bearer\\s+[^\\s]+",     # Auth tokens
    "api[_-]?key.*=",
    "secret.*=",
]

# Sensitive env vars
redact_env_patterns = [
    ".*PASSWORD.*",
    ".*TOKEN.*",
    ".*KEY.*",
    ".*SECRET.*",
    ".*CREDENTIAL.*",
]

# File extensions to scan
scan_extensions = ["sh", "env", "toml", "json", "rs"]
```

**Implementation:**

```rust
pub fn redact_sensitive_data(
    input: &str,
    config: &SecurityConfig,
) -> String {
    let mut redacted = input.to_string();

    for pattern in &config.redact_patterns {
        let regex = Regex::new(pattern).unwrap_or_else(|_| {
            Regex::new(&regex::escape(pattern)).unwrap()
        });

        redacted = regex
            .replace_all(&redacted, "[REDACTED]")
            .into_owned();
    }

    redacted
}

pub fn redact_environment_vars(
    vars: &HashMap<String, String>,
    config: &SecurityConfig,
) -> HashMap<String, String> {
    let mut redacted = HashMap::new();

    for (key, value) in vars {
        let should_redact = config.redact_env_patterns
            .iter()
            .any(|pattern| {
                Regex::new(pattern)
                    .map(|re| re.is_match(key))
                    .unwrap_or(false)
            });

        redacted.insert(
            key.clone(),
            if should_redact {
                "[REDACTED]".to_string()
            } else {
                value.clone()
            }
        );
    }

    redacted
}
```

---

## Configuration for Strict Mode

### Strict Mode Definition

Strict mode maximizes security by:
- Denying all queries by default
- Requiring explicit allowlisting
- Aggressive redaction
- Detailed audit logging
- Minimal timeouts

### Full Strict Configuration

```toml
[query_bridge.strict_mode]
enabled = true

# Rate limiting (very restrictive)
[security.rate_limiting]
max_queries_per_session_per_minute = 10
max_concurrent_queries_per_session = 1
read_file_per_minute = 5

# Validation (strict bounds)
[security.validation]
max_query_size_bytes = 1000
max_path_length_bytes = 100
min_timeout_secs = 1
max_timeout_secs = 10

# Authorization (deny by default)
[security.authorization.query_type_limits]
read_file_roles = ["admin"]
get_environment_roles = ["admin"]

# Path handling (allowlist only)
[security.path_handling]
use_whitelist = true
whitelist_mode = "allowlist"
allow_symlinks = false

# Redaction (aggressive)
[security.redaction]
enabled = true
redact_patterns = [".*"]  # Redact everything by default

# Audit logging (verbose)
[audit_logging]
enabled = true
log_level = "DEBUG"
include_full_response = false
include_user_agent = true
include_ip_address = true
retention_days = 90
```

---

## Audit Logging Setup

### Audit Log Schema

```rust
#[derive(serde::Serialize)]
pub struct AuditLogEntry {
    // Identifiers
    pub timestamp: String,  // RFC3339
    pub audit_id: String,    // Unique audit log ID
    pub session_id: String,
    pub user_id: Option<String>,

    // Request context
    pub query_id: String,
    pub query_kind: String,
    pub query_path: Option<String>,

    // Source
    pub source_ip: String,
    pub user_agent: Option<String>,

    // Action & outcome
    pub action: String,  // "READ", "GET_ENV", "DENIED", etc.
    pub status: String,  // "ALLOWED", "DENIED", "ERROR"
    pub reason: Option<String>,  // Why denied

    // Results
    pub response_size_bytes: Option<usize>,
    pub duration_ms: u64,

    // Security context
    pub session_valid: bool,
    pub rate_limit_remaining: u32,
}
```

### Audit Logging Configuration

```toml
[audit_logging]
enabled = true

# Output
destination = "file"  # or "syslog", "cloudwatch", "splunk"
path = "/var/log/loom/audit.log"
rotation = "daily"
retention_days = 90

# Level
log_level = "INFO"
# Log all queries
log_all_queries = true
# Log only denied
log_denied_only = false
# Log with details
include_full_response = false  # Security: don't log response content
include_user_agent = true
include_ip_address = true
include_timing = true

# Sampling (reduce log volume in high-traffic)
sampling_rate = 1.0  # 1.0 = log everything, 0.1 = log 10%

# Encryption
sign_logs = true
signing_key = "${AUDIT_LOG_KEY}"  # From environment
```

### Structured Logging Implementation

```rust
use tracing::{info, warn, error, debug};
use tracing_subscriber::fmt::format::FmtSpan;

pub async fn log_query_audit(
    query: &ServerQuery,
    session: &Session,
    decision: QueryDecision,
    context: &AuditContext,
) {
    let audit_entry = AuditLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        audit_id: uuid7(),
        session_id: session.id.clone(),
        user_id: session.user_id.clone(),
        query_id: query.id.clone(),
        query_kind: format!("{:?}", query.kind),
        query_path: extract_path(&query),
        source_ip: context.source_ip.clone(),
        user_agent: context.user_agent.clone(),
        action: format!("{:?}", query.kind),
        status: format!("{:?}", decision),
        reason: None,
        response_size_bytes: None,
        duration_ms: context.duration.as_millis() as u64,
        session_valid: session.is_valid(),
        rate_limit_remaining: context.rate_limit_remaining,
    };

    match decision {
        QueryDecision::Allowed => {
            info!(
                query_id = %audit_entry.query_id,
                session_id = %audit_entry.session_id,
                query_kind = %audit_entry.query_kind,
                source_ip = %audit_entry.source_ip,
                duration_ms = audit_entry.duration_ms,
                "Query allowed"
            );
        }
        QueryDecision::Denied(reason) => {
            warn!(
                query_id = %audit_entry.query_id,
                session_id = %audit_entry.session_id,
                query_kind = %audit_entry.query_kind,
                reason = %reason,
                source_ip = %audit_entry.source_ip,
                "Query denied"
            );
        }
        QueryDecision::Error(err) => {
            error!(
                query_id = %audit_entry.query_id,
                session_id = %audit_entry.session_id,
                error = %err,
                source_ip = %audit_entry.source_ip,
                "Query error"
            );
        }
    }

    // Persist audit log
    persist_audit_log(&audit_entry).await;
}
```

### Audit Log Analysis

```bash
# Find all denied queries
grep "Query denied" /var/log/loom/audit.log

# Find queries from specific IP
grep "source_ip: 192.168.1.100" /var/log/loom/audit.log

# Count queries per session
grep "session_id" /var/log/loom/audit.log | cut -d: -f3 | sort | uniq -c

# Find slow queries
grep "duration_ms" /var/log/loom/audit.log | awk -F'duration_ms: ' '{print $2}' | sort -rn | head -20

# Export to CSV for analysis
jq -r '[.timestamp, .session_id, .query_kind, .status, .duration_ms] | @csv' /var/log/loom/audit.log > audit_report.csv
```

---

## Rate Limiting Configuration

### Advanced Rate Limiting Strategies

**Token Bucket Algorithm (Recommended):**
```rust
pub struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64,  // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    pub fn try_consume(&mut self, tokens: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        // Refill tokens
        self.tokens = (self.tokens + (elapsed * self.refill_rate)).min(self.capacity);
        self.last_refill = now;

        // Try to consume
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }
}
```

**Sliding Window Algorithm:**
```rust
pub struct SlidingWindowCounter {
    window: Vec<Instant>,
    max_requests: usize,
    window_secs: u64,
}

impl SlidingWindowCounter {
    pub fn try_add(&mut self) -> bool {
        let now = Instant::now();
        let cutoff = now - Duration::from_secs(self.window_secs);

        // Remove old entries
        self.window.retain(|&t| t > cutoff);

        // Check limit
        if self.window.len() < self.max_requests {
            self.window.push(now);
            true
        } else {
            false
        }
    }
}
```

**Configuration:**
```toml
[security.rate_limiting]
algorithm = "token_bucket"  # or "sliding_window"

# Token bucket
tokens_per_second = 10
burst_capacity = 50

# Sliding window
window_duration_secs = 60
max_requests_per_window = 100

# Backpressure
on_limit_exceeded = "reject"  # or "delay"
delay_ms = 100
```

---

## Path Whitelisting

### Whitelist Configuration

```toml
[security.path_whitelist]
mode = "allowlist"  # Only these paths allowed

# Allowed directories
allowed_dirs = [
    "src/",
    "tests/",
    "docs/",
    "Cargo.toml",
    ".github/workflows/",
]

# Allowed files
allowed_files = [
    "README.md",
    "Cargo.lock",
    "Makefile",
]

# Allowed file extensions
allowed_extensions = ["rs", "toml", "json", "md", "yml", "yaml"]

# Blocked patterns
blocked_patterns = [
    ".env*",
    ".git*",
    "target/*",
    "*.swp",
    "*.bak",
]

[security.path_whitelist.blocklist]
mode = "blocklist"  # These paths denied

blocked_dirs = [
    ".git/",
    ".env",
    "target/",
    "secrets/",
    "private/",
]

blocked_patterns = [
    "*.key",
    "*.pem",
    "*.pass",
    "*password*",
    "*secret*",
]
```

### Dynamic Whitelist Management

```rust
pub struct PathWhitelist {
    allowed: HashSet<PathBuf>,
    blocked: HashSet<Pattern>,
    allow_extensions: HashSet<String>,
}

impl PathWhitelist {
    pub fn is_allowed(&self, path: &Path) -> bool {
        // Check blocked patterns first
        let path_str = path.to_string_lossy();
        if self.blocked.iter().any(|p| p.matches(&path_str)) {
            return false;
        }

        // Check extension
        if let Some(ext) = path.extension() {
            if !self.allow_extensions.contains(ext.to_string_lossy().as_ref()) {
                return false;
            }
        }

        // Check allowed paths
        self.allowed.iter().any(|allowed| {
            path.starts_with(allowed)
        })
    }

    pub fn add_allowed(&mut self, path: PathBuf) {
        self.allowed.insert(path);
    }

    pub fn add_blocked(&mut self, pattern: Pattern) {
        self.blocked.insert(pattern);
    }
}
```

---

## Secret Management

### Environment Variable Redaction

```toml
[security.secret_management]
enabled = true

# Patterns to redact
redact_env_vars = [
    ".*PASSWORD.*",
    ".*TOKEN.*",
    ".*KEY.*",
    ".*SECRET.*",
    "DATABASE_URL",
    "PRIVATE_KEY",
    "API_KEY",
]

# Pattern for detecting secrets in file content
file_redaction_patterns = [
    "password\\s*[:=]",
    "api[_-]?key\\s*[:=]",
    "secret\\s*[:=]",
    "token\\s*[:=]",
]
```

### Secure Key Storage

```rust
use std::env;

pub struct SecureEnv {
    keys_to_redact: Vec<String>,
}

impl SecureEnv {
    pub fn get(&self, key: &str) -> Option<String> {
        if self.should_redact(key) {
            return None;  // Never return sensitive env vars
        }
        env::var(key).ok()
    }

    pub fn list_safe(&self) -> HashMap<String, String> {
        std::env::vars()
            .filter(|(k, _)| !self.should_redact(k))
            .collect()
    }

    fn should_redact(&self, key: &str) -> bool {
        self.keys_to_redact.iter().any(|pattern| {
            let re = Regex::new(pattern).unwrap();
            re.is_match(key)
        })
    }
}
```

---

## Security Checklist

### Pre-Deployment Security Review

- [ ] All queries validated (Layer 2)
- [ ] Authorization checks in place (Layer 3)
- [ ] Path normalization tested (Layer 4)
- [ ] Audit logging enabled (Layer 5)
- [ ] Timeout protection active (Layer 6)
- [ ] Response redaction configured (Layer 7)
- [ ] Rate limiting configured
- [ ] Path whitelist defined
- [ ] Sensitive env vars redacted
- [ ] Secrets stored securely

### Configuration Review

- [ ] Strict mode enabled in production
- [ ] Rate limits appropriate for workload
- [ ] Whitelist covers all legitimate paths
- [ ] Redaction patterns catch secrets
- [ ] Audit logs retained 90+ days
- [ ] Log rotation configured
- [ ] Monitoring alerts set up

### Operational Security

- [ ] Audit logs reviewed weekly
- [ ] Access logs monitored for anomalies
- [ ] Security patches applied promptly
- [ ] Secrets rotated regularly
- [ ] Incident response plan documented
- [ ] Staff trained on security policies
- [ ] Regular security audits scheduled

### Compliance

- [ ] GDPR: User data redaction configured
- [ ] SOC 2: Audit logs retained and encrypted
- [ ] HIPAA: Sensitive data patterns identified
- [ ] PCI DSS: Credential redaction active
- [ ] Compliance documentation updated

---

## Security Incident Response

### If Breach Suspected

1. **Immediate (< 5 minutes)**
   ```bash
   # Disable query bridge
   query_bridge.enabled = false
   
   # Save audit logs
   cp /var/log/loom/audit.log /secure/backup/audit-$(date +%s).log
   ```

2. **Investigation (< 1 hour)**
   ```bash
   # Analyze audit logs
   grep "DENIED\|ERROR" /var/log/loom/audit.log | tail -1000
   
   # Find suspicious sessions
   grep "rate_limit_remaining: 0" /var/log/loom/audit.log
   ```

3. **Remediation (varies)**
   - Revoke compromised credentials
   - Rotate secrets
   - Update whitelists
   - Apply patches

4. **Communication**
   - Notify security team
   - Notify affected users
   - Document incident

---

## Summary

Query Bridge security is multi-layered:

1. **Rate Limiting** - Control query volume
2. **Validation** - Enforce format/bounds
3. **Authorization** - Check permissions
4. **Path Normalization** - Prevent escapes
5. **Audit Logging** - Track all access
6. **Safe Execution** - Timeouts & isolation
7. **Redaction** - Remove secrets

Implement in order, test thoroughly, and monitor continuously.
