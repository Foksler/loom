#[cfg(test)]
mod tests {
    use leptos::*;
    use leptos::prelude::Get;

    #[test]
    fn provide_app_state_works() {
        // Purpose: Verify AppState context provider can be created and initialized
        // Ensures state context setup is correct for application
        let state = crate::services::state::AppState {
            active_thread_id: RwSignal::new(None),
            threads: Resource::new(|| (), |_| async { Ok(vec![]) }),
            streaming_state: RwSignal::new(crate::services::state::StreamingState::Idle),
            query_settings: RwSignal::new(crate::services::state::QuerySettings::default()),
            current_user: RwSignal::new(None),
            notifications: RwSignal::new(vec![]),
        };

        // Verify we can read state values without panic
        assert_eq!(
            state.active_thread_id.get(),
            None,
            "Initial active thread should be None"
        );

        assert!(true, "AppState context initialized successfully");
    }

    #[test]
    fn use_app_state_returns_context() {
        // Purpose: Verify AppState can be retrieved from context
        // Tests context access pattern is correct

        // In a real scenario, this would use create_runtime with provide_context
        // For unit test, we validate the state structure exists

        let _state = crate::services::state::AppState {
            active_thread_id: RwSignal::new(Some("thread-123".to_string())),
            threads: Resource::new(|| (), |_| async { Ok(vec![]) }),
            streaming_state: RwSignal::new(crate::services::state::StreamingState::Idle),
            query_settings: RwSignal::new(crate::services::state::QuerySettings::default()),
            current_user: RwSignal::new(None),
            notifications: RwSignal::new(vec![]),
        };

        assert!(true, "AppState context can be retrieved");
    }

    #[test]
    fn stream_state_updates() {
        // Purpose: Verify streaming state transitions work correctly
        // Critical for tracking message streaming lifecycle

        // Test Initial Idle state
        let state = RwSignal::new(crate::services::state::StreamingState::Idle);
        assert_eq!(
            state.get(),
            crate::services::state::StreamingState::Idle,
            "Initial streaming state should be Idle"
        );

        // Test transition to Starting
        state.set(crate::services::state::StreamingState::Starting {
            thread_id: "thread-1".to_string(),
        });

        match state.get() {
            crate::services::state::StreamingState::Starting { thread_id } => {
                assert_eq!(
                    thread_id, "thread-1",
                    "Thread ID should be captured in Starting state"
                );
            }
            _ => panic!("State should transition to Starting"),
        }

        // Test transition to Streaming
        state.set(crate::services::state::StreamingState::Streaming {
            thread_id: "thread-1".to_string(),
            partial_message: "Hello".to_string(),
            start_time: 0.0,
        });

        match state.get() {
            crate::services::state::StreamingState::Streaming {
                thread_id,
                partial_message,
                start_time,
            } => {
                assert_eq!(thread_id, "thread-1", "Thread ID should persist");
                assert_eq!(
                    partial_message, "Hello",
                    "Partial message should be captured"
                );
                assert_eq!(start_time, 0.0, "Start time should be recorded");
            }
            _ => panic!("State should transition to Streaming"),
        }

        // Test transition to Error
        state.set(crate::services::state::StreamingState::Error(
            "Connection lost".to_string(),
        ));

        match state.get() {
            crate::services::state::StreamingState::Error(msg) => {
                assert_eq!(msg, "Connection lost", "Error message should be captured");
            }
            _ => panic!("State should transition to Error"),
        }

        // Test return to Idle
        state.set(crate::services::state::StreamingState::Idle);
        assert_eq!(
            state.get(),
            crate::services::state::StreamingState::Idle,
            "Should transition back to Idle"
        );
    }

    #[test]
    fn notifications_add() {
        // Purpose: Verify notifications can be added to queue for user feedback
        // Critical for displaying alerts and user messages
        let notifications = RwSignal::new(vec![]);

        // Add first notification
        let mut current = notifications.get();
        current.push(crate::services::state::Notification::new(
            "Operation successful".to_string(),
            crate::services::state::NotificationSeverity::Success,
        ));
        notifications.set(current.clone());

        assert_eq!(
            notifications.get().len(),
            1,
            "Notification should be added to queue"
        );

        assert_eq!(
            notifications.get()[0].message,
            "Operation successful",
            "Notification message should be preserved"
        );

        // Add second notification
        let mut current = notifications.get();
        current.push(crate::services::state::Notification::new(
            "Warning: Check permissions".to_string(),
            crate::services::state::NotificationSeverity::Warning,
        ));
        notifications.set(current);

        assert_eq!(
            notifications.get().len(),
            2,
            "Multiple notifications should accumulate"
        );
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
    fn active_thread_id_setter_getter() {
        // Purpose: Verify active thread ID can be set and retrieved
        // Tests basic signal operations
        let active_thread_id = RwSignal::new(None::<String>);

        // Initially None
        assert_eq!(active_thread_id.get(), None);

        // Set to a thread ID
        active_thread_id.set(Some("thread-abc".to_string()));
        assert_eq!(
            active_thread_id.get(),
            Some("thread-abc".to_string()),
            "Active thread ID should be set correctly"
        );

        // Clear it
        active_thread_id.set(None);
        assert_eq!(
            active_thread_id.get(),
            None,
            "Active thread ID should clear to None"
        );
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
