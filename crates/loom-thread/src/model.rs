use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::ThreadIdError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThreadId(String);

impl ThreadId {
    pub fn new() -> Self {
        let uuid = uuid7::uuid7();
        Self(format!("T-{}", uuid))
    }

    /// Create a ThreadId from an existing string without validation.
    /// Use `parse()` if you need validation.
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    pub fn parse(s: &str) -> Result<Self, ThreadIdError> {
        if !s.starts_with("T-") {
            return Err(ThreadIdError::InvalidPrefix(
                s.chars().take(2).collect::<String>(),
            ));
        }

        let uuid_part = &s[2..];
        uuid::Uuid::parse_str(uuid_part)?;

        Ok(Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ThreadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ThreadId {
    type Err = ThreadIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl From<&loom_core::Role> for MessageRole {
    fn from(role: &loom_core::Role) -> Self {
        match role {
            loom_core::Role::System => Self::System,
            loom_core::Role::User => Self::User,
            loom_core::Role::Assistant => Self::Assistant,
            loom_core::Role::Tool => Self::Tool,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageSnapshot {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallSnapshot>>,
}

impl From<&loom_core::Message> for MessageSnapshot {
    fn from(msg: &loom_core::Message) -> Self {
        Self {
            role: MessageRole::from(&msg.role),
            content: msg.content.clone(),
            tool_call_id: msg.tool_call_id.clone(),
            tool_name: msg.name.clone(),
            tool_calls: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallSnapshot {
    pub id: String,
    pub tool_name: String,
    pub arguments_json: serde_json::Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConversationSnapshot {
    pub messages: Vec<MessageSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStateKind {
    WaitingForUserInput,
    CallingLlm,
    ProcessingLlmResponse,
    ExecutingTools,
    Error,
    ShuttingDown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingToolCallSnapshot {
    pub call_id: String,
    pub tool_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentStateSnapshot {
    pub kind: AgentStateKind,
    #[serde(default)]
    pub retries: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_tool_calls: Vec<PendingToolCallSnapshot>,
}

impl From<&loom_core::AgentState> for AgentStateSnapshot {
    fn from(state: &loom_core::AgentState) -> Self {
        match state {
            loom_core::AgentState::WaitingForUserInput { .. } => Self {
                kind: AgentStateKind::WaitingForUserInput,
                retries: 0,
                last_error: None,
                pending_tool_calls: Vec::new(),
            },
            loom_core::AgentState::CallingLlm { retries, .. } => Self {
                kind: AgentStateKind::CallingLlm,
                retries: *retries,
                last_error: None,
                pending_tool_calls: Vec::new(),
            },
            loom_core::AgentState::ProcessingLlmResponse { .. } => Self {
                kind: AgentStateKind::ProcessingLlmResponse,
                retries: 0,
                last_error: None,
                pending_tool_calls: Vec::new(),
            },
            loom_core::AgentState::ExecutingTools { executions, .. } => Self {
                kind: AgentStateKind::ExecutingTools,
                retries: 0,
                last_error: None,
                pending_tool_calls: executions
                    .iter()
                    .map(|e| PendingToolCallSnapshot {
                        call_id: e.call_id().to_string(),
                        tool_name: e.tool_name().to_string(),
                    })
                    .collect(),
            },
            loom_core::AgentState::Error {
                error, retries, ..
            } => Self {
                kind: AgentStateKind::Error,
                retries: *retries,
                last_error: Some(error.to_string()),
                pending_tool_calls: Vec::new(),
            },
            loom_core::AgentState::ShuttingDown => Self {
                kind: AgentStateKind::ShuttingDown,
                retries: 0,
                last_error: None,
                pending_tool_calls: Vec::new(),
            },
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThreadMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thread {
    pub id: ThreadId,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub last_activity_at: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loom_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    pub conversation: ConversationSnapshot,
    pub agent_state: AgentStateSnapshot,
    pub metadata: ThreadMetadata,
}

impl Thread {
    pub fn new() -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: ThreadId::new(),
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_activity_at: now,
            workspace_root: None,
            cwd: None,
            loom_version: None,
            provider: None,
            model: None,
            conversation: ConversationSnapshot::default(),
            agent_state: AgentStateSnapshot {
                kind: AgentStateKind::WaitingForUserInput,
                retries: 0,
                last_error: None,
                pending_tool_calls: Vec::new(),
            },
            metadata: ThreadMetadata::default(),
        }
    }

    pub fn touch(&mut self) {
        let now = Utc::now().to_rfc3339();
        self.updated_at = now.clone();
        self.last_activity_at = now;
        self.version += 1;
    }
}

impl Default for Thread {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub id: ThreadId,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub last_activity_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub message_count: usize,
    pub is_pinned: bool,
}

impl From<&Thread> for ThreadSummary {
    fn from(thread: &Thread) -> Self {
        Self {
            id: thread.id.clone(),
            version: thread.version,
            created_at: thread.created_at.clone(),
            updated_at: thread.updated_at.clone(),
            last_activity_at: thread.last_activity_at.clone(),
            title: thread.metadata.title.clone(),
            workspace_root: thread.workspace_root.clone(),
            provider: thread.provider.clone(),
            model: thread.model.clone(),
            tags: thread.metadata.tags.clone(),
            message_count: thread.conversation.messages.len(),
            is_pinned: thread.metadata.is_pinned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// **Property: Thread JSON roundtrip preserves all data**
    ///
    /// Why this is important: Thread persistence relies on JSON serialization.
    /// Any data loss during serialization/deserialization would corrupt user
    /// conversations, leading to lost work and broken state restoration.
    ///
    /// Invariant: serialize(deserialize(serialize(thread))) == serialize(thread)
    #[test]
    fn test_thread_json_roundtrip() {
        let mut thread = Thread::new();
        thread.workspace_root = Some("/home/user/project".to_string());
        thread.metadata.title = Some("Test conversation".to_string());
        thread.metadata.tags = vec!["rust".to_string(), "testing".to_string()];
        thread.conversation.messages.push(MessageSnapshot {
            role: MessageRole::User,
            content: "Hello".to_string(),
            tool_call_id: None,
            tool_name: None,
            tool_calls: None,
        });

        let json = serde_json::to_string(&thread).expect("serialize");
        let restored: Thread = serde_json::from_str(&json).expect("deserialize");
        let json2 = serde_json::to_string(&restored).expect("serialize again");

        assert_eq!(json, json2);
    }

    /// **Property: ThreadId format is always T-{uuid7}**
    ///
    /// Why this is important: ThreadIds are used as filenames and API identifiers.
    /// Inconsistent format would break file lookups, URL routing, and cross-system
    /// thread identification.
    ///
    /// Invariant: All generated ThreadIds start with "T-" followed by a valid UUID
    #[test]
    fn test_thread_id_format_validation() {
        let id = ThreadId::new();
        let s = id.to_string();

        assert!(s.starts_with("T-"), "ThreadId must start with 'T-'");
        assert_eq!(s.len(), 2 + 36, "ThreadId must be T- plus 36-char UUID");

        let uuid_part = &s[2..];
        uuid::Uuid::parse_str(uuid_part).expect("UUID part must be valid");
    }

    /// **Property: ThreadId parsing rejects invalid formats**
    ///
    /// Why this is important: Accepting malformed IDs could lead to filesystem
    /// path injection, broken lookups, or inconsistent state across systems.
    ///
    /// Invariant: parse() returns Err for any string not matching T-{valid-uuid}
    #[test]
    fn test_thread_id_parse_rejects_invalid() {
        assert!(ThreadId::parse("invalid").is_err());
        assert!(ThreadId::parse("X-12345678-1234-1234-1234-123456789abc").is_err());
        assert!(ThreadId::parse("T-not-a-uuid").is_err());
        assert!(ThreadId::parse("T-").is_err());
        assert!(ThreadId::parse("").is_err());
    }

    /// **Property: ThreadId roundtrip through string preserves identity**
    ///
    /// Why this is important: ThreadIds are frequently converted to strings for
    /// storage and back for lookups. Any mutation would cause lookup failures.
    ///
    /// Invariant: parse(id.to_string()) == Ok(id)
    #[test]
    fn test_thread_id_string_roundtrip() {
        let id = ThreadId::new();
        let s = id.to_string();
        let parsed = ThreadId::parse(&s).expect("should parse");
        assert_eq!(id, parsed);
    }

    proptest! {
        /// **Property: Version always increases on touch()**
        ///
        /// Why this is important: Version numbers enable optimistic concurrency
        /// control for sync. Non-monotonic versions would cause sync conflicts
        /// and potential data loss.
        ///
        /// Invariant: After any number of touch() calls, version > initial_version
        #[test]
        fn test_version_monotonicity(touch_count in 1usize..100) {
            let mut thread = Thread::new();
            let initial_version = thread.version;

            for _ in 0..touch_count {
                thread.touch();
            }

            prop_assert!(thread.version > initial_version);
            prop_assert_eq!(thread.version, initial_version + touch_count as u64);
        }

        /// **Property: AgentStateKind serializes to snake_case**
        ///
        /// Why this is important: API contracts expect snake_case for JSON field
        /// values. Inconsistent casing would break deserialization on other systems.
        ///
        /// Invariant: All AgentStateKind variants serialize to lowercase snake_case
        #[test]
        fn test_agent_state_kind_serde_format(_dummy in 0u8..1u8) {
            let kinds = [
                (AgentStateKind::WaitingForUserInput, "\"waiting_for_user_input\""),
                (AgentStateKind::CallingLlm, "\"calling_llm\""),
                (AgentStateKind::ProcessingLlmResponse, "\"processing_llm_response\""),
                (AgentStateKind::ExecutingTools, "\"executing_tools\""),
                (AgentStateKind::Error, "\"error\""),
                (AgentStateKind::ShuttingDown, "\"shutting_down\""),
            ];

            for (kind, expected) in kinds {
                let json = serde_json::to_string(&kind).unwrap();
                prop_assert_eq!(json, expected);
            }
        }
    }
}
