<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Observability Suite Implementation Plan (UI Work)

**Status:** UI Components Complete (39/39)\
**Last Updated:** 2026-01-25

Reference: [specs/observability-ui.md](specs/observability-ui.md)

---

## Quick Reference

| System | Spec | Crates | Web Packages | Migration |
|--------|------|--------|--------------|-----------|
| Crash | [specs/crash-system.md](specs/crash-system.md) | `loom-crash-core`, `loom-crash` ✅, `loom-crash-symbolicate` ✅, `loom-server-crash` ✅ | `@loom/crash` | `033_crash_analytics.sql` |
| Crons | [specs/crons-system.md](specs/crons-system.md) | `loom-crons-core` ✅, `loom-crons` ✅, `loom-server-crons` ✅ | `@loom/crons` ✅ | `034_cron_monitoring.sql` |
| Sessions | [specs/sessions-system.md](specs/sessions-system.md) | `loom-sessions-core` ✅, `loom-server-sessions` ✅ | (in `@loom/crash`) | `035_sessions.sql` (tables: `app_sessions`, `app_session_aggregates`) |
| UI | [specs/observability-ui.md](specs/observability-ui.md) | — | `web/loom-web/src/lib/components/` | — |
---


## Phase 9: Web UI Components

**Goal:** Build Svelte 5 components for the observability UI.

Reference pattern: [web/loom-web/src/lib/ui/](web/loom-web/src/lib/ui/), [web/loom-web/src/lib/components/](web/loom-web/src/lib/components/)

### 9.1 Common Components

**Path:** `web/loom-web/src/lib/components/common/`

- [x] `StatCard.svelte` — Metric display with trend
- [x] `Sparkline.svelte` — Mini inline chart
- [x] `TimeRangePicker.svelte` — Time range selector
- [x] `RelativeTime.svelte` — "5 minutes ago" display
- [x] `CopyButton.svelte` — Copy to clipboard

### 9.2 Crash Components

**Path:** `web/loom-web/src/lib/components/crash/`

Reference: [specs/observability-ui.md#42-core-component-examples](specs/observability-ui.md)

- [x] `IssueList.svelte` — Paginated issue list with filters
- [x] `IssueListItem.svelte` — Single issue row
- [x] `IssueDetail.svelte` — Full issue view
- [x] `IssueStatusBadge.svelte` — Status indicator (Unresolved, Resolved, Regressed)
- [x] `CrashEventCard.svelte` — Event summary
- [x] `CrashEventDetail.svelte` — Full event with context
- [x] `Stacktrace.svelte` — Collapsible frame viewer
- [x] `StacktraceFrame.svelte` — Single frame with expand
- [x] `SourceContext.svelte` — Syntax-highlighted source lines
- [x] `Breadcrumbs.svelte` — Breadcrumb timeline
- [x] `ActiveFlags.svelte` — Feature flags at crash time
- [x] `UserContext.svelte` — User info display
- [x] `SymbolUpload.svelte` — Source map upload form

### 9.3 Crons Components

**Path:** `web/loom-web/src/lib/components/crons/`

- [x] `MonitorList.svelte` — Monitor list with health
- [x] `MonitorListItem.svelte` — Single monitor row
- [x] `MonitorDetail.svelte` — Monitor with history
- [x] `MonitorForm.svelte` — Create/edit monitor
- [x] `MonitorStatusBadge.svelte` — Status indicator
- [x] `MonitorHealthBadge.svelte` — Health indicator
- [x] `CheckInTimeline.svelte` — Check-in history
- [x] `CheckInItem.svelte` — Single check-in
- [x] `CronScheduleInput.svelte` — Cron expression input
- [x] `PingUrlDisplay.svelte` — Ping URL with copy
- [x] `UptimeChart.svelte` — Uptime visualization

### 9.4 Sessions Components

**Path:** `web/loom-web/src/lib/components/sessions/`

- [x] `ReleaseHealthOverview.svelte` — Dashboard card
- [x] `ReleaseHealthCard.svelte` — Single release health
- [x] `ReleaseList.svelte` — All releases with metrics
- [x] `ReleaseListItem.svelte` — Single release row
- [x] `ReleaseDetail.svelte` — Release detail page
- [x] `CrashFreeChart.svelte` — Crash-free rate over time
- [x] `AdoptionChart.svelte` — Release adoption stacked area
- [x] `SessionList.svelte` — Recent sessions
- [x] `AdoptionStageBadge.svelte` — Adoption stage indicator

### 9.5 Create Storybook Stories

Following pattern: [web/loom-web/src/lib/ui/Button.stories.ts](web/loom-web/src/lib/ui/Button.stories.ts)

- [ ] Add `.stories.ts` file for each component
- [ ] Define argTypes for interactive controls
- [ ] Create multiple story variations
- [ ] Use `createRawSnippet()` for snippet props

---

## Phase 10: Page Routes

**Goal:** Create SvelteKit page routes for observability UI.

**Status:** Core routes implemented ✅

### 10.1 Route Files Created

**Path:** `web/loom-web/src/routes/(app)/`

Implemented routes:
- [x] `/crashes/` — Project list
- [x] `/crashes/[projectId]/` — Issue list with filters
- [x] `/crashes/[projectId]/issues/[issueId]/` — Issue detail with events
- [x] `/crons/` — Monitor list with health filtering
- [x] `/crons/new/` — Create new monitor
- [x] `/crons/[slug]/` — Monitor detail with check-in timeline
- [x] `/sessions/` — Release health overview
- [x] `/sessions/releases/[version]/` — Release detail

### 10.2 API Client Methods

- [x] Crash: listCrashProjects, getCrashProject, listIssues, getIssue, resolveIssue, etc.
- [x] Crons: listMonitors, getMonitor, createMonitor, updateMonitor, pauseMonitor, resumeMonitor
- [x] Sessions: listAppSessions, listReleaseHealth, getReleaseHealth
- [ ] Handle authentication and authorization

### 10.3 Create Layout Components

- [x] Update header navigation to include observability sections (dropdown menu)
- [x] Add `/crons/new` route for creating monitors
- [ ] Create sub-navigation for each section (deferred)

---

## Phase 11: SSE Real-time Integration

**Goal:** Wire up SSE for real-time updates across the UI.

### 11.1 Create SSE Client

**Path:** `web/loom-web/src/lib/realtime/`

- [x] `observability-sse.ts` — SSE connection manager for observability (CronsSSEClient, CrashSSEClient)
- [x] Event handlers for: `issue.new`, `issue.regressed`, `monitor.missed`, `checkin_ok`, `checkin_error`

### 11.2 Integrate with Components

- [ ] Add SSE subscription to overview dashboard
- [ ] Add SSE subscription to issue list
- [x] Add SSE subscription to monitor list (`/crons` page with LIVE badge indicator)
- [ ] Add SSE subscription to release health

### 11.3 Notification System

Reference: [specs/observability-ui.md#62-notification-system](specs/observability-ui.md)

- [x] Create `NotificationProvider.svelte`
- [x] Create `showNotification()` utility
- [ ] Wire up regression alerts to observability pages

---

## Phase 13: UI Tests

- [ ] Component unit tests with Testing Library
- [ ] Storybook interaction tests
- [ ] Visual regression tests (optional)

---

## Phase 14: Documentation

### 14.1 SDK Documentation

- [ ] README for `@loom/crash`
- [ ] README for `@loom/crons`
- [ ] README for `loom-crash` crate
- [ ] README for `loom-crons` crate

### 14.3 Integration Guides

- [ ] Getting started with crash analytics
- [ ] Setting up cron monitoring
- [ ] Understanding release health

---

---

## Verification Log

### 2026-01-24: Crash Analytics & Sessions Endpoints

**Crash API endpoints verified via curl:**
- `GET /api/crash/projects?org_id=...` — List crash projects ✓
- `POST /api/crash/capture` — Capture crash event ✓
- `GET /api/crash/projects/{id}/issues` — List issues ✓
- `GET /api/crash/projects/{id}/issues/{id}` — Get issue details ✓
- `POST /api/crash/projects/{id}/issues/{id}/resolve` — Resolve issue ✓
- `POST /api/crash/projects/{id}/issues/{id}/unresolve` — Unresolve issue ✓
- `POST /api/crash/projects/{id}/issues/{id}/ignore` — Ignore issue ✓
- `GET /api/crash/projects/{id}/events` — List crash events ✓
- `GET /api/crash/projects/{id}/api-keys` — List API keys ✓
- `GET /api/crash/projects/{id}/releases` — List releases ✓

**Crash CLI commands verified:**
- `loom crash projects --org ...` — List projects ✓
- `loom crash issues --project ...` — List issues ✓

**Sessions API endpoints verified via curl:**
- `POST /api/sessions/start` — Start a session ✓
- `POST /api/sessions/end` — End a session ✓
- `GET /api/app-sessions?project_id=...` — List sessions ✓
- `GET /api/app-sessions/releases?project_id=...` — List release health ✓
- `GET /api/app-sessions/releases/{version}?project_id=...` — Get release health detail ✓

**Sessions CLI commands verified:**
- `loom sessions list --project ...` — List sessions ✓
- `loom sessions releases --project ...` — List release health ✓
- `loom sessions release --project ... --version ...` — Get release health detail ✓

**Tests:** All 28 sessions authz tests pass (`cargo test -p loom-server --test authz_tests sessions`)

**Bug fix:** Fixed CLI display of crash-free rates (was multiplying by 100 twice, showing 10000% instead of 100%)

---

### 2026-01-24: Complete Crons System Verification

**Full API endpoints verified via curl:**
- `GET /api/crons/monitors?org_id=...` — List monitors ✓
- `POST /api/crons/monitors` — Create monitor (returns ping_url) ✓
- `GET /api/crons/monitors/{slug}?org_id=...` — Get monitor details ✓
- `PATCH /api/crons/monitors/{slug}` — Update monitor (org_id in body) ✓
- `DELETE /api/crons/monitors/{slug}?org_id=...` — Delete monitor ✓
- `POST /api/crons/monitors/{slug}/pause?org_id=...` — Pause monitoring ✓
- `POST /api/crons/monitors/{slug}/resume?org_id=...` — Resume monitoring ✓
- `GET /api/crons/monitors/{slug}/checkins?org_id=...` — List check-ins ✓

**Ping endpoints verified via curl (no auth required):**
- `GET /ping/{key}` — Success ping ✓
- `GET /ping/{key}/start` — Start ping (returns checkin_id) ✓
- `GET /ping/{key}/fail?exit_code=...` — Fail ping ✓
- `POST /ping/{key}` — Ping with body (output capture) ✓

**All CLI commands verified:**
- `loom crons monitors --org ...` — List monitors ✓
- `loom crons create --org ... --slug ... --name ... --cron "..."` — Create monitor ✓
- `loom crons get --org ... --slug ...` — Get monitor details ✓
- `loom crons update --org ... --slug ... --name ...` — Update monitor ✓
- `loom crons delete --org ... --slug ...` — Delete monitor ✓
- `loom crons pause --org ... --slug ...` — Pause monitoring ✓
- `loom crons resume --org ... --slug ...` — Resume monitoring ✓
- `loom crons checkins --org ... --slug ...` — List check-ins ✓
- `loom crons ping <key>` — Send success ping ✓
- `loom crons ping-fail <key>` — Send fail ping ✓

**Tests:** All 42 crons authz tests pass (`cargo test -p loom-server --test authz_tests crons`)

---

### 2026-01-25: Crons Stats Endpoints Implementation

**Stats API endpoints implemented and verified via curl:**
- `GET /api/crons/monitors/{slug}/stats?org_id=...&period=...` — Get monitor stats ✓
  - Response includes: total_checkins, successful_checkins, failed_checkins, missed_checkins, timeout_checkins
  - Duration metrics: avg_duration_ms, p50_duration_ms, p95_duration_ms, max_duration_ms
  - Uptime percentage calculation
  - Period options: day, week (default), month
- `GET /api/crons/stats/overview?org_id=...` — Get org-wide stats overview ✓
  - Monitor counts: total_monitors, active_monitors, paused_monitors
  - Health counts: healthy_monitors, failing_monitors, missed_monitors
  - 24h metrics: total_checkins_24h, total_failures_24h, overall_uptime_percentage

**CLI commands verified:**
- `loom crons stats --org ... --slug ... --period ...` — Get monitor stats ✓
- `loom crons overview --org ...` — Get stats overview ✓

**Tests:** All 49 crons authz tests pass (`cargo test -p loom-server --test authz_tests crons`)
- Added tests for: org_member_can_get_monitor_stats, unauthenticated_cannot_get_monitor_stats,
  org_b_member_cannot_get_org_a_monitor_stats, nonexistent_monitor_stats_returns_not_found,
  org_member_can_get_stats_overview, unauthenticated_cannot_get_stats_overview,
  org_b_member_cannot_get_org_a_stats_overview

---

### 2026-01-25: Web UI Navigation Integration

**Added navigation for observability features:**
- Added dropdown menu in header navigation with Crashes, Crons, Sessions links
- Added i18n keys for navigation: `nav.observability`, `nav.crashes`, `nav.crons`, `nav.sessions`
- Created `/crons/new` route for creating new monitors using MonitorForm component

**Files modified:**
- `web/loom-web/src/routes/(app)/+layout.svelte` — Added observability dropdown menu
- `web/loom-web/src/locales/en/messages.po` — Added navigation i18n keys
- `web/loom-web/src/routes/(app)/crons/new/+page.svelte` — New route for creating monitors

**Build:** Verified pnpm build succeeds

---

### 2026-01-25: SSE Client and Notification System

**Created SSE clients for real-time observability updates:**
- `CronsSSEClient` — Connects to `/api/crons/stream` for cron monitor events
- `CrashSSEClient` — Connects to `/api/crash/projects/{id}/stream` for crash events
- Both support auto-reconnect with exponential backoff

**Created notification system:**
- `NotificationProvider.svelte` — Displays toast notifications for alerts
- `showNotification()` / `dismissNotification()` utilities
- Supports info, success, warning, error types with optional links

**Files created:**
- `web/loom-web/src/lib/realtime/observability-sse.ts` — SSE clients
- `web/loom-web/src/lib/components/notifications/NotificationProvider.svelte`
- `web/loom-web/src/lib/components/notifications/index.ts`

**Files modified:**
- `web/loom-web/src/lib/realtime/index.ts` — Export new SSE clients
- `web/loom-web/src/routes/(app)/+layout.svelte` — Added NotificationProvider

**Build:** Verified pnpm build succeeds

**Routes verified (all return HTTP 200):**
- `https://loom.ghuntley.com/crashes` ✓
- `https://loom.ghuntley.com/crons` ✓
- `https://loom.ghuntley.com/crons/new` ✓
- `https://loom.ghuntley.com/sessions` ✓

Note: Routes use plural form (`/crons` not `/cron`)

**API fix deployed:**
- Fixed `GET /api/crash/projects` to return `{ projects: [...] }` instead of plain array
- Matches expected format in web client `CrashProjectListResponse` type

---

### 2026-01-25: Analytics Web Integration (Phase A6)

**Analytics self-monitoring infrastructure:**
- Added `analytics_api_key` field to `SelfMonitoringConfig` struct
- Created `ensure_analytics_api_key()` function to auto-generate internal analytics API keys
- Added `GET /api/self-monitoring/analytics-config` endpoint for web SDK configuration

**loom-web analytics integration:**
- Added `@loom/analytics` workspace dependency to package.json
- Created `$lib/analytics/self-monitoring.ts` — Fetches config from server and initializes AnalyticsClient
- Created `$lib/analytics/AnalyticsProvider.svelte` — Wraps app with auto-identification:
  - Identifies users on login via `identify()` with user ID, email, and display_name
  - Resets analytics identity on logout via `reset()`
- Integrated into `(app)/+layout.svelte` with user data
- Enabled autocapture for pageviews and pageleave events

**Files created:**
- `web/loom-web/src/lib/analytics/self-monitoring.ts`
- `web/loom-web/src/lib/analytics/index.ts`
- `web/loom-web/src/lib/analytics/AnalyticsProvider.svelte`

**Files modified:**
- `crates/loom-server/src/self_monitoring.rs` — Added analytics API key support
- `crates/loom-server/src/routes/self_monitoring.rs` — Added analytics config endpoint
- `crates/loom-server/src/api.rs` — Registered new route
- `web/loom-web/package.json` — Added @loom/analytics dependency
- `web/loom-web/src/routes/(app)/+layout.svelte` — Integrated AnalyticsProvider

**API endpoints verified via curl:**
- `POST /api/analytics/capture` — Event capture ✓
- `POST /api/analytics/batch` — Batch capture ✓
- `POST /api/analytics/identify` — User identification ✓
- `POST /api/analytics/set` — Set properties ✓
- `GET /api/orgs/{org_id}/analytics/api-keys` — List API keys ✓
- `POST /api/orgs/{org_id}/analytics/api-keys` — Create API key ✓

**Build:** All components build successfully

**Deployment verified:**
- `GET /api/self-monitoring/analytics-config` returns API key, release, environment ✓
- `POST /api/analytics/capture` with self-monitoring key — Event captured ✓
- `POST /api/analytics/identify` with self-monitoring key — User identified ✓
- `POST /api/analytics/batch` with self-monitoring key — Batch captured ✓

---

### 2026-01-25: Comprehensive Analytics Click Tracking

**Added explicit click tracking across all major pages in loom-web:**

**Tracking helper functions added to `$lib/analytics/self-monitoring.ts`:**
- `trackLinkClick(linkName, href, properties)` — Track link clicks with destination
- `trackButtonClick(buttonName, properties)` — Track button clicks
- `trackFormSubmit(formName, properties)` — Track form submissions
- `trackModalOpen(modalName, properties)` — Track modal opens
- `trackModalClose(modalName, properties)` — Track modal closes
- `trackFilterChange(filterName, value, properties)` — Track filter changes
- `trackAction(action, resourceType, resourceId, properties)` — Track user actions

**Pages with tracking added:**

1. **App Layout (`+layout.svelte`):**
   - All header navigation links (threads, repos, weavers, crashes, crons, sessions, settings, admin)
   - Logout button

2. **Weavers page (`/weavers`):**
   - New Weaver button, Logs button, Attach link, Delete button
   - Modal tracking (create weaver modal open/close)
   - Image preset selection

3. **Crashes pages (`/crashes`, `/crashes/[projectId]`, `/crashes/[projectId]/issues/[issueId]`):**
   - Project card links, org filter changes
   - Issue clicks, status filter, time range picker
   - Resolve/unresolve/ignore actions, back links, event clicks

4. **Crons pages (`/crons`, `/crons/new`, `/crons/[slug]`):**
   - New Monitor button, health filter, org filter, monitor clicks
   - Form submit, cancel button
   - Pause/resume/delete actions, back link

5. **Sessions page (`/sessions`):**
   - Org/project/time range filters
   - Release clicks

6. **Repos page (`/repos`):**
   - New repo button, repo links
   - Modal tracking (create repo modal open/close)
   - Retry button

7. **Settings pages:**
   - Settings nav links (sessions, profile, orgs)
   - Profile save form, locale change

**Commits:**
- `f371b777` — Add analytics click tracking to all major pages
- `7ef80ff2` — Add analytics tracking to detail pages and settings

**Deployment verified:** Changes pushed to trunk and auto-deployed

---

### 2026-01-25: Self-Monitoring Implementation

**Self-monitoring infrastructure for Loom monitoring itself:**
- Created `self_monitoring.rs` module in loom-server
- Automatically creates "Loom Internal" organization with well-known UUID
- Creates internal crash projects: loom-server, loom-web, loom-cli
- Generates internal API keys for crash capture
- Installs panic hook for automatic loom-server crash reporting

**API endpoints for self-monitoring configuration:**
- `GET /api/self-monitoring/web-config` - Returns crash SDK config for loom-web
- `GET /api/self-monitoring/cli-config` - Returns crash SDK config for loom-cli
- `GET /api/self-monitoring/projects` - Returns internal project IDs

**loom-web integration:**
- Created `$lib/crash/self-monitoring.ts` - Fetches config and initializes CrashClient
- Created `SelfMonitoringProvider.svelte` - Wraps app layout with crash monitoring
- Added @loom/crash dependency
- Installs global error handlers for automatic crash capture

**loom-cli integration:**
- Created `self_monitoring.rs` module
- Added loom-crash dependency
- Initializes crash monitoring on CLI startup
- Installs panic hook for automatic crash reporting

**Files created:**
- `crates/loom-server/src/self_monitoring.rs`
- `crates/loom-server/src/routes/self_monitoring.rs`
- `crates/loom-cli/src/self_monitoring.rs`
- `web/loom-web/src/lib/crash/self-monitoring.ts`
- `web/loom-web/src/lib/crash/index.ts`
- `web/loom-web/src/lib/crash/SelfMonitoringProvider.svelte`

**Build:** All components build successfully (cargo build -p loom-server -p loom-cli, pnpm build)

---

## Summary

| Phase | Description | Status |
|-------|-------------|--------|
| 1-6 | Backend foundation | ✅ Complete |
| 7-8 | SDKs | ✅ Complete |
| 9 | Web UI components | ✅ Complete (39/39 components) |
| 10 | Page routes | ✅ Complete (8 routes) |
| 11 | SSE integration | In Progress (SSE clients + notifications done) |
| 12 | Background jobs | ✅ Complete |
| 13 | Testing (backend) | ✅ Complete |
| 13 | Testing (UI) | Pending |
| 14 | Documentation (OpenAPI) | ✅ Complete |
| 14 | Documentation (SDK/Guides) | Pending |
| 15 | Deployment & verification | ✅ Complete |

**Remaining effort:** SSE integration, Storybook stories, UI tests

---

## Product Analytics System

**Status:** Backend Complete, Frontend Integration Pending\
**Spec:** [specs/analytics-system.md](specs/analytics-system.md)

### Quick Reference

| Component | Location | Status |
|-----------|----------|--------|
| Core types | `crates/loom-analytics-core/` | ✅ Complete |
| Rust SDK | `crates/loom-analytics/` | ✅ Complete |
| Server handlers | `crates/loom-server-analytics/` | ✅ Complete |
| TypeScript SDK | `web/packages/analytics/` | ✅ Complete |
| Database | `migrations/032_analytics.sql` | ✅ Complete |
| API routes | `/api/analytics/*` | ✅ Complete |
| Config | `loom-server-config/sections/analytics.rs` | ✅ Complete |
| Flag integration | `loom-flags/src/analytics.rs` | ✅ Complete |
| Authz tests | `tests/authz/analytics.rs` | ✅ Complete |
| loom-web integration | `$lib/analytics/` | ✅ Complete |
| Analytics UI pages | — | ❌ Not started |

---

### Phase A1: Backend Crates ✅ Complete

#### A1.1 loom-analytics-core
**Path:** `crates/loom-analytics-core/`

- [x] `person.rs` — Person, PersonWithIdentities types
- [x] `identity.rs` — PersonIdentity, IdentityType enum
- [x] `event.rs` — Event, EventProperty types
- [x] `identify.rs` — IdentifyPayload, AliasPayload
- [x] `api_key.rs` — AnalyticsApiKey, AnalyticsKeyType
- [x] `error.rs` — Error types with thiserror

#### A1.2 loom-analytics (Rust SDK)
**Path:** `crates/loom-analytics/`

- [x] `client.rs` — AnalyticsClient with builder pattern
- [x] `batch.rs` — Event batching with flush interval
- [x] `properties.rs` — Properties helper type
- [x] `error.rs` — SDK error types

#### A1.3 loom-server-analytics
**Path:** `crates/loom-server-analytics/`

- [x] `routes.rs` — Axum route definitions
- [x] `handlers/capture.rs` — Event capture endpoint
- [x] `handlers/identify.rs` — Identity resolution
- [x] `handlers/persons.rs` — Person queries
- [x] `handlers/events.rs` — Event queries
- [x] `handlers/api_keys.rs` — API key management
- [x] `repository.rs` — Database operations
- [x] `identity_resolution.rs` — Merge logic
- [x] `middleware.rs` — API key auth middleware
- [x] `api_key.rs` — Key validation

---

### Phase A2: Database Schema ✅ Complete

**Migration:** `crates/loom-server/migrations/032_analytics.sql`

- [x] `analytics_persons` — Tracked users
- [x] `analytics_person_identities` — distinct_id → person mapping
- [x] `analytics_events` — Event records
- [x] `analytics_person_merges` — Merge audit trail
- [x] `analytics_api_keys` — API key storage

---

### Phase A3: API Endpoints ✅ Complete

**Path:** `crates/loom-server/src/routes/analytics.rs`

SDK Routes (API Key Auth):
- [x] `POST /api/analytics/capture` — Single event (Write key)
- [x] `POST /api/analytics/batch` — Batch events (Write key)
- [x] `POST /api/analytics/identify` — Identity resolution (Write key)
- [x] `POST /api/analytics/alias` — Alias distinct_ids (Write key)
- [x] `POST /api/analytics/set` — Set person properties (Write key)
- [x] `GET /api/analytics/persons` — List persons (ReadWrite key)
- [x] `GET /api/analytics/persons/{id}` — Get person (ReadWrite key)
- [x] `GET /api/analytics/persons/by-distinct-id/{id}` — Lookup by distinct_id (ReadWrite key)
- [x] `GET /api/analytics/events` — List events (ReadWrite key)
- [x] `GET /api/analytics/events/count` — Count events (ReadWrite key)
- [x] `POST /api/analytics/events/export` — Bulk export (ReadWrite key)

Management Routes (User Auth):
- [x] `GET /api/orgs/{org_id}/analytics/api-keys` — List keys
- [x] `POST /api/orgs/{org_id}/analytics/api-keys` — Create key
- [x] `DELETE /api/orgs/{org_id}/analytics/api-keys/{id}` — Revoke key

---

### Phase A4: TypeScript SDK ✅ Complete

**Path:** `web/packages/analytics/`

- [x] `client.ts` — AnalyticsClient class
- [x] `batch.ts` — BatchProcessor for event queuing
- [x] `storage.ts` — DistinctIdManager (localStorage, cookie, memory)
- [x] `types.ts` — TypeScript type definitions
- [x] `errors.ts` — Error classes with isRetryable()
- [x] `index.ts` — Public exports
- [x] Unit tests for all modules

**Features:**
- [x] `capture()` — Track events
- [x] `identify()` — Link anonymous → authenticated
- [x] `alias()` — Link two distinct_ids
- [x] `set()` — Set person properties
- [x] `reset()` — Generate new anonymous ID (logout)
- [x] Autocapture ($pageview, $pageleave)
- [x] Configurable batching (interval, max size)
- [x] Multiple storage modes

---

### Phase A5: Integration & Configuration ✅ Complete

- [x] Config section: `loom-server-config/sections/analytics.rs`
  - `LOOM_ANALYTICS_ENABLED`
  - `LOOM_ANALYTICS_BATCH_SIZE`
  - `LOOM_ANALYTICS_FLUSH_INTERVAL_SECS`
  - `LOOM_ANALYTICS_EVENT_RETENTION_DAYS`
- [x] Feature flag integration: `loom-flags/src/analytics.rs`
  - `$feature_flag_called` event capture
- [x] Authorization tests: `tests/authz/analytics.rs` (1017 lines)

---

### Phase A6: loom-web Integration ✅ Complete

**Goal:** Integrate @loom/analytics SDK into the web frontend.

- [x] Add `@loom/analytics` dependency to loom-web package.json
- [x] Create `$lib/analytics/self-monitoring.ts` — Fetch config and initialize AnalyticsClient
- [x] Create `AnalyticsProvider.svelte` — Wrap app layout with user identification
- [x] Auto-track pageviews on route changes (via autocapture)
- [x] Wire identify() to auth state changes
- [x] Call reset() on logout
- [x] Add self-monitoring analytics endpoint to loom-server (`GET /api/self-monitoring/analytics-config`)

---

### Phase A7: Analytics UI Pages ❌ Not Started

**Goal:** Create pages to view analytics data (persons, events).

**Path:** `web/loom-web/src/routes/(app)/analytics/`

- [ ] `/analytics/` — Overview dashboard
- [ ] `/analytics/persons/` — Person list with search
- [ ] `/analytics/persons/[id]/` — Person detail with events
- [ ] `/analytics/events/` — Event explorer with filters
- [ ] `/analytics/api-keys/` — API key management

**Components needed:** `web/loom-web/src/lib/components/analytics/`

- [ ] `PersonList.svelte` — Paginated person list
- [ ] `PersonDetail.svelte` — Person profile with identities
- [ ] `EventList.svelte` — Event timeline/table
- [ ] `EventDetail.svelte` — Single event view
- [ ] `ApiKeyList.svelte` — API key management
- [ ] `ApiKeyForm.svelte` — Create API key form

---

### Phase A8: SDK Documentation ❌ Not Started

- [ ] README for `@loom/analytics`
- [ ] README for `loom-analytics` crate
- [ ] Integration guide: Getting started with product analytics
- [ ] API reference documentation

---

### Analytics Summary

| Phase | Description | Status |
|-------|-------------|--------|
| A1 | Backend crates | ✅ Complete |
| A2 | Database schema | ✅ Complete |
| A3 | API endpoints | ✅ Complete |
| A4 | TypeScript SDK | ✅ Complete |
| A5 | Config & integration | ✅ Complete |
| A6 | loom-web integration | ✅ Complete |
| A7 | Analytics UI pages | ❌ Not started |
| A8 | SDK documentation | ❌ Not started |

**Remaining effort:** Analytics UI pages, SDK documentation
