/// Streaming integration - SSE and WebSocket handling
///
/// Provides a unified interface for streaming LLM responses via Server-Sent Events (SSE).
/// Handles real-time text chunks, tool calls, and completion notifications.
///
/// # SSE Event Format
///
/// Events are received as SSE with `event: llm` and JSON data:
/// ```json
/// {"type":"text_delta","content":"Hello"}
/// {"type":"tool_call_delta","call_id":"...", "tool_name":"...", "arguments_fragment":"..."}
/// {"type":"completed","response":{...}}
/// {"type":"error","message":"..."}
/// ```
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(test)]
#[path = "streaming_test.rs"]
mod streaming_test;

/// Streaming event from the server
///
/// This enum represents individual events received from the SSE stream.
/// Events are emitted as they arrive from the LLM provider and should be
/// collected and aggregated by the client.
#[derive(Clone, Debug)]
pub enum StreamEvent {
    /// Text chunk from LLM response - incremental text content
    Chunk { text: String },
    /// Tool call delta with arguments fragment - incremental tool arguments
    ToolCallDelta {
        call_id: String,
        tool_name: String,
        arguments_fragment: String,
    },
    /// Stream completed successfully with full response
    Done { full_response: String },
    /// Error occurred during streaming
    Error(String),
    /// Connection was lost unexpectedly
    ConnectionLost,
}

/// Wire format for LLM streaming events from SSE
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
enum SseStreamEvent {
    TextDelta {
        content: String,
    },
    ToolCallDelta {
        call_id: String,
        tool_name: String,
        arguments_fragment: String,
    },
    Completed {
        response: serde_json::Value,
    },
    Error {
        message: String,
    },
}

/// Manages a single streaming connection
struct StreamConnection {
    event_source: Option<web_sys::EventSource>,
    #[allow(dead_code)]
    closure: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
    #[allow(dead_code)]
    error_closure: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

impl Drop for StreamConnection {
    fn drop(&mut self) {
        if let Some(es) = self.event_source.take() {
            es.close();
        }
    }
}

/// Manages active streaming connections
pub struct StreamingManager {
    connections: Rc<RefCell<HashMap<String, StreamConnection>>>,
}

impl StreamingManager {
    /// Create a new streaming manager
    pub fn new() -> Self {
        Self {
            connections: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Start streaming a response
    ///
    /// # Arguments
    /// - `stream_id`: Unique identifier for this stream (e.g., thread_id)
    /// - `on_event`: Callback for each streaming event
    pub fn start_streaming<F>(&self, _stream_id: String, _on_event: F) -> Result<(), String>
    where
        F: Fn(StreamEvent) + 'static,
    {
        #[cfg(target_arch = "wasm32")]
        {
            let stream_id_clone = stream_id.clone();
            let url = format!("/api/threads/{}/stream", stream_id);

            // Create EventSource
            let es = web_sys::EventSource::new(&url)
                .map_err(|_| "Failed to create EventSource".to_string())?;

            let on_event = Rc::new(on_event);

            // Message event handler
            let on_event_msg = on_event.clone();
            let stream_id_msg = stream_id_clone.clone();
            let message_closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
                if let Ok(text) = event.data().as_string() {
                    // Parse SSE event data
                    match serde_json::from_str::<SseStreamEvent>(&text) {
                        Ok(sse_event) => {
                            let stream_event = match sse_event {
                                SseStreamEvent::TextDelta { content } => {
                                    tracing::debug!(
                                        stream_id = %stream_id_msg,
                                        len = content.len(),
                                        "Received text delta"
                                    );
                                    StreamEvent::Chunk { text: content }
                                }
                                SseStreamEvent::ToolCallDelta {
                                    call_id,
                                    tool_name,
                                    arguments_fragment,
                                } => {
                                    tracing::debug!(
                                        stream_id = %stream_id_msg,
                                        call_id = %call_id,
                                        tool_name = %tool_name,
                                        "Received tool call delta"
                                    );
                                    StreamEvent::ToolCallDelta {
                                        call_id,
                                        tool_name,
                                        arguments_fragment,
                                    }
                                }
                                SseStreamEvent::Completed { response } => {
                                    let full_response =
                                        serde_json::to_string(&response).unwrap_or_default();
                                    tracing::info!(
                                        stream_id = %stream_id_msg,
                                        response_len = full_response.len(),
                                        "Stream completed"
                                    );
                                    StreamEvent::Done { full_response }
                                }
                                SseStreamEvent::Error { message } => {
                                    tracing::error!(
                                        stream_id = %stream_id_msg,
                                        error = %message,
                                        "Stream error received"
                                    );
                                    StreamEvent::Error(message)
                                }
                            };

                            on_event_msg(stream_event);
                        }
                        Err(e) => {
                            tracing::warn!(
                                stream_id = %stream_id_msg,
                                error = %e,
                                "Failed to parse SSE event"
                            );
                            on_event_msg(StreamEvent::Error(format!(
                                "Failed to parse event: {}",
                                e
                            )));
                        }
                    }
                } else {
                    tracing::warn!(
                        stream_id = %stream_id_msg,
                        "Event data is not a string"
                    );
                }
            })
                as Box<dyn FnMut(web_sys::MessageEvent)>);

            // Error event handler
            let on_event_err = on_event.clone();
            let stream_id_err = stream_id_clone.clone();
            let error_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                tracing::error!(stream_id = %stream_id_err, "EventSource connection lost");
                on_event_err(StreamEvent::ConnectionLost);
            }) as Box<dyn FnMut(web_sys::Event)>);

            es.set_onmessage(Some(message_closure.as_ref().unchecked_ref()));
            es.set_onerror(Some(error_closure.as_ref().unchecked_ref()));

            tracing::info!(stream_id = %stream_id_clone, "Started streaming");

            // Store connection
            let connection = StreamConnection {
                event_source: Some(es),
                closure: Some(message_closure),
                error_closure: Some(error_closure),
            };

            self.connections
                .borrow_mut()
                .insert(stream_id_clone, connection);

            Ok(())
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Err("Streaming only available in WASM environment".to_string())
        }
    }

    /// Stop streaming for a specific stream
    pub fn stop_streaming(&self, stream_id: &str) {
        if let Some(connection) = self.connections.borrow_mut().remove(stream_id) {
            drop(connection);
            tracing::info!(stream_id = %stream_id, "Stopped streaming");
        }
    }

    /// Stop all active streams
    pub fn stop_all(&self) {
        let count = self.connections.borrow_mut().len();
        self.connections.borrow_mut().clear();
        tracing::info!("Stopped all {} streaming connections", count);
    }

    /// Check if a stream is active
    pub fn is_streaming(&self, stream_id: &str) -> bool {
        self.connections.borrow().contains_key(stream_id)
    }

    /// Get count of active streams
    pub fn active_stream_count(&self) -> usize {
        self.connections.borrow().len()
    }
}

impl Default for StreamingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Start streaming LLM response via SSE
///
/// # Arguments
/// - `thread_id`: Thread to stream response into
/// - `on_event`: Callback for each streaming event
///
/// # Example
/// ```rust,no_run
/// start_streaming(
///     "thread-123".to_string(),
///     |event| {
///         match event {
///             StreamEvent::Chunk { text } => println!("Got: {}", text),
///             StreamEvent::Done { full_response } => println!("Done: {}", full_response),
///             StreamEvent::Error(e) => eprintln!("Error: {}", e),
///             StreamEvent::ConnectionLost => eprintln!("Connection lost"),
///             _ => {}
///         }
///     }
/// );
/// ```
#[cfg(target_arch = "wasm32")]
pub fn start_streaming<F>(thread_id: String, on_event: F) -> Result<(), String>
where
    F: Fn(StreamEvent) + 'static,
{
    let manager = StreamingManager::new();
    manager.start_streaming(thread_id, on_event)
}

/// Stop streaming for a specific thread
#[cfg(target_arch = "wasm32")]
pub fn stop_streaming(_thread_id: &str) {
    // This would need to be called on a stored manager instance
    tracing::debug!("stop_streaming called");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that StreamingManager can be created and tracks connections
    #[test]
    fn test_streaming_manager_creation() {
        let manager = StreamingManager::new();
        assert_eq!(manager.active_stream_count(), 0);
        assert!(!manager.is_streaming("test-123"));
    }

    /// Test SSE wire format parsing with text delta events
    #[test]
    fn test_serde_text_delta_event() {
        let json = r#"{"type":"text_delta","content":"Hello world"}"#;
        let event: SseStreamEvent = serde_json::from_str(json).unwrap();
        match event {
            SseStreamEvent::TextDelta { content } => {
                assert_eq!(content, "Hello world");
            }
            _ => panic!("Expected TextDelta event"),
        }
    }

    /// Test SSE wire format parsing with error events
    #[test]
    fn test_serde_error_event() {
        let json = r#"{"type":"error","message":"Something went wrong"}"#;
        let event: SseStreamEvent = serde_json::from_str(json).unwrap();
        match event {
            SseStreamEvent::Error { message } => {
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Expected Error event"),
        }
    }

    /// Test SSE wire format parsing with tool call delta events
    #[test]
    fn test_serde_tool_call_delta_event() {
        let json = r#"{"type":"tool_call_delta","call_id":"C-123","tool_name":"read_file","arguments_fragment":"{\"path\""}"#;
        let event: SseStreamEvent = serde_json::from_str(json).unwrap();
        match event {
            SseStreamEvent::ToolCallDelta {
                call_id,
                tool_name,
                arguments_fragment,
            } => {
                assert_eq!(call_id, "C-123");
                assert_eq!(tool_name, "read_file");
                assert_eq!(arguments_fragment, r#"{"path""#);
            }
            _ => panic!("Expected ToolCallDelta event"),
        }
    }

    /// Test SSE wire format parsing with completion events
    #[test]
    fn test_serde_completed_event() {
        let json = r#"{"type":"completed","response":{"message":"test","tool_calls":[]}}"#;
        let event: SseStreamEvent = serde_json::from_str(json).unwrap();
        match event {
            SseStreamEvent::Completed { response } => {
                assert!(response.is_object());
            }
            _ => panic!("Expected Completed event"),
        }
    }

    /// Test that all StreamEvent variants can be constructed
    #[test]
    fn test_stream_event_variants() {
        let chunk = StreamEvent::Chunk {
            text: "test".to_string(),
        };
        let tool_call = StreamEvent::ToolCallDelta {
            call_id: "C-1".to_string(),
            tool_name: "test_tool".to_string(),
            arguments_fragment: "{".to_string(),
        };
        let done = StreamEvent::Done {
            full_response: "complete".to_string(),
        };
        let error = StreamEvent::Error("failed".to_string());
        let connection_lost = StreamEvent::ConnectionLost;

        // Just verify we can construct all variants
        drop((chunk, tool_call, done, error, connection_lost));
    }
}
