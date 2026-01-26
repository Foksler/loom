<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# MCP System

Model Context Protocol (MCP) server endpoint for Loom, enabling MCP clients (like Claude Desktop) to create and manage ephemeral K8s execution environments (weavers).

## Overview

The MCP system exposes Loom's weaver provisioning capabilities through the standardized Model Context Protocol, allowing AI assistants to programmatically create sandboxed execution environments.

### Goals

- Provide MCP-compliant JSON-RPC 2.0 endpoint at `POST /mcp`
- Expose `create_weaver` tool for creating ephemeral K8s pods
- Reuse existing authentication and authorization infrastructure
- Maintain audit trail for all MCP operations

### Non-Goals

- SSE streaming (server-initiated notifications)
- MCP Resources (exposing thread/repo data)
- MCP Prompts
- Sampling capability
- JSON-RPC batch requests

## Protocol Compliance

### MCP Version

Protocol version: `2025-11-25`

### JSON-RPC 2.0

All requests and responses follow JSON-RPC 2.0 specification:

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "params": {},
  "id": 1
}

// Success Response
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}

// Error Response
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Method not found"
  },
  "id": 1
}
```

### Standard Error Codes

| Code | Name | Description |
|------|------|-------------|
| `-32700` | Parse error | Invalid JSON |
| `-32600` | Invalid request | Missing required fields |
| `-32601` | Method not found | Unknown method |
| `-32602` | Invalid params | Invalid method parameters |
| `-32603` | Internal error | Server error |

### Custom Error Codes

| Code | Name | Description |
|------|------|-------------|
| `-32001` | Tool not found | Unknown tool name |
| `-32002` | Forbidden | Insufficient permissions |
| `-32003` | Unauthorized | Authentication required |

## Transport

### Streamable HTTP

Single endpoint handling all MCP methods:

```
POST /mcp
Content-Type: application/json
Authorization: Bearer <token>
```

### Session Management

Optional session tracking via `Mcp-Session-Id` header:

1. Client sends `initialize` request
2. Server responds with `Mcp-Session-Id` header
3. Client includes header in subsequent requests
4. Sessions expire after 1 hour of inactivity

Sessions are optional - stateless operation is fully supported.

## Authentication

### Bearer Token Authentication

Uses existing Loom authentication infrastructure:

```
Authorization: Bearer <api-key-or-session-token>
```

### 401 Challenge

Unauthenticated requests receive:

```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Bearer realm="loom-mcp"
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "error": {
    "code": -32003,
    "message": "Authentication required"
  },
  "id": null
}
```

## Server Capabilities

Declared during `initialize` handshake:

```json
{
  "protocolVersion": "2025-11-25",
  "capabilities": {
    "tools": {}
  },
  "serverInfo": {
    "name": "loom-mcp",
    "version": "0.1.0"
  }
}
```

## MCP Methods

### initialize

Handshake establishing protocol version and capabilities.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": {},
    "clientInfo": {
      "name": "claude-desktop",
      "version": "1.0.0"
    }
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "tools": {}
    },
    "serverInfo": {
      "name": "loom-mcp",
      "version": "0.1.0"
    }
  },
  "id": 1
}
```

### notifications/initialized

Client notification that initialization is complete. No response expected.

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

### tools/list

List available tools.

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {
        "name": "create_weaver",
        "description": "Create an ephemeral Kubernetes pod for code execution",
        "inputSchema": { ... }
      }
    ]
  },
  "id": 2
}
```

### tools/call

Execute a tool.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "create_weaver",
    "arguments": {
      "image": "python:3.12",
      "org_id": "550e8400-e29b-41d4-a716-446655440000"
    }
  },
  "id": 3
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Created weaver abc123 with image python:3.12"
      }
    ],
    "isError": false
  },
  "id": 3
}
```

## Tools

### create_weaver

Create an ephemeral Kubernetes pod for code execution.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "image": {
      "type": "string",
      "description": "Container image (e.g., 'python:3.12', 'node:20')"
    },
    "org_id": {
      "type": "string",
      "description": "Organization ID (UUID) for billing and isolation"
    },
    "env": {
      "type": "object",
      "additionalProperties": { "type": "string" },
      "description": "Environment variables to set in the container"
    },
    "memory_limit": {
      "type": "string",
      "description": "Memory limit (e.g., '8Gi')"
    },
    "cpu_limit": {
      "type": "string",
      "description": "CPU limit (e.g., '4')"
    },
    "lifetime_hours": {
      "type": "integer",
      "description": "TTL in hours (max 48, default from server config)"
    },
    "tags": {
      "type": "object",
      "additionalProperties": { "type": "string" },
      "description": "Custom tags for the weaver"
    }
  },
  "required": ["image", "org_id"]
}
```

**Execution Flow:**

1. Parse and validate arguments
2. Check user is member of specified organization
3. Build `CreateWeaverRequest`
4. Call `provisioner.create_weaver()`
5. Log audit event with `source: "mcp"`
6. Return result with weaver ID and status

**Success Result:**
```json
{
  "content": [
    {
      "type": "text",
      "text": "Created weaver 01234567-89ab-cdef-0123-456789abcdef\n\nImage: python:3.12\nStatus: pending\nPod: weaver-01234567-89ab-cdef-0123-456789abcdef\nLifetime: 24 hours"
    }
  ],
  "isError": false
}
```

**Error Result:**
```json
{
  "content": [
    {
      "type": "text",
      "text": "Failed to create weaver: Not a member of organization"
    }
  ],
  "isError": true
}
```

## Security

### Organization Membership

All tool executions verify organization membership:

```rust
if !current_user.user.is_system_admin() {
    match org_repo.get_membership(&org_id, &current_user.user.id).await {
        Ok(Some(_)) => { /* allowed */ }
        Ok(None) => { return Err(McpError::Forbidden(...)); }
        Err(e) => { return Err(McpError::Internal(...)); }
    }
}
```

### Audit Logging

All MCP operations are logged:

```rust
audit_service.log(
    AuditLogBuilder::new(AuditEventType::WeaverCreated)
        .actor(user_id)
        .resource("weaver", weaver_id)
        .details(json!({
            "source": "mcp",
            "image": image,
            "org_id": org_id,
        }))
        .build(),
);
```

## Implementation

### Module Structure

```
crates/loom-server/src/routes/mcp/
├── mod.rs        # Module exports, route registration
├── types.rs      # JSON-RPC 2.0 and MCP protocol types
├── handler.rs    # Main POST handler and method routing
├── session.rs    # Session state management
├── tools.rs      # Tool definitions and execution
└── error.rs      # MCP-specific errors
```

### Key Types

```rust
// JSON-RPC request
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<JsonRpcId>,
}

// JSON-RPC response
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<JsonRpcId>,
}

// MCP tool definition
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
```

### Route Registration

```rust
// In api.rs authed routes
.route("/mcp", post(routes::mcp::mcp_handler))
```

## Testing

### Unit Tests

```rust
#[test]
fn test_parse_json_rpc_request() { ... }

#[test]
fn test_initialize_response() { ... }

#[test]
fn test_tools_list_response() { ... }
```

### Integration Tests

```rust
#[tokio::test]
async fn test_mcp_initialize() {
    let response = client
        .post("/mcp")
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "1.0" }
            },
            "id": 1
        }))
        .send()
        .await;

    assert_eq!(response.status(), 200);
}
```

## Future Extensions

### Additional Tools

- `list_weavers` - List user's weavers
- `get_weaver` - Get weaver status
- `delete_weaver` - Delete a weaver
- `attach_weaver` - Get attach URL

### MCP Resources

Expose thread and repository data:

```json
{
  "capabilities": {
    "tools": {},
    "resources": {}
  }
}
```

### SSE Streaming

Add GET endpoint for server-initiated notifications:

```
GET /mcp
Accept: text/event-stream
```

## References

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [Weaver Provisioner](./weaver-provisioner.md)
