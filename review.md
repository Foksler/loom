<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Security Review: Authentication & ABAC System Implementation

**Review Date:** 2025-01-02  
**Spec Version:** 1.0  
**Reviewer:** Automated Security Analysis  
**Oracle Validation:** 2025-01-02

---

## Executive Summary

The authentication and ABAC implementation is **generally well-designed** with proper token hashing (Argon2), defense-in-depth authorization, and comprehensive audit logging infrastructure. However, several security gaps require immediate attention before production deployment.

After Oracle validation, severity classifications have been refined to distinguish between **real security vulnerabilities** and **spec compliance gaps**.

### Risk Summary (Revised)

| Severity | Count | Status |
|----------|-------|--------|
| 🔴 CRITICAL | 2 | Must fix before production (C1, C7) |
| 🟠 HIGH | 9 | Should fix before production |
| 🟡 MEDIUM | 15 | Address soon after launch |
| 🟢 LOW | 14 | Minor improvements / spec compliance |

### Key Insight from Oracle Review

> "The most dangerous paths are those that let an attacker become another user (session cookies, access tokens, magic links), let an attacker access/alter data they don't own (IDORs, share links), or enable mass abuse (lack of rate limiting)."

Several original "critical" findings are **spec compliance issues** rather than exploitable vulnerabilities (e.g., SHA-256 vs Argon2 for high-entropy tokens, OIDC nonce for code flow).

---

## 🔴 CRITICAL Issues

### C1. Missing `Secure` Flag on Session Cookies
**Files:** 
- [crates/loom-server/src/routes/auth.rs#L1185](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L1185)
- [crates/loom-server/src/routes/auth.rs#L1335](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L1335)
- [crates/loom-server/src/routes/auth.rs#L206](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L206)

**Spec Requirement:** "HttpOnly, Secure, SameSite=Lax"

**Issue:** Session cookies are set without the `Secure` flag:
```rust
"{}={}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax"
```

**Impact:** Session cookies can be transmitted over unencrypted HTTP, enabling session hijacking via MITM attacks.

**Fix:** Add `; Secure` to all session cookie strings in production.

---

### ~~C2.~~ → H1. No CSRF Token Protection for State-Changing Requests *(Downgraded)*
**Spec Requirement:** "CSRF token for state-changing requests"

**Issue:** No CSRF token middleware exists. POST/PUT/PATCH/DELETE API endpoints rely solely on `SameSite=Lax` cookies.

**Oracle Note:** `SameSite=Lax` blocks cross-site POSTs directly in modern browsers. This is HIGH rather than CRITICAL because exploitation requires specific browser quirks or GET endpoints with side effects.

**Impact:** Vulnerable endpoints include:
- `POST /api/users/me/delete`
- `DELETE /api/sessions/{id}`
- `PATCH /api/admin/users/{id}/roles`
- `POST /api/admin/users/{id}/impersonate`

**Fix:** Implement `X-CSRF-Token` header validation for all mutating requests.

---

### ~~C3.~~ → L1. Session Tokens Use SHA-256 Instead of Argon2 *(Downgraded to LOW)*
**Files:** 
- [crates/loom-server/src/routes/auth.rs#L1349-L1356](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L1349-L1356)
- [crates/loom-server/src/auth_middleware.rs#L74-L80](file:///home/ghuntley/loom/crates/loom-server/src/auth_middleware.rs#L74-L80)

**Spec Requirement:** "All tokens stored as Argon2 hashes"

**Issue:** Session tokens, invitation tokens, and API keys (in server) use SHA-256 while other tokens use Argon2.

| Token Type | Implementation | Spec |
|------------|----------------|------|
| Session tokens | SHA-256 ❌ | Argon2 |
| Invitation tokens | SHA-256 ❌ | Argon2 |
| API keys (server) | SHA-256 ❌ | Argon2 |
| Access tokens | Argon2 ✓ | Argon2 |
| Magic link tokens | Argon2 ✓ | Argon2 |
| Share link tokens | Argon2 ✓ | Argon2 |

**Oracle Note:** These are *high-entropy random tokens*, not user passwords. For such tokens, SHA-256 is cryptographically acceptable; GPU brute-force is not realistic because the token space is enormous (~128+ bits). This is a **spec compliance gap**, not a critical vulnerability.

**Fix:** Optionally migrate to Argon2id for consistency, but don't block launch on this.

---

### ~~C4.~~ → H2. OAuth Tokens Stored as Plaintext *(Downgraded)*
**File:** [crates/loom-server/migrations/008_auth_users.sql#L30-L31](file:///home/ghuntley/loom/crates/loom-server/migrations/008_auth_users.sql#L30-L31)

**Issue:** OAuth access/refresh tokens from providers (GitHub, Google) stored as plaintext:
```sql
access_token TEXT,
refresh_token TEXT,
```

**Oracle Note:** A DB compromise already yields full account takeover via sessions, access tokens, PII, etc. Storing OAuth tokens plaintext increases damage incrementally, but it's not a new class of compromise. HIGH priority for "this sprint" rather than pre-launch blocker.

**Impact:** Database compromise grants access to all linked OAuth provider accounts.

**Fix:** Encrypt tokens at rest using envelope encryption with KMS or derived key.

---

### ~~C5.~~ → M1. Missing OIDC Nonce Validation (Google & Okta) *(Downgraded to MEDIUM)*
**Files:**
- [crates/loom-server/src/routes/auth.rs#L946-L960](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L946-L960)
- [crates/loom-server/src/routes/auth.rs#L1073-L1087](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L1073-L1087)

**Issue:** Nonce is generated and stored but never validated when ID token is returned.

**Oracle Note:** Current code uses **authorization code flow** and calls `/userinfo` API with access token. It does NOT accept ID token from browser as credential. In this pattern, `state` protects against CSRF; `nonce` primarily protects clients that accept ID tokens directly (implicit/hybrid flow). Since the server doesn't consume ID tokens from browser, this is **spec noncompliance / future-proofing**, not an exploitable auth bypass.

**Impact:** Low - would only matter if flow changes to rely on ID tokens directly.

**Fix:** Implement nonce validation if/when ID tokens are used for login.

---

### ~~C6.~~ → H3. TOCTOU Race Condition in Magic Link Verification *(Downgraded)*
**Files:**
- [crates/loom-server/src/routes/auth.rs#L1248-L1269](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L1248-L1269)
- [crates/loom-server/src/db/session.rs#L626-L643](file:///home/ghuntley/loom/crates/loom-server/src/db/session.rs#L626-L643)

**Issue:** Verification fetches pending links, verifies, then marks used in separate query. Two concurrent requests could both verify successfully.

**Oracle Note:** Magic link tokens are already bearer credentials: once attacker has the link, they can log in. Double-use doesn't meaningfully increase compromise; it mainly breaks "single-use" guarantee and complicates auditing. HIGH if you rely on single-use for security properties; otherwise MEDIUM.

**Impact:** Magic links may be used multiple times (violates spec).

**Fix:** Use `UPDATE ... WHERE used_at IS NULL RETURNING *` or transaction with `SELECT ... FOR UPDATE`.

---

### C7. IDOR in Share Link Creation/Revocation
**File:** [crates/loom-server/src/routes/share.rs#L156-L273](file:///home/ghuntley/loom/crates/loom-server/src/routes/share.rs#L156-L273)

**Issue:** `create_share_link` and `revoke_share_link` fetch thread but do NOT verify current user owns it.

**Impact:** Any authenticated user can create/revoke share links for any thread.

**Fix:** Add ownership check:
```rust
let owner_id = state.repo.get_thread_owner_user_id(&id).await?;
if owner_id != current_user.user.id.to_string() {
    return (StatusCode::FORBIDDEN, ...).into_response();
}
```

---

## 🟠 HIGH Issues (Including Oracle-Identified)

### H0. Dev Mode Authentication Bypass *(NEW - Oracle Identified)*
**File:** [crates/loom-server/src/auth_middleware.rs](file:///home/ghuntley/loom/crates/loom-server/src/auth_middleware.rs)

**Issue:** `auth_middleware` has a dev-mode auto-auth path with full admin privileges. Misconfiguration can silently grant admin access to any unauthenticated request.

**Impact:** If `LOOM_AUTH_DEV_MODE=1` is accidentally set in production, complete authentication bypass.

**Fix:** 
- Gate dev_mode by **compile-time feature** AND env var
- Log loud warning at startup
- Refuse to start if dev_mode is set with `LOOM_ENV=production`

### H1. Missing Email Verification Check (Google & Okta)
**Files:**
- [crates/loom-server/src/routes/auth.rs#L977-L1004](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L977-L1004)
- [crates/loom-server/src/routes/auth.rs#L1104-L1121](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L1104-L1121)

**Spec Requirement:** "OAuth provider says verified: true → Trust it; verified: false → Require verification"

**Issue:** Callbacks don't check `email_verified` before creating/linking accounts.

**Impact:** Attackers with unverified emails could hijack accounts.

**Fix:** Check `email_verified == true` before proceeding.

---

### H2. GitHub Fallback Email Generation
**File:** [crates/loom-server/src/routes/auth.rs#L852-L863](file:///home/ghuntley/loom/crates/loom-server/src/routes/auth.rs#L852-L863)

**Issue:** When no verified email exists, generates synthetic `{login}@github.user`.

**Impact:** Account enumeration risk; confuses account linking.

**Fix:** Reject login if no verified email available.

---

### H3. Access Token Has No Expiry Validation
**File:** [crates/loom-server/src/auth_middleware.rs#L344-L386](file:///home/ghuntley/loom/crates/loom-server/src/auth_middleware.rs#L344-L386)

**Issue:** `authenticate_access_token` does not check token expiry (sessions do).

**Impact:** Access tokens remain valid indefinitely.

**Fix:** Add expiry validation similar to session authentication.

---

### H4. Org Deletion Allows Admin (Should Be Owner-Only)
**File:** [crates/loom-auth/src/abac/policies/org.rs#L16-L18](file:///home/ghuntley/loom/crates/loom-auth/src/abac/policies/org.rs#L16-L18)

**Issue:** Policy allows `Action::Delete` for `is_org_admin()` which includes both Owner AND Admin.

**Spec:** "Admins cannot delete org"

**Fix:** Separate `Action::Delete` to require `is_org_owner()` only.

---

### H5. Access Denials Not Audit Logged to Database
**Files:**
- [crates/loom-server/src/abac_middleware.rs#L180-L186](file:///home/ghuntley/loom/crates/loom-server/src/abac_middleware.rs#L180-L186)
- [crates/loom-server/src/abac_middleware.rs#L692](file:///home/ghuntley/loom/crates/loom-server/src/abac_middleware.rs#L692)

**Spec Requirement:** "All denials must be audit logged"

**Issue:** `AuditEventType::AccessDenied` exists but is never emitted. Denials only go to tracing logs.

**Fix:** Emit `AuditEventType::AccessDenied` audit events with actor, resource, and action.

---

### H6. Share Links Not Integrated with ABAC
**Files:**
- [crates/loom-auth/src/share_link.rs](file:///home/ghuntley/loom/crates/loom-auth/src/share_link.rs)
- [crates/loom-auth/src/abac/types.rs](file:///home/ghuntley/loom/crates/loom-auth/src/abac/types.rs)

**Issue:** Share links exist but ABAC policies have no evaluation path for share-link-based access.

**Risk:** Access via share links bypasses ABAC entirely.

**Fix:** Add `shared_via_link: bool` to `ResourceAttrs` and integrate into `thread::can_read()`.

---

### H7. Support Access Expiry Not Checked in ABAC
**File:** [crates/loom-auth/src/abac/policies/thread.rs#L29-L31](file:///home/ghuntley/loom/crates/loom-auth/src/abac/policies/thread.rs#L29-L31)

**Spec Requirement:** "Support access expires 31 days after approval"

**Issue:** ABAC only checks `is_shared_with_support` boolean without expiration verification.

**Fix:** Add `support_access_expires_at` to `ResourceAttrs` or document caller MUST use `SupportAccess::is_active()`.

---

### H8. No Rate Limiting on Device Code Endpoints
**Files:**
- [crates/loom-server/src/api.rs#L405](file:///home/ghuntley/loom/crates/loom-server/src/api.rs#L405)
- [crates/loom-auth-devicecode/src/lib.rs#L334-L341](file:///home/ghuntley/loom/crates/loom-auth-devicecode/src/lib.rs#L334-L341)

**Spec Requirement:** "5 second minimum polling interval with rate limiting"

**Issue:** No rate limiting on `/api/auth/device/poll` or user code verification.

**Impact:** Brute-force of user codes (only ~30 bits entropy).

**Fix:** Implement IP-based rate limiting (e.g., 1 req/5s per device_code, 5 attempts/15min for user codes).

---

### H9. Missing Route-Level Admin Authorization
**File:** [crates/loom-server/src/api.rs#L457-L461](file:///home/ghuntley/loom/crates/loom-server/src/api.rs#L457-L461)

**Issue:** Admin routes defined without `RequireRole::admin()` route layer. Handlers check manually.

**Impact:** Defense-in-depth violation; missed check could expose admin functionality.

**Fix:** Add `.route_layer(RequireRole::admin())` to admin routes.

---

### H10. Open Redirect via `redirectTo` Parameter
**File:** [web/loom-web/src/routes/login/+page.svelte#L16-L39](file:///home/ghuntley/loom/web/loom-web/src/routes/login/+page.svelte#L16-L39)

**Issue:** `redirectTo` parameter taken from URL without validation.

**Impact:** After OAuth, user redirected to malicious site for phishing.

**Fix:** Validate `redirectTo` is relative path or same-origin URL.

---

### H11. Missing `token_hash` NOT NULL Constraint
**Files:**
- [crates/loom-server/migrations/009_auth_sessions.sql](file:///home/ghuntley/loom/crates/loom-server/migrations/009_auth_sessions.sql)
- [crates/loom-server/migrations/017_fix_sessions_token_hash.sql](file:///home/ghuntley/loom/crates/loom-server/migrations/017_fix_sessions_token_hash.sql)

**Issue:** `sessions.token_hash` allows NULL values.

**Impact:** Sessions without token_hash can't be validated; logic bugs possible.

**Fix:** Add data migration to backfill or invalidate old sessions; add NOT NULL constraint.

---

## 🟡 MEDIUM Issues

### M1. ID Token Signature Not Verified (Google)
**File:** [crates/loom-auth-google/src/lib.rs#L578-L623](file:///home/ghuntley/loom/crates/loom-auth-google/src/lib.rs#L578-L623)

**Issue:** JWT payload decoded without cryptographic signature verification.

**Mitigation:** HTTPS provides protection; but defense-in-depth requires signature verification.

---

### M2. Device Code Expiry Mismatch
**File:** [crates/loom-auth-devicecode/src/lib.rs#L77](file:///home/ghuntley/loom/crates/loom-auth-devicecode/src/lib.rs#L77)

**Issue:** Spec requires 15-minute expiry; implementation uses 10 minutes.

---

### M3. Polling Interval Below Spec
**File:** [crates/loom-auth-devicecode/src/lib.rs#L83](file:///home/ghuntley/loom/crates/loom-auth-devicecode/src/lib.rs#L83)

**Issue:** Spec requires 5-second minimum; implementation documents 1 second.

---

### M4. Non-CSPRNG for Token Generation
**Files:**
- [crates/loom-auth-magiclink/src/lib.rs#L226-L228](file:///home/ghuntley/loom/crates/loom-auth-magiclink/src/lib.rs#L226-L228)
- [crates/loom-auth-devicecode/src/lib.rs#L335](file:///home/ghuntley/loom/crates/loom-auth-devicecode/src/lib.rs#L335)

**Issue:** Uses `rand::thread_rng()` instead of `OsRng` for security tokens.

---

### M5. Team Visibility Doesn't Require Org Membership
**File:** [crates/loom-auth/src/abac/policies/thread.rs#L42-L47](file:///home/ghuntley/loom/crates/loom-auth/src/abac/policies/thread.rs#L42-L47)

**Issue:** User with team membership but revoked org membership could still access team-visible threads.

---

### M6. No Maximum Duration for Impersonation Sessions
**File:** [crates/loom-auth/src/admin.rs#L52-L96](file:///home/ghuntley/loom/crates/loom-auth/src/admin.rs#L52-L96)

**Issue:** `ImpersonationSession` has no `expires_at` field.

**Fix:** Add 4-hour maximum with automatic expiry.

---

### M7. Missing OAuth State Store Size Limit
**File:** [crates/loom-server/src/oauth_state.rs](file:///home/ghuntley/loom/crates/loom-server/src/oauth_state.rs)

**Issue:** In-memory store with no size limit allows memory exhaustion via mass OAuth flow initiation.

---

### M8. Missing Audit Events
**File:** [crates/loom-auth/src/audit.rs](file:///home/ghuntley/loom/crates/loom-auth/src/audit.rs)

**Issue:** Missing audit event types for: `AccessTokenCreated/Revoked`, `ShareLinkCreated/Revoked`, `MagicLinkCreated/Used`.

---

### M9. Device Code Not Hashed
**File:** [crates/loom-auth-devicecode/src/lib.rs#L188-L192](file:///home/ghuntley/loom/crates/loom-auth-devicecode/src/lib.rs#L188-L192)

**Issue:** Device code stored as plaintext (UUID, time-limited).

---

### M10. Missing Index on `magic_links.token_hash`
**File:** [crates/loom-server/migrations/009_auth_sessions.sql](file:///home/ghuntley/loom/crates/loom-server/migrations/009_auth_sessions.sql)

**Issue:** Token lookups require full table scan.

---

### M11. Missing Index on `org_invitations.token_hash`
**File:** [crates/loom-server/migrations/010_auth_orgs.sql](file:///home/ghuntley/loom/crates/loom-server/migrations/010_auth_orgs.sql)

---

### M12. Missing `credentials: 'include'` in Web API Client
**File:** [web/loom-web/src/lib/api/client.ts#L43-L51](file:///home/ghuntley/loom/web/loom-web/src/lib/api/client.ts#L43-L51)

**Issue:** Client-side API calls may fail to send cookies on cross-origin requests.

---

## 🟢 LOW Issues

### L1. `loom-secret::SecretString` Underused for Tokens
Token generation functions return `String` instead of `SecretString`.

### L2. Constant-Time Comparison Edge Case
Early return on hash parse failure leaks minor timing info.

### L3. Magic Link Token in URL Query Parameter
May appear in server logs, browser history, referrer headers.

### L4. Missing UNIQUE Constraints on token_hash Columns
Argon2 collisions astronomically unlikely but constraint enforces integrity.

### L5. Timestamps Stored as TEXT
Acceptable for SQLite portability but slower comparisons.

### L6. Teams Missing Soft Delete
Inconsistent with users/orgs which have `deleted_at`.

### L7. Missing ON DELETE for impersonation_sessions FKs
Orphaned records on user deletion.

### L8. Logger Redaction Missing Token Patterns
Missing: `token`, `key`, `auth`, `bearer`, `session`, `cookie`.

### L9. Device Code User Code Low Entropy
~30 bits; consider 8 alphanumeric or 12 digits.

### L10. Minimal Email Validation
`email.contains('@')` doesn't catch `@.com`.

---

## ✅ Correctly Implemented (Positive Observations)

| Area | Status |
|------|--------|
| 60-day sliding session expiry | ✓ `SESSION_EXPIRY_DAYS = 60` with `extend()` |
| Support access 31-day expiry | ✓ `SUPPORT_ACCESS_DAYS = 31` |
| Tokens shown once at creation | ✓ All `::new()` methods return plaintext only once |
| ABAC thread ownership enforcement | ✓ Owner check first in policy |
| ABAC deny-by-default | ✓ All policies fall through to `false` |
| SystemAdmin bypass is global | ✓ Checked in `check_global_roles()` first |
| OAuth CSRF state parameter | ✓ State stored, validated, consumed |
| Token hashing before DB lookup | ✓ Tokens never stored plaintext |
| Session metadata (GeoIP) | ✓ `ClientInfo::from_headers()` |
| Magic links single-use and time-limited | ✓ `is_used()`, `is_expired()` |
| API keys org-level only with scopes | ✓ `ApiKeyScope` enum |
| Parameterized SQL queries | ✓ sqlx `.bind()` throughout |
| No XSS in web frontend | ✓ No `@html` or innerHTML usage |
| Auth state machine | ✓ Proper XState with tested transitions |
| Cannot remove own admin status | ✓ Check in admin.rs |

---

## Remediation Priority (Oracle-Validated)

### 🚨 Immediate (Before Production)
| Priority | Issue | Effort | Description |
|----------|-------|--------|-------------|
| P0 | **C1** | S | Add `Secure` to all session cookies |
| P0 | **C7** | S | Fix share link IDOR (ownership check) |
| P1 | **H0** | S | Dev mode hardening (prevent production misconfig) |
| P1 | **H1** | S | Implement CSRF protection for cookie flows |
| P1 | **H4** | S-M | Enforce access token expiry |
| P1 | **H1/H2** | S | Email verification for OAuth (Google/Okta/GitHub) |
| P1 | **H3** | S-M | Fix magic link TOCTOU (atomic single-use) |

### 📋 High Priority (This Sprint)
| Priority | Issue | Effort | Description |
|----------|-------|--------|-------------|
| P2 | **H2** | M-L | Encrypt OAuth provider tokens at rest |
| P2 | **H5** | S | Restrict org deletion to owners only |
| P2 | **H8** | M | Rate limiting for device code endpoints |
| P2 | **H10** | S | Fix open redirect (validate redirectTo) |
| P2 | **H6/H7/H9/H11** | M | ABAC hardening, audit logging, route layers |

### 📦 Backlog (Spec Compliance & Cleanup)
| Priority | Issue | Effort | Description |
|----------|-------|--------|-------------|
| P3 | **L1 (ex-C3)** | M | Migrate to Argon2 for all tokens (spec compliance) |
| P3 | **M1 (ex-C5)** | S-M | OIDC nonce validation (future-proofing) |
| P3 | **M2-M12** | Varies | Medium issues (indexes, intervals, etc.) |
| P3 | **L2-L10** | Low | Minor improvements |

### Effort Key
- **S** = Small (< 1 day)
- **M** = Medium (1-3 days)  
- **L** = Large (3+ days)

---

## Appendix: Files Reviewed

### New Crates
- `crates/loom-auth/` - Core auth types, ABAC engine
- `crates/loom-auth-github/` - GitHub OAuth
- `crates/loom-auth-google/` - Google OAuth  
- `crates/loom-auth-okta/` - Okta OAuth/OIDC
- `crates/loom-auth-magiclink/` - Magic link auth
- `crates/loom-auth-devicecode/` - Device code flow
- `crates/loom-smtp/` - Email sending
- `crates/loom-geoip/` - GeoIP lookup

### Modified Server Files
- `crates/loom-server/src/routes/auth.rs`
- `crates/loom-server/src/routes/sessions.rs`
- `crates/loom-server/src/routes/admin.rs`
- `crates/loom-server/src/routes/orgs.rs`
- `crates/loom-server/src/routes/teams.rs`
- `crates/loom-server/src/routes/share.rs`
- `crates/loom-server/src/routes/api_keys.rs`
- `crates/loom-server/src/auth_middleware.rs`
- `crates/loom-server/src/abac_middleware.rs`
- `crates/loom-server/src/oauth_state.rs`

### Migrations
- `008_auth_users.sql` through `017_fix_sessions_token_hash.sql`

### Web Frontend
- `web/loom-web/src/lib/auth/`
- `web/loom-web/src/lib/api/client.ts`
- `web/loom-web/src/routes/login/`
- `web/loom-web/src/routes/device/`
