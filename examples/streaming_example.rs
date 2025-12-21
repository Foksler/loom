// Example of streaming integration in loom-web
//
// This example demonstrates how to use the StreamingManager for real-time LLM responses.
// Note: This is a conceptual example. The actual usage is in Leptos components.

#[cfg(target_arch = "wasm32")]
fn example_basic_streaming() {
    use loom_web::services::streaming::{StreamingManager, StreamEvent};

    let manager = StreamingManager::new();

    // Start streaming a response
    let thread_id = "thread-123".to_string();
    match manager.start_streaming(thread_id.clone(), |event| {
        match event {
            StreamEvent::Chunk { text } => {
                web_sys::console::log_1(&format!("Chunk: {}", text).into());
            }
            StreamEvent::ToolCallDelta {
                call_id,
                tool_name,
                arguments_fragment,
            } => {
                web_sys::console::log_1(
                    &format!(
                        "Tool: {} ({}): {}",
                        tool_name, call_id, arguments_fragment
                    )
                    .into(),
                );
            }
            StreamEvent::Done { full_response } => {
                web_sys::console::log_1(&format!("Done: {}", full_response).into());
            }
            StreamEvent::Error(e) => {
                web_sys::console::error_1(&format!("Error: {}", e).into());
            }
            StreamEvent::ConnectionLost => {
                web_sys::console::error_1(&"Connection lost!".into());
            }
        }
    }) {
        Ok(()) => web_sys::console::log_1(&"Streaming started".into()),
        Err(e) => web_sys::console::error_1(&format!("Failed to start streaming: {}", e).into()),
    }

    // Later: stop the stream
    manager.stop_streaming(&thread_id);
}

#[cfg(target_arch = "wasm32")]
fn example_leptos_component() {
    use loom_web::services::streaming::{StreamingManager, StreamEvent, StreamingState};
    use leptos::prelude::*;

    // This would be inside a Leptos component
    let _manager = StreamingManager::new();
    let _streaming_state = RwSignal::new(StreamingState::Idle);
    let _accumulated_text = RwSignal::new(String::new());

    // Example event handler
    let _on_stream_event = |event: StreamEvent| {
        match event {
            StreamEvent::Chunk { text } => {
                // Update UI with chunk
                _accumulated_text.update(|s| s.push_str(&text));
            }
            StreamEvent::Done { full_response } => {
                // Mark as complete
                _streaming_state.set(StreamingState::Idle);
                // Save full response
                web_sys::console::log_1(&full_response.into());
            }
            StreamEvent::Error(e) => {
                // Show error to user
                _streaming_state.set(StreamingState::Error(e));
            }
            StreamEvent::ConnectionLost => {
                // Handle connection loss with retry logic
                _streaming_state.set(StreamingState::Error(
                    "Connection lost. Reconnecting...".into(),
                ));
            }
            _ => {}
        }
    };
}

#[cfg(target_arch = "wasm32")]
fn example_multiple_streams() {
    use loom_web::services::streaming::{StreamingManager, StreamEvent};

    let manager = StreamingManager::new();

    // Stream 1: Main thread
    let manager1 = manager.clone();
    let _ = manager1.start_streaming("thread-1".to_string(), |event| {
        match event {
            StreamEvent::Chunk { text } => {
                web_sys::console::log_1(&format!("[Thread 1] {}", text).into());
            }
            _ => {}
        }
    });

    // Stream 2: Background research
    let manager2 = manager.clone();
    let _ = manager2.start_streaming("thread-2".to_string(), |event| {
        match event {
            StreamEvent::Chunk { text } => {
                web_sys::console::log_1(&format!("[Thread 2] {}", text).into());
            }
            _ => {}
        }
    });

    // Check active streams
    let active_count = manager.active_stream_count();
    web_sys::console::log_1(&format!("Active streams: {}", active_count).into());

    // Stop one stream
    manager.stop_streaming("thread-1");

    // Verify count decreased
    let active_count = manager.active_stream_count();
    web_sys::console::log_1(&format!("Active streams after stop: {}", active_count).into());

    // Stop all streams
    manager.stop_all();
}

// Server-side endpoint example (from loom-server)
//
// The backend provides streaming via SSE:
//
// #[axum::debug_handler]
// pub async fn stream_response(
//     State(state): State<AppState>,
//     Path(thread_id): Path<String>,
//     Json(request): Json<LlmRequest>,
// ) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, ServerError> {
//     let stream = state.llm_service
//         .complete_streaming_anthropic(request)
//         .await?;
//
//     Ok(create_sse_response(stream))
// }
//
// The SSE response uses this format:
// ```
// event: llm
// data: {"type":"text_delta","content":"Hello, "}
//
// event: llm
// data: {"type":"text_delta","content":"world!"}
//
// event: llm
// data: {"type":"completed","response":{...}}
// ```

// Testing utilities
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_manager_creation() {
        #[cfg(target_arch = "wasm32")]
        {
            use loom_web::services::streaming::StreamingManager;

            let manager = StreamingManager::new();
            assert_eq!(manager.active_stream_count(), 0);
        }
    }

    #[test]
    fn test_stream_event_types() {
        use loom_web::services::streaming::StreamEvent;

        let events = vec![
            StreamEvent::Chunk {
                text: "test".to_string(),
            },
            StreamEvent::Done {
                full_response: "response".to_string(),
            },
            StreamEvent::Error("error".to_string()),
            StreamEvent::ConnectionLost,
        ];

        assert_eq!(events.len(), 4);
    }

    #[test]
    fn test_streaming_state_transitions() {
        use loom_web::services::streaming::StreamingState;

        let idle = StreamingState::Idle;
        let starting = StreamingState::Starting;
        let streaming = StreamingState::Streaming {
            thread_id: "test".to_string(),
            partial_message: "hello".to_string(),
        };
        let error = StreamingState::Error("test error".to_string());

        // Verify we can pattern match
        match streaming {
            StreamingState::Streaming {
                thread_id,
                partial_message,
            } => {
                assert_eq!(thread_id, "test");
                assert_eq!(partial_message, "hello");
            }
            _ => panic!("Expected Streaming state"),
        }

        // Verify all states are different
        assert_ne!(idle, starting);
        assert_ne!(idle, error);
    }
}

fn main() {
    println!("Streaming integration examples");
    println!("See source code for usage patterns");
}
