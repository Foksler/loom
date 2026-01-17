# Loom Server API Test Plan

This document provides a comprehensive test plan for all loom-server API endpoints. Use this as a guide for a Claude Code session to systematically test each endpoint.

## Test Environment Setup

- **Server URL:** `http://localhost:9090` (local) or `https://loom.ghuntley.com` (production)
- **Dev mode:** Set `LOOM_SERVER_AUTH_DEV_MODE=1` for testing without real auth
- **Database:** `LOOM_SERVER_DB_PATH=/tmp/loom-test.db`

---

## 1. Health & Metrics Endpoints

**Spec:** `specs/health-check.md`
**Source:** `crates/loom-server/src/routes/health.rs`
**Tests:** None specific (public endpoints)

### Endpoints

- [x] `GET /health` - Health check with component status ✅ **TESTED 2026-01-16 via curl**
  - Source: `routes/health.rs:24`
  - Returns: `HealthResponse` with status, timestamp, version, components
  - Components checked: database, bin_dir, llm_providers, google_cse, serper, github_app, kubernetes, smtp, geoip, jobs, secrets, scim, auth_providers
  - HTTP 200 for healthy/degraded, 503 for unhealthy

- [x] `GET /metrics` - Prometheus metrics ✅ **TESTED 2026-01-17 via curl** (required nginx fix to proxy /metrics route)
  - Source: `routes/health.rs:117`
  - Returns: Prometheus text format
  - Content-Type: `text/plain; version=0.0.4; charset=utf-8`

---

## 2. Authentication Endpoints

**Spec:** `specs/auth-abac-system.md`
**Source:** `crates/loom-server/src/routes/auth.rs`
**Tests:** `crates/loom-server/tests/auth_integration_tests.rs`, `crates/loom-server/tests/authz/auth.rs`

### Public Auth Routes

- [x] `GET /auth/providers` - List available auth providers ✅ **TESTED 2026-01-16 via curl**
  - Source: `routes/auth.rs:46-77`
  - Returns: List of enabled providers (github, google, okta, magic_link)

- [ ] `POST /auth/magic-link` - Request magic link email
  - Source: `routes/auth.rs:225-324`
  - Body: `{ "email": "user@example.com" }`
  - Test: `auth_integration_tests.rs:129` - `test_magic_link_request_accepts_any_email`

- [ ] `GET /auth/magic-link/verify` - Verify magic link token
  - Source: `routes/auth.rs:1320-1464`
  - Query: `?token=<token>`

- [ ] `POST /auth/device/start` - Start device code flow (CLI/IDE)
  - Source: `routes/auth.rs:326-379`
  - Returns: `{ device_code, user_code, verification_uri, expires_in, interval }`
  - Test: `auth_integration_tests.rs:178` - `test_device_start_returns_codes`

- [ ] `POST /auth/device/poll` - Poll device code status
  - Source: `routes/auth.rs:381-478`
  - Body: `{ "device_code": "..." }`
  - Test: `auth_integration_tests.rs:206` - `test_device_poll_pending_initially`

- [ ] `GET /auth/login/github` - Initiate GitHub OAuth
  - Source: `routes/auth.rs:589-648`
  - Test: `auth_integration_tests.rs:64` - `test_github_callback_without_provider_config_returns_501`

- [ ] `GET /auth/github/callback` - GitHub OAuth callback
  - Source: `routes/auth.rs:774-953`

- [ ] `GET /auth/login/google` - Initiate Google OAuth
  - Source: `routes/auth.rs:650-710`
  - Test: `auth_integration_tests.rs:83` - `test_google_callback_without_provider_config_returns_501`

- [ ] `GET /auth/google/callback` - Google OAuth callback
  - Source: `routes/auth.rs:955-1103`

- [ ] `GET /auth/login/okta` - Initiate Okta OAuth
  - Source: `routes/auth.rs:712-772`

- [ ] `GET /auth/okta/callback` - Okta OAuth callback
  - Source: `routes/auth.rs:1105-1251`

### Authenticated Auth Routes

- [x] `GET /auth/me` - Get current user ✅ **TESTED 2026-01-16 via curl with Bearer token**
  - Source: `routes/auth.rs:79-113`
  - Returns: User profile with id, email, name, roles, locale

- [ ] `GET /auth/ws-token` - Get WebSocket auth token (30s, single-use)
  - Source: `routes/auth.rs:115-171`
  - Test: `authz/auth.rs` - `get_ws_token_requires_auth`, `get_ws_token_returns_token_for_authenticated_user`

- [ ] `POST /auth/logout` - Logout and invalidate session
  - Source: `routes/auth.rs:173-223`

- [ ] `POST /auth/device/complete` - Complete device code flow
  - Source: `routes/auth.rs:480-556`

---

## 3. Session Management Endpoints

**Source:** `crates/loom-server/src/routes/sessions.rs`
**Tests:** `crates/loom-server/tests/authz/users.rs`

- [x] `GET /api/sessions` - List user's active sessions ✅ **TESTED 2026-01-17 via curl**
  - Source: `routes/sessions.rs:32-91`
  - Test: `authz/users.rs` - `user_can_list_own_sessions`, `sessions_scoped_to_user`

- [ ] `DELETE /api/sessions/{id}` - Revoke a session
  - Source: `routes/sessions.rs:93-196`
  - Test: `authz/users.rs` - `user_can_revoke_own_session`, `user_cannot_revoke_other_session`

---

## 4. Thread Endpoints

**Spec:** `specs/thread-system.md`
**Source:** `crates/loom-server/src/routes/threads.rs`
**Tests:** `crates/loom-server/tests/authz/threads.rs`

- [x] `GET /api/threads` - List threads with pagination ✅ **TESTED 2026-01-16 via curl + loom-cli**
  - Source: `routes/threads.rs:135`
  - Query: `?workspace=<path>&limit=50&offset=0`
  - Test: `authz/threads.rs` - `owner_can_list_threads`, `member_can_list_threads`

- [x] `GET /api/threads/search` - Full-text search threads (FTS5) ✅ **TESTED 2026-01-16 via curl + loom-cli**
  - Source: `routes/threads.rs:297`
  - Query: `?q=<query>&limit=20&offset=0`
  - Supports commit SHA prefix search
  - Test: `authz/threads.rs` - `authenticated_can_search_threads`

- [x] `PUT /api/threads/{id}` - Create or update thread (optimistic concurrency) ✅ **TESTED 2026-01-16 via curl**
  - Source: `routes/threads.rs:40`
  - Header: `If-Match: <version>` for updates
  - Test: `authz/threads.rs` - `owner_can_upsert_thread`

- [x] `GET /api/threads/{id}` - Get thread by ID ✅ **TESTED 2026-01-16 via curl**
  - Source: `routes/threads.rs:106`
  - Test: `authz/threads.rs` - `owner_can_get_own_thread`, `member_can_get_org_thread`

- [x] `DELETE /api/threads/{id}` - Soft-delete thread ✅ **TESTED 2026-01-16 via curl**
  - Source: `routes/threads.rs:178`
  - Test: `authz/threads.rs` - `owner_can_delete_thread`

- [x] `POST /api/threads/{id}/visibility` - Update thread visibility ✅ **TESTED 2026-01-16 via curl**
  - Source: `routes/threads.rs:223`
  - Body: `{ "visibility": "organization|private|public" }` (lowercase required)
  - Test: `authz/threads.rs` - `owner_can_update_visibility`

---

## 5. Thread Sharing Endpoints

**Source:** `crates/loom-server/src/routes/share.rs`
**Tests:** `crates/loom-server/tests/share_tests.rs`

- [ ] `POST /api/threads/{id}/share` - Create share link
  - Source: `api.rs:939`
  - Test: `share_tests.rs:213` - `test_owner_can_manage_share_links`

- [ ] `DELETE /api/threads/{id}/share` - Revoke share link
  - Source: `api.rs:943`
  - Test: `share_tests.rs:150` - `test_cannot_revoke_share_link_for_others_thread`

- [ ] `GET /api/threads/{id}/share/{token}` - Get shared thread (public)
  - Source: `api.rs:832`

- [ ] `POST /api/threads/{id}/support-access/request` - Request support access
  - Source: `api.rs:948`

- [ ] `POST /api/threads/{id}/support-access/approve` - Approve support access
  - Source: `api.rs:952`

- [ ] `DELETE /api/threads/{id}/support-access` - Revoke support access
  - Source: `api.rs:956`

---

## 6. Organization Endpoints

**Spec:** `specs/auth-abac-system.md`
**Source:** `crates/loom-server/src/routes/orgs.rs`
**Tests:** `crates/loom-server/tests/authz/orgs.rs`

- [x] `GET /api/orgs` - List user's organizations ✅ **TESTED 2026-01-17 via curl**
  - Source: `api.rs:973`
  - Test: `authz/orgs.rs` - `test_org_authorization`

- [x] `POST /api/orgs` - Create organization ✅ **TESTED 2026-01-17 via curl**
  - Source: `api.rs:974`
  - Test: `authz/orgs.rs` - `test_org_create_authorization`

- [x] `GET /api/orgs/{id}` - Get organization ✅ **TESTED 2026-01-17 via curl**
  - Source: `api.rs:975`

- [x] `PATCH /api/orgs/{id}` - Update organization ✅ **TESTED 2026-01-17 via curl**
  - Source: `api.rs:976`

- [x] `DELETE /api/orgs/{id}` - Delete organization ✅ **TESTED 2026-01-17 via curl**
  - Source: `api.rs:977`
  - Test: `authz/orgs.rs` - `test_org_delete_authorization`

- [x] `GET /api/orgs/{id}/members` - List organization members ✅ **TESTED 2026-01-17 via curl**
  - Source: `api.rs:980`

- [ ] `POST /api/orgs/{id}/members` - Add organization member
  - Source: `api.rs:984`

- [ ] `DELETE /api/orgs/{org_id}/members/{user_id}` - Remove organization member
  - Source: `api.rs:988`

---

## 7. Team Endpoints

**Source:** `crates/loom-server/src/routes/teams.rs`
**Tests:** `crates/loom-server/tests/authz/orgs.rs`, `crates/loom-server/tests/authz/authz_scm_team_tests.rs`

- [ ] `GET /api/orgs/{org_id}/teams` - List teams
  - Source: `api.rs:993`
  - Test: `authz/orgs.rs` - `test_team_authorization`

- [ ] `POST /api/orgs/{org_id}/teams` - Create team
  - Source: `api.rs:997`

- [ ] `GET /api/orgs/{org_id}/teams/{team_id}` - Get team
  - Source: `api.rs:1001`

- [ ] `PATCH /api/orgs/{org_id}/teams/{team_id}` - Update team
  - Source: `api.rs:1005`

- [ ] `DELETE /api/orgs/{org_id}/teams/{team_id}` - Delete team
  - Source: `api.rs:1009`

- [ ] `GET /api/orgs/{org_id}/teams/{team_id}/members` - List team members
  - Source: `api.rs:1013`

- [ ] `POST /api/orgs/{org_id}/teams/{team_id}/members` - Add team member
  - Source: `api.rs:1017`

- [ ] `DELETE /api/orgs/{org_id}/teams/{team_id}/members/{user_id}` - Remove team member
  - Source: `api.rs:1021`
  - Test: `authz_scm_team_tests.rs` - `test_team_member_can_read_org_repo`, `test_revoke_team_access`

---

## 8. User Endpoints

**Source:** `crates/loom-server/src/routes/users.rs`
**Tests:** `crates/loom-server/tests/authz/users.rs`

- [x] `GET /api/users/{id}` - Get user profile ✅ **TESTED 2026-01-17 via curl**
  - Source: `api.rs:1232`
  - Test: `authz/users.rs` - `user_can_get_own_profile`, `user_can_get_other_profile`

- [ ] `PATCH /api/users/me` - Update current user
  - Source: `api.rs:1235`
  - Test: `authz/users.rs` - `user_can_update_own_profile`, `user_can_update_locale`

- [ ] `POST /api/users/me/delete` - Request account deletion
  - Source: `api.rs:1239`
  - Test: `authz/users.rs` - `user_can_request_deletion`

- [ ] `POST /api/users/me/restore` - Restore deleted account
  - Source: `api.rs:1243`
  - Test: `authz/users.rs` - `user_can_restore_account`

- [x] `GET /api/users/me/identities` - List linked identities ✅ **TESTED 2026-01-17 via curl**
  - Source: `api.rs:1248`

- [ ] `DELETE /api/users/me/identities/{id}` - Unlink identity
  - Source: `api.rs:1252`

---

## 9. API Key Management Endpoints

**Source:** `crates/loom-server/src/routes/api_keys.rs`

- [ ] `GET /api/orgs/{org_id}/api-keys` - List API keys
  - Source: `api.rs:1026`

- [ ] `POST /api/orgs/{org_id}/api-keys` - Create API key
  - Source: `api.rs:1030`

- [ ] `DELETE /api/orgs/{org_id}/api-keys/{id}` - Revoke API key
  - Source: `api.rs:1034`

- [ ] `GET /api/orgs/{org_id}/api-keys/{id}/usage` - Get API key usage
  - Source: `api.rs:1038`

---

## 10. Invitation Endpoints

**Source:** `crates/loom-server/src/routes/invitations.rs`

- [ ] `GET /api/invitations/{token}` - Get invitation (public)
  - Source: `api.rs:827`

- [ ] `GET /api/orgs/{org_id}/invitations` - List invitations
  - Source: `api.rs:1200`

- [ ] `POST /api/orgs/{org_id}/invitations` - Create invitation
  - Source: `api.rs:1204`

- [ ] `DELETE /api/orgs/{org_id}/invitations/{id}` - Cancel invitation
  - Source: `api.rs:1208`

- [ ] `POST /api/invitations/accept` - Accept invitation
  - Source: `api.rs:1212`

- [ ] `GET /api/orgs/{org_id}/join-requests` - List join requests
  - Source: `api.rs:1217`

- [ ] `POST /api/orgs/{org_id}/join-requests` - Create join request
  - Source: `api.rs:1221`

- [ ] `POST /api/orgs/{org_id}/join-requests/{request_id}/approve` - Approve join request
  - Source: `api.rs:1225`

- [ ] `POST /api/orgs/{org_id}/join-requests/{request_id}/reject` - Reject join request
  - Source: `api.rs:1229`

---

## 11. Repository Endpoints

**Spec:** `specs/scm-system.md`
**Source:** `crates/loom-server/src/routes/repos.rs`
**Tests:** `crates/loom-server/tests/authz/repos.rs`

- [ ] `POST /api/repos` - Create repository
  - Source: `routes/repos.rs:217-450`
  - Test: `authz/repos.rs` - `test_repo_create_authorization`

- [ ] `GET /api/repos/{id}` - Get repository
  - Source: `routes/repos.rs:467-567`
  - Test: `authz/repos.rs` - `test_repo_get_authorization`

- [ ] `PATCH /api/repos/{id}` - Update repository
  - Source: `routes/repos.rs:586-738`
  - Test: `authz/repos.rs` - `test_repo_update_authorization`

- [ ] `DELETE /api/repos/{id}` - Delete repository
  - Source: `routes/repos.rs:755-836`
  - Test: `authz/repos.rs` - `test_repo_delete_authorization`, `test_repo_delete_removes_repo`

- [ ] `GET /api/users/{id}/repos` - List user repositories
  - Source: `routes/repos.rs:852-934`
  - Test: `authz/repos.rs` - `test_repo_list_authorization`

- [ ] `GET /api/orgs/{id}/repos` - List organization repositories
  - Source: `routes/repos.rs:950-1036`

- [ ] `GET /api/repos/{id}/teams` - List team access
  - Source: `routes/repos.rs:1097-1184`

- [ ] `POST /api/repos/{id}/teams` - Grant team access
  - Source: `routes/repos.rs:1202-1300`
  - Test: `authz_scm_team_tests.rs` - `test_only_admin_can_grant_team_access`

- [ ] `DELETE /api/repos/{id}/teams/{tid}` - Revoke team access
  - Source: `routes/repos.rs:1318-1412`

---

## 12. Branch Protection Endpoints

**Spec:** `specs/scm-system.md`
**Source:** `crates/loom-server/src/routes/protection.rs`

- [ ] `GET /api/repos/{id}/protection` - List protection rules
  - Source: `api.rs:1283`

- [ ] `POST /api/repos/{id}/protection` - Create protection rule
  - Source: `api.rs:1287`

- [ ] `DELETE /api/repos/{id}/protection/{rule_id}` - Delete protection rule
  - Source: `api.rs:1291`

---

## 13. Webhook Endpoints

**Spec:** `specs/scm-system.md`
**Source:** `crates/loom-server/src/routes/webhooks.rs`

- [ ] `GET /api/repos/{id}/webhooks` - List repository webhooks
  - Source: `routes/webhooks.rs:255-299`

- [ ] `POST /api/repos/{id}/webhooks` - Create repository webhook
  - Source: `routes/webhooks.rs:318-421`

- [ ] `DELETE /api/repos/{id}/webhooks/{wid}` - Delete repository webhook
  - Source: `routes/webhooks.rs:439-541`

- [ ] `GET /api/orgs/{id}/webhooks` - List organization webhooks
  - Source: `api.rs:1308`

- [ ] `POST /api/orgs/{id}/webhooks` - Create organization webhook
  - Source: `api.rs:1312`

- [ ] `DELETE /api/orgs/{id}/webhooks/{wid}` - Delete organization webhook
  - Source: `api.rs:1316`

---

## 14. Mirror Endpoints

**Spec:** `specs/scm-system.md`
**Source:** `crates/loom-server/src/routes/mirrors.rs`

- [ ] `GET /api/repos/{id}/mirrors` - List mirrors
  - Source: `api.rs:1363`

- [ ] `POST /api/repos/{id}/mirrors` - Create mirror
  - Source: `api.rs:1367`

- [ ] `DELETE /api/repos/{id}/mirrors/{mirror_id}` - Delete mirror
  - Source: `api.rs:1371`

- [ ] `POST /api/repos/{id}/mirrors/{mirror_id}/sync` - Trigger sync
  - Source: `api.rs:1375`

---

## 15. Secrets Endpoints

**Spec:** `specs/weaver-secrets-system.md`
**Source:** `crates/loom-server/src/routes/secrets.rs`

### Organization Secrets

- [ ] `GET /api/orgs/{org_id}/secrets` - List organization secrets
  - Source: `api.rs:1321`

- [ ] `POST /api/orgs/{org_id}/secrets` - Create organization secret
  - Source: `api.rs:1325`

- [ ] `GET /api/orgs/{org_id}/secrets/{name}` - Get organization secret
  - Source: `api.rs:1329`

- [ ] `PUT /api/orgs/{org_id}/secrets/{name}` - Update organization secret
  - Source: `api.rs:1333`

- [ ] `DELETE /api/orgs/{org_id}/secrets/{name}` - Delete organization secret
  - Source: `api.rs:1337`

### Repository Secrets

- [ ] `GET /api/repos/{repo_id}/secrets` - List repository secrets
  - Source: `api.rs:1342`

- [ ] `POST /api/repos/{repo_id}/secrets` - Create repository secret
  - Source: `api.rs:1346`

- [ ] `GET /api/repos/{repo_id}/secrets/{name}` - Get repository secret
  - Source: `api.rs:1350`

- [ ] `PUT /api/repos/{repo_id}/secrets/{name}` - Update repository secret
  - Source: `api.rs:1354`

- [ ] `DELETE /api/repos/{repo_id}/secrets/{name}` - Delete repository secret
  - Source: `api.rs:1358`

---

## 16. Git Smart Protocol Endpoints

**Spec:** `specs/scm-system.md`
**Source:** `crates/loom-server/src/routes/git.rs`
**Tests:** `crates/loom-server/tests/authz/authz_git_tests.rs`

- [ ] `GET /git/{owner}/{repo}/info/refs` - Advertise refs
  - Source: `routes/git.rs:717-809`
  - Query: `?service=git-upload-pack` or `?service=git-receive-pack`
  - Test: `authz_git_tests.rs` - `test_git_info_refs_nonexistent_repo_returns_404`

- [ ] `POST /git/{owner}/{repo}/git-upload-pack` - Clone/fetch
  - Source: `routes/git.rs:812-885`
  - Test: `authz_git_tests.rs` - `test_git_upload_pack_nonexistent_repo`

- [ ] `POST /git/{owner}/{repo}/git-receive-pack` - Push
  - Source: `routes/git.rs:888-1000`
  - Test: `authz_git_tests.rs` - `test_git_receive_pack_requires_auth`

- [ ] `GET/POST /git/mirrors/{*path}` - Mirror clone/fetch/push
  - Source: `routes/git.rs:1021-1077`
  - Test: `authz_git_tests.rs` - `test_git_mirror_path_routing`

---

## 17. Weaver Endpoints

**Spec:** `specs/weaver-provisioner.md`, `specs/weaver-cli.md`
**Source:** `crates/loom-server/src/routes/weaver.rs`
**Tests:** `crates/loom-server/tests/authz/weaver.rs`

- [ ] `POST /api/weaver` - Create weaver
  - Source: `routes/weaver.rs:93-177`
  - Test: `authz/weaver.rs` - `test_weaver_create_org_member_can_create`, `test_weaver_create_non_member_forbidden`

- [x] `GET /api/weavers` - List weavers ✅ **TESTED 2026-01-16 via loom-cli `weaver ps`**
  - Source: `routes/weaver.rs:193-228`

- [ ] `GET /api/weaver/{id}` - Get weaver
  - Source: `routes/weaver.rs:248-277`

- [ ] `DELETE /api/weaver/{id}` - Delete weaver
  - Source: `routes/weaver.rs:297-343`

- [ ] `GET /api/weaver/{id}/logs` - Stream weaver logs (SSE)
  - Source: `routes/weaver.rs:364-416`

- [ ] `GET /api/weaver/{id}/attach` - WebSocket terminal attach
  - Source: `routes/weaver.rs:500-560`

- [ ] `POST /api/weavers/cleanup` - Cleanup expired weavers (admin)
  - Source: `routes/weaver.rs:433-481`

---

## 18. Internal Weaver Endpoints

**Spec:** `specs/weaver-secrets-system.md`
**Source:** `crates/loom-server/src/routes/weaver_auth.rs`, `weaver_secrets.rs`, `weaver_audit.rs`

- [ ] `POST /internal/weaver-auth/token` - Exchange K8s SA JWT for SVID
  - Source: `routes/weaver_auth.rs:66-335`

- [ ] `GET /internal/weaver-auth/.well-known/jwks.json` - JWKS discovery
  - Source: `routes/weaver_auth.rs:347-364`

- [ ] `GET /internal/weaver-secrets/v1/secrets/{scope}/{name}` - Get secret (SVID auth)
  - Source: `routes/weaver_secrets.rs:92-222`

- [ ] `POST /internal/weaver-audit/events` - Submit audit events
  - Source: `routes/weaver_audit.rs:165-198`

---

## 19. WireGuard Tunnel Endpoints

**Spec:** `specs/wgtunnel-system.md`
**Source:** `crates/loom-server/src/routes/wgtunnel.rs`

### Internal (Public)

- [ ] `POST /internal/wg/weavers` - Register weaver
  - Source: `api.rs:856`

- [ ] `DELETE /internal/wg/weavers/{id}` - Unregister weaver
  - Source: `api.rs:860`

- [ ] `GET /internal/wg/weavers/{id}` - Get weaver
  - Source: `api.rs:864`

- [ ] `GET /internal/wg/weavers/{id}/peers` - Stream peers
  - Source: `api.rs:868`

### Authenticated

- [ ] `POST /api/wg/devices` - Register device
  - Source: `api.rs:1484`

- [ ] `GET /api/wg/devices` - List devices
  - Source: `api.rs:1485`

- [ ] `DELETE /api/wg/devices/{id}` - Revoke device
  - Source: `api.rs:1487`

- [ ] `POST /api/wg/sessions` - Create session
  - Source: `api.rs:1490`

- [ ] `GET /api/wg/sessions` - List sessions
  - Source: `api.rs:1491`

- [ ] `DELETE /api/wg/sessions/{id}` - Terminate session
  - Source: `api.rs:1494`

- [ ] `GET /api/wg/derp-map` - Get DERP map
  - Source: `api.rs:1496`

---

## 20. Feature Flags Endpoints

**Spec:** `specs/feature-flags-system.md`
**Source:** `crates/loom-server/src/routes/flags.rs`
**Tests:** `crates/loom-server/tests/authz/flags.rs`

### Environments

- [ ] `GET /api/orgs/{org_id}/flags/environments` - List environments
  - Source: `routes/flags.rs:78`
  - Test: `authz/flags.rs` - `org_member_can_list_environments`

- [ ] `POST /api/orgs/{org_id}/flags/environments` - Create environment
  - Source: `routes/flags.rs:154`
  - Test: `authz/flags.rs` - `org_member_can_create_environment`

- [ ] `GET /api/orgs/{org_id}/flags/environments/{env_id}` - Get environment
  - Source: `routes/flags.rs:276`

- [ ] `PATCH /api/orgs/{org_id}/flags/environments/{env_id}` - Update environment
  - Source: `routes/flags.rs:367`

- [ ] `DELETE /api/orgs/{org_id}/flags/environments/{env_id}` - Delete environment
  - Source: `routes/flags.rs:517`

### SDK Keys

- [ ] `GET /api/orgs/{org_id}/flags/environments/{env_id}/sdk-keys` - List SDK keys
  - Source: `routes/flags.rs:662`

- [ ] `POST /api/orgs/{org_id}/flags/environments/{env_id}/sdk-keys` - Create SDK key
  - Source: `routes/flags.rs:772`

- [ ] `DELETE /api/orgs/{org_id}/flags/sdk-keys/{key_id}` - Revoke SDK key
  - Source: `routes/flags.rs:921`

### Flags

- [ ] `GET /api/orgs/{org_id}/flags` - List flags
  - Source: `routes/flags.rs:1129`
  - Test: `authz/flags.rs` - `org_member_can_list_flags`

- [ ] `POST /api/orgs/{org_id}/flags` - Create flag
  - Source: `routes/flags.rs:1201`
  - Test: `authz/flags.rs` - `org_member_can_create_flag`

- [ ] `GET /api/orgs/{org_id}/flags/{flag_id}` - Get flag
  - Source: `routes/flags.rs:1391`

- [ ] `PATCH /api/orgs/{org_id}/flags/{flag_id}` - Update flag
  - Source: `routes/flags.rs:1474`

- [ ] `POST /api/orgs/{org_id}/flags/{flag_id}/archive` - Archive flag
  - Source: `routes/flags.rs:1660`

- [ ] `POST /api/orgs/{org_id}/flags/{flag_id}/restore` - Restore flag
  - Source: `routes/flags.rs:1786`

### Flag Configs

- [ ] `GET /api/orgs/{org_id}/flags/{flag_id}/configs` - List flag configs
  - Source: `routes/flags.rs:1931`

- [ ] `GET /api/orgs/{org_id}/flags/{flag_id}/configs/{env_id}` - Get flag config
  - Source: `routes/flags.rs:2062`

- [ ] `PATCH /api/orgs/{org_id}/flags/{flag_id}/configs/{env_id}` - Update flag config
  - Source: `routes/flags.rs:2205`

### Strategies

- [ ] `GET /api/orgs/{org_id}/flags/strategies` - List strategies
  - Source: `routes/flags.rs:2609`
  - Test: `authz/flags.rs` - `org_member_can_list_strategies`

- [ ] `POST /api/orgs/{org_id}/flags/strategies` - Create strategy
  - Source: `routes/flags.rs:2676`
  - Test: `authz/flags.rs` - `org_member_can_create_strategy`

- [ ] `GET /api/orgs/{org_id}/flags/strategies/{strategy_id}` - Get strategy
  - Source: `routes/flags.rs:2795`

- [ ] `PATCH /api/orgs/{org_id}/flags/strategies/{strategy_id}` - Update strategy
  - Source: `routes/flags.rs:2878`

- [ ] `DELETE /api/orgs/{org_id}/flags/strategies/{strategy_id}` - Delete strategy
  - Source: `routes/flags.rs:3044`

### Kill Switches

- [ ] `GET /api/orgs/{org_id}/flags/kill-switches` - List kill switches
  - Source: `routes/flags.rs:3205`
  - Test: `authz/flags.rs` - `org_member_can_list_kill_switches`

- [ ] `POST /api/orgs/{org_id}/flags/kill-switches` - Create kill switch
  - Source: `routes/flags.rs:3273`
  - Test: `authz/flags.rs` - `org_member_can_create_kill_switch`

- [ ] `GET /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}` - Get kill switch
  - Source: `routes/flags.rs:3384`

- [ ] `PATCH /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}` - Update kill switch
  - Source: `routes/flags.rs:3470`

- [ ] `DELETE /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}` - Delete kill switch
  - Source: `routes/flags.rs:3854`

- [ ] `POST /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}/activate` - Activate
  - Source: `routes/flags.rs:3587`

- [ ] `POST /api/orgs/{org_id}/flags/kill-switches/{kill_switch_id}/deactivate` - Deactivate
  - Source: `routes/flags.rs:3728`

### Evaluation

- [ ] `POST /api/orgs/{org_id}/flags/evaluate` - Evaluate all flags
  - Source: `routes/flags.rs:4080`
  - Test: `authz/flags.rs` - `org_member_can_evaluate_flags`

- [ ] `POST /api/orgs/{org_id}/flags/{flag_key}/evaluate` - Evaluate single flag
  - Source: `routes/flags.rs:4345`

### Streaming & Stats

- [ ] `GET /api/flags/stream` - SSE flag updates (SDK key auth)
  - Source: `routes/flags.rs:4673`
  - Test: `authz/flags.rs` - `stream_requires_sdk_key_authentication`

- [ ] `GET /api/flags/stream/stats` - Stream statistics (admin)
  - Source: `routes/flags.rs:4793`
  - Test: `authz/flags.rs` - `admin_can_view_stream_stats`

- [ ] `GET /api/orgs/{org_id}/flags/stale` - List stale flags
  - Source: `routes/flags.rs:4835`
  - Test: `authz/flags.rs` - `org_member_can_list_stale_flags`

- [ ] `GET /api/orgs/{org_id}/flags/{flag_key}/stats` - Get flag stats
  - Source: `routes/flags.rs:4928`

---

## 21. Analytics Endpoints

**Spec:** `specs/analytics-system.md`
**Source:** `crates/loom-server/src/routes/analytics.rs`
**Tests:** `crates/loom-server/tests/authz/analytics.rs`

### Event Capture (API Key - Write)

- [ ] `POST /api/analytics/capture` - Capture single event
  - Source: `routes/analytics.rs:82-119`
  - Test: `authz/analytics.rs` - `write_key_can_capture_event`

- [ ] `POST /api/analytics/batch` - Batch capture events (up to 100)
  - Source: `routes/analytics.rs:122-158`
  - Test: `authz/analytics.rs` - `write_key_can_batch_capture`

### Identity Resolution (API Key - Write)

- [ ] `POST /api/analytics/identify` - Link distinct_id to user_id
  - Source: `routes/analytics.rs:160-197`
  - Test: `authz/analytics.rs` - `write_key_can_identify`

- [ ] `POST /api/analytics/alias` - Link two distinct_ids
  - Source: `routes/analytics.rs:199-236`
  - Test: `authz/analytics.rs` - `write_key_can_alias`

- [ ] `POST /api/analytics/set` - Set person properties
  - Source: `routes/analytics.rs:238-275`
  - Test: `authz/analytics.rs` - `write_key_can_set_properties`

### Person Queries (API Key - ReadWrite)

- [ ] `GET /api/analytics/persons` - List persons
  - Source: `routes/analytics.rs:282-321`
  - Test: `authz/analytics.rs` - `read_write_key_can_list_persons`

- [ ] `GET /api/analytics/persons/{person_id}` - Get person
  - Source: `routes/analytics.rs:323-363`

- [ ] `GET /api/analytics/persons/by-distinct-id/{distinct_id}` - Get by distinct_id
  - Source: `routes/analytics.rs:365-405`

### Event Queries (API Key - ReadWrite)

- [ ] `GET /api/analytics/events` - List events
  - Source: `routes/analytics.rs:407-446`
  - Test: `authz/analytics.rs` - `read_write_key_can_list_events`

- [ ] `GET /api/analytics/events/count` - Count events
  - Source: `routes/analytics.rs:448-487`
  - Test: `authz/analytics.rs` - `read_write_key_can_count_events`

- [ ] `POST /api/analytics/events/export` - Export events
  - Source: `routes/analytics.rs:489-542`
  - Test: `authz/analytics.rs` - `read_write_key_can_export_events`

### API Key Management (User Auth)

- [ ] `GET /api/orgs/{org_id}/analytics/api-keys` - List API keys
  - Source: `routes/analytics.rs:548-608`
  - Test: `authz/analytics.rs` - `org_member_can_list_api_keys`

- [ ] `POST /api/orgs/{org_id}/analytics/api-keys` - Create API key
  - Source: `routes/analytics.rs:610-691`
  - Test: `authz/analytics.rs` - `org_member_can_create_api_key`

- [ ] `DELETE /api/orgs/{org_id}/analytics/api-keys/{key_id}` - Revoke API key
  - Source: `routes/analytics.rs:693-767`
  - Test: `authz/analytics.rs` - `org_member_can_revoke_api_key`

---

## 22. SCIM Endpoints

**Spec:** `specs/scim-system.md`
**Source:** `crates/loom-server-scim/src/`
**Base Path:** `/api/scim`

### Discovery

- [ ] `GET /api/scim/ServiceProviderConfig` - Service provider config
  - Source: `handlers/service_provider.rs:9`

- [ ] `GET /api/scim/Schemas` - List schemas
  - Source: `handlers/schemas.rs:112`

- [ ] `GET /api/scim/Schemas/{id}` - Get schema
  - Source: `handlers/schemas.rs:117`

- [ ] `GET /api/scim/ResourceTypes` - List resource types
  - Source: `handlers/resource_types.rs:47`

- [ ] `GET /api/scim/ResourceTypes/{id}` - Get resource type
  - Source: `handlers/resource_types.rs:52`

### Users

- [ ] `GET /api/scim/Users` - List users
  - Source: `handlers/users.rs:70`

- [ ] `POST /api/scim/Users` - Create user
  - Source: `handlers/users.rs:113`

- [ ] `GET /api/scim/Users/{id}` - Get user
  - Source: `handlers/users.rs:161`

- [ ] `PUT /api/scim/Users/{id}` - Replace user
  - Source: `handlers/users.rs:177`

- [ ] `PATCH /api/scim/Users/{id}` - Patch user
  - Source: `handlers/users.rs:220`

- [ ] `DELETE /api/scim/Users/{id}` - Delete user
  - Source: `handlers/users.rs:265`

### Groups

- [ ] `GET /api/scim/Groups` - List groups
  - Source: `handlers/groups.rs:98`

- [ ] `POST /api/scim/Groups` - Create group
  - Source: `handlers/groups.rs:145`

- [ ] `GET /api/scim/Groups/{id}` - Get group
  - Source: `handlers/groups.rs:189`

- [ ] `PUT /api/scim/Groups/{id}` - Replace group
  - Source: `handlers/groups.rs:206`

- [ ] `PATCH /api/scim/Groups/{id}` - Patch group
  - Source: `handlers/groups.rs:248`

- [ ] `DELETE /api/scim/Groups/{id}` - Delete group
  - Source: `handlers/groups.rs:330`

### Bulk Operations

- [ ] `POST /api/scim/Bulk` - Bulk operations
  - Source: `handlers/bulk.rs:111`

---

## 23. LLM Proxy Endpoints

**Spec:** `specs/llm-client.md`, `specs/streaming.md`
**Source:** `crates/loom-server/src/llm_proxy.rs`

- [ ] `POST /proxy/anthropic/complete` - Anthropic completion
  - Source: `llm_proxy.rs:93-128`

- [ ] `POST /proxy/anthropic/stream` - Anthropic streaming (SSE)
  - Source: `llm_proxy.rs:132-160`

- [ ] `POST /proxy/openai/complete` - OpenAI completion
  - Source: `llm_proxy.rs:164-199`

- [ ] `POST /proxy/openai/stream` - OpenAI streaming (SSE)
  - Source: `llm_proxy.rs:203-231`

- [ ] `POST /proxy/vertex/complete` - Vertex AI completion
  - Source: `llm_proxy.rs:235-270`

- [ ] `POST /proxy/vertex/stream` - Vertex AI streaming (SSE)
  - Source: `llm_proxy.rs:274-302`

---

## 24. Server Query Endpoints

**Spec:** `specs/server-query-phase-2.md`
**Source:** `crates/loom-server/src/server_query.rs`
**Tests:** `crates/loom-server/tests/query_*.rs`, `crates/loom-server/tests/end_to_end_tests.rs`

- [ ] `POST /api/sessions/{session_id}/query-response` - Send query response
  - Source: `api.rs:1443`
  - Test: `query_integration_test.rs:55` - `test_single_query_send_and_receive`

- [ ] `GET /api/sessions/{session_id}/queries` - List pending queries
  - Source: `api.rs:1447`
  - Test: `query_manager_integration_tests.rs:99` - `test_list_pending_queries`

---

## 25. Debug/Tracing Endpoints

**Source:** `crates/loom-server/src/routes/debug.rs`
**Tests:** `crates/loom-server/tests/query_tracing_tests.rs`, `crates/loom-server/tests/tracing_integration_tests.rs`

- [ ] `GET /api/debug/query-traces/{trace_id}` - Get query trace
  - Source: `api.rs:1452`

- [ ] `GET /api/debug/query-traces` - List query traces
  - Source: `api.rs:1456`

- [ ] `GET /api/debug/query-traces/stats` - Get trace statistics
  - Source: `api.rs:1460`

---

## 26. Search Proxy Endpoints

**Spec:** `specs/web-search-system.md`
**Source:** `crates/loom-server/src/routes/cse.rs`, `serper.rs`

- [ ] `POST /proxy/cse` - Google Custom Search Engine proxy
  - Source: `api.rs:1391`

- [ ] `POST /proxy/serper` - Serper search proxy
  - Source: `api.rs:1393`

---

## 27. GitHub Integration Endpoints

**Spec:** `specs/github-app-system.md`
**Source:** `crates/loom-server/src/routes/github.rs`

- [ ] `POST /api/github/webhook` - GitHub webhook receiver (public, signature verified)
  - Source: `api.rs:837`

- [ ] `GET /api/github/app` - Get GitHub App info
  - Source: `api.rs:1397`

- [ ] `GET /api/github/installations/by-repo` - Get installation by repo
  - Source: `api.rs:1401`

- [ ] `POST /proxy/github/search-code` - GitHub code search proxy
  - Source: `api.rs:1405`

- [ ] `POST /proxy/github/repo-info` - GitHub repo info proxy
  - Source: `api.rs:1409`

- [ ] `POST /proxy/github/file-contents` - GitHub file contents proxy
  - Source: `api.rs:1413`

---

## 28. Maintenance Endpoints

**Source:** `crates/loom-server/src/routes/maintenance.rs`

- [ ] `POST /api/repos/{id}/maintenance` - Trigger repository maintenance
  - Source: `api.rs:1380`

- [ ] `GET /api/repos/{id}/maintenance/jobs` - List maintenance jobs
  - Source: `api.rs:1384`

- [ ] `POST /api/admin/maintenance/sweep` - Global maintenance sweep (admin)
  - Source: `api.rs:1388`

---

## 29. Admin Endpoints

**Source:** `crates/loom-server/src/routes/admin.rs`, `admin_anthropic.rs`, `admin_jobs.rs`, `admin_logs.rs`, `admin_flags.rs`
**Tests:** `crates/loom-server/tests/authz/admin.rs`

### User Management

- [ ] `GET /api/admin/users` - List all users
  - Source: `api.rs:674`
  - Test: `authz/admin.rs` - `admin_can_list_all_users`

- [ ] `DELETE /api/admin/users/{id}` - Delete user
  - Source: `api.rs:675`

- [ ] `PATCH /api/admin/users/{id}/roles` - Update user roles
  - Source: `api.rs:678`
  - Test: `authz/admin.rs` - `admin_can_update_roles`

### Impersonation

- [ ] `GET /api/admin/impersonate/state` - Get impersonation state
  - Source: `api.rs:682`
  - Test: `authz/admin.rs` - `admin_can_get_impersonation_state`

- [ ] `POST /api/admin/users/{id}/impersonate` - Start impersonation
  - Source: `api.rs:686`
  - Test: `authz/admin.rs` - `admin_can_impersonate`

- [ ] `POST /api/admin/impersonate/stop` - Stop impersonation
  - Source: `api.rs:690`
  - Test: `authz/admin.rs` - `admin_can_stop_impersonation`

### Audit Logs

- [ ] `GET /api/admin/audit-logs` - List audit logs
  - Source: `api.rs:692`
  - Test: `authz/admin.rs` - `admin_can_list_audit_logs`

### Anthropic OAuth Pool

- [ ] `GET /api/admin/anthropic/accounts` - List Anthropic accounts
  - Source: `api.rs:696`

- [ ] `DELETE /api/admin/anthropic/accounts/{id}` - Remove Anthropic account
  - Source: `api.rs:700`

- [ ] `POST /api/admin/anthropic/oauth/initiate` - Initiate OAuth
  - Source: `api.rs:704`

- [ ] `POST /api/admin/anthropic/oauth/complete` - Complete OAuth
  - Source: `api.rs:708`

### Job Management

- [ ] `GET /api/admin/jobs` - List jobs
  - Source: `api.rs:711`

- [ ] `POST /api/admin/jobs/{job_id}/run` - Trigger job
  - Source: `api.rs:712`

- [ ] `POST /api/admin/jobs/{job_id}/cancel` - Cancel job
  - Source: `api.rs:715`

- [ ] `GET /api/admin/jobs/{job_id}/history` - Job history
  - Source: `api.rs:719`

- [ ] `POST /api/admin/jobs/{job_id}/enable` - Enable job
  - Source: `api.rs:723`

- [ ] `POST /api/admin/jobs/{job_id}/disable` - Disable job
  - Source: `api.rs:727`

### Logs

- [ ] `GET /api/admin/logs` - List logs
  - Source: `api.rs:730`

- [ ] `GET /api/admin/logs/stream` - Stream logs (SSE)
  - Source: `api.rs:731`

### Platform Flags

- [ ] `GET /api/admin/flags` - List platform flags
  - Source: `admin_flags.rs:157`
  - Test: `authz/admin.rs` - `admin_can_list_platform_flags`

- [ ] `POST /api/admin/flags` - Create platform flag
  - Source: `admin_flags.rs:225`
  - Test: `authz/admin.rs` - `admin_can_create_platform_flag`

- [ ] `GET /api/admin/flags/{key}` - Get platform flag
  - Source: `admin_flags.rs:357`

- [ ] `PATCH /api/admin/flags/{key}` - Update platform flag
  - Source: `admin_flags.rs:414`

- [ ] `DELETE /api/admin/flags/{key}` - Archive platform flag
  - Source: `admin_flags.rs:527`

- [ ] `POST /api/admin/flags/{key}/restore` - Restore platform flag
  - Source: `admin_flags.rs:612`

### Platform Kill Switches

- [ ] `GET /api/admin/flags/kill-switches` - List platform kill switches
  - Source: `admin_flags.rs:697`
  - Test: `authz/admin.rs` - `admin_can_list_platform_kill_switches`

- [ ] `POST /api/admin/flags/kill-switches` - Create platform kill switch
  - Source: `admin_flags.rs:761`
  - Test: `authz/admin.rs` - `admin_can_create_platform_kill_switch`

- [ ] `GET /api/admin/flags/kill-switches/{key}` - Get platform kill switch
  - Source: `admin_flags.rs:871`

- [ ] `PATCH /api/admin/flags/kill-switches/{key}` - Update platform kill switch
  - Source: `admin_flags.rs:928`

- [ ] `DELETE /api/admin/flags/kill-switches/{key}` - Delete platform kill switch
  - Source: `admin_flags.rs:1021`

- [ ] `POST /api/admin/flags/kill-switches/{key}/activate` - Activate
  - Source: `admin_flags.rs:1108`

- [ ] `POST /api/admin/flags/kill-switches/{key}/deactivate` - Deactivate
  - Source: `admin_flags.rs:1216`

### Platform Strategies

- [ ] `GET /api/admin/flags/strategies` - List platform strategies
  - Source: `admin_flags.rs:1308`
  - Test: `authz/admin.rs` - `admin_can_list_platform_strategies`

---

## 30. WebSocket Endpoint

**Spec:** `specs/phase3_websocket_planning.md`
**Source:** `crates/loom-server/src/websocket/handler.rs`

- [ ] `GET /api/ws/sessions/{session_id}` - WebSocket upgrade (first-message auth)
  - Source: `api.rs:1521`

---

## 31. Documentation Endpoints

**Spec:** `specs/docs-system.md`
**Source:** `crates/loom-server/src/routes/docs.rs`

- [ ] `GET /docs/search` - Search documentation
  - Source: `routes/docs.rs:55`
  - Query: `?q=<query>&diataxis=<type>&limit=20&offset=0`

---

## 32. Binary Distribution Endpoints

**Spec:** `specs/distribution.md`
**Source:** `crates/loom-server/src/routes/bin.rs`

- [ ] `GET /bin/*` - Serve binary files
  - Source: `api.rs:1525-1530`

---

## 33. OpenAPI Documentation

**Spec:** `specs/api-documentation.md`
**Source:** `crates/loom-server/src/api_docs.rs`

- [ ] `GET /api` - Swagger UI
  - Source: `api.rs:1560`

- [ ] `GET /api/openapi.json` - OpenAPI specification
  - Source: `api.rs:1560`

---

## Test Execution Guidelines

### Running Existing Tests

```bash
# Run all server tests
cargo test -p loom-server

# Run specific test file
cargo test -p loom-server --test auth_integration_tests

# Run authorization tests
cargo test -p loom-server --test authz_tests

# Run query-related tests
cargo test -p loom-server --test query_integration_test
cargo test -p loom-server --test query_detection_tests
cargo test -p loom-server --test query_security_tests

# Run end-to-end tests
cargo test -p loom-server --test end_to_end_tests
```

### Manual Testing with curl

```bash
# Health check
curl http://localhost:9090/health | jq .

# Metrics
curl http://localhost:9090/metrics

# Auth providers (public)
curl http://localhost:9090/auth/providers | jq .

# Authenticated request (with session cookie or bearer token)
curl -H "Authorization: Bearer <token>" http://localhost:9090/auth/me | jq .
```

### Test Categories Summary

| Category | Endpoint Count | Test File(s) |
|----------|---------------|--------------|
| Health/Metrics | 2 | (manual) |
| Auth | 16 | auth_integration_tests.rs, authz/auth.rs |
| Sessions | 2 | authz/users.rs |
| Threads | 6 | authz/threads.rs |
| Sharing | 5 | share_tests.rs |
| Organizations | 8 | authz/orgs.rs |
| Teams | 8 | authz/orgs.rs, authz_scm_team_tests.rs |
| Users | 6 | authz/users.rs |
| API Keys | 4 | (none) |
| Invitations | 8 | (none) |
| Repos | 9 | authz/repos.rs |
| Branch Protection | 3 | (none) |
| Webhooks | 6 | (none) |
| Mirrors | 4 | (none) |
| Secrets | 10 | (none) |
| Git Protocol | 4 | authz_git_tests.rs |
| Weavers | 7 | authz/weaver.rs |
| Internal Weaver | 4 | (none) |
| WireGuard | 9 | (none) |
| Feature Flags | 30 | authz/flags.rs |
| Analytics | 14 | authz/analytics.rs |
| SCIM | 14 | (none) |
| LLM Proxy | 6 | (none) |
| Server Query | 2 | query_*.rs, end_to_end_tests.rs |
| Debug | 3 | query_tracing_tests.rs |
| Search Proxy | 2 | (none) |
| GitHub | 6 | (none) |
| Maintenance | 3 | (none) |
| Admin | 31 | authz/admin.rs |
| WebSocket | 1 | (none) |
| Docs | 1 | (none) |
| Binary | 1 | (none) |
| OpenAPI | 2 | (none) |

**Total Endpoints: ~250+**
