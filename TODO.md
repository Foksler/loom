# Authentication & ABAC Implementation Plan

Implementation checklist for the Authentication and ABAC system. See
[specs/auth-abac-system.md](./specs/auth-abac-system.md) for full specification.

---

## ✅ Phase 0: Foundation (COMPLETED)

### 0.1 Create loom-auth Crate

- [x] Create `crates/loom-auth/Cargo.toml` with dependencies
- [x] Create `crates/loom-auth/src/lib.rs` with module structure
- [x] Add to workspace members in root `Cargo.toml`

### 0.2 Core Types

- [x] Create `src/types.rs` with all ID newtypes, roles, and enums
- [x] Create `src/error.rs` with `AuthError` enum

### 0.3 Database Migrations

- [x] Create `migrations/008_auth_users.sql` (users, identities)
- [x] Create `migrations/009_auth_sessions.sql` (sessions, access_tokens, device_codes, magic_links)
- [x] Create `migrations/010_auth_orgs.sql` (organizations, memberships, invitations)
- [x] Create `migrations/011_auth_teams.sql` (teams, team_memberships)
- [x] Create `migrations/012_auth_api_keys.sql` (api_keys, api_key_usage)
- [x] Create `migrations/013_auth_threads_ext.sql` (thread extensions, share_links, support_access)
- [x] Create `migrations/014_auth_audit.sql` (audit_logs)
- [x] Update `db.rs` to run new migrations

---

## ✅ Phase 1: Basic Web Auth (COMPLETED)

- [x] Create `src/session.rs` - Session management with 60-day sliding expiry
- [x] Create `src/user.rs` - User struct, Identity, Provider enum
- [x] Create `src/middleware.rs` - CurrentUser, AuthContext, token extraction
- [x] Create auth routes in loom-server:
  - [x] `GET /auth/providers`
  - [x] `GET /auth/me`
  - [x] `POST /auth/logout`

---

## ✅ Phase 2: Magic Link (COMPLETED)

- [x] Create `src/magic_link.rs` - 10-minute single-use tokens
- [x] Create `src/email.rs` - SMTP config, email templates
- [x] Create routes:
  - [x] `POST /auth/magic-link`
  - [x] `GET /auth/magic-link/verify`

---

## ✅ Phase 3: CLI Auth (COMPLETED)

- [x] Create `src/device_code.rs` - Device code flow (123-456-789 format)
- [x] Create `src/access_token.rs` - Bearer tokens with 60-day sliding expiry
- [x] Create routes:
  - [x] `POST /auth/device/start`
  - [x] `POST /auth/device/poll`

---

## ✅ Phase 4: Organizations (COMPLETED)

- [x] Create `src/org.rs` - Organization, OrgMembership, OrgInvitation, OrgJoinRequest
- [x] Create routes in `routes/orgs.rs`:
  - [x] `GET /api/orgs`
  - [x] `POST /api/orgs`
  - [x] `GET /api/orgs/{id}`
  - [x] `PATCH /api/orgs/{id}`
  - [x] `DELETE /api/orgs/{id}`
  - [x] `GET /api/orgs/{id}/members`
  - [x] `POST /api/orgs/{id}/members`
  - [x] `DELETE /api/orgs/{id}/members/{user_id}`

---

## ✅ Phase 5: Teams (COMPLETED)

- [x] Create `src/team.rs` - Team, TeamMembership
- [x] Create routes in `routes/teams.rs`:
  - [x] `GET /api/orgs/{org_id}/teams`
  - [x] `POST /api/orgs/{org_id}/teams`
  - [x] `GET /api/orgs/{org_id}/teams/{team_id}`
  - [x] `PATCH /api/orgs/{org_id}/teams/{team_id}`
  - [x] `DELETE /api/orgs/{org_id}/teams/{team_id}`
  - [x] `GET /api/orgs/{org_id}/teams/{team_id}/members`
  - [x] `POST /api/orgs/{org_id}/teams/{team_id}/members`
  - [x] `DELETE /api/orgs/{org_id}/teams/{team_id}/members/{user_id}`

---

## ✅ Phase 6: ABAC Engine (COMPLETED)

- [x] Create `src/abac/types.rs` - SubjectAttrs, ResourceAttrs, Action
- [x] Create `src/abac/engine.rs` - `is_allowed()` policy dispatcher
- [x] Create `src/abac/policies/thread.rs` - Thread visibility policies
- [x] Create `src/abac/policies/org.rs` - Org/team management policies
- [x] Create `src/abac/policies/llm.rs` - LLM/tool access policies

---

## ✅ Phase 7: API Keys (COMPLETED)

- [x] Create `src/api_key.rs` - lk_ prefixed keys, Argon2 hashing
- [x] Create routes in `routes/api_keys.rs`:
  - [x] `GET /api/orgs/{org_id}/api-keys`
  - [x] `POST /api/orgs/{org_id}/api-keys`
  - [x] `DELETE /api/orgs/{org_id}/api-keys/{id}`
  - [x] `GET /api/orgs/{org_id}/api-keys/{id}/usage`

---

## ✅ Phase 8: Audit & Security (COMPLETED)

- [x] Create `src/audit.rs` - AuditEventType, AuditLogEntry, 90-day retention
- [x] CSRF protection ready (SameSite cookies + tokens)

---

## ✅ Phase 9: Admin Features (COMPLETED)

- [x] Create `src/admin.rs` - ImpersonationSession, promotion/demotion checks
- [x] Create routes in `routes/admin.rs`:
  - [x] `GET /api/admin/users`
  - [x] `PATCH /api/admin/users/{id}/roles`
  - [x] `POST /api/admin/users/{id}/impersonate`
  - [x] `POST /api/admin/impersonate/stop`
  - [x] `GET /api/admin/audit-logs`

---

## ✅ Phase 10: Sharing & Support (COMPLETED)

- [x] Create `src/share_link.rs` - 48-hex token, expiry, revocation
- [x] Create `src/support_access.rs` - 31-day auto-expiry
- [x] Create routes in `routes/share.rs`:
  - [x] `POST /api/threads/{id}/share`
  - [x] `DELETE /api/threads/{id}/share`
  - [x] `GET /api/threads/{id}/share/{token}` (public)
  - [x] `POST /api/threads/{id}/support-access/request`
  - [x] `POST /api/threads/{id}/support-access/approve`
  - [x] `DELETE /api/threads/{id}/support-access`

---

## ✅ Phase 11: User Profile & Account (COMPLETED)

- [x] Create `src/account_deletion.rs` - 90-day grace, tombstone users
- [x] Create routes in `routes/users.rs`:
  - [x] `GET /api/users/{id}`
  - [x] `PATCH /api/users/me`
  - [x] `POST /api/users/me/delete`
  - [x] `POST /api/users/me/restore`

---

## ✅ Phase 12: WebSocket Auth (COMPLETED)

- [x] Update WebSocket handler to validate session cookie
- [x] Implement first-message auth for CLI (30s timeout)
- [x] Add bearer token support for WebSocket connections
- [x] Add WebSocket upgrade route at `/v1/ws/sessions/{session_id}`
- [x] Implement keepalive ping/pong with 30s interval
- [x] Add comprehensive tests (31 tests in loom-server, 15 in loom-auth)

---

## ✅ Phase 13: Session Routes (COMPLETED)

- [x] Create routes in `routes/sessions.rs`:
  - [x] `GET /api/sessions`
  - [x] `DELETE /api/sessions/{id}`

---

## ✅ Phase 14: Documentation & OpenAPI (COMPLETED)

- [x] Added utoipa annotations to all route handlers
- [x] Added schemas to api_docs.rs
- [x] Added tags: auth, sessions, organizations, teams, users, api-keys, admin, share

---

## ✅ Phase 15: Testing (COMPLETED)

- [x] 365+ unit tests in loom-auth covering:
  - Session management
  - Token generation and verification
  - ABAC policy enforcement
  - Magic link flow
  - Device code flow
  - API key management
  - Audit logging
  - Share links and support access

---

## Summary

| Component | Status | Tests |
|-----------|--------|-------|
| loom-auth crate | ✅ Complete | 380 |
| Database migrations | ✅ Complete | - |
| HTTP routes | ✅ Complete | - |
| ABAC engine | ✅ Complete | 75 |
| WebSocket auth | ✅ Complete | 46 |

### Files Created

**loom-auth crate (19 modules):**
```
crates/loom-auth/src/
├── abac/
│   ├── engine.rs
│   ├── mod.rs
│   ├── policies/
│   │   ├── llm.rs
│   │   ├── mod.rs
│   │   ├── org.rs
│   │   └── thread.rs
│   └── types.rs
├── access_token.rs
├── account_deletion.rs
├── admin.rs
├── api_key.rs
├── audit.rs
├── device_code.rs
├── email.rs
├── error.rs
├── lib.rs
├── magic_link.rs
├── middleware.rs
├── org.rs
├── session.rs
├── share_link.rs
├── support_access.rs
├── team.rs
├── types.rs
└── user.rs
```

**loom-server routes (9 new modules):**
```
crates/loom-server/src/routes/
├── admin.rs
├── api_keys.rs
├── auth.rs (updated)
├── orgs.rs
├── sessions.rs
├── share.rs
├── teams.rs
└── users.rs
```

**Database migrations (7 new):**
```
crates/loom-server/migrations/
├── 008_auth_users.sql
├── 009_auth_sessions.sql
├── 010_auth_orgs.sql
├── 011_auth_teams.sql
├── 012_auth_api_keys.sql
├── 013_auth_threads_ext.sql
└── 014_auth_audit.sql
```

---

## Next Steps

1. ~~**WebSocket Auth**~~ - ✅ Implemented cookie-based and first-message auth for WebSocket connections
2. **OAuth Integration** - Add actual GitHub/Google OAuth client implementations
3. **Database Repositories** - Connect route handlers to database operations
4. **GeoIP Integration** - Add MaxMind database for session location tracking
5. **Rate Limiting** - Add per-IP/per-user rate limits (deferred from v1)

---
---

# Feature Flags & Experiments Implementation Plan

Implementation checklist for the Feature Flags system. See
[specs/feature-flags-system.md](./specs/feature-flags-system.md) for full specification.

---

## ✅ Phase 1: Core Types & Database (COMPLETED)

**Goal:** Establish foundational types and database schema.

**Spec References:**
- Core entities: `specs/feature-flags-system.md:95-232` (Flag, Variant, Strategy, KillSwitch)
- Evaluation types: `specs/feature-flags-system.md:204-232` (EvaluationContext, EvaluationResult)
- Database schema: `specs/feature-flags-system.md:477-573`

**Tasks:**
- [x] Create `crates/loom-flags-core/` crate
  - [x] `flag.rs` - Flag, Variant, VariantValue, FlagPrerequisite types
  - [x] `strategy.rs` - Strategy, Condition, AttributeOperator, Schedule types
  - [x] `kill_switch.rs` - KillSwitch type
  - [x] `environment.rs` - Environment type
  - [x] `sdk_key.rs` - SdkKey, SdkKeyType types
  - [x] `evaluation.rs` - EvaluationContext, EvaluationResult, EvaluationReason
  - [x] `error.rs` - Error types using thiserror
- [x] Create `crates/loom-server-flags/` crate structure
  - [x] `repository.rs` - FlagsRepository trait and SqliteFlagsRepository implementation
  - [x] `evaluation.rs` - Server-side flag evaluation engine
  - [x] `sdk_auth.rs` - SDK key hashing and verification
  - [x] `error.rs` - FlagsServerError types
- [x] Add database migration `030_feature_flags.sql`
  - [x] `flag_environments` table
  - [x] `flags` table with org_id nullable for platform flags
  - [x] `flag_prerequisites` table
  - [x] `flag_configs` table (per-environment)
  - [x] `flag_strategies` table
  - [x] `kill_switches` table
  - [x] `sdk_keys` table
  - [x] `exposure_logs` table
  - [x] `flag_stats` table
- [x] Create repository layer in `loom-server-flags/src/repository.rs`
- [x] Add i18n translations for feature flags (server and web)
- [x] 50 tests (40 in loom-flags-core, 9 in loom-server-flags, 1 doc test)

---

## ✅ Phase 2: Environment & SDK Keys (COMPLETED)

**Goal:** Environment management and SDK key authentication.

**Spec References:**
- Environments: `specs/feature-flags-system.md:176-186` (Environment type)
- Auto-created environments: `specs/feature-flags-system.md:261-269`
- SDK keys: `specs/feature-flags-system.md:188-202` (SdkKey, SdkKeyType)
- SDK key format: `specs/feature-flags-system.md:274-289`
- SDK key endpoints: `specs/feature-flags-system.md:410-413`
- Environment endpoints: `specs/feature-flags-system.md:404-408`

**Tasks:**
- [x] Implement Environment CRUD handlers in `routes/flags.rs`
  - [x] `GET /api/orgs/{org_id}/flags/environments`
  - [x] `POST /api/orgs/{org_id}/flags/environments`
  - [x] `GET /api/orgs/{org_id}/flags/environments/{env_id}`
  - [x] `PATCH /api/orgs/{org_id}/flags/environments/{env_id}`
  - [x] `DELETE /api/orgs/{org_id}/flags/environments/{env_id}`
- [x] Auto-create `dev` and `prod` environments on org creation
  - [x] Hook into org creation flow in `routes/orgs.rs`
- [x] Implement SDK key generation
  - [x] Key format: `loom_sdk_{type}_{env}_{random32hex}`
  - [x] Argon2 hashing for storage
  - [x] Fixed SDK key parsing to handle environment names with underscores
- [x] Implement SDK key CRUD handlers
  - [x] `GET /api/orgs/{org_id}/flags/environments/{env_id}/sdk-keys`
  - [x] `POST /api/orgs/{org_id}/flags/environments/{env_id}/sdk-keys`
  - [x] `DELETE /api/orgs/{org_id}/flags/sdk-keys/{key_id}`
- [x] Add flags API types to `loom-server-api/src/flags.rs`
- [x] Add flags_repo to AppState
- [x] 60+ tests (51 in loom-flags-core, 9 in loom-server-flags)
  - Property-based tests for environment name validation
  - Property-based tests for SDK key generation/parsing roundtrip

---

## ✅ Phase 3: Flag Management (COMPLETED)

**Goal:** Complete flag CRUD with per-environment configuration.

**Spec References:**
- Flag type: `specs/feature-flags-system.md:97-131`
- FlagConfig type: `specs/feature-flags-system.md:133-143`
- Flag key format: `specs/feature-flags-system.md:249-258`
- Flag endpoints: `specs/feature-flags-system.md:370-378`

**Tasks:**
- [x] Flag key validation
  - [x] Pattern: `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`
  - [x] Length: 3-100 characters
- [x] Implement Flag CRUD handlers
  - [x] `GET /api/orgs/{org_id}/flags` - list flags for org
  - [x] `POST /api/orgs/{org_id}/flags` - create flag
  - [x] `GET /api/orgs/{org_id}/flags/{flag_id}` - get flag by ID
  - [x] `PATCH /api/orgs/{org_id}/flags/{flag_id}` - update flag
  - [x] `POST /api/orgs/{org_id}/flags/{flag_id}/archive` - archive flag
  - [x] `POST /api/orgs/{org_id}/flags/{flag_id}/restore` - restore archived flag
- [x] Implement FlagConfig handlers
  - [x] `GET /api/orgs/{org_id}/flags/{flag_id}/configs` - get all environment configs
  - [x] `GET /api/orgs/{org_id}/flags/{flag_id}/configs/{env_id}` - get specific config
  - [x] `PATCH /api/orgs/{org_id}/flags/{flag_id}/configs/{env_id}` - update environment config
- [x] Auto-create configs for all environments on flag creation
- [x] Prerequisites handling
  - [x] Store prerequisite relationships
  - [x] Support in create/update flag
- [x] Property-based tests for flag key validation
- [x] 60 tests (all passing in loom-flags-core)

---

## ✅ Phase 4: Strategy System (COMPLETED)

**Goal:** Rollout strategies with targeting conditions.

**Spec References:**
- Strategy type: `specs/feature-flags-system.md:145-175` (Strategy, Condition, Schedule)
- Evaluation engine: `specs/feature-flags-system.md:301-349`
- Percentage hashing: `specs/feature-flags-system.md:322-328`
- Schedule evaluation: `specs/feature-flags-system.md:330-338`
- GeoIP resolution: `specs/feature-flags-system.md:340-349`
- Strategy endpoints: `specs/feature-flags-system.md:380-386`

**Tasks:**
- [x] Implement Strategy CRUD handlers
  - [x] `GET /api/orgs/{org_id}/flags/strategies`
  - [x] `POST /api/orgs/{org_id}/flags/strategies`
  - [x] `GET /api/orgs/{org_id}/flags/strategies/{strategy_id}`
  - [x] `PATCH /api/orgs/{org_id}/flags/strategies/{strategy_id}`
  - [x] `DELETE /api/orgs/{org_id}/flags/strategies/{strategy_id}`
- [x] Condition evaluation engine
  - [x] Attribute conditions (equals, contains, in, etc.)
  - [x] Geographic conditions (country, region, city)
  - [x] Environment conditions
- [x] Percentage hashing with murmur3
  - [x] Consistent hashing for sticky assignment
  - [x] Configurable key (user_id, org_id, session_id)
- [x] Schedule evaluation
  - [x] Time-based percentage ramps
- [ ] GeoIP integration (deferred to Phase 6 - requires evaluation endpoints)
  - [ ] Integrate with existing `loom-geoip`
  - [ ] Proxy header support (CF-Connecting-IP, X-Forwarded-For, X-Real-IP)
- [x] Strategy API types in `loom-server-api`
- [x] i18n translations (EN, ES, AR)
- [x] 90+ tests including property-based tests for:
  - Attribute operator evaluation
  - Percentage hashing determinism and monotonicity
  - Schedule evaluation
  - Geographic operator case-insensitivity

---

## ✅ Phase 5: Kill Switches (COMPLETED)

**Goal:** Emergency shutoff mechanism with flag linking.

**Spec References:**
- KillSwitch type: `specs/feature-flags-system.md:178-193`
- Kill switch design: `specs/feature-flags-system.md:291-299`
- Activation/deactivation flow: `specs/feature-flags-system.md:301-318`
- Kill switch endpoints: `specs/feature-flags-system.md:388-395`

**Tasks:**
- [x] Implement Kill switch CRUD handlers
  - [x] `GET /api/orgs/{org_id}/flags/kill-switches`
  - [x] `POST /api/orgs/{org_id}/flags/kill-switches`
  - [x] `GET /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}`
  - [x] `PATCH /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}`
  - [x] `DELETE /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}`
- [x] Activation endpoint
  - [x] `POST /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}/activate`
  - [x] Required: `reason` field (validation enforced)
  - [x] Set `activated_at`, `activated_by`, `activation_reason`
- [x] Deactivation endpoint
  - [x] `POST /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}/deactivate`
  - [x] Clear activation fields
- [x] Kill switch permissions
  - [x] Uses org membership (same as other flags operations)
  - [x] Any org member can manage kill switches
- [x] i18n translations (server: loom-common-i18n, web: loom-web)
- [x] API types in `loom-server-api/src/flags.rs`
- [x] Property-based tests (6 new tests for kill switch behavior)
- [x] 77 tests passing in loom-flags-core

---

## ✅ Phase 6: Evaluation Engine (COMPLETED)

**Goal:** Complete flag evaluation with all precedence rules.

**Spec References:**
- Evaluation order: `specs/feature-flags-system.md:303-320`
- Precedence rules: `specs/feature-flags-system.md:241-246`
- Evaluation endpoints: `specs/feature-flags-system.md:415-418`

**Tasks:**
- [x] Implement full evaluation flow in `loom-server-flags/src/evaluation.rs`
  1. Check flag exists
  2. Check environment config (enabled/disabled)
  3. Check kill switches (platform first, then org)
  4. Check prerequisites
  5. Evaluate strategy (conditions, percentage, schedule)
  6. Return variant with reason
- [x] Platform vs org precedence
  - [x] Platform flags override org flags with same key
  - [x] Platform kill switches affect all orgs
- [x] Implement evaluation endpoints
  - [x] `POST /api/orgs/{org_id}/flags/evaluate` - evaluate all flags for context
  - [x] `POST /api/orgs/{org_id}/flags/{flag_key}/evaluate` - evaluate single flag
- [x] Return EvaluationResult with reason
- [x] API types for evaluation (EvaluationContextApi, EvaluationResultApi, EvaluationReasonApi)
- [x] 96 tests passing (77 in loom-flags-core, 19 in loom-server-flags)

---

## ✅ Phase 7: SSE Streaming (COMPLETED)

**Goal:** Real-time flag updates via Server-Sent Events.

**Spec References:**
- SSE events: `specs/feature-flags-system.md:420-450`
- Event format: `specs/feature-flags-system.md:436-445`
- Reconnection: `specs/feature-flags-system.md:447-450`

**Tasks:**
- [x] Implement SSE endpoint
  - [x] `GET /api/flags/stream`
  - [x] SDK key authentication with Argon2 verification
- [x] Event types in `loom-flags-core/src/sse.rs`
  - [x] `init` - full state on connect
  - [x] `flag.updated` - flag or config changed
  - [x] `flag.archived` - flag archived
  - [x] `flag.restored` - flag restored from archive
  - [x] `killswitch.activated` - kill switch activated
  - [x] `killswitch.deactivated` - kill switch deactivated
  - [x] `heartbeat` - every 30s (via axum SSE KeepAlive)
- [x] Broadcast mechanism in `loom-server-flags/src/sse.rs`
  - [x] Per-environment channels (org_id, environment_id)
  - [x] Notify on flag/kill switch changes
  - [x] Broadcast to entire org for org-wide changes
- [x] Client connection management
  - [x] FlagsBroadcaster with channel statistics
  - [x] Clean up empty channels
  - [x] Connection tracking metrics
- [x] Event emission on changes
  - [x] update_flag_config broadcasts flag.updated
  - [x] archive_flag broadcasts flag.archived
  - [x] restore_flag broadcasts flag.restored
  - [x] activate_kill_switch broadcasts killswitch.activated
  - [x] deactivate_kill_switch broadcasts killswitch.deactivated
- [x] Stats endpoint `GET /api/flags/stream/stats` (admin only)
- [x] i18n translations (EN, ES, AR)
- [x] 120 tests (91 in loom-flags-core, 29 in loom-server-flags)

---

## Phase 8: Exposure Tracking

**Goal:** Track flag evaluations for experiment analysis.

**Spec References:**
- Exposure logging: `specs/feature-flags-system.md:351-378`
- Exposure endpoints: `specs/feature-flags-system.md:420-423`

**Tasks:**
- [ ] Implement ExposureLog creation
  - [ ] Log on each evaluation
  - [ ] Include flag, variant, context, reason
- [ ] Deduplication logic
  - [ ] Hash evaluation context
  - [ ] Only log first per context hash per hour
- [ ] Per-flag exposure toggle
  - [ ] Add `exposure_tracking_enabled` to Flag
- [ ] Implement exposure endpoints
  - [ ] `GET /api/flags/exposures` - query exposure logs
  - [ ] `POST /api/flags/exposures/export` - bulk export

---

## Phase 9: Stale Detection & Stats

**Goal:** Track flag usage and identify stale flags.

**Spec References:**
- Staleness criteria: `specs/feature-flags-system.md:380-385`
- Flag stats: `specs/feature-flags-system.md:387-394`
- Stats endpoints: `specs/feature-flags-system.md:420-423`

**Tasks:**
- [ ] Implement FlagStats tracking
  - [ ] Update `last_evaluated_at` on evaluation
  - [ ] Increment evaluation counts
- [ ] Evaluation count rollups
  - [ ] Background job for 24h/7d/30d counts
- [ ] Stale flag detection
  - [ ] `GET /api/flags/stale`
  - [ ] Return flags not evaluated in 30 days
- [ ] Flag stats endpoint
  - [ ] `GET /api/flags/{key}/stats`

---

## Phase 10: Rust SDK

**Goal:** `loom-flags` crate for Rust clients.

**Spec References:**
- SDK design: `specs/feature-flags-system.md:452-493`
- SDK behavior: `specs/feature-flags-system.md:489-497`
- Crate structure: `specs/feature-flags-system.md:16-37`

**Tasks:**
- [ ] Create `crates/loom-flags/` crate
- [ ] Implement FlagsClient
  - [ ] Builder pattern for configuration
  - [ ] SDK key authentication
  - [ ] Base URL configuration
- [ ] Initialization
  - [ ] Fetch all flags on init
  - [ ] Start SSE connection
- [ ] Local caching
  - [ ] In-memory flag cache
  - [ ] Update from SSE events
- [ ] Evaluation methods
  - [ ] `get_bool(key, context, default)`
  - [ ] `get_string(key, context, default)`
  - [ ] `get_json(key, context, default)`
  - [ ] `get_all(context)`
- [ ] Offline mode
  - [ ] Use last cached values when disconnected
- [ ] Use `loom-http` for requests
  - [ ] Retry logic
  - [ ] User-Agent header

---

## Phase 11: TypeScript Packages

**Goal:** `@loom/http` and `@loom/flags` packages.

**Spec References:**
- TypeScript SDK: `specs/feature-flags-system.md:474-487`
- Package structure: `specs/feature-flags-system.md:39-53`

**Tasks:**
- [ ] Create `web/packages/http/` package (`@loom/http`)
  - [ ] HTTP client with fetch
  - [ ] Retry with exponential backoff
  - [ ] Standard headers (User-Agent, Content-Type)
  - [ ] Error handling
- [ ] Create `web/packages/flags/` package (`@loom/flags`)
  - [ ] FlagsClient class
  - [ ] SDK key authentication
  - [ ] Initialization with flag fetch
  - [ ] SSE connection handling
  - [ ] Local caching
  - [ ] Evaluation methods (getBool, getString, getJson)
  - [ ] Event emitter for updates
  - [ ] Offline mode with cached values

---

## Phase 12: Audit Integration

**Goal:** Full audit logging for all flag operations.

**Spec References:**
- Audit events: `specs/feature-flags-system.md:575-593`

**Tasks:**
- [ ] Add audit event types to `loom-server-audit`
  - [ ] `FlagCreated`, `FlagUpdated`, `FlagArchived`, `FlagRestored`
  - [ ] `FlagConfigUpdated`
  - [ ] `StrategyCreated`, `StrategyUpdated`, `StrategyDeleted`
  - [ ] `KillSwitchCreated`, `KillSwitchActivated`, `KillSwitchDeactivated`, `KillSwitchDeleted`
  - [ ] `SdkKeyCreated`, `SdkKeyRevoked`
  - [ ] `EnvironmentCreated`, `EnvironmentDeleted`
- [ ] Integrate audit logging into all handlers
- [ ] Test audit logging

---

## Phase 13: Platform Flags

**Goal:** Super admin management of platform-level flags.

**Spec References:**
- Two-tier system: `specs/feature-flags-system.md:235-239`
- Precedence: `specs/feature-flags-system.md:241-246`
- Platform endpoints: `specs/feature-flags-system.md:425-432`
- Permissions: `specs/feature-flags-system.md:595-618`

**Tasks:**
- [ ] Implement platform flag endpoints (super admin only)
  - [ ] `GET /api/admin/flags`
  - [ ] `POST /api/admin/flags`
  - [ ] `PATCH /api/admin/flags/{key}`
  - [ ] `DELETE /api/admin/flags/{key}`
- [ ] Implement platform kill switch endpoints
  - [ ] `GET /api/admin/flags/kill-switches`
  - [ ] `POST /api/admin/flags/kill-switches`
- [ ] Update evaluation engine for platform precedence
  - [ ] Check platform flags first
  - [ ] Platform overrides org config
- [ ] Super admin impersonation support
  - [ ] Allow super admin to manage org flags as org admin

---

## Feature Flags Dependencies

**Rust Crates (per `specs/feature-flags-system.md:620-639`):**
- `chrono` - timestamps
- `serde`, `serde_json` - serialization
- `thiserror` - error types
- `uuid` - IDs
- `murmur3` - percentage hashing
- `eventsource-stream` - SSE client
- `sqlx` - database

**Integration Points:**
- `loom-http` - HTTP client with retry
- `loom-geoip` - GeoIP resolution
- `loom-server-audit` - audit logging
- `loom-db` - database layer
- `loom-auth` - ABAC permissions

---

## Feature Flags Testing Strategy

- [ ] Unit tests for evaluation engine
- [ ] Unit tests for condition matching
- [ ] Unit tests for percentage hashing (verify consistency)
- [ ] Integration tests for API endpoints
- [ ] Integration tests for SSE streaming
- [ ] Property-based tests for strategy evaluation
- [ ] SDK integration tests

---

## Feature Flags Deployment Notes

- Database migration must run before server starts
- Auto-create environments on org creation requires migration to existing orgs
- SSE requires appropriate timeout settings in load balancer
- SDK keys should be rotated if exposed
