# Weaver Provisioner Implementation Plan

Implementation checklist for the Weaver Provisioner feature. See
[specs/weaver-provisioner.md](./specs/weaver-provisioner.md) for specification.

---

## Phase 1: Foundation

### 1.1 Workspace Setup

- [x] Add `uuid7 = "1"` to workspace dependencies in `Cargo.toml`
- [x] Add `kube = { version = "0.98", features = ["runtime", "client", "derive"] }` to workspace dependencies
- [x] Add `k8s-openapi = { version = "0.24", features = ["v1_32"] }` to workspace dependencies
- [x] Add `hmac`, `sha2` for webhook signatures

### 1.2 Create loom-k8s Crate

- [x] Create `crates/loom-k8s/Cargo.toml`
- [x] Create `crates/loom-k8s/src/lib.rs`
- [x] Define `K8sClient` trait with methods:
  - [x] `create_pod()`
  - [x] `delete_pod()`
  - [x] `list_pods()`
  - [x] `get_pod()`
  - [x] `get_namespace()`
  - [x] `stream_logs()`
- [x] Define `K8sError` error type
- [x] Define `LogStream` type
- [x] Define `LogOptions` struct (tail, timestamps)
- [x] Implement `KubeClient` struct using `kube` crate
- [x] Implement `K8sClient` trait for `KubeClient`
- [x] Add to workspace members in root `Cargo.toml`

### 1.3 Create loom-weaver Crate

- [x] Create `crates/loom-weaver/Cargo.toml`
- [x] Create `crates/loom-weaver/src/lib.rs`
- [x] Define `WeaverId` type (wraps uuid7)
- [x] Define `WeaverStatus` enum (Pending, Running, Succeeded, Failed)
- [x] Define `Weaver` struct
- [x] Define `CreateWeaverRequest` struct
- [x] Define `ResourceSpec` struct
- [x] Define `WeaverConfig` struct (from env vars)
- [x] Define `ProvisionerError` error type
- [x] Add to workspace members in root `Cargo.toml`

---

## Phase 2: Core Provisioner Logic

### 2.1 Provisioner Implementation

- [x] Create `Provisioner` struct
- [x] Implement `Provisioner::new(client, config)`
- [x] Implement `create_weaver()` with:
  - [x] Lifetime validation
  - [x] Max concurrent limit check
  - [x] UUID7 weaver ID generation
  - [x] Pod spec building with security context
  - [x] Poll until ready with timeout
- [x] Implement `list_weavers()` with tag filtering
- [x] Implement `get_weaver()`
- [x] Implement `delete_weaver()` with 5s grace period
- [x] Implement `count_active_weavers()`
- [x] Implement `validate_namespace()`

### 2.2 Cleanup System

- [x] Implement `find_expired_weavers()`
- [x] Implement `cleanup_expired_weavers()`
- [x] Implement `start_cleanup_task()` background task
- [x] Define `CleanupResult` struct

### 2.3 Log Streaming

- [x] Implement `stream_logs()`
- [x] Define `LogStreamOptions` struct

---

## Phase 3: Webhook System

- [x] Define `WebhookPayload` struct
- [x] Define `WebhookWeaverPayload` struct
- [x] Implement `WebhookDispatcher` struct
- [x] Implement HMAC-SHA256 signature computation
- [x] Implement fire-and-forget webhook dispatch
- [x] Create payload helper methods

---

## Phase 4: HTTP API Integration

### 4.1 Configuration

- [x] Add weaver config fields to `ServerConfig`
- [x] Parse from environment variables

### 4.2 API Middleware

- [x] Implement `require_weaver_api_key` middleware

### 4.3 API Handlers

- [x] Create `routes/weaver.rs`
- [x] Implement `POST /api/weaver` handler
- [x] Implement `GET /api/weavers` handler
- [x] Implement `GET /api/weaver/:id` handler
- [x] Implement `DELETE /api/weaver/:id` handler
- [x] Implement `GET /api/weaver/:id/logs` handler (SSE)
- [x] Implement `POST /api/weavers/cleanup` handler

### 4.4 Router Integration

- [x] Create `weaver_routes()` function
- [x] Merge into main router
- [x] Apply API key middleware

### 4.5 Error Handling

- [x] Map `ProvisionerError` to HTTP responses

---

## Phase 5: Health Check & Metrics

### 5.1 Health Check

- [x] Add `KubernetesHealth` struct
- [x] Implement K8s connectivity check
- [x] Add to `/health` response

### 5.2 Prometheus Metrics

- [x] Define `WeaverMetrics` struct with counters/gauges
- [x] Implement metric methods
- [x] Register with Prometheus registry

---

## Phase 6: Startup & Lifecycle

- [x] Validate namespace on startup
- [x] Spawn cleanup background task
- [x] Graceful shutdown with task cancellation

---

## Phase 7: Documentation & OpenAPI

- [x] Add `#[utoipa::path]` to all handlers
- [x] Add weaver endpoints to `ApiDoc`
- [x] Add weaver schemas to components
- [x] Add `weavers` tag

---

## Files Created

| Crate | File | Description |
|-------|------|-------------|
| loom-k8s | `Cargo.toml` | Crate manifest |
| loom-k8s | `src/lib.rs` | Module exports |
| loom-k8s | `src/error.rs` | `K8sError` enum |
| loom-k8s | `src/types.rs` | `LogOptions`, `LogStream` |
| loom-k8s | `src/client.rs` | `K8sClient` trait |
| loom-k8s | `src/kube_client.rs` | `KubeClient` implementation |
| loom-weaver | `Cargo.toml` | Crate manifest |
| loom-weaver | `src/lib.rs` | Module exports |
| loom-weaver | `src/error.rs` | `ProvisionerError` enum |
| loom-weaver | `src/types.rs` | Core types |
| loom-weaver | `src/config.rs` | `WeaverConfig`, webhooks |
| loom-weaver | `src/provisioner.rs` | Main provisioner logic |
| loom-weaver | `src/cleanup.rs` | Background cleanup task |
| loom-weaver | `src/webhook.rs` | Webhook dispatcher |
| loom-server | `src/routes/weaver.rs` | HTTP handlers |
| loom-server | `src/weaver_metrics.rs` | Prometheus metrics |

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LOOM_SERVER_WEAVER_ENABLED` | `false` | Enable weaver provisioning |
| `LOOM_SERVER_WEAVER_API_KEY` | (required) | API key for authentication |
| `LOOM_SERVER_WEAVER_K8S_NAMESPACE` | `loom-weavers` | Target namespace |
| `LOOM_SERVER_WEAVER_CLEANUP_INTERVAL_SECS` | `1800` | Cleanup interval (30 min) |
| `LOOM_SERVER_WEAVER_DEFAULT_TTL_HOURS` | `4` | Default weaver lifetime |
| `LOOM_SERVER_WEAVER_MAX_TTL_HOURS` | `48` | Maximum lifetime |
| `LOOM_SERVER_WEAVER_MAX_CONCURRENT` | `64` | Maximum running weavers |
| `LOOM_SERVER_WEAVER_READY_TIMEOUT_SECS` | `60` | Timeout for pod ready |
| `LOOM_SERVER_WEAVER_WEBHOOKS` | `[]` | JSON array of webhooks |

---

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/weaver` | Provision new weaver |
| GET | `/api/weavers` | List managed weavers |
| GET | `/api/weaver/{id}` | Get weaver details |
| DELETE | `/api/weaver/{id}` | Delete weaver |
| GET | `/api/weaver/{id}/logs` | SSE log stream |
| POST | `/api/weavers/cleanup` | Manual cleanup |

---

## NixOS Infrastructure

| Component | File |
|-----------|------|
| k3s module | `infra/nixos-modules/k3s.nix` |
| loom-server weaver config | `infra/nixos-modules/loom-server.nix` |
| Machine config | `infra/machines/loom.nix` |
| Weaver API key secret | `infra/secrets/loom.yaml` |
