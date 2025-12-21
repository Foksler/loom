/// Results and code display components
///
/// Components for displaying code, diffs, file trees, and execution results.
/// Includes:
/// - CodeBlock: Syntax-highlighted code display with copy button
/// - DiffView: Side-by-side or inline diff viewer
/// - FileTree: Nested file structure explorer
/// - ExecutionResult: Generic execution status and output display
/// - LLMResultPanel: Tabbed container for LLM execution results
pub mod code_block;
pub mod diff_view;
pub mod execution_result;
pub mod file_tree;
pub mod llm_result_panel;
pub mod placeholder;
pub mod types;

pub use code_block::CodeBlock;
pub use diff_view::DiffView;
pub use execution_result::ExecutionResult;
pub use file_tree::FileTree;
pub use llm_result_panel::LLMResultPanel;
pub use types::{CodeBlockProps, ExecutionStatus, FileNode, LLMResult, Tab};
