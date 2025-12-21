/// Thread management components
///
/// Components for displaying, filtering, and managing conversation threads.
/// Includes thread list, detail view, and metadata panels.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod thread_header;
mod thread_list;
mod thread_list_item;
mod thread_metadata_panel;

pub use thread_header::ThreadHeader;
pub use thread_list::ThreadList;
pub use thread_list_item::ThreadListItem;
pub use thread_metadata_panel::ThreadMetadataPanel;

/// Thread execution status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadStatus {
    /// Thread is actively being processed
    Active,
    /// Thread has been archived
    Archived,
    /// Thread is currently processing
    Processing,
}

impl ThreadStatus {
    /// Get string representation of status
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreadStatus::Active => "Active",
            ThreadStatus::Archived => "Archived",
            ThreadStatus::Processing => "Processing",
        }
    }
}

/// Summary of a thread for list display
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreadSummary {
    /// Unique thread identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Provider name (e.g., "OpenAI", "Claude")
    pub provider: String,
    /// Current status
    pub status: ThreadStatus,
}

/// Message in a thread
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Unique message identifier
    pub id: String,
    /// Message content
    pub content: String,
    /// Role (user/assistant/system)
    pub role: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Complete thread with all details
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    /// Unique thread identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Model identifier (e.g., "gpt-4", "claude-3")
    pub model: String,
    /// Current status
    pub status: ThreadStatus,
    /// Messages in thread
    pub messages: Vec<Message>,
    /// Repository path (optional)
    pub repository: Option<String>,
    /// Tools enabled (optional)
    pub tools: Vec<String>,
}

/// Mock data for styleguide and testing
pub mod mock_data {
    use super::*;

    /// Create mock thread summary
    pub fn mock_thread_summary() -> ThreadSummary {
        ThreadSummary {
            id: "thread_001".to_string(),
            title: "Debug webpack configuration issues".to_string(),
            updated_at: Utc::now(),
            provider: "OpenAI".to_string(),
            status: ThreadStatus::Active,
        }
    }

    /// Create mock threads list
    pub fn mock_threads() -> Vec<ThreadSummary> {
        vec![
            ThreadSummary {
                id: "thread_001".to_string(),
                title: "Debug webpack configuration issues".to_string(),
                updated_at: Utc::now(),
                provider: "OpenAI".to_string(),
                status: ThreadStatus::Active,
            },
            ThreadSummary {
                id: "thread_002".to_string(),
                title: "Rust async/await patterns".to_string(),
                updated_at: Utc::now() - chrono::Duration::hours(2),
                provider: "Claude".to_string(),
                status: ThreadStatus::Active,
            },
            ThreadSummary {
                id: "thread_003".to_string(),
                title: "Database schema migration".to_string(),
                updated_at: Utc::now() - chrono::Duration::days(1),
                provider: "OpenAI".to_string(),
                status: ThreadStatus::Archived,
            },
            ThreadSummary {
                id: "thread_004".to_string(),
                title: "TypeScript generics deep dive".to_string(),
                updated_at: Utc::now() - chrono::Duration::hours(12),
                provider: "Claude".to_string(),
                status: ThreadStatus::Processing,
            },
        ]
    }

    /// Create mock thread with details
    pub fn mock_thread() -> Thread {
        Thread {
            id: "thread_001".to_string(),
            title: "Debug webpack configuration issues".to_string(),
            created_at: Utc::now() - chrono::Duration::days(5),
            updated_at: Utc::now(),
            model: "gpt-4-turbo".to_string(),
            status: ThreadStatus::Active,
            messages: vec![
                Message {
                    id: "msg_001".to_string(),
                    content: "Help me debug this webpack config error".to_string(),
                    role: "user".to_string(),
                    created_at: Utc::now() - chrono::Duration::hours(2),
                },
                Message {
                    id: "msg_002".to_string(),
                    content: "I can help with that. What's the error message?".to_string(),
                    role: "assistant".to_string(),
                    created_at: Utc::now() - chrono::Duration::hours(2),
                },
            ],
            repository: Some("/home/user/projects/my-app".to_string()),
            tools: vec!["file-browser".to_string(), "code-executor".to_string()],
        }
    }
}
