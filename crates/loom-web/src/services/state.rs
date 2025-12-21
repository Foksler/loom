use crate::components::threads::ThreadSummary as ComponentThreadSummary;
/// Global application state via Context
///
/// Provides centralized, reactive state management for:
/// - Active thread
/// - Thread list
/// - Streaming state
/// - Query settings
/// - User information
/// - Notifications
use crate::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[cfg(test)]
#[path = "state_test.rs"]
mod state_test;

// ============================================================================
// Core Types
// ============================================================================

/// Global application state
///
/// Provided to the app via `use_app_state()` context.
/// All state is reactive and updated via Leptos signals.
#[derive(Clone)]
pub struct AppState {
    /// Currently active/selected thread ID
    pub active_thread_id: RwSignal<Option<String>>,
    /// List of all thread summaries with metadata
    pub threads: RwSignal<Option<Vec<ComponentThreadSummary>>>,
    /// Current streaming state for message processing
    pub streaming_state: RwSignal<StreamingState>,
    /// Query execution settings (model, temperature, etc.)
    pub query_settings: RwSignal<QuerySettings>,
    /// Current authenticated user
    pub current_user: RwSignal<Option<User>>,
    /// Notification queue for UI feedback
    pub notifications: RwSignal<Vec<Notification>>,
}

/// Current streaming state for LLM message processing
///
/// Tracks the lifecycle of incoming streaming responses:
/// - `Idle`: No streaming in progress
/// - `Starting`: Stream initiated, waiting for first chunk
/// - `Streaming`: Actively receiving and buffering message chunks
/// - `Error`: Stream failed with error message
#[derive(Clone, Debug, PartialEq)]
pub enum StreamingState {
    Idle,
    Starting {
        thread_id: String,
    },
    Streaming {
        thread_id: String,
        partial_message: String,
        start_time: f64,
    },
    Error(String),
}

/// Query execution settings for LLM interactions
#[derive(Clone, Debug)]
pub struct QuerySettings {
    /// LLM model identifier (e.g., "claude-sonnet")
    pub model: String,
    /// Temperature for sampling: 0.0 (deterministic) to 1.0 (creative)
    pub temperature: f32,
    /// Maximum tokens to generate in response
    pub max_tokens: u32,
    /// Optional system prompt to guide LLM behavior
    pub system_prompt: Option<String>,
    /// Whether to enable tool use in queries
    pub tools_enabled: bool,
}

impl Default for QuerySettings {
    fn default() -> Self {
        Self {
            model: "claude-sonnet".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            system_prompt: None,
            tools_enabled: true,
        }
    }
}

/// Authenticated user information
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    /// Unique user identifier
    pub id: String,
    /// User's display name
    pub name: String,
    /// User's email address
    pub email: String,
}

/// Notification for user feedback and alerts
#[derive(Clone, Debug, PartialEq)]
pub struct Notification {
    /// Unique notification identifier
    pub id: String,
    /// Message content to display
    pub message: String,
    /// Severity level for styling
    pub severity: NotificationSeverity,
    /// Unix timestamp when notification was created
    pub timestamp: f64,
}

impl Notification {
    /// Create a new notification with current timestamp
    pub fn new(message: String, severity: NotificationSeverity) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message,
            severity,
            timestamp: current_timestamp(),
        }
    }
}

/// Notification severity levels for UI styling
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotificationSeverity {
    /// Informational notification
    Info,
    /// Success confirmation
    Success,
    /// Warning alert
    Warning,
    /// Error notification
    Error,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get current Unix timestamp in seconds
fn current_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

// ============================================================================
// Context Providers
// ============================================================================

/// Initialize app state and provide to context
///
/// This function:
/// - Creates reactive signals for all state
/// - Initializes with empty thread list
/// - Provides the AppState to the Leptos context
///
/// Should be called once at app initialization, typically in the root component.
pub fn provide_app_state() {
    let active_thread_id = RwSignal::new(None);
    let threads = RwSignal::new(None);
    let streaming_state = RwSignal::new(StreamingState::Idle);
    let query_settings = RwSignal::new(QuerySettings::default());
    let current_user = RwSignal::new(None);
    let notifications = RwSignal::new(Vec::new());

    provide_context(AppState {
        active_thread_id,
        threads,
        streaming_state,
        query_settings,
        current_user,
        notifications,
    });
}

/// Access app state from context
///
/// # Panics
/// Panics if AppState has not been provided to context.
/// Ensure `provide_app_state()` is called before using this hook.
#[inline]
pub fn use_app_state() -> AppState {
    use_context::<AppState>()
        .expect("AppState not provided. Ensure provide_app_state() is called at app root.")
}

// ============================================================================
// Derived Hooks
// ============================================================================

/// Access the currently active thread ID
#[inline]
pub fn use_active_thread() -> RwSignal<Option<String>> {
    use_app_state().active_thread_id
}

/// Access the streaming state signal
#[inline]
pub fn use_streaming_state() -> RwSignal<StreamingState> {
    use_app_state().streaming_state
}

/// Access the notifications signal
#[inline]
pub fn use_notifications() -> RwSignal<Vec<Notification>> {
    use_app_state().notifications
}

/// Access the query settings signal
#[inline]
pub fn use_query_settings() -> RwSignal<QuerySettings> {
    use_app_state().query_settings
}

/// Access the current user signal
#[inline]
pub fn use_current_user() -> RwSignal<Option<User>> {
    use_app_state().current_user
}

// ============================================================================
// State Mutation Helpers
// ============================================================================

/// Add a notification to the queue
///
/// Creates a new notification with the current timestamp and adds it to
/// the notifications signal. The ID is auto-generated.
///
/// # Example
/// ```ignore
/// add_notification(
///     "Query completed successfully".to_string(),
///     NotificationSeverity::Success,
/// );
/// ```
pub fn add_notification(message: String, severity: NotificationSeverity) {
    let notifications = use_notifications();
    let notification = Notification::new(message, severity);
    notifications.update(|notifs| notifs.push(notification));
}

/// Clear all notifications from the queue
pub fn clear_notifications() {
    use_notifications().set(Vec::new());
}

/// Remove a notification by ID
pub fn remove_notification(id: &str) {
    use_notifications().update(|notifs| {
        notifs.retain(|n| n.id != id);
    });
}

/// Add a message chunk to the current streaming buffer
///
/// This is called during streaming response processing to accumulate
/// chunks of text from the LLM.
///
/// # Panics
/// Panics if streaming state is not in `Streaming` variant.
pub fn set_streaming_chunk(text: String) {
    let state = use_streaming_state();
    state.update(|s| {
        if let StreamingState::Streaming {
            ref mut partial_message,
            ..
        } = s
        {
            partial_message.push_str(&text);
        }
    });
}

/// Finalize streaming and transition to idle state
///
/// Call this when streaming is complete to clean up and make UI
/// aware that the stream has finished.
pub fn complete_streaming() {
    use_streaming_state().set(StreamingState::Idle);
}

/// Set streaming error state with message
pub fn set_streaming_error(error: String) {
    use_streaming_state().set(StreamingState::Error(error));
}

/// Start streaming for a thread
pub fn start_streaming(thread_id: String) {
    use_streaming_state().set(StreamingState::Starting {
        thread_id: thread_id.clone(),
    });

    let state = use_streaming_state();
    state.set(StreamingState::Streaming {
        thread_id,
        partial_message: String::new(),
        start_time: current_timestamp(),
    });
}

// ============================================================================
// Settings Helpers
// ============================================================================

/// Update a single query setting field
pub fn update_query_setting<F>(updater: F)
where
    F: Fn(&mut QuerySettings),
{
    use_query_settings().update(updater);
}

/// Reset query settings to defaults
pub fn reset_query_settings() {
    use_query_settings().set(QuerySettings::default());
}

/// Set the active thread
pub fn set_active_thread(thread_id: Option<String>) {
    use_active_thread().set(thread_id);
}

// ============================================================================
// User Helpers
// ============================================================================

/// Set the current authenticated user
pub fn set_current_user(user: Option<User>) {
    use_current_user().set(user);
}

/// Logout the current user
pub fn logout_user() {
    set_current_user(None);
    clear_notifications();
}
