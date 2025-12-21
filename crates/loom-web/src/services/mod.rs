/// Application services - API, streaming, state management
pub mod api;
pub mod state;
pub mod streaming;

// Re-export server functions for convenience
pub use api::{
    add_message, create_thread, delete_thread, get_thread, get_threads, search_threads,
    update_thread,
};

// Re-export state types for convenient access
pub use state::{
    add_notification, clear_notifications, complete_streaming, logout_user, provide_app_state,
    remove_notification, reset_query_settings, set_active_thread, set_current_user,
    set_streaming_chunk, set_streaming_error, start_streaming, update_query_setting,
    use_active_thread, use_app_state, use_current_user, use_notifications, use_query_settings,
    use_streaming_state, AppState, Notification, NotificationSeverity, QuerySettings,
    StreamingState, User,
};

// Re-export streaming types for convenient access
pub use streaming::{StreamEvent, StreamingManager};
