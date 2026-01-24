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
- [x] `/crons/[slug]/` — Monitor detail with check-in timeline
- [x] `/sessions/` — Release health overview
- [x] `/sessions/releases/[version]/` — Release detail

### 10.2 API Client Methods

- [x] Crash: listCrashProjects, getCrashProject, listIssues, getIssue, resolveIssue, etc.
- [x] Crons: listMonitors, getMonitor, createMonitor, updateMonitor, pauseMonitor, resumeMonitor
- [x] Sessions: listAppSessions, listReleaseHealth, getReleaseHealth
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

## Summary

| Phase | Description | Status |
|-------|-------------|--------|
| 1-6 | Backend foundation | ✅ Complete |
| 7-8 | SDKs | ✅ Complete |
| 9 | Web UI components | ✅ Complete (39/39 components) |
| 10 | Page routes | ✅ Complete (7 routes) |
| 11 | SSE integration | Pending |
| 12 | Background jobs | ✅ Complete |
| 13 | Testing (backend) | ✅ Complete |
| 13 | Testing (UI) | Pending |
| 14 | Documentation (OpenAPI) | ✅ Complete |
| 14 | Documentation (SDK/Guides) | Pending |
| 15 | Deployment & verification | ✅ Complete |

**Remaining effort:** SSE integration, Storybook stories, UI tests
