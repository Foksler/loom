<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Analytics System Implementation Plan

Implementation checklist for `specs/analytics-system.md`. Each item cites the relevant specification section and source code to modify.

---

## Phase 1: Core Types (`loom-analytics-core`) ✅ COMPLETED

**Reference:** [analytics-system.md §3](./analytics-system.md#3-core-entities)

**Completed in commit:** d1dd21f (2026-01-11)

- [x] Create `crates/loom-analytics-core/Cargo.toml`
  - Dependencies: `chrono`, `serde`, `serde_json`, `thiserror`, `uuid` (v4/v7), `loom-common-secret`
  - See [analytics-system.md §15](./analytics-system.md#15-rust-dependencies)

- [x] Create `crates/loom-analytics-core/src/lib.rs`
  - Re-export all types

- [x] Create `crates/loom-analytics-core/src/person.rs`
  - `Person` struct with `id`, `org_id`, `properties`, timestamps
  - `PersonWithIdentities` wrapper
  - See [analytics-system.md §3.1](./analytics-system.md#31-person)

- [x] Create `crates/loom-analytics-core/src/identity.rs`
  - `PersonIdentity` struct with `distinct_id`, `identity_type`
  - `IdentityType` enum: `Anonymous`, `Identified`
  - See [analytics-system.md §3.2](./analytics-system.md#32-personidentity)

- [x] Create `crates/loom-analytics-core/src/event.rs`
  - `Event` struct with `ip_address: Option<SecretString>`
  - Use `loom-common-secret` for IP addresses per [analytics-system.md §14.1](./analytics-system.md#141-ip-address-handling)
  - See [analytics-system.md §3.3](./analytics-system.md#33-event)

- [x] Create `crates/loom-analytics-core/src/identify.rs`
  - `IdentifyPayload`, `AliasPayload`, `SetPayload`, `SetOncePayload`, `UnsetPayload` structs
  - `PersonMerge` and `MergeReason` for merge audit trail
  - See [analytics-system.md §4.2](./analytics-system.md#42-identify-operation)

- [x] Create `crates/loom-analytics-core/src/api_key.rs`
  - `AnalyticsApiKey` struct
  - `AnalyticsKeyType` enum: `Write`, `ReadWrite`
  - Key format: `loom_analytics_write_` / `loom_analytics_rw_`
  - See [analytics-system.md §3.4](./analytics-system.md#34-analytics-api-key), [§10](./analytics-system.md#10-api-key-management)

- [x] Create `crates/loom-analytics-core/src/error.rs`
  - Error types using `thiserror`
  - Pattern: follow `crates/loom-flags-core/src/error.rs`

- [x] Add to workspace `Cargo.toml`

- [x] Run `cargo2nix-update` to regenerate `Cargo.nix`

**Tests:** 75 property-based and unit tests passing

---

## Phase 2: Database Schema ✅ COMPLETED

**Reference:** [analytics-system.md §9](./analytics-system.md#9-database-schema)

**Completed in commit:** (2026-01-11)

- [x] Create migration `crates/loom-server/migrations/032_analytics.sql`
  - `analytics_persons` table
  - `analytics_person_identities` table
  - `analytics_events` table
  - `analytics_person_merges` table
  - `analytics_api_keys` table
  - All indexes as specified
  - Composite index `idx_analytics_events_org_timestamp` for common query pattern
  - Pattern: follow `crates/loom-server/migrations/030_feature_flags.sql`

- [x] Run `cargo2nix-update` after adding migration (per CLAUDE.md)

---

## Phase 3: Server Repository Layer (`loom-server-analytics`) ✅ COMPLETED

**Reference:** [analytics-system.md §2](./analytics-system.md#2-architecture)

**Completed in commit:** (2026-01-11)

- [x] Create `crates/loom-server-analytics/Cargo.toml`
  - Dependencies: `loom-analytics-core`, `loom-common-secret`, `sqlx`, `argon2`, `async-trait`, `tracing`

- [x] Create `crates/loom-server-analytics/src/lib.rs`
  - Re-exports all modules and core types

- [x] Create `crates/loom-server-analytics/src/repository.rs`
  - `AnalyticsRepository` trait with all CRUD operations
  - `SqliteAnalyticsRepository` implementation
  - CRUD for `analytics_persons`
  - CRUD for `analytics_person_identities`
  - Insert/query for `analytics_events` with filters
  - CRUD for `analytics_person_merges`
  - CRUD for `analytics_api_keys`
  - Pattern: follows `crates/loom-server-flags/src/repository.rs`

- [x] Create `crates/loom-server-analytics/src/api_key.rs`
  - `hash_api_key()` - Argon2 hashing
  - `verify_api_key()` - Key verification
  - Pattern: follows `crates/loom-server-flags/src/sdk_auth.rs`

- [x] Create `crates/loom-server-analytics/src/error.rs`
  - `AnalyticsServerError` enum using `thiserror`

- [x] Add to workspace `Cargo.toml`

- [x] Run `cargo2nix-update` to regenerate `Cargo.nix`

**Tests:** 9 property-based and unit tests passing

---

## Phase 4: Identity Resolution ✅ COMPLETED

**Reference:** [analytics-system.md §4](./analytics-system.md#4-identity-resolution)

**Completed in commit:** (2026-01-11)

- [x] Create `crates/loom-server-analytics/src/identity_resolution.rs`
  - `resolve_person_for_distinct_id(org_id, distinct_id)` → creates Person if needed
  - `identify(org_id, IdentifyPayload)` → links anonymous to identified
  - `alias(org_id, AliasPayload)` → links two distinct_ids
  - Person merge logic per [analytics-system.md §4.3](./analytics-system.md#43-person-merge)
    - Winner selection rules (identified > anonymous, older > newer)
    - Event reassignment via `reassign_events()`
    - Identity transfer via `transfer_identities()`
    - Property merge (winner precedence, loser fills gaps)

- [x] Add `analytics_person_merges` audit trail insert

**Tests:** 14 unit tests passing covering all identity resolution scenarios

---

## Phase 5: API Handlers ✅ COMPLETED

**Reference:** [analytics-system.md §7](./analytics-system.md#7-api-endpoints)

**Completed in commit:** (2026-01-11)

- [x] Create `crates/loom-server-analytics/src/handlers/mod.rs`

- [x] Create `crates/loom-server-analytics/src/handlers/capture.rs`
  - `capture_event_impl` - single event capture
  - `batch_capture_impl` - batch events
  - Add automatic properties (`$ip`, `$user_agent`, `$lib`, etc.) per [§5.2](./analytics-system.md#52-automatic-properties)
  - Validate event per [§14.3](./analytics-system.md#143-event-validation)

- [x] Create `crates/loom-server-analytics/src/handlers/identify.rs`
  - `identify_impl` - identify user
  - `alias_impl` - create alias
  - `set_properties_impl` - set person properties

- [x] Create `crates/loom-server-analytics/src/handlers/persons.rs`
  - `list_persons_impl` (requires ReadWrite key)
  - `get_person_impl`
  - `get_person_by_distinct_id_impl`

- [x] Create `crates/loom-server-analytics/src/handlers/events.rs`
  - `list_events_impl` (requires ReadWrite key)
  - `count_events_impl`
  - `export_events_impl`

- [x] Create `crates/loom-server-analytics/src/handlers/api_keys.rs`
  - `list_api_keys_impl` (requires User Auth)
  - `create_api_key_impl`
  - `revoke_api_key_impl`

- [x] Create `crates/loom-server-analytics/src/routes.rs`
  - Exports all handler implementations for use in loom-server
  - Auth middleware applied in loom-server integration layer

- [x] Create `crates/loom-server-analytics/src/middleware.rs`
  - `AnalyticsApiKeyContext` for API key auth
  - `parse_key_type` and `extract_bearer_token` utilities

- [x] Add API types to `crates/loom-server-api/src/analytics.rs`
  - Request/response types for all endpoints
  - OpenAPI schema support via utoipa

**Tests:** 42 unit tests passing (handlers, validation, middleware)

---

## Phase 6: Integration with loom-server

**Reference:** [analytics-system.md §2](./analytics-system.md#2-architecture)

- [ ] Update `crates/loom-server/Cargo.toml`
  - Add `loom-server-analytics` dependency

- [ ] Update `crates/loom-server/src/routes/mod.rs`
  - Mount analytics routes at `/api/analytics/*`
  - Pattern: follow how flags routes are mounted

- [ ] Add configuration for analytics
  - `LOOM_ANALYTICS_ENABLED`
  - `LOOM_ANALYTICS_BATCH_SIZE`
  - `LOOM_ANALYTICS_EVENT_RETENTION_DAYS`
  - See [analytics-system.md §11](./analytics-system.md#11-configuration)

---

## Phase 7: Experiment Integration

**Reference:** [analytics-system.md §6](./analytics-system.md#6-experiment-integration)

- [ ] Update `crates/loom-flags/src/client.rs`
  - When flag evaluated, optionally call analytics capture
  - Event: `$feature_flag_called` with `$feature_flag` and `$feature_flag_response` properties
  - See [analytics-system.md §6.1](./analytics-system.md#61-feature-flag-exposure-tracking)

- [ ] Document query pattern for experiment analysis
  - Join `exposure_logs` with `analytics_events`
  - See [analytics-system.md §6.3](./analytics-system.md#63-experiment-metrics)

---

## Phase 8: Rust SDK (`loom-analytics`)

**Reference:** [analytics-system.md §8.1](./analytics-system.md#81-rust-sdk-loom-analytics)

- [ ] Create `crates/loom-analytics/Cargo.toml`
  - Dependencies: `loom-analytics-core`, `loom-http`, `tokio`, `reqwest`, `tracing`

- [ ] Create `crates/loom-analytics/src/lib.rs`
  - Re-export `AnalyticsClient`, `Properties`

- [ ] Create `crates/loom-analytics/src/client.rs`
  - `AnalyticsClient` with builder pattern
  - `capture(event, distinct_id, properties)`
  - `identify(distinct_id, user_id, properties)`
  - `alias(distinct_id, alias)`
  - `set(distinct_id, properties)`
  - `shutdown()` - flush pending events

- [ ] Create `crates/loom-analytics/src/batch.rs`
  - Event queue with background flush
  - Flush on interval (default 10s) or batch size (default 10)
  - Use `loom-http` for requests with retry
  - See [analytics-system.md §8.3](./analytics-system.md#83-sdk-behavior)

- [ ] Create `crates/loom-analytics/src/error.rs`

- [ ] Add to workspace `Cargo.toml`

---

## Phase 9: TypeScript SDK (`@loom/analytics`)

**Reference:** [analytics-system.md §8.2](./analytics-system.md#82-typescript-sdk-loomanalytics)

- [ ] Create `web/packages/analytics/package.json`
  - Dependencies: `@loom/http`

- [ ] Create `web/packages/analytics/src/index.ts`
  - Export `AnalyticsClient`

- [ ] Create `web/packages/analytics/src/client.ts`
  - `AnalyticsClient` class
  - `capture(event, properties)`
  - `identify(userId, properties)`
  - `alias(alias)`
  - `reset()`
  - `getDistinctId()`

- [ ] Create `web/packages/analytics/src/storage.ts`
  - Generate UUIDv7 for distinct_id
  - Store in localStorage + cookie (cross-subdomain)
  - Cookie name: `loom_analytics_distinct_id`
  - See PostHog persistence patterns

- [ ] Create `web/packages/analytics/src/batch.ts`
  - Event queue with background flush
  - Flush on interval (10s) or batch size (10)
  - Retry with exponential backoff via `@loom/http`

- [ ] Add autocapture option
  - `$pageview` on page load
  - `$pageleave` on page unload
  - See [analytics-system.md §5.3](./analytics-system.md#53-special-events)

- [ ] Update `web/packages/http/` if needed
  - Ensure shared HTTP client with retry exists
  - Pattern: follow `web/packages/flags/` if it exists

---

## Phase 10: Audit Integration

**Reference:** [analytics-system.md §12](./analytics-system.md#12-audit-events)

- [ ] Add audit event types to `crates/loom-server-audit/`
  - `AnalyticsApiKeyCreated`
  - `AnalyticsApiKeyRevoked`
  - `AnalyticsPersonMerged`
  - `AnalyticsEventsExported`
  - Pattern: follow existing audit events in `crates/loom-server-audit/src/events.rs`

- [ ] Call audit logging from handlers
  - API key create/revoke
  - Person merge
  - Bulk export

---

## Phase 11: Authorization Tests

**Reference:** [CLAUDE.md routes section](../CLAUDE.md)

- [ ] Create `crates/loom-server/tests/authz_analytics_tests.rs`
  - Test Write key can only capture, not query
  - Test ReadWrite key can capture and query
  - Test User auth required for API key management
  - Pattern: follow `crates/loom-server/tests/authz_*_tests.rs`

---

## Phase 12: Documentation

- [ ] Add inline rustdoc to all public types

- [ ] Update main README if analytics is a significant feature

---

## Files to Create

```
crates/
├── loom-analytics-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── person.rs
│       ├── identity.rs
│       ├── event.rs
│       ├── identify.rs
│       ├── api_key.rs
│       └── error.rs
├── loom-analytics/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client.rs
│       ├── batch.rs
│       └── error.rs
├── loom-server-analytics/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── routes.rs
│       ├── repository.rs
│       ├── api_key.rs
│       ├── identity_resolution.rs
│       └── handlers/
│           ├── mod.rs
│           ├── capture.rs
│           ├── identify.rs
│           ├── persons.rs
│           ├── events.rs
│           └── api_keys.rs
├── loom-server/
│   └── migrations/
│       └── 032_analytics.sql

web/
└── packages/
    └── analytics/
        ├── package.json
        └── src/
            ├── index.ts
            ├── client.ts
            ├── storage.ts
            └── batch.ts
```

---

## Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `loom-analytics-core`, `loom-analytics`, `loom-server-analytics` to members |
| `crates/loom-server/Cargo.toml` | Add `loom-server-analytics` dependency |
| `crates/loom-server/src/routes/mod.rs` | Mount `/api/analytics/*` routes |
| `crates/loom-server/src/config.rs` | Add `LOOM_ANALYTICS_*` env vars |
| `crates/loom-server-audit/src/events.rs` | Add analytics audit event types |
| `crates/loom-flags/src/client.rs` | Optional: auto-capture `$feature_flag_called` |
| `Cargo.nix` | Regenerate via `cargo2nix-update` |

---

## Verification Checklist

After implementation:

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo fmt --all` applied
- [ ] `cargo2nix-update` run if Cargo.lock changed
- [ ] Migration runs on fresh database
- [ ] Capture endpoint accepts events
- [ ] Identify links anonymous to authenticated
- [ ] API keys authenticate correctly
- [ ] Rust SDK can capture and identify
- [ ] TypeScript SDK can capture and identify
- [ ] Distinct_id persists across page reloads (browser)
- [ ] Events visible in query endpoint (with ReadWrite key)
