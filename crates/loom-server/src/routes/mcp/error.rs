// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! MCP-specific error types and JSON-RPC error code mappings.

use axum::{
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use loom_server_weaver::ProvisionerError;

use super::types::{JsonRpcError, JsonRpcId, JsonRpcResponse};

/// Standard JSON-RPC 2.0 error codes.
pub mod error_codes {
	/// Invalid JSON was received by the server.
	pub const PARSE_ERROR: i32 = -32700;
	/// The JSON sent is not a valid Request object.
	pub const INVALID_REQUEST: i32 = -32600;
	/// The method does not exist / is not available.
	pub const METHOD_NOT_FOUND: i32 = -32601;
	/// Invalid method parameter(s).
	pub const INVALID_PARAMS: i32 = -32602;
	/// Internal JSON-RPC error.
	pub const INTERNAL_ERROR: i32 = -32603;

	/// Custom: Tool not found.
	pub const TOOL_NOT_FOUND: i32 = -32001;
	/// Custom: Forbidden (insufficient permissions).
	pub const FORBIDDEN: i32 = -32002;
	/// Custom: Unauthorized (authentication required).
	pub const UNAUTHORIZED: i32 = -32003;
}

/// MCP-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
	/// Invalid JSON received.
	#[error("Parse error: {0}")]
	ParseError(String),

	/// Invalid JSON-RPC request.
	#[error("Invalid request: {0}")]
	InvalidRequest(String),

	/// Method not found.
	#[error("Method not found: {0}")]
	MethodNotFound(String),

	/// Invalid method parameters.
	#[error("Invalid params: {0}")]
	InvalidParams(String),

	/// Internal server error.
	#[error("Internal error: {0}")]
	Internal(String),

	/// Tool not found.
	#[error("Tool not found: {0}")]
	ToolNotFound(String),

	/// Forbidden (insufficient permissions).
	#[error("Forbidden: {0}")]
	Forbidden(String),

	/// Unauthorized (authentication required).
	#[error("Unauthorized: {0}")]
	Unauthorized(String),

	/// Provisioner error.
	#[error("Provisioner error: {0}")]
	Provisioner(ProvisionerError),
}

impl McpError {
	/// Get the JSON-RPC error code for this error.
	pub fn code(&self) -> i32 {
		match self {
			McpError::ParseError(_) => error_codes::PARSE_ERROR,
			McpError::InvalidRequest(_) => error_codes::INVALID_REQUEST,
			McpError::MethodNotFound(_) => error_codes::METHOD_NOT_FOUND,
			McpError::InvalidParams(_) => error_codes::INVALID_PARAMS,
			McpError::Internal(_) => error_codes::INTERNAL_ERROR,
			McpError::ToolNotFound(_) => error_codes::TOOL_NOT_FOUND,
			McpError::Forbidden(_) => error_codes::FORBIDDEN,
			McpError::Unauthorized(_) => error_codes::UNAUTHORIZED,
			McpError::Provisioner(_) => error_codes::INTERNAL_ERROR,
		}
	}

	/// Convert to a JSON-RPC error object.
	pub fn to_json_rpc_error(&self) -> JsonRpcError {
		JsonRpcError {
			code: self.code(),
			message: self.to_string(),
			data: None,
		}
	}

	/// Convert to a JSON-RPC response with the given request ID.
	pub fn to_response(&self, id: Option<JsonRpcId>) -> JsonRpcResponse {
		JsonRpcResponse::error(id, self.to_json_rpc_error())
	}
}

/// Response wrapper for MCP errors that implements IntoResponse.
pub struct McpErrorResponse {
	pub error: McpError,
	pub id: Option<JsonRpcId>,
}

impl McpErrorResponse {
	pub fn new(error: McpError, id: Option<JsonRpcId>) -> Self {
		Self { error, id }
	}
}

impl IntoResponse for McpErrorResponse {
	fn into_response(self) -> Response {
		let status = match &self.error {
			McpError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
			_ => StatusCode::OK,
		};

		let response = self.error.to_response(self.id);
		(status, Json(response)).into_response()
	}
}

impl From<ProvisionerError> for McpError {
	fn from(err: ProvisionerError) -> Self {
		match &err {
			ProvisionerError::WeaverNotFound { .. } => {
				McpError::InvalidParams(format!("Weaver not found: {err}"))
			}
			ProvisionerError::TooManyWeavers { .. } => {
				McpError::InvalidParams(format!("Too many weavers: {err}"))
			}
			ProvisionerError::InvalidLifetime { .. } => {
				McpError::InvalidParams(format!("Invalid lifetime: {err}"))
			}
			_ => McpError::Internal(err.to_string()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_error_codes() {
		assert_eq!(McpError::ParseError("test".into()).code(), -32700);
		assert_eq!(McpError::InvalidRequest("test".into()).code(), -32600);
		assert_eq!(McpError::MethodNotFound("test".into()).code(), -32601);
		assert_eq!(McpError::InvalidParams("test".into()).code(), -32602);
		assert_eq!(McpError::Internal("test".into()).code(), -32603);
		assert_eq!(McpError::ToolNotFound("test".into()).code(), -32001);
		assert_eq!(McpError::Forbidden("test".into()).code(), -32002);
		assert_eq!(McpError::Unauthorized("test".into()).code(), -32003);
	}

	#[test]
	fn test_to_json_rpc_error() {
		let err = McpError::MethodNotFound("tools/unknown".into());
		let json_err = err.to_json_rpc_error();
		assert_eq!(json_err.code, -32601);
		assert!(json_err.message.contains("tools/unknown"));
	}

	#[test]
	fn test_to_response() {
		let err = McpError::InvalidParams("missing required field".into());
		let response = err.to_response(Some(JsonRpcId::Number(1)));
		assert!(response.error.is_some());
		assert!(response.result.is_none());
		assert_eq!(response.id, Some(JsonRpcId::Number(1)));
	}
}
