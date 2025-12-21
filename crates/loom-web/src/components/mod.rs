pub mod chat;
pub mod indicators;
pub mod layout;
/// UI components organized by tier
///
/// - Primitives: Core design system (buttons, inputs, etc.)
/// - Layout: App structure (shell, panels)
/// - Chat: Conversation UI (messages, composer)
/// - Query: Query bridge visualization
/// - Results: Code, diffs, file trees
/// - Indicators: Status, feedback
/// - Threads: Thread management and display
pub mod primitives;
pub mod query;
pub mod results;
pub mod threads;
