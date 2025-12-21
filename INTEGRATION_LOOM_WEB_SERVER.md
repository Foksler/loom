# Loom Web + Loom Server Integration

## Current State

### loom-server
- **Database**: SQLite with migrations (threads, messages, github_app, etc.)
- **Endpoints**: REST API at `/v1/*` with comprehensive thread management
  - `GET /v1/threads` - List threads
  - `GET /v1/threads/{id}` - Get single thread
  - `PUT /v1/threads/{id}` - Upsert thread
  - `DELETE /v1/threads/{id}` - Delete thread
  - `POST /v1/threads/{id}/visibility` - Update visibility
  - Search, proxy, and debug endpoints
- **Features**: Full-text search (FTS), GitHub app integration, metrics, tracing

### loom-web
- **Framework**: Leptos 0.7 SPA with server functions (#[server] macro)
- **Server Functions** (not yet integrated):
  - `get_threads()` → Mock data
  - `get_thread(id)` → Mock data
  - `create_thread(title)` → Mock data
  - `add_message(thread_id, content, role)` → Mock data
  - `update_thread(id, title)` → Mock data
  - `delete_thread(id)` → Mock data
  - `search_threads(query)` → Empty mock

## Integration Architecture

```
loom-web Client
  ↓
Leptos #[server] functions
  ↓
Server Handler (crates/loom-server/src/integration.rs) - NEW
  ↓
HTTP Client (reqwest)
  ↓
loom-server REST API
  ↓
ThreadRepository (SQLite)
```

## Implementation Plan

### Phase 1: Core Integration Layer
1. Create `crates/loom-server/src/integration.rs` - HTTP client wrapper
2. Create `crates/loom-web/src/server_fns/integration.rs` - Server handlers
3. Implement server function handlers that call loom-server endpoints
4. Add configuration for loom-server URL

### Phase 2: Data Mapping
1. Map `ThreadSummary` and `Thread` between loom-web and loom-core types
2. Ensure message serialization/deserialization
3. Handle error responses and validation

### Phase 3: Streaming Integration (Future)
1. Implement SSE streaming for LLM responses
2. Create event stream handler

### Phase 4: Testing & Verification
1. Unit tests for integration layer
2. Integration tests with real loom-server
3. End-to-end UI tests

## Endpoints Mapping

| loom-web Server Fn | HTTP Method | loom-server Endpoint | Status |
|---|---|---|---|
| `get_threads()` | GET | `/v1/threads` | ✓ Exists |
| `get_thread(id)` | GET | `/v1/threads/{id}` | ✓ Exists |
| `create_thread(title)` | PUT | `/v1/threads/{id}` | ✓ Exists (needs title mapping) |
| `add_message(...)` | PUT | `/v1/threads/{id}` | ✓ Via conversation update |
| `update_thread(id, title)` | PUT | `/v1/threads/{id}` | ✓ Exists |
| `delete_thread(id)` | DELETE | `/v1/threads/{id}` | ✓ Exists |
| `search_threads(query)` | GET | `/v1/threads/search?q=...` | ✓ Exists |

## Configuration

Add to `loom-web`:
```toml
# Cargo.toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

Environment variables:
```
LOOM_SERVER_URL=http://localhost:3000  # loom-server base URL
```

## Files to Create/Modify

1. **NEW**: `crates/loom-server/src/integration.rs` - Integration layer
2. **NEW**: `crates/loom-server/src/client.rs` - HTTP client wrapper
3. **MODIFY**: `crates/loom-server/src/lib.rs` - Export integration module
4. **NEW**: `crates/loom-web/src/server_fns/integration.rs` - Server function implementations
5. **MODIFY**: `crates/loom-web/src/services/api.rs` - Replace mock implementations
6. **MODIFY**: `crates/loom-web/Cargo.toml` - Add reqwest features if needed
7. **NEW**: Integration tests and documentation

## Next Steps
1. Review and confirm architecture
2. Implement Phase 1: Core integration layer
3. Implement Phase 2: Data mapping
4. Test with running loom-server
5. Implement streaming (Phase 3)
