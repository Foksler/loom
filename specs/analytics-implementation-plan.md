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

## Phase 2: Database Schema

**Reference:** [analytics-system.md §9](./analytics-system.md#9-database-schema)

- [ ] Create migration `crates/loom-server/migrations/032_analytics.sql`
  - `analytics_persons` table
  - `analytics_person_identities` table
  - `analytics_events` table
  - `analytics_person_merges` table
  - `analytics_api_keys` table
  - All indexes as specified
  - Pattern: follow `crates/loom-server/migrations/030_feature_flags.sql`

- [ ] Run `cargo2nix-update` after adding migration (per CLAUDE.md)

---

## Phase 3: Server Repository Layer (`loom-server-analytics`)

**Reference:** [analytics-system.md §2](./analytics-system.md#2-architecture)

- [ ] Create `crates/loom-server-analytics/Cargo.toml`
  - Dependencies: `loom-analytics-core`, `loom-db`, `loom-server-audit`, `loom-secret`, `axum`, `sqlx`, `argon2`

- [ ] Create `crates/loom-server-analytics/src/lib.rs`

- [ ] Create `crates/loom-server-analytics/src/repository.rs`
  - `AnalyticsRepository` struct
  - CRUD for `analytics_persons`
  - CRUD for `analytics_person_identities`
  - Insert for `analytics_events`
  - Query for `analytics_events` with filters
  - Pattern: follow `crates/loom-server-flags/src/repository.rs`

- [ ] Create `crates/loom-server-analytics/src/api_key.rs`
  - `AnalyticsApiKeyRepository`
  - Key generation with prefix
  - Argon2 hashing (pattern: `crates/loom-server-flags/src/handlers/sdk_keys.rs`)
  - Validation middleware

---

## Phase 4: Identity Resolution

**Reference:** [analytics-system.md §4](./analytics-system.md#4-identity-resolution)

- [ ] Create `crates/loom-server-analytics/src/identity_resolution.rs`
  - `resolve_person_for_distinct_id(org_id, distinct_id)` → creates Person if needed
  - `identify(org_id, IdentifyPayload)` → links anonymous to identified
  - `alias(org_id, AliasPayload)` → links two distinct_ids
  - Person merge logic per [analytics-system.md §4.3](./analytics-system.md#43-person-merge)
    - Winner selection rules
    - Event reassignment
    - Identity transfer
    - Property merge (winner precedence)

- [ ] Add `analytics_person_merges` audit trail insert

---

## Phase 5: API Handlers

**Reference:** [analytics-system.md §7](./analytics-system.md#7-api-endpoints)

- [ ] Create `crates/loom-server-analytics/src/handlers/mod.rs`

- [ ] Create `crates/loom-server-analytics/src/handlers/capture.rs`
  - `POST /api/analytics/capture` - single event
  - `POST /api/analytics/batch` - batch events
  - Add automatic properties (`$ip`, `$user_agent`, `$lib`, etc.) per [§5.2](./analytics-system.md#52-automatic-properties)
  - Validate event per [§14.3](./analytics-system.md#143-event-validation)

- [ ] Create `crates/loom-server-analytics/src/handlers/identify.rs`
  - `POST /api/analytics/identify`
  - `POST /api/analytics/alias`
  - `POST /api/analytics/set` (person properties)

- [ ] Create `crates/loom-server-analytics/src/handlers/persons.rs`
  - `GET /api/analytics/persons` (requires ReadWrite key)
  - `GET /api/analytics/persons/{id}`
  - `GET /api/analytics/persons/by-distinct-id/{distinct_id}`

- [ ] Create `crates/loom-server-analytics/src/handlers/events.rs`
  - `GET /api/analytics/events` (requires ReadWrite key)
  - `GET /api/analytics/events/count`
  - `POST /api/analytics/events/export`

- [ ] Create `crates/loom-server-analytics/src/handlers/api_keys.rs`
  - `GET /api/analytics/api-keys` (requires User Auth)
  - `POST /api/analytics/api-keys`
  - `DELETE /api/analytics/api-keys/{id}`

- [ ] Create `crates/loom-server-analytics/src/routes.rs`
  - Mount all handlers
  - Apply API key auth middleware for capture/query routes
  - Apply user auth middleware for management routes
  - Pattern: follow `crates/loom-server-flags/src/routes.rs`

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
