// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! MCP (Model Context Protocol) server endpoint.
//!
//! This module implements the MCP protocol, enabling MCP clients (like Claude Desktop)
//! to create and manage ephemeral K8s execution environments (weavers).
//!
//! ## Protocol
//!
//! - Transport: Streamable HTTP (POST /mcp)
//! - Format: JSON-RPC 2.0
//! - Version: MCP 2025-11-25
//!
//! ## Available Tools
//!
//! - `create_weaver`: Create an ephemeral Kubernetes pod for code execution
//!
//! ## Example Usage
//!
//! ```json
//! // Initialize
//! POST /mcp
//! {
//!   "jsonrpc": "2.0",
//!   "method": "initialize",
//!   "params": {
//!     "protocolVersion": "2025-11-25",
//!     "capabilities": {},
//!     "clientInfo": { "name": "test", "version": "1.0" }
//!   },
//!   "id": 1
//! }
//!
//! // List tools
//! POST /mcp
//! { "jsonrpc": "2.0", "method": "tools/list", "id": 2 }
//!
//! // Call create_weaver
//! POST /mcp
//! {
//!   "jsonrpc": "2.0",
//!   "method": "tools/call",
//!   "params": {
//!     "name": "create_weaver",
//!     "arguments": {
//!       "image": "python:3.12",
//!       "org_id": "550e8400-e29b-41d4-a716-446655440000"
//!     }
//!   },
//!   "id": 3
//! }
//! ```

mod error;
mod handler;
pub mod session;
mod tools;
mod types;

pub use error::McpError;
pub use handler::{mcp_handler, MCP_SESSION_HEADER};
pub use session::{create_session_store, McpSession, McpSessionStore};
pub use types::{
	JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, MCP_PROTOCOL_VERSION, MCP_SERVER_NAME,
	MCP_SERVER_VERSION,
};
