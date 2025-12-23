// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use serde::{Deserialize, Serialize};

/// Role of a message participant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
	System,
	User,
	Assistant,
	Tool,
}

/// A message in a conversation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
	pub role: Role,
	pub content: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tool_call_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
}

impl Message {
	pub fn system(content: impl Into<String>) -> Self {
		Self {
			role: Role::System,
			content: content.into(),
			tool_call_id: None,
			name: None,
		}
	}

	pub fn user(content: impl Into<String>) -> Self {
		Self {
			role: Role::User,
			content: content.into(),
			tool_call_id: None,
			name: None,
		}
	}

	pub fn assistant(content: impl Into<String>) -> Self {
		Self {
			role: Role::Assistant,
			content: content.into(),
			tool_call_id: None,
			name: None,
		}
	}

	pub fn tool(
		tool_call_id: impl Into<String>,
		name: impl Into<String>,
		content: impl Into<String>,
	) -> Self {
		Self {
			role: Role::Tool,
			content: content.into(),
			tool_call_id: Some(tool_call_id.into()),
			name: Some(name.into()),
		}
	}
}

/// A tool call requested by the LLM.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
	pub id: String,
	pub tool_name: String,
	pub arguments_json: serde_json::Value,
}
