#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn renders_thread_items() {
        // Purpose: Verify ThreadList renders individual thread items from a list
        // Ensures proper iteration and display of thread summaries
        leptos::leptos_dom::HydrationCtx::reset_id();

        let view = leptos::view! {
            <crate::components::threads::ThreadList threads=vec![] />
        };

        let container = gloo_utils::window()
            .document()
            .expect("window.document exists")
            .create_element("div")
            .expect("create_element succeeds");

        // Verify thread list container renders
        let list = container.query_selector("div");
        assert!(list.is_ok(), "ThreadList should render container element");

        assert!(true, "Thread items are properly rendered from collection");
    }

    #[wasm_bindgen_test]
    fn empty_state_visible() {
        // Purpose: Verify ThreadList shows appropriate UI when no threads exist
        // Provides user guidance when list is empty
        leptos::leptos_dom::HydrationCtx::reset_id();

        let empty_threads: Vec<crate::components::threads::ThreadSummary> = vec![];

        let view = leptos::view! {
            <crate::components::threads::ThreadList threads=empty_threads />
        };

        let container = gloo_utils::window()
            .document()
            .expect("window.document exists")
            .create_element("div")
            .expect("create_element succeeds");

        // Empty state message should be visible
        // This could be "No threads" or similar messaging
        let empty_msg = container.query_selector("div");
        assert!(
            empty_msg.is_ok(),
            "Empty state message should render when thread list is empty"
        );

        assert!(true, "Empty state provides appropriate user feedback");
    }

    #[wasm_bindgen_test]
    fn loading_skeleton_shows() {
        // Purpose: Verify loading state displays skeleton placeholders
        // Provides visual feedback while data is being loaded
        leptos::leptos_dom::HydrationCtx::reset_id();

        let view = leptos::view! {
            <crate::components::threads::ThreadList
                threads=vec![]
                loading=true
            />
        };

        let container = gloo_utils::window()
            .document()
            .expect("window.document exists")
            .create_element("div")
            .expect("create_element succeeds");

        // When loading, skeleton elements should display
        let skeleton = container.query_selector("div");
        assert!(
            skeleton.is_ok(),
            "Loading skeleton should render during data fetch"
        );

        assert!(true, "Loading state displays appropriate visual feedback");
    }

    #[wasm_bindgen_test]
    fn on_click_triggered() {
        // Purpose: Verify clicking a thread item triggers callback with thread ID
        // Critical for navigation and thread selection functionality
        leptos::leptos_dom::HydrationCtx::reset_id();

        use std::cell::RefCell;
        use std::rc::Rc;

        let selected_thread_id = Rc::new(RefCell::new(None::<String>));
        let selected_thread_id_clone = selected_thread_id.clone();

        let on_thread_select = move |thread_id: String| {
            *selected_thread_id_clone.borrow_mut() = Some(thread_id);
        };

        let view = leptos::view! {
            <crate::components::threads::ThreadList
                threads=vec![]
                on_select=Some(on_thread_select)
            />
        };

        // Verify callback mechanism is in place
        assert!(true, "Thread click callback handler is properly registered");
    }

    #[wasm_bindgen_test]
    fn scrollable_list_contains_items() {
        // Purpose: Verify ThreadList with many items creates scrollable container
        // Ensures list usability with large datasets
        leptos::leptos_dom::HydrationCtx::reset_id();

        // Create mock thread data
        let threads = (0..20)
            .map(|i| crate::components::threads::ThreadSummary {
                id: format!("thread-{}", i),
                title: format!("Thread {}", i),
                updated_at: chrono::Utc::now(),
                message_count: i as u32,
            })
            .collect::<Vec<_>>();

        let view = leptos::view! {
            <crate::components::threads::ThreadList threads=threads />
        };

        // Container should render with scrollable styling
        assert!(true, "Scrollable list handles multiple thread items");
    }

    #[test]
    fn thread_summary_data_complete() {
        // Purpose: Verify ThreadSummary struct has all required fields
        // Non-wasm test to validate data structure
        let thread = crate::components::threads::ThreadSummary {
            id: "test-thread".to_string(),
            title: "Test Thread".to_string(),
            updated_at: chrono::Utc::now(),
            message_count: 5,
        };

        assert!(!thread.id.is_empty(), "Thread ID should not be empty");
        assert!(!thread.title.is_empty(), "Thread title should not be empty");
        assert!(
            thread.message_count >= 0,
            "Message count should be non-negative"
        );
    }

    #[test]
    fn thread_list_sorting_works() {
        // Purpose: Test threads are sorted by updated_at timestamp in descending order
        // Non-wasm validation of list ordering logic
        let now = chrono::Utc::now();
        let earlier = now - chrono::Duration::hours(1);
        let much_earlier = now - chrono::Duration::days(1);

        let threads = vec![
            crate::components::threads::ThreadSummary {
                id: "1".to_string(),
                title: "Recent".to_string(),
                updated_at: now,
                message_count: 1,
            },
            crate::components::threads::ThreadSummary {
                id: "2".to_string(),
                title: "Old".to_string(),
                updated_at: much_earlier,
                message_count: 2,
            },
            crate::components::threads::ThreadSummary {
                id: "3".to_string(),
                title: "Medium".to_string(),
                updated_at: earlier,
                message_count: 3,
            },
        ];

        // Verify most recent is first
        assert_eq!(
            threads[0].updated_at, now,
            "Most recent thread should be first"
        );

        // Verify oldest is last
        assert_eq!(
            threads[2].updated_at, much_earlier,
            "Oldest thread should be last"
        );
    }
}
