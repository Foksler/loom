# Agent Provisioner Implementation Plan

This document tracks the implementation of the Agent Provisioner feature as specified in
[specs/agent-provisioner.md](./specs/agent-provisioner.md).

**Status: ✅ IMPLEMENTED**

---

## Phase 1: Foundation ✅

### 1.1 Workspace Setup ✅

- [x] Add `uuid7 = "1"` to workspace dependencies in `Cargo.toml`
- [x] Add `kube = { version = "0.98", features = ["runtime", "client", "derive"] }` to workspace dependencies
- [x] Add `k8s-openapi = { version = "0.24", features = ["v1_32"] }` to workspace dependencies
- [x] Add `hmac`, `sha2` for webhook signatures

### 1.2 Create loom-k8s Crate ✅

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

### 1.3 Create loom-agent-provisioner Crate ✅

- [x] Create `crates/loom-agent-provisioner/Cargo.toml`
- [x] Create `crates/loom-agent-provisioner/src/lib.rs`
- [x] Define `AgentId` type (wraps uuid7)
- [x] Define `AgentStatus` enum (Pending, Running, Succeeded, Failed)
- [x] Define `Agent` struct
- [x] Define `CreateAgentRequest` struct
- [x] Define `ResourceSpec` struct
- [x] Define `AgentConfig` struct (from env vars)
- [x] Define `ProvisionerError` error type
- [x] Add to workspace members in root `Cargo.toml`

---

## Phase 2: Core Provisioner Logic ✅

### 2.1 Provisioner Implementation ✅

- [x] Create `Provisioner` struct
- [x] Implement `Provisioner::new(client, config)`
- [x] Implement `create_agent()` with:
  - [x] Lifetime validation
  - [x] Max concurrent limit check
  - [x] UUID7 agent ID generation
  - [x] Pod spec building with security context
  - [x] Poll until ready with timeout
- [x] Implement `list_agents()` with tag filtering
- [x] Implement `get_agent()`
- [x] Implement `delete_agent()` with 5s grace period
- [x] Implement `count_active_agents()`
- [x] Implement `validate_namespace()`

### 2.2 Cleanup System ✅

- [x] Implement `find_expired_agents()`
- [x] Implement `cleanup_expired_agents()`
- [x] Implement `start_cleanup_task()` background task
- [x] Define `CleanupResult` struct

### 2.3 Log Streaming ✅

- [x] Implement `stream_logs()`
- [x] Define `LogStreamOptions` struct

---

## Phase 3: Webhook System ✅

- [x] Define `WebhookPayload` struct
- [x] Define `WebhookAgentPayload` struct
- [x] Implement `WebhookDispatcher` struct
- [x] Implement HMAC-SHA256 signature computation
- [x] Implement fire-and-forget webhook dispatch
- [x] Create payload helper methods

---

## Phase 4: HTTP API Integration ✅

### 4.1 Configuration ✅

- [x] Add agent config fields to `ServerConfig`
- [x] Parse from environment variables

### 4.2 API Middleware ✅

- [x] Implement `require_agent_api_key` middleware

### 4.3 API Handlers ✅

- [x] Create `routes/agent.rs`
- [x] Implement `POST /api/agent` handler
- [x] Implement `GET /api/agents` handler
- [x] Implement `GET /api/agent/:id` handler
- [x] Implement `DELETE /api/agent/:id` handler
- [x] Implement `GET /api/agent/:id/logs` handler (SSE)
- [x] Implement `POST /api/agents/cleanup` handler

### 4.4 Router Integration ✅

- [x] Create `agent_routes()` function
- [x] Merge into main router
- [x] Apply API key middleware

### 4.5 Error Handling ✅

- [x] Map `ProvisionerError` to HTTP responses

---

## Phase 5: Health Check & Metrics ✅

### 5.1 Health Check ✅

- [x] Add `KubernetesHealth` struct
- [x] Implement K8s connectivity check
- [x] Add to `/health` response

### 5.2 Prometheus Metrics ✅

- [x] Define `AgentMetrics` struct with counters/gauges
- [x] Implement metric methods
- [x] Register with Prometheus registry

---

## Phase 6: Startup & Lifecycle ✅

- [x] Validate namespace on startup
- [x] Spawn cleanup background task
- [x] Graceful shutdown with task cancellation

---

## Phase 7: Documentation & OpenAPI ✅

- [x] Add `#[utoipa::path]` to all handlers
- [x] Add agent endpoints to `ApiDoc`
- [x] Add agent schemas to components
- [x] Add `agents` tag

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
| loom-agent-provisioner | `Cargo.toml` | Crate manifest |
| loom-agent-provisioner | `src/lib.rs` | Module exports |
| loom-agent-provisioner | `src/error.rs` | `ProvisionerError` enum |
| loom-agent-provisioner | `src/types.rs` | Core types |
| loom-agent-provisioner | `src/config.rs` | `AgentConfig`, webhooks |
| loom-agent-provisioner | `src/provisioner.rs` | Main provisioner logic |
| loom-agent-provisioner | `src/cleanup.rs` | Background cleanup task |
| loom-agent-provisioner | `src/webhook.rs` | Webhook dispatcher |
| loom-server | `src/routes/agent.rs` | HTTP handlers |
| loom-server | `src/agent_metrics.rs` | Prometheus metrics |

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LOOM_SERVER_AGENT_ENABLED` | `false` | Enable agent provisioning |
| `LOOM_SERVER_AGENT_API_KEY` | (required) | API key for authentication |
| `LOOM_SERVER_K8S_NAMESPACE` | `loom-agents` | Target namespace |
| `LOOM_SERVER_AGENT_CLEANUP_INTERVAL_SECS` | `1800` | Cleanup interval (30 min) |
| `LOOM_SERVER_AGENT_DEFAULT_TTL_HOURS` | `4` | Default agent lifetime |
| `LOOM_SERVER_AGENT_MAX_TTL_HOURS` | `48` | Maximum lifetime |
| `LOOM_SERVER_AGENT_MAX_CONCURRENT` | `64` | Maximum running agents |
| `LOOM_SERVER_AGENT_READY_TIMEOUT_SECS` | `60` | Timeout for pod ready |
| `LOOM_SERVER_AGENT_WEBHOOKS` | `[]` | JSON array of webhooks |

---

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/agent` | Provision new agent |
| GET | `/api/agents` | List managed agents |
| GET | `/api/agent/{id}` | Get agent details |
| DELETE | `/api/agent/{id}` | Delete agent |
| GET | `/api/agent/{id}/logs` | SSE log stream |
| POST | `/api/agents/cleanup` | Manual cleanup |
