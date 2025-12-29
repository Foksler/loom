<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Agent Provisioner Specification

**Status:** Draft\
**Version:** 1.0\
**Last Updated:** 2025-01-29

---

## 1. Overview

### Purpose

The Agent Provisioner is a Rust-based infrastructure component for creating, managing, and monitoring
ephemeral, isolated execution environments using Kubernetes Pods. It provides a REST API for
provisioning agents with automatic TTL-based cleanup.

### Goals

- **Ephemerality**: All workloads are short-lived (default TTL: 4 hours, max: 48 hours)
- **Isolation**: Pods run with security-hardened contexts (non-root, dropped capabilities)
- **Observability**: Real-time SSE log streaming, Prometheus metrics
- **Simplicity**: K8s is source of truth, no database for agent state
- **Testability**: Separate crates for K8s abstraction and business logic

### Non-Goals

- Persistent storage for agents
- Multi-cluster support
- Custom network policies (use cluster defaults)
- Image registry authentication (handled by cluster)

---

## 2. Architecture

### 2.1 Crate Structure

```
crates/
├── loom-k8s/                # K8s client trait + kube implementation
├── loom-agent-provisioner/  # Business logic, cleanup, webhooks
└── loom-server/             # HTTP endpoints (/api/agent*)
```

### 2.2 Dependency Graph

```
┌─────────────────┐
│   loom-server   │
│  (HTTP routes)  │
└────────┬────────┘
         │
         ▼
┌─────────────────────────┐
│  loom-agent-provisioner │
│   (business logic)      │
└────────┬────────────────┘
         │
         ▼
┌─────────────────┐
│    loom-k8s     │
│  (K8s client)   │
└─────────────────┘
```

### 2.3 K8s Client Abstraction

The `loom-k8s` crate provides a trait-based abstraction for testability:

```rust
#[async_trait]
pub trait K8sClient: Send + Sync {
    async fn create_pod(&self, spec: PodSpec) -> Result<Pod, K8sError>;
    async fn delete_pod(&self, name: &str, namespace: &str, grace_period: u32) -> Result<(), K8sError>;
    async fn list_pods(&self, namespace: &str, label_selector: &str) -> Result<Vec<Pod>, K8sError>;
    async fn get_pod(&self, name: &str, namespace: &str) -> Result<Pod, K8sError>;
    async fn get_namespace(&self, name: &str) -> Result<Namespace, K8sError>;
    async fn stream_logs(&self, name: &str, namespace: &str, opts: LogOptions) -> Result<LogStream, K8sError>;
}
```

Real implementation uses the `kube` crate. Tests use mock implementations.

---

## 3. Agent Identification

### 3.1 UUID7

Agents are identified using UUID7 (time-ordered, globally unique):

```rust
use uuid7::uuid7;

pub struct AgentId(uuid7::Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(uuid7())
    }
}
```

Add `uuid7` as a workspace dependency.

### 3.2 Pod Naming

Pod name format: `agent-{uuid7}`

Example: `agent-018f6b2a-3b4c-7d8e-9f0a-1b2c3d4e5f6g`

### 3.3 K8s Labels and Annotations

```yaml
metadata:
  name: agent-018f6b2a-...
  labels:
    loom.dev/managed: "true"
    loom.dev/agent-id: "018f6b2a-..."
  annotations:
    loom.dev/tags: '{"project":"ai-worker","env":"prod"}'
    loom.dev/lifetime-hours: "4"
```

---

## 4. State Management

### 4.1 K8s as Source of Truth

- **No database** for agent state
- All operations query K8s directly
- Labels enable filtering (`loom.dev/managed=true`)
- Annotations store metadata (tags, lifetime)

### 4.2 Agent Status

Mapped from K8s Pod phase:

```rust
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Pending,    // Pod created, containers starting
    Running,    // Containers running
    Succeeded,  // Completed successfully (exit 0)
    Failed,     // Container failed (non-zero exit)
}
```

---

## 5. API Endpoints

### 5.1 Endpoint Summary

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/agent` | Provision new agent |
| GET | `/api/agents` | List managed agents |
| GET | `/api/agent/{id}` | Get agent details |
| DELETE | `/api/agent/{id}` | Delete agent |
| GET | `/api/agent/{id}/logs` | SSE log stream |
| POST | `/api/agents/cleanup` | Manual cleanup trigger |

### 5.2 Authentication

All `/api/agent*` endpoints require API key authentication:

```
X-API-Key: sk-xxxxx
```

Configured via `LOOM_SERVER_AGENT_API_KEY` environment variable.

### 5.3 POST /api/agent

Provision a new agent.

**Request:**

```json
{
  "image": "python:3.12",
  "env": {
    "TASK_ID": "abc123",
    "API_URL": "https://api.example.com"
  },
  "resources": {
    "memory_limit": "8Gi",
    "cpu_limit": "4"
  },
  "tags": {
    "project": "ai-worker",
    "env": "prod"
  },
  "lifetime_hours": 8,
  "command": ["/bin/sh", "-c"],
  "args": ["python worker.py"],
  "workdir": "/app"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `image` | string | Yes | Container image |
| `env` | object | No | Environment variables |
| `resources` | object | No | Resource limits |
| `tags` | object | No | User-defined metadata |
| `lifetime_hours` | u32 | No | TTL override (max: 48) |
| `command` | string[] | No | Override ENTRYPOINT |
| `args` | string[] | No | Override CMD |
| `workdir` | string | No | Override WORKDIR |

**Response (201 Created):**

```json
{
  "id": "018f6b2a-3b4c-7d8e-9f0a-1b2c3d4e5f6g",
  "pod_name": "agent-018f6b2a-3b4c-7d8e-9f0a-1b2c3d4e5f6g",
  "status": "running",
  "created_at": "2025-01-15T12:34:56Z"
}
```

**Behavior:**
- Waits until Pod reaches `Running` or error state (timeout configurable)
- Returns `429 Too Many Requests` if max concurrent limit reached
- Returns `400 Bad Request` if lifetime exceeds max

### 5.4 GET /api/agents

List all managed agents.

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `tag` | string | Filter by tag (e.g., `project:ai-worker`). Multiple allowed. |

**Response:**

```json
{
  "agents": [
    {
      "id": "018f6b2a-...",
      "pod_name": "agent-018f6b2a-...",
      "status": "running",
      "image": "python:3.12",
      "tags": {"project": "ai-worker"},
      "created_at": "2025-01-15T12:34:56Z",
      "lifetime_hours": 4
    }
  ],
  "count": 1
}
```

### 5.5 GET /api/agent/{id}

Get agent details.

**Response:**

```json
{
  "id": "018f6b2a-...",
  "pod_name": "agent-018f6b2a-...",
  "status": "running",
  "image": "python:3.12",
  "tags": {"project": "ai-worker"},
  "created_at": "2025-01-15T12:34:56Z",
  "lifetime_hours": 4,
  "age_hours": 2.5
}
```

### 5.6 DELETE /api/agent/{id}

Delete an agent.

**Response (204 No Content)**

Grace period: 5 seconds (hardcoded).

### 5.7 GET /api/agent/{id}/logs

Stream logs via SSE.

**Query Parameters:**

| Param | Default | Description |
|-------|---------|-------------|
| `tail` | 256 | Last N lines on connect |
| `timestamps` | true | Include RFC3339 timestamps |

**SSE Response:**

```
event: log
data: {"line": "2025-01-15T12:34:56Z Starting worker..."}

event: log
data: {"line": "2025-01-15T12:34:57Z Processing request..."}
```

### 5.8 POST /api/agents/cleanup

Trigger manual cleanup.

**Query Parameters:**

| Param | Default | Description |
|-------|---------|-------------|
| `dry_run` | false | Preview without deleting |

**Response (dry_run=true):**

```json
{
  "dry_run": true,
  "would_delete": [
    {"id": "018f6b2a-...", "age_hours": 26, "lifetime_hours": 4}
  ],
  "count": 1
}
```

**Response (dry_run=false):**

```json
{
  "dry_run": false,
  "deleted": [
    {"id": "018f6b2a-..."}
  ],
  "count": 1
}
```

---

## 6. Error Responses

Follow loom-server's standard error format:

```json
{
  "code": "agent_not_found",
  "message": "Agent with ID 018f6b2a-... not found"
}
```

### Error Codes

| Code | HTTP | Description |
|------|------|-------------|
| `agent_not_found` | 404 | Agent ID doesn't exist |
| `too_many_agents` | 429 | Max concurrent limit reached |
| `invalid_lifetime` | 400 | TTL exceeds max (48h) |
| `agent_failed` | 500 | Pod failed to start |
| `agent_timeout` | 504 | Pod didn't reach running state in time |
| `k8s_error` | 502 | K8s API failure |
| `unauthorized` | 401 | Missing/invalid API key |

---

## 7. Configuration

### 7.1 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LOOM_SERVER_K8S_NAMESPACE` | `loom-agents` | Target namespace |
| `LOOM_SERVER_AGENT_API_KEY` | (required) | API key for authentication |
| `LOOM_SERVER_AGENT_CLEANUP_INTERVAL_SECS` | `1800` | Cleanup task interval (30 min) |
| `LOOM_SERVER_AGENT_DEFAULT_TTL_HOURS` | `4` | Default agent lifetime |
| `LOOM_SERVER_AGENT_MAX_TTL_HOURS` | `48` | Maximum lifetime override |
| `LOOM_SERVER_AGENT_MAX_CONCURRENT` | `64` | Maximum running agents |
| `LOOM_SERVER_AGENT_READY_TIMEOUT_SECS` | `60` | Timeout waiting for running state |
| `LOOM_SERVER_AGENT_WEBHOOKS` | `[]` | JSON array of webhook configs |

### 7.2 Webhook Configuration

```bash
LOOM_SERVER_AGENT_WEBHOOKS='[
  {
    "url": "https://billing.example.com/hooks",
    "events": ["agent.created", "agent.deleted"],
    "secret": "whsec_xxxxx"
  }
]'
```

Webhooks are admin-only (configured at deploy time, no CRUD API).

---

## 8. Resource Defaults

### 8.1 Pod Resources

```yaml
resources:
  requests: {}           # None - allows overcommit
  limits:
    memory: "16Gi"       # Default max
    # No CPU limit - can use all available cores
```

Override via request:

```json
{
  "resources": {
    "memory_limit": "8Gi",
    "cpu_limit": "4"
  }
}
```

### 8.2 Pod Configuration

| Setting | Value |
|---------|-------|
| Restart Policy | `Never` |
| Grace Period | `5s` |
| Container Name | `agent` |
| Service Account | `default` (namespace default) |

---

## 9. Security Context

All agent Pods run with hardened security:

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  runAsGroup: 1000
  allowPrivilegeEscalation: false
  readOnlyRootFilesystem: true
  capabilities:
    drop:
      - ALL
```

### 9.1 Not Supported

- Volume mounts (Secrets, ConfigMaps, PVCs)
- Node selection (nodeSelector, tolerations)
- Priority classes
- Custom service accounts
- Custom network policies (use cluster defaults)
- Custom DNS (use cluster defaults)

---

## 10. Cleanup System

### 10.1 Automatic Cleanup

Background task runs every `CLEANUP_INTERVAL_SECS`:

1. List Pods with `loom.dev/managed=true`
2. Calculate age from `creationTimestamp`
3. Compare against `loom.dev/lifetime-hours` annotation
4. Delete expired Pods (grace period: 5s)

### 10.2 Startup Behavior

On server start:
1. Validate namespace exists (fail if not)
2. Run cleanup immediately (reconcile orphaned agents)
3. Start interval-based cleanup task

### 10.3 Shutdown Behavior

Leave agents running. Cleanup resumes when server restarts.

---

## 11. Webhooks

### 11.1 Events

| Event | Trigger |
|-------|---------|
| `agent.created` | POST `/api/agent` success |
| `agent.deleted` | DELETE or cleanup |
| `agent.failed` | Pod enters failed state |
| `agents.cleanup` | Cleanup task completes |

### 11.2 Payload Format

```json
{
  "event": "agent.created",
  "timestamp": "2025-01-15T12:34:56Z",
  "agent": {
    "id": "018f6b2a-...",
    "image": "python:3.12",
    "tags": {"project": "ai-worker"}
  }
}
```

### 11.3 Delivery

- HMAC-SHA256 signature in `X-Webhook-Signature` header (if secret configured)
- Fire-and-forget (no retries)

---

## 12. Prometheus Metrics

Added to existing `/metrics` endpoint:

| Metric | Type | Description |
|--------|------|-------------|
| `loom_agents_created_total` | Counter | Agents provisioned |
| `loom_agents_deleted_total` | Counter | Agents deleted (manual + cleanup) |
| `loom_agents_failed_total` | Counter | Agents that entered failed state |
| `loom_agents_cleanup_total` | Counter | Cleanup runs completed |
| `loom_agents_cleanup_deleted_total` | Counter | Agents deleted by cleanup |
| `loom_agents_active` | Gauge | Currently running agents |

---

## 13. Health Check

K8s connectivity added to `/health` response:

```json
{
  "status": "healthy",
  "components": {
    "kubernetes": {
      "status": "healthy",
      "latency_ms": 45,
      "namespace": "loom-agents",
      "reachable": true
    }
  }
}
```

K8s unreachable → overall status `unhealthy` (critical component).

---

## 14. Testing Strategy

### 14.1 Unit Tests (loom-agent-provisioner)

Mock `K8sClient` trait:

```rust
struct MockK8sClient {
    pods: Arc<Mutex<Vec<Pod>>>,
}

impl K8sClient for MockK8sClient {
    async fn create_pod(&self, spec: PodSpec) -> Result<Pod, K8sError> {
        // Return canned response
    }
}
```

### 14.2 Integration Tests

Use Minikube or kind for real K8s:

```rust
#[tokio::test]
#[ignore] // Requires K8s cluster
async fn test_full_agent_lifecycle() {
    let client = KubeClient::new().await;
    let provisioner = Provisioner::new(client);
    
    let agent = provisioner.create_agent(req).await.unwrap();
    assert_eq!(agent.status, AgentStatus::Running);
    
    provisioner.delete_agent(&agent.id).await.unwrap();
}
```

---

## 15. Future Considerations

### 15.1 Potential Extensions

- Multi-namespace support
- Agent exec (interactive shell)
- Resource usage metrics per agent
- Agent logs persistence
- Webhook retry with backoff
- Agent templates/presets

### 15.2 Not Planned

- Multi-cluster support
- Persistent volumes
- Sidecar containers
- Init containers
