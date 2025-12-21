/// Type definitions for results and execution components
///
/// Defines data structures for representing code execution results,
/// diffs, file trees, and related metadata.
use serde::{Deserialize, Serialize};

/// Status of an execution or operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Operation completed successfully
    Success,
    /// Operation failed with errors
    Error,
    /// Operation completed with warnings
    Warning,
    /// Operation currently running
    Processing,
}

impl ExecutionStatus {
    /// Get Tailwind color class for status
    pub fn color_class(&self) -> &'static str {
        match self {
            ExecutionStatus::Success => "text-green-700 bg-green-50",
            ExecutionStatus::Error => "text-red-700 bg-red-50",
            ExecutionStatus::Warning => "text-yellow-700 bg-yellow-50",
            ExecutionStatus::Processing => "text-blue-700 bg-blue-50",
        }
    }

    /// Get badge variant for status
    pub fn badge_variant(&self) -> crate::components::primitives::BadgeVariant {
        match self {
            ExecutionStatus::Success => crate::components::primitives::BadgeVariant::Green,
            ExecutionStatus::Error => crate::components::primitives::BadgeVariant::Red,
            ExecutionStatus::Warning => crate::components::primitives::BadgeVariant::Yellow,
            ExecutionStatus::Processing => crate::components::primitives::BadgeVariant::Blue,
        }
    }
}

/// Result from LLM code execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LLMResult {
    /// Unique result ID
    pub id: String,
    /// Execution status
    pub status: ExecutionStatus,
    /// Main output/stdout
    pub output: String,
    /// Error messages
    pub errors: Vec<String>,
    /// Execution logs
    pub logs: Vec<String>,
    /// Execution time in milliseconds
    pub execution_ms: u32,
}

/// Properties for CodeBlock component
#[derive(Clone, Debug)]
pub struct CodeBlockProps {
    /// Source code to display
    pub code: String,
    /// Language for syntax highlighting
    pub language: String,
    /// Show line numbers
    pub line_numbers: bool,
}

/// Represents a file or folder in a tree structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileNode {
    /// File or folder name
    pub name: String,
    /// Full path to file
    pub path: String,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Child files/folders (only for directories)
    #[serde(default)]
    pub children: Vec<FileNode>,
}

impl FileNode {
    /// Create a new file node
    pub fn new(name: String, path: String, is_dir: bool) -> Self {
        Self {
            name,
            path,
            is_dir,
            children: Vec::new(),
        }
    }

    /// Add a child to this node
    pub fn add_child(mut self, child: FileNode) -> Self {
        self.children.push(child);
        self
    }

    /// Get file extension if applicable
    pub fn extension(&self) -> Option<&str> {
        self.name
            .rfind('.')
            .and_then(|idx| self.name.get(idx + 1..))
    }

    /// Get icon based on file type
    pub fn icon(&self) -> &'static str {
        if self.is_dir {
            "📁"
        } else {
            match self.extension() {
                Some("rs") => "🦀",
                Some("ts") | Some("tsx") | Some("js") | Some("jsx") => "⚙️",
                Some("json") => "📋",
                Some("md") | Some("txt") => "📄",
                Some("html") | Some("css") => "🎨",
                _ => "📄",
            }
        }
    }
}

/// Tab configuration for result panels
#[derive(Clone, Debug)]
pub struct Tab {
    /// Tab ID/key
    pub id: String,
    /// Tab label
    pub label: String,
    /// Tab content
    pub content: String,
}
