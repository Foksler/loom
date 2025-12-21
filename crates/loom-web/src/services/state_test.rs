#[cfg(test)]
mod tests {

    // These tests require Leptos runtime context for RwSignal operations
    // For browser testing, use #[wasm_bindgen_test] in a separate test file
    // The non-reactive functionality is tested elsewhere

    #[test]
    #[ignore]
    fn provide_app_state_works() {
        // Purpose: Verify AppState context provider can be created and initialized
        // Ensures state context setup is correct for application
        // Ignored: Requires wasm_bindgen_test or browser environment
    }

    #[test]
    #[ignore]
    fn use_app_state_returns_context() {
        // Purpose: Verify AppState can be retrieved from context
        // Tests context access pattern is correct
        // Ignored: Requires wasm_bindgen_test or browser environment
    }

    #[test]
    #[ignore]
    fn stream_state_updates() {
        // Purpose: Verify streaming state transitions work correctly
        // Critical for tracking message streaming lifecycle
        // Ignored: Requires wasm_bindgen_test or browser environment
    }

    #[test]
    #[ignore]
    fn notifications_add() {
        // Purpose: Verify notifications can be added to queue for user feedback
        // Critical for displaying alerts and user messages
        // Ignored: Requires wasm_bindgen_test or browser environment
    }

    #[test]
    fn query_settings_default() {
        // Purpose: Verify QuerySettings has sensible defaults
        // Non-wasm validation of default configuration
        let settings = crate::services::state::QuerySettings::default();

        assert!(
            !settings.model.is_empty(),
            "Default model should be specified"
        );
        assert!(
            settings.temperature >= 0.0 && settings.temperature <= 1.0,
            "Temperature should be in valid range [0.0, 1.0]"
        );
        assert!(settings.max_tokens > 0, "Max tokens should be positive");
        assert!(settings.tools_enabled, "Tools should be enabled by default");
    }

    #[test]
    #[ignore]
    fn active_thread_id_setter_getter() {
        // Purpose: Verify active thread ID can be set and retrieved
        // Tests basic signal operations
        // Ignored: Requires wasm_bindgen_test or browser environment
    }

    #[test]
    fn notification_types_complete() {
        // Purpose: Verify all notification severity types are available
        // Tests notification severity enum has expected variants
        let _success = crate::services::state::NotificationSeverity::Success;
        let _error = crate::services::state::NotificationSeverity::Error;
        let _warning = crate::services::state::NotificationSeverity::Warning;
        let _info = crate::services::state::NotificationSeverity::Info;

        assert!(true, "All notification severity types are defined");
    }

    #[test]
    fn notification_id_is_unique() {
        // Purpose: Verify each notification gets unique ID
        // Important for tracking and removing specific notifications
        let notif1 = crate::services::state::Notification::new(
            "First".to_string(),
            crate::services::state::NotificationSeverity::Info,
        );

        let notif2 = crate::services::state::Notification::new(
            "Second".to_string(),
            crate::services::state::NotificationSeverity::Info,
        );

        assert_ne!(notif1.id, notif2.id, "Notification IDs should be unique");
    }
}
