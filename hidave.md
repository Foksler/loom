<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Observability Suite Implementation Plan

**Status:** In Progress\
**Version:** 1.1\
**Last Updated:** 2026-01-19

### Recent Progress

**2026-01-19:** Added SSE stream endpoint for crons monitoring ✅ VERIFIED IN PRODUCTION
- Added `GET /api/crons/stream?org_id={org_id}` SSE endpoint for real-time cron events
- Created `CronStreamEvent` types in `loom-crons-core/src/sse.rs` for event serialization
- Created `CronsBroadcaster` in `loom-server-crons/src/sse.rs` for per-org event broadcasting
- Added `crons_broadcaster` to AppState in `api.rs`
- Events broadcast: `init`, `checkin.started`, `checkin.ok`, `checkin.error`, `monitor.missed`, `monitor.timeout`, `monitor.healthy`, `heartbeat`
- All ping handlers and SDK endpoints now broadcast events after check-ins
- Added 3 authorization tests for stream endpoint (authenticated, unauthenticated, cross-org isolation)
- Verified working in production: init event returns monitors, ping triggers checkin.ok broadcast
- Commit: `0e8e538`
- Crons monitoring system is now FEATURE COMPLETE (all routes implemented)

**2026-01-19:** Added crons authorization tests and fixed cross-org security issue
- Created `crates/loom-server/tests/authz/crons.rs` with 26 comprehensive authorization tests
- Fixed security vulnerability: crons API endpoints weren't checking org membership
- Added `verify_org_membership()` to all authenticated crons handlers
- Tests cover: ping endpoints (public), monitor CRUD, check-in operations, cross-org isolation
- All authenticated endpoints now properly return 403 Forbidden for non-members
- Verified working in production via curl
- Commit: `3c50bda`

**2026-01-19:** Added missed run and timeout detector background jobs
- Added `list_overdue_monitors()` and `list_timed_out_checkins()` to CronsRepository
- Created `CronMissedRunDetectorJob` for detecting monitors that miss expected check-ins
- Created `CronTimeoutDetectorJob` for detecting in-progress check-ins exceeding max_runtime
- Both jobs registered in main.rs, running every 60 seconds
- Verified working via curl: monitor correctly transitions to "missed" health, creates system check-in
- Commit: `4347d1e`

**2026-01-19:** Added cron schedule parsing and next_expected_at calculation
- Added `schedule.rs` module with `calculate_next_expected()` function
- Support 5-field Unix cron expressions (auto-converted to 7-field format for cron crate)
- Support interval-based schedules
- Validate cron expressions and IANA timezones
- Wire up `next_expected_at` calculation in all check-in handlers
- Monitor creation now calculates initial `next_expected_at`
- Commits: `5b30c4d` (schedule implementation), `91334b4` (Cargo.nix fix)

**2026-01-19:** Added SDK check-in endpoints for programmatic cron monitoring
- Added `POST /api/crons/monitors/{slug}/checkins` for SDK check-in creation
- Added `PATCH /api/crons/checkins/{id}` for updating in-progress check-ins
- Added `GET /api/crons/checkins/{id}` for retrieving check-in details
- Full SDK monitoring flow verified via curl (in_progress → ok, error check-ins)
- Monitor health state correctly updates on check-in completion
- Commit: `87ecbb0` (SDK check-in endpoints)

**2026-01-19:** Completed crons monitoring system MVP
- Created `loom-crons-core` crate with core types (Monitor, CheckIn, Stats)
- Created `loom-server-crons` crate with SQLite repository
- Wired up HTTP routes in `loom-server/src/routes/crons.rs`
- Added nginx proxy for `/ping/` endpoints
- All ping endpoints verified working: `/ping/{key}`, `/ping/{key}/start`, `/ping/{key}/fail`
- API endpoints verified: create/list/get/delete monitors, list check-ins
- Commits: `2987673` (initial implementation), `034b4cb` (nginx proxy fix)

This document provides a detailed, phased implementation plan for Loom's observability suite: crash analytics, cron monitoring, session tracking, and unified UI. All work follows existing codebase patterns.

---

## Quick Reference

| System | Spec | Crates | Web Packages | Migration |
|--------|------|--------|--------------|-----------|
| Crash | [specs/crash-system.md](specs/crash-system.md) | `loom-crash-core`, `loom-crash`, `loom-crash-symbolicate`, `loom-server-crash` | `@loom/crash` | `033_crash_analytics.sql` |
| Crons | [specs/crons-system.md](specs/crons-system.md) | `loom-crons-core`, `loom-crons`, `loom-server-crons` | `@loom/crons` | `034_cron_monitoring.sql` |
| Sessions | [specs/sessions-system.md](specs/sessions-system.md) | `loom-sessions-core`, `loom-server-sessions` | (in `@loom/crash`) | `035_sessions.sql` (tables: `app_sessions`, `app_session_aggregates`) |
| UI | [specs/observability-ui.md](specs/observability-ui.md) | — | `web/loom-web/src/lib/components/` | — |

---

## Phase 1: Database Foundation ✅ COMPLETED

**Goal:** Create all database tables and indexes for the observability suite.

**Status:** Completed 2026-01-18

### 1.1 Create Migration Files

Based on [migration patterns](crates/loom-server/migrations/) (latest: `032_analytics.sql`):

- [x] **`crates/loom-server/migrations/033_crash_analytics.sql`**
  - Tables: `crash_projects`, `crash_api_keys`, `crash_issues`, `crash_events`, `crash_issue_persons`, `symbol_artifacts`, `crash_releases`
  - Indexes for: project lookups, fingerprint matching, timestamp ordering, person correlation
  - Reference: [specs/crash-system.md#12-database-schema](specs/crash-system.md)

- [x] **`crates/loom-server/migrations/034_cron_monitoring.sql`**
  - Tables: `cron_monitors`, `cron_checkins`, `cron_monitor_stats`
  - Indexes for: ping_key lookups, status filtering, next_expected_at ordering
  - Reference: [specs/crons-system.md#10-database-schema](specs/crons-system.md)

- [x] **`crates/loom-server/migrations/035_sessions.sql`**
  - Tables: `app_sessions`, `app_session_aggregates` (renamed from `sessions` to avoid conflict with auth sessions)
  - Indexes for: release health queries, person lookups, time-based aggregation
  - Reference: [specs/sessions-system.md#11-database-schema](specs/sessions-system.md)

### 1.2 Verification

- [x] Run `cargo build -p loom-server` to verify migrations compile
- [x] Run `cargo2nix-update` to regenerate `Cargo.nix` (migrations are `include_str!`)
- [x] Deployed to production and verified migrations ran successfully

**Notes:**
- Session tables renamed to `app_sessions` and `app_session_aggregates` to avoid conflict with existing auth `sessions` table
- Commits: `50bb10f` (initial migrations), `f4b25de` (fix session table naming)

---

## Phase 2: Core Type Crates (Partial)

**Goal:** Create shared type definitions following the `-core` crate pattern.

Reference pattern: [crates/loom-flags-core/](crates/loom-flags-core/), [crates/loom-analytics-core/](crates/loom-analytics-core/)

### 2.1 Create `loom-crash-core`

**Path:** `crates/loom-crash-core/`

**Structure:**
```
loom-crash-core/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public exports
    ├── event.rs         # CrashEvent, Stacktrace, Frame, Platform
    ├── issue.rs         # Issue, IssueStatus, IssueLevel, fingerprinting
    ├── symbol.rs        # SymbolArtifact, ArtifactType
    ├── release.rs       # Release tracking
    ├── project.rs       # CrashProject, CrashApiKey
    ├── context.rs       # UserContext, DeviceContext, BrowserContext, etc.
    ├── breadcrumb.rs    # Breadcrumb, BreadcrumbLevel
    └── error.rs         # Error types with thiserror
```

**Implementation checklist:**
- [ ] Create `Cargo.toml` with dependencies:
  ```toml
  [dependencies]
  chrono = { version = "0.4", features = ["serde"] }
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  thiserror = "2"
  uuid = { version = "1", features = ["v7", "serde"] }
  loom-secret = { path = "../loom-secret" }

  [features]
  openapi = ["utoipa"]

  [dependencies.utoipa]
  version = "5"
  optional = true
  ```
- [ ] Define newtype IDs: `CrashEventId`, `IssueId`, `ProjectId`, `SymbolArtifactId`
- [ ] Implement `CrashEvent` struct ([specs/crash-system.md#31-crashevent](specs/crash-system.md))
- [ ] Implement `Issue` struct with `IssueStatus` enum ([specs/crash-system.md#32-issue](specs/crash-system.md))
- [ ] Implement fingerprinting function ([specs/crash-system.md#4-fingerprinting](specs/crash-system.md))
- [ ] Add `#[cfg_attr(feature = "openapi", derive(ToSchema))]` to all public types
- [ ] Add proptest tests for ID validation

### 2.2 Create `loom-crons-core` ✅ COMPLETED

**Path:** `crates/loom-crons-core/`

**Status:** Completed 2026-01-19

**Structure:**
```
loom-crons-core/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public exports
    ├── monitor.rs       # Monitor, MonitorSchedule, MonitorStatus, MonitorHealth
    ├── checkin.rs       # CheckIn, CheckInStatus, CheckInSource
    ├── stats.rs         # MonitorStats, StatsPeriod
    └── error.rs         # Error types
```

**Implementation checklist:**
- [x] Create `Cargo.toml` (similar to crash-core)
- [x] Define newtype IDs: `MonitorId`, `CheckInId`, `OrgId`
- [x] Implement `Monitor` struct with schedule types ([specs/crons-system.md#31-monitor](specs/crons-system.md))
- [x] Implement `CheckIn` struct ([specs/crons-system.md#32-checkin](specs/crons-system.md))
- [x] Implement `MonitorStats` struct ([specs/crons-system.md#33-monitorstats](specs/crons-system.md))
- [x] Add proptest tests for type roundtrips

### 2.3 Create `loom-sessions-core`

**Path:** `crates/loom-sessions-core/`

**Structure:**
```
loom-sessions-core/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public exports
    ├── session.rs       # Session, SessionStatus
    ├── aggregate.rs     # SessionAggregate
    ├── release_health.rs # ReleaseHealth, AdoptionStage
    └── error.rs         # Error types
```

**Implementation checklist:**
- [ ] Create `Cargo.toml`
- [ ] Define newtype ID: `SessionId`, `SessionAggregateId`
- [ ] Implement `Session` struct ([specs/sessions-system.md#31-session](specs/sessions-system.md))
- [ ] Implement `SessionAggregate` struct ([specs/sessions-system.md#32-sessionaggregate](specs/sessions-system.md))
- [ ] Implement `ReleaseHealth` struct ([specs/sessions-system.md#33-releasehealth](specs/sessions-system.md))

### 2.4 Workspace Integration

- [x] Add crons crates to `Cargo.toml` workspace members ✅
- [ ] Add crash crates to `Cargo.toml` workspace members
- [ ] Add sessions crates to `Cargo.toml` workspace members
- [x] Run `cargo build --workspace` to verify crons compilation ✅
- [x] Run `cargo2nix-update` ✅

---

## Phase 3: Source Map Symbolication

**Goal:** Build the symbolication engine for JavaScript/TypeScript stack traces.

Reference: [specs/crash-system.md#6-symbolication](specs/crash-system.md)

### 3.1 Create `loom-crash-symbolicate`

**Path:** `crates/loom-crash-symbolicate/`

**Structure:**
```
loom-crash-symbolicate/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public exports
    ├── sourcemap.rs     # SourceMap parsing and lookup
    ├── vlq.rs           # VLQ decoder for mappings
    ├── rust.rs          # Rust symbol demangling
    ├── cache.rs         # Symbolication cache
    └── error.rs         # Error types
```

**Implementation checklist:**
- [ ] Create `Cargo.toml`:
  ```toml
  [dependencies]
  loom-crash-core = { path = "../loom-crash-core" }
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  sha2 = "0.10"
  hex = "0.4"
  thiserror = "2"
  rustc-demangle = "0.1"
  ```
- [ ] Implement VLQ decoder ([specs/crash-system.md#63-vlq-decoding](specs/crash-system.md))
  - `decode_vlq_segment()` function
  - `decode_vlq_mappings()` function
- [ ] Implement `ParsedSourceMap` struct
  - `from_bytes()` constructor
  - `lookup(line, col)` method
- [ ] Implement `SourceMapProcessor`
  - `symbolicate_js()` method
  - Source context extraction (pre/post lines)
- [ ] Implement Rust demangling wrapper using `rustc-demangle`
- [ ] Add unit tests with sample source maps

---

## Phase 4: Server Repositories

**Goal:** Implement database access layer following repository trait pattern.

Reference pattern: [crates/loom-server-flags/src/repository.rs](crates/loom-server-flags/src/repository.rs), [crates/loom-server-analytics/src/repository.rs](crates/loom-server-analytics/src/repository.rs)

### 4.1 Create `loom-server-crash`

**Path:** `crates/loom-server-crash/`

**Structure:**
```
loom-server-crash/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── repository.rs    # CrashRepository trait + SqliteCrashRepository
    ├── fingerprint.rs   # Server-side fingerprinting
    ├── symbolicate.rs   # Symbolication pipeline integration
    ├── sse.rs           # SSE broadcaster for crash events
    ├── api_key.rs       # API key validation (Argon2)
    └── handlers/
        ├── mod.rs
        ├── capture.rs   # POST /api/crash/capture
        ├── symbols.rs   # Symbol artifact upload
        ├── issues.rs    # Issue CRUD
        ├── events.rs    # Event queries
        └── releases.rs  # Release management
```

**Implementation checklist:**
- [ ] Create `Cargo.toml`:
  ```toml
  [dependencies]
  loom-crash-core = { path = "../loom-crash-core" }
  loom-crash-symbolicate = { path = "../loom-crash-symbolicate" }
  loom-db = { path = "../loom-db" }
  loom-server-audit = { path = "../loom-server-audit" }
  async-trait = "0.1"
  axum = "0.8"
  sqlx = { version = "0.8", features = ["sqlite"] }
  argon2 = "0.5"
  tokio = { version = "1", features = ["sync"] }
  tokio-stream = "0.1"
  tracing = "0.1"
  ```
- [ ] Define `CrashRepository` trait with methods:
  - `create_project()`, `get_project()`, `list_projects()`
  - `create_issue()`, `get_issue()`, `update_issue()`, `list_issues()`
  - `create_event()`, `get_event()`, `list_events_for_issue()`
  - `create_artifact()`, `get_artifact()`, `list_artifacts()`
  - `create_release()`, `get_release()`, `list_releases()`
- [ ] Implement `SqliteCrashRepository`
- [ ] Implement fingerprinting on ingest ([specs/crash-system.md#41-default-fingerprinting-algorithm](specs/crash-system.md))
- [ ] Implement regression detection ([specs/crash-system.md#52-regression-detection](specs/crash-system.md))
- [ ] Implement API key hashing with Argon2 (pattern: [crates/loom-server-analytics/src/api_key.rs](crates/loom-server-analytics/src/api_key.rs))
- [ ] Implement SSE broadcaster for events

### 4.2 Create `loom-server-crons` ✅ COMPLETED (Repository Layer)

**Path:** `crates/loom-server-crons/`

**Status:** Repository layer completed 2026-01-19. Handlers moved to loom-server/src/routes/crons.rs.

**Structure:**
```
loom-server-crons/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── repository.rs    # CronsRepository trait + SqliteCronsRepository
    └── error.rs         # Error types
```

**Implementation checklist:**
- [x] Create `Cargo.toml`
- [x] Define `CronsRepository` trait
- [x] Implement `SqliteCronsRepository`
- [x] Implement cron expression parser ([specs/crons-system.md#6-schedule-parsing](specs/crons-system.md)) ✅
- [x] Implement `calculate_next_expected()` function ✅
- [x] Implement ping handlers (in loom-server/src/routes/crons.rs) ([specs/crons-system.md#42-ping-endpoints](specs/crons-system.md))
- [x] Implement missed run detector job ([specs/crons-system.md#71-background-scheduler](specs/crons-system.md)) ✅
- [x] Implement timeout detector job ([specs/crons-system.md#72-timeout-detection](specs/crons-system.md)) ✅

### 4.3 Create `loom-server-sessions`

**Path:** `crates/loom-server-sessions/`

**Structure:**
```
loom-server-sessions/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── repository.rs    # SessionsRepository trait + SqliteSessionsRepository
    ├── aggregator.rs    # Hourly aggregation job
    ├── cleanup.rs       # Old session cleanup job
    ├── release_health.rs # Health calculation
    ├── sse.rs           # SSE broadcaster
    └── handlers/
        ├── mod.rs
        ├── sessions.rs  # Session ingest
        └── releases.rs  # Release health queries
```

**Implementation checklist:**
- [ ] Create `Cargo.toml`
- [ ] Define `SessionsRepository` trait
- [ ] Implement `SqliteSessionsRepository`
- [ ] Implement sampling logic ([specs/sessions-system.md#6-sampling](specs/sessions-system.md))
- [ ] Implement hourly aggregation job ([specs/sessions-system.md#71-hourly-aggregation-job](specs/sessions-system.md))
- [ ] Implement cleanup job ([specs/sessions-system.md#72-cleanup-job](specs/sessions-system.md))
- [ ] Implement release health calculation ([specs/sessions-system.md#81-query-for-release-health](specs/sessions-system.md))

---

## Phase 5: HTTP Route Integration

**Goal:** Wire up HTTP handlers in loom-server.

Reference pattern: [crates/loom-server/src/routes/analytics.rs](crates/loom-server/src/routes/analytics.rs), [crates/loom-server/src/routes/flags.rs](crates/loom-server/src/routes/flags.rs)

### 5.1 Add Dependencies to loom-server

- [x] Add `loom-server-crons` dependency ✅
- [ ] Add `loom-server-crash` dependency
- [ ] Add `loom-server-sessions` dependency

### 5.2 Create Route Files

**Path:** `crates/loom-server/src/routes/`

- [ ] **`crash.rs`** — Crash analytics routes
  - `POST /api/crash/capture` — Ingest crash event
  - `POST /api/crash/batch` — Batch ingest
  - `POST /api/crash/projects/{id}/artifacts` — Upload symbols (multipart)
  - `GET /api/crash/projects/{id}/issues` — List issues
  - `GET /api/crash/projects/{id}/issues/{id}` — Issue detail
  - `POST /api/crash/projects/{id}/issues/{id}/resolve` — Resolve issue
  - `GET /api/crash/projects/{id}/stream` — SSE stream
  - Reference: [specs/crash-system.md#9-api-endpoints](specs/crash-system.md)

- [x] **`crons.rs`** — Cron monitoring routes ✅ COMPLETED 2026-01-19
  - `GET /ping/{key}` — Success ping ✅
  - `GET /ping/{key}/start` — Job starting ✅
  - `GET /ping/{key}/fail` — Job failed ✅
  - `POST /ping/{key}` — Ping with body ✅
  - `GET /api/crons/monitors` — List monitors ✅
  - `POST /api/crons/monitors` — Create monitor ✅
  - `GET /api/crons/monitors/{slug}` — Monitor detail ✅
  - `DELETE /api/crons/monitors/{slug}` — Delete monitor ✅
  - `GET /api/crons/monitors/{slug}/checkins` — List check-ins ✅
  - `POST /api/crons/monitors/{slug}/checkins` — SDK check-in ✅
  - `PATCH /api/crons/checkins/{id}` — Update check-in ✅
  - `GET /api/crons/checkins/{id}` — Get check-in ✅
  - `GET /api/crons/stream` — SSE stream ✅
  - Reference: [specs/crons-system.md#8-api-endpoints](specs/crons-system.md)
  - Nginx proxy added in `infra/nixos-modules/loom-web.nix` for `/ping/` routes

- [ ] **`sessions.rs`** — Session analytics routes
  - `POST /api/sessions/start` — Start session
  - `POST /api/sessions/end` — End session
  - `GET /api/projects/{id}/releases` — Release health list
  - `GET /api/projects/{id}/releases/{version}` — Release detail
  - `GET /api/projects/{id}/sessions` — Session list
  - Reference: [specs/sessions-system.md#9-api-endpoints](specs/sessions-system.md)

### 5.3 Register Routes

- [x] Update `crates/loom-server/src/routes/mod.rs` to include crons module ✅
- [x] Update `crates/loom-server/src/api.rs` to add crons_repo to `AppState` ✅
- [x] Wire up cron route handlers in router configuration ✅
  - `/ping/*` routes on PublicRouter (unauthenticated)
  - `/api/crons/*` routes on AuthedRouter (authenticated)
- [ ] Update for crash routes
- [ ] Update for sessions routes

### 5.4 Add OpenAPI Documentation

- [ ] Add `#[utoipa::path(...)]` attributes to all handlers
- [ ] Add request/response types to API schemas
- [ ] Update OpenAPI tags for new sections

---

## Phase 6: Audit Integration

**Goal:** Add audit logging for all observability operations.

Reference: [crates/loom-server-audit/src/event.rs](crates/loom-server-audit/src/event.rs), [specs/crash-system.md#16-audit-events](specs/crash-system.md)

### 6.1 Define Audit Event Types

- [ ] Add to `AuditEventType` enum:
  ```rust
  // Crash events
  CrashProjectCreated,
  CrashProjectUpdated,
  CrashProjectDeleted,
  CrashApiKeyCreated,
  CrashApiKeyRevoked,
  CrashIssueResolved,
  CrashIssueIgnored,
  CrashIssueAssigned,
  CrashIssueDeleted,
  CrashSymbolsUploaded,
  CrashSymbolsDeleted,
  CrashReleaseCreated,

  // Cron events
  CronMonitorCreated,
  CronMonitorUpdated,
  CronMonitorDeleted,
  CronMonitorPaused,
  CronMonitorResumed,

  // Session events (minimal - mostly automated)
  SessionSamplingConfigUpdated,
  ```

### 6.2 Integrate with Handlers

- [ ] Add audit logging to crash handlers
- [ ] Add audit logging to cron handlers
- [ ] Add audit logging to session config handlers

---

## Phase 7: Rust SDK Crates

**Goal:** Create client SDKs for Rust applications.

Reference pattern: [crates/loom-analytics/](crates/loom-analytics/) (if exists), HTTP client: [crates/loom-common-http/](crates/loom-common-http/)

### 7.1 Create `loom-crash`

**Path:** `crates/loom-crash/`

**Structure:**
```
loom-crash/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── client.rs        # CrashClient builder and main API
    ├── panic_hook.rs    # std::panic::set_hook integration
    ├── backtrace.rs     # Backtrace capture and parsing
    ├── context.rs       # Crash context management
    ├── session.rs       # Session tracking (optional)
    ├── transport.rs     # HTTP transport with batching
    └── error.rs         # Error types
```

**Implementation checklist:**
- [ ] Create `Cargo.toml`:
  ```toml
  [dependencies]
  loom-crash-core = { path = "../loom-crash-core" }
  loom-common-http = { path = "../loom-common-http" }
  loom-analytics = { path = "../loom-analytics", optional = true }
  loom-flags = { path = "../loom-flags", optional = true }
  async-trait = "0.1"
  backtrace = "0.3"
  rustc-demangle = "0.1"
  tokio = { version = "1", features = ["sync", "time"] }
  tracing = "0.1"

  [features]
  default = ["session-tracking"]
  session-tracking = []
  analytics = ["loom-analytics"]
  flags = ["loom-flags"]
  ```
- [ ] Implement `CrashClient` builder pattern ([specs/crash-system.md#81-rust-sdk-loom-crash](specs/crash-system.md))
- [ ] Implement panic hook ([specs/crash-system.md#82-panic-hook-implementation](specs/crash-system.md))
- [ ] Implement backtrace parsing
- [ ] Implement session tracking integration ([specs/sessions-system.md#52-rust-sdk-session-tracking](specs/sessions-system.md))
- [ ] Implement analytics/flags integration if features enabled

### 7.2 Create `loom-crons`

**Path:** `crates/loom-crons/`

**Structure:**
```
loom-crons/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── client.rs        # CronsClient
    ├── checkin.rs       # Check-in helpers
    ├── integration.rs   # loom-jobs integration
    └── error.rs         # Error types
```

**Implementation checklist:**
- [ ] Create `Cargo.toml`:
  ```toml
  [dependencies]
  loom-crons-core = { path = "../loom-crons-core" }
  loom-common-http = { path = "../loom-common-http" }
  loom-crash = { path = "../loom-crash", optional = true }
  async-trait = "0.1"
  tokio = { version = "1", features = ["sync", "time"] }
  tracing = "0.1"

  [features]
  crash = ["loom-crash"]
  ```
- [ ] Implement `CronsClient` ([specs/crons-system.md#51-rust-sdk-loom-crons](specs/crons-system.md))
- [ ] Implement `checkin_start()`, `checkin_ok()`, `checkin_error()`
- [ ] Implement `with_monitor()` convenience wrapper
- [ ] Implement loom-jobs auto-instrumentation hook ([specs/crons-system.md#54-integration-with-loom-jobs](specs/crons-system.md))

---

## Phase 8: TypeScript SDK Packages

**Goal:** Create client SDKs for browser and Node.js applications.

Reference pattern: [web/packages/flags/](web/packages/flags/), [web/packages/analytics/](web/packages/analytics/)

### 8.1 Create `@loom/crash`

**Path:** `web/packages/crash/`

**Structure:**
```
crash/
├── package.json
├── tsconfig.json
├── vitest.config.ts
└── src/
    ├── index.ts         # Public exports
    ├── client.ts        # CrashClient
    ├── types.ts         # Type definitions
    ├── stacktrace.ts    # Stack trace parsing
    ├── global-handler.ts # window.onerror, unhandledrejection
    ├── session.ts       # Session tracking
    ├── breadcrumb.ts    # Breadcrumb management
    ├── transport.ts     # HTTP transport
    ├── errors.ts        # Error types
    └── react/
        └── error-boundary.tsx  # React error boundary
```

**Implementation checklist:**
- [ ] Create `package.json`:
  ```json
  {
    "name": "@loom/crash",
    "version": "0.1.0",
    "type": "module",
    "dependencies": {
      "@loom/http": "workspace:*"
    },
    "peerDependencies": {
      "@loom/analytics": "workspace:*",
      "@loom/flags": "workspace:*"
    },
    "peerDependenciesMeta": {
      "@loom/analytics": { "optional": true },
      "@loom/flags": { "optional": true }
    }
  }
  ```
- [ ] Implement `CrashClient` class ([specs/crash-system.md#83-typescript-sdk-loomcrash](specs/crash-system.md))
- [ ] Implement global error handlers ([specs/crash-system.md#84-global-handler-browser](specs/crash-system.md))
- [ ] Implement stack trace parsing ([specs/crash-system.md#85-stack-trace-parsing-javascript](specs/crash-system.md))
- [ ] Implement React error boundary
- [ ] Implement session tracking ([specs/sessions-system.md#55-browser-session-tracking](specs/sessions-system.md))
- [ ] Implement breadcrumb API
- [ ] Add vitest tests

### 8.2 Create `@loom/crons`

**Path:** `web/packages/crons/`

**Structure:**
```
crons/
├── package.json
├── tsconfig.json
├── vitest.config.ts
└── src/
    ├── index.ts         # Public exports
    ├── client.ts        # CronsClient
    ├── types.ts         # Type definitions
    ├── checkin.ts       # Check-in helpers
    └── errors.ts        # Error types
```

**Implementation checklist:**
- [ ] Create `package.json` (following flags/analytics pattern)
- [ ] Implement `CronsClient` class ([specs/crons-system.md#53-typescript-sdk-loomcrons](specs/crons-system.md))
- [ ] Implement `checkinStart()`, `checkinOk()`, `checkinError()`
- [ ] Implement `withMonitor()` async wrapper
- [ ] Add vitest tests

### 8.3 Update Workspace

- [ ] Add new packages to `web/pnpm-workspace.yaml`
- [ ] Run `pnpm install` to link workspaces

---

## Phase 9: Web UI Components

**Goal:** Build Svelte 5 components for the observability UI.

Reference pattern: [web/loom-web/src/lib/ui/](web/loom-web/src/lib/ui/), [web/loom-web/src/lib/components/](web/loom-web/src/lib/components/)

### 9.1 Common Components

**Path:** `web/loom-web/src/lib/components/common/`

- [ ] `StatCard.svelte` — Metric display with trend
- [ ] `Sparkline.svelte` — Mini inline chart
- [ ] `TimeRangePicker.svelte` — Time range selector
- [ ] `RelativeTime.svelte` — "5 minutes ago" display
- [ ] `CopyButton.svelte` — Copy to clipboard

### 9.2 Crash Components

**Path:** `web/loom-web/src/lib/components/crash/`

Reference: [specs/observability-ui.md#42-core-component-examples](specs/observability-ui.md)

- [ ] `IssueList.svelte` — Paginated issue list with filters
- [ ] `IssueListItem.svelte` — Single issue row
- [ ] `IssueDetail.svelte` — Full issue view
- [ ] `IssueStatusBadge.svelte` — Status indicator (Unresolved, Resolved, Regressed)
- [ ] `CrashEventCard.svelte` — Event summary
- [ ] `CrashEventDetail.svelte` — Full event with context
- [ ] `Stacktrace.svelte` — Collapsible frame viewer
- [ ] `StacktraceFrame.svelte` — Single frame with expand
- [ ] `SourceContext.svelte` — Syntax-highlighted source lines
- [ ] `Breadcrumbs.svelte` — Breadcrumb timeline
- [ ] `ActiveFlags.svelte` — Feature flags at crash time
- [ ] `UserContext.svelte` — User info display
- [ ] `SymbolUpload.svelte` — Source map upload form

### 9.3 Crons Components

**Path:** `web/loom-web/src/lib/components/crons/`

- [ ] `MonitorList.svelte` — Monitor list with health
- [ ] `MonitorListItem.svelte` — Single monitor row
- [ ] `MonitorDetail.svelte` — Monitor with history
- [ ] `MonitorForm.svelte` — Create/edit monitor
- [ ] `MonitorStatusBadge.svelte` — Status indicator
- [ ] `MonitorHealthBadge.svelte` — Health indicator
- [ ] `CheckInTimeline.svelte` — Check-in history
- [ ] `CheckInItem.svelte` — Single check-in
- [ ] `CronScheduleInput.svelte` — Cron expression input
- [ ] `PingUrlDisplay.svelte` — Ping URL with copy
- [ ] `UptimeChart.svelte` — Uptime visualization

### 9.4 Sessions Components

**Path:** `web/loom-web/src/lib/components/sessions/`

- [ ] `ReleaseHealthOverview.svelte` — Dashboard card
- [ ] `ReleaseHealthCard.svelte` — Single release health
- [ ] `ReleaseList.svelte` — All releases with metrics
- [ ] `ReleaseListItem.svelte` — Single release row
- [ ] `ReleaseDetail.svelte` — Release detail page
- [ ] `CrashFreeChart.svelte` — Crash-free rate over time
- [ ] `AdoptionChart.svelte` — Release adoption stacked area
- [ ] `SessionList.svelte` — Recent sessions
- [ ] `AdoptionStageBadge.svelte` — Adoption stage indicator

### 9.5 Create Storybook Stories

Following pattern: [web/loom-web/src/lib/ui/Button.stories.ts](web/loom-web/src/lib/ui/Button.stories.ts)

- [ ] Add `.stories.ts` file for each component
- [ ] Define argTypes for interactive controls
- [ ] Create multiple story variations
- [ ] Use `createRawSnippet()` for snippet props

---

## Phase 10: Page Routes

**Goal:** Create SvelteKit page routes for observability UI.

### 10.1 Create Route Files

**Path:** `web/loom-web/src/routes/`

```
routes/
├── (app)/
│   └── [org]/
│       └── [project]/
│           ├── overview/
│           │   └── +page.svelte
│           ├── crashes/
│           │   ├── +page.svelte          # Issue list
│           │   ├── [issueId]/
│           │   │   ├── +page.svelte      # Issue detail
│           │   │   └── events/
│           │   │       ├── +page.svelte  # Events list
│           │   │       └── [eventId]/
│           │   │           └── +page.svelte
│           │   └── releases/
│           │       ├── +page.svelte
│           │       └── [version]/
│           │           └── +page.svelte
│           ├── crons/
│           │   ├── +page.svelte          # Monitor list
│           │   ├── new/
│           │   │   └── +page.svelte
│           │   └── [slug]/
│           │       ├── +page.svelte
│           │       └── checkins/
│           │           └── +page.svelte
│           ├── sessions/
│           │   ├── +page.svelte          # Release health
│           │   ├── releases/
│           │   │   ├── +page.svelte
│           │   │   └── [version]/
│           │   │       └── +page.svelte
│           │   └── users/
│           │       ├── +page.svelte
│           │       └── [sessionId]/
│           │           └── +page.svelte
│           └── settings/
│               ├── +page.svelte
│               ├── api-keys/
│               │   └── +page.svelte
│               └── team/
│                   └── +page.svelte
```

### 10.2 Create Page Load Functions

- [ ] Create `+page.server.ts` files for data loading
- [ ] Implement API calls to observability endpoints
- [ ] Handle authentication and authorization

### 10.3 Create Layout Components

- [ ] Update sidebar navigation to include observability sections
- [ ] Create sub-navigation for each section

---

## Phase 11: SSE Real-time Integration

**Goal:** Wire up SSE for real-time updates across the UI.

### 11.1 Create SSE Client

**Path:** `web/loom-web/src/lib/realtime/`

- [ ] `observability-sse.ts` — SSE connection manager for observability
- [ ] Event handlers for: `issue.new`, `issue.regressed`, `monitor.missed`, `release.health_changed`

### 11.2 Integrate with Components

- [ ] Add SSE subscription to overview dashboard
- [ ] Add SSE subscription to issue list
- [ ] Add SSE subscription to monitor list
- [ ] Add SSE subscription to release health

### 11.3 Notification System

Reference: [specs/observability-ui.md#62-notification-system](specs/observability-ui.md)

- [ ] Create `NotificationProvider.svelte`
- [ ] Create `showNotification()` utility
- [ ] Wire up regression alerts

---

## Phase 12: Background Jobs

**Goal:** Register background jobs for observability maintenance.

Reference: [crates/loom-jobs/](crates/loom-jobs/)

### 12.1 Register Jobs

- [x] **Cron missed run detector** — Runs every minute ✅
  - Reference: [specs/crons-system.md#71-background-scheduler](specs/crons-system.md)
  - Implemented in `loom-server/src/jobs/cron_missed_run.rs`

- [x] **Cron timeout detector** — Runs every minute ✅
  - Reference: [specs/crons-system.md#72-timeout-detection](specs/crons-system.md)
  - Implemented in `loom-server/src/jobs/cron_timeout.rs`

- [ ] **Session aggregator** — Runs every hour
  - Reference: [specs/sessions-system.md#71-hourly-aggregation-job](specs/sessions-system.md)

- [ ] **Session cleanup** — Runs daily
  - Reference: [specs/sessions-system.md#72-cleanup-job](specs/sessions-system.md)

- [ ] **Symbol artifact cleanup** — Runs daily
  - Reference: [specs/crash-system.md#13-retention-policy](specs/crash-system.md)

- [ ] **Crash event cleanup** — Runs daily (90 day retention)

### 12.2 Job Implementation

- [ ] Add job definitions to loom-jobs
- [ ] Register jobs in server startup
- [ ] Add health checks for job execution

---

## Phase 13: Testing

**Goal:** Comprehensive test coverage for all components.

### 13.1 Unit Tests

- [ ] Core type validation (proptest)
- [ ] Fingerprinting algorithm tests
- [ ] VLQ decoder tests
- [ ] Cron expression parsing tests
- [ ] Sampling logic tests

### 13.2 Integration Tests

- [ ] Crash ingestion and fingerprinting
- [ ] Issue state transitions
- [ ] Regression detection
- [ ] Ping endpoint handling
- [ ] Missed run detection
- [ ] Session aggregation
- [ ] Release health calculation

### 13.3 Authorization Tests

Reference pattern: [crates/loom-server/tests/authz_*_tests.rs](crates/loom-server/tests/)

- [ ] `tests/authz_crash_tests.rs` — Crash endpoint authorization
- [x] `tests/authz/crons.rs` — Cron endpoint authorization ✅ (29 tests including stream endpoint)
- [ ] `tests/authz_sessions_tests.rs` — Session endpoint authorization

### 13.4 UI Tests

- [ ] Component unit tests with Testing Library
- [ ] Storybook interaction tests
- [ ] Visual regression tests (optional)

---

## Phase 14: Documentation

**Goal:** Document APIs and SDK usage.

### 14.1 OpenAPI Documentation

- [ ] Verify all endpoints have `#[utoipa::path]` attributes
- [ ] Add request/response examples
- [ ] Organize under appropriate tags

### 14.2 SDK Documentation

- [ ] README for `@loom/crash`
- [ ] README for `@loom/crons`
- [ ] README for `loom-crash` crate
- [ ] README for `loom-crons` crate

### 14.3 Integration Guides

- [ ] Getting started with crash analytics
- [ ] Setting up cron monitoring
- [ ] Understanding release health

---

## Phase 15: Deployment & Verification

**Goal:** Deploy and verify in production.

### 15.1 Pre-deployment

- [ ] Run `make check` (format + lint + build + test)
- [ ] Run `cargo2nix-update` to regenerate Cargo.nix
- [ ] Verify migrations run on clean database
- [ ] Test SDK packages locally

### 15.2 Deployment

- [ ] Commit all changes
- [ ] Push to trunk: `git push origin trunk`
- [ ] Monitor auto-update: `sudo journalctl -u nixos-auto-update.service -f`

### 15.3 Verification

- [ ] Check deployed revision: `cat /var/lib/nixos-auto-update/deployed-revision`
- [ ] Check loom-server started: `sudo systemctl status loom-server`
- [ ] Check health endpoint: `curl -s https://loom.ghuntley.com/health | jq .`
- [ ] Test crash ingestion with SDK
- [ ] Test ping endpoint with curl
- [ ] Verify UI loads correctly

---

## Summary

| Phase | Description | Estimated Effort |
|-------|-------------|------------------|
| 1 | Database migrations | 2-3 hours |
| 2 | Core type crates | 4-5 hours |
| 3 | Symbolication engine | 4-5 hours |
| 4 | Server repositories | 8-10 hours |
| 5 | HTTP route integration | 4-5 hours |
| 6 | Audit integration | 2-3 hours |
| 7 | Rust SDK crates | 6-8 hours |
| 8 | TypeScript SDK packages | 6-8 hours |
| 9 | Web UI components | 10-12 hours |
| 10 | Page routes | 4-5 hours |
| 11 | SSE integration | 3-4 hours |
| 12 | Background jobs | 3-4 hours |
| 13 | Testing | 6-8 hours |
| 14 | Documentation | 3-4 hours |
| 15 | Deployment & verification | 2-3 hours |

**Total estimated effort:** 68-87 hours

---

## Dependencies

### Build Order

```
Phase 1: Migrations (no deps)
    ↓
Phase 2: Core types (no deps except workspace)
    ↓
Phase 3: Symbolication (depends on crash-core)
    ↓
Phase 4: Server repos (depends on core + symbolicate)
    ↓
Phase 5: Routes (depends on server repos)
    ↓
Phase 6: Audit (parallel with Phase 5)
    ↓
Phase 7: Rust SDKs (depends on core)
Phase 8: TS SDKs (parallel with Phase 7)
    ↓
Phase 9-11: UI (depends on routes being ready)
    ↓
Phase 12: Background jobs (depends on server repos)
    ↓
Phase 13-15: Testing, docs, deployment
```

### Parallel Work Opportunities

- Phase 7 (Rust SDKs) and Phase 8 (TS SDKs) can run in parallel
- Phase 9 (UI components) can start once API structure is defined
- Phase 6 (Audit) can run in parallel with Phase 5 (Routes)
- Phase 13 (Testing) components can be written alongside development
