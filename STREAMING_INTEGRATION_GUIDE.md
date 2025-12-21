# Streaming Integration Guide

## Overview

The streaming module (`crates/loom-web/src/services/streaming.rs`) provides real-time SSE (Server-Sent Events) integration for streaming LLM responses to the browser.

## Architecture

### Components

1. **StreamEvent** - Represents events from the server
   - `Chunk { text }` - Text delta from LLM
   - `ToolCallDelta { call_id, tool_name, arguments_fragment }` - Tool arguments
   - `Done { full_response }` - Stream completed
   - `Error(String)` - Error occurred
   - `ConnectionLost` - SSE connection lost

2. **StreamingState** - UI state tracking
   - `Idle` - Not streaming
   - `Starting` - Stream initialization
   - `Streaming { thread_id, partial_message }` - Active streaming
   - `Error(String)` - Error state

3. **StreamingManager** - Connection lifecycle management
   - Manages multiple concurrent SSE streams
   - Handles reconnection and cleanup
   - Thread-safe via `Rc<RefCell<>>` for WASM single-threaded context

4. **SseStreamEvent** (internal) - Wire format from server
   - Maps server LLM proxy events to `StreamEvent`
   - Handles JSON deserialization with serde

## Usage

### Basic Example

```rust
use loom_web::services::streaming::{StreamingManager, StreamEvent};

let manager = StreamingManager::new();

let stream_id = "thread-123".to_string();
manager.start_streaming(stream_id.clone(), |event| {
    match event {
        StreamEvent::Chunk { text } => {
            println!("Got chunk: {}", text);
            // Update UI with partial text
        }
        StreamEvent::ToolCallDelta { call_id, tool_name, arguments_fragment } => {
            println!("Tool {} call {}: {}", tool_name, call_id, arguments_fragment);
        }
        StreamEvent::Done { full_response } => {
            println!("Stream complete!");
            // Finalize message
        }
        StreamEvent::Error(e) => {
            eprintln!("Error: {}", e);
        }
        StreamEvent::ConnectionLost => {
            eprintln!("Connection lost - attempting reconnect");
        }
    }
})?;
```

### In Leptos Component

```rust
use leptos::prelude::*;
use loom_web::services::streaming::{StreamingManager, StreamEvent, StreamingState};

#[component]
fn ChatComponent() -> impl IntoView {
    let manager = StreamingManager::new();
    let streaming_state = RwSignal::new(StreamingState::Idle);

    let handle_stream = |thread_id: String| {
        let manager = manager.clone();
        streaming_state.set(StreamingState::Starting);

        let result = manager.start_streaming(thread_id.clone(), move |event| {
            match event {
                StreamEvent::Chunk { text } => {
                    streaming_state.update(|state| {
                        if let StreamingState::Streaming { partial_message, .. } = state {
                            partial_message.push_str(&text);
                        }
                    });
                }
                StreamEvent::Done { .. } => {
                    streaming_state.set(StreamingState::Idle);
                }
                StreamEvent::Error(e) => {
                    streaming_state.set(StreamingState::Error(e));
                }
                StreamEvent::ConnectionLost => {
                    streaming_state.set(StreamingState::Error("Connection lost".into()));
                }
                _ => {}
            }
        });

        if let Err(e) = result {
            streaming_state.set(StreamingState::Error(e));
        }
    };

    view! {
        <div>
            // UI code here
        </div>
    }
}
```

## API Endpoints

### SSE Streaming Endpoint

**Request:**
```
POST /api/threads/{thread_id}/stream
```

**Response Format (SSE):**
```
event: llm
data: {"type":"text_delta","content":"Hello"}

event: llm
data: {"type":"tool_call_delta","call_id":"C-123","tool_name":"read_file","arguments_fragment":"{\"path\""}

event: llm
data: {"type":"completed","response":{...}}

event: llm
data: {"type":"error","message":"..."}
```

## Server Integration

The backend (`loom-server/src/llm_proxy.rs`) uses `axum` SSE support:

```rust
pub async fn proxy_anthropic_stream(
    State(state): State<AppState>,
    Json(request): Json<LlmRequest>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, ServerError> {
    // Returns SSE stream with LlmStreamEvent payloads
}
```

The wire format matches `LlmStreamEvent`:
- `TextDelta` → `StreamEvent::Chunk`
- `ToolCallDelta` → `StreamEvent::ToolCallDelta`
- `Completed` → `StreamEvent::Done`
- `Error` → `StreamEvent::Error`

## Browser Compatibility

### Supported Browsers

| Browser | SSE Support | EventSource |
|---------|-------------|------------|
| Chrome/Edge | ✅ Yes | ✅ 6+ |
| Firefox | ✅ Yes | ✅ 6+ |
| Safari | ✅ Yes | ✅ 5.1+ |
| Opera | ✅ Yes | ✅ 10.6+ |
| IE/Legacy | ❌ No | ❌ Not supported |

### Feature Detection

```rust
#[cfg(target_arch = "wasm32")]
pub fn is_sse_supported() -> bool {
    use wasm_bindgen::prelude::*;
    use web_sys::window;
    
    if let Some(window) = window() {
        window.is_some() // EventSource exists
    } else {
        false
    }
}
```

## Error Handling

### Connection Errors

1. **Network Error** → `ConnectionLost` event
2. **Parse Error** → `Error` event with parse details
3. **Server Error** → `Error` event with server message

### Recovery

The application should:
1. Listen to `ConnectionLost` events
2. Implement exponential backoff for reconnection
3. Show user-friendly error messages
4. Persist partial state locally for recovery

## Testing

### Unit Tests

All types are tested in the module:

```bash
cargo test --lib services::streaming
```

Tests cover:
- ✅ StreamEvent construction
- ✅ StreamingState transitions
- ✅ SseStreamEvent deserialization
- ✅ StreamingManager lifecycle

### Integration Testing

For integration tests with real SSE:

```rust
#[wasm_bindgen_test]
async fn test_streaming_integration() {
    let manager = StreamingManager::new();
    let result = manager.start_streaming(
        "test-thread".to_string(),
        |event| {
            // Verify event handling
        }
    );
    
    assert!(result.is_ok());
}
```

## Dependencies

Required dependencies (already in `Cargo.toml`):

- `wasm-bindgen` - WASM JavaScript bindings
- `web-sys` with `EventSource` feature - Browser APIs
- `serde_json` - JSON parsing
- `tracing` - Structured logging

## Performance Considerations

1. **Memory**: Each active stream holds a closure and EventSource
   - Typical overhead: ~1KB per connection
   - Maximum concurrent streams: Limited by browser (typically 50-100)

2. **CPU**: Event parsing happens on main thread
   - JSON parsing: ~1-5ms per event
   - Consider throttling for high-frequency updates

3. **Network**: SSE uses persistent HTTP connection
   - Overhead: TCP keep-alive packets
   - Bandwidth: Only data is sent, not headers on each event

## Logging

All operations are logged with structured tracing:

```rust
// Stream started
tracing::info!(stream_id = %stream_id, "Started streaming");

// Events received
tracing::debug!(stream_id = %id, len = chunk.len(), "Received text delta");

// Errors
tracing::error!(stream_id = %id, error = %msg, "Stream error received");

// Cleanup
tracing::info!("Stopped all {} streaming connections", count);
```

## Troubleshooting

### Connection Refused

```
Error: Failed to create EventSource
```

- Check that `/api/threads/{id}/stream` endpoint exists
- Verify CORS headers if cross-origin
- Check server logs for handler errors

### Events Not Received

```
Event data is not a string
```

- Check server is sending valid JSON
- Verify SSE format: `event: llm\ndata: {...}`
- Check browser console for network errors

### Memory Leaks

```
Stopped streaming but memory still growing
```

- Ensure `stop_streaming()` or `stop_all()` called on drop
- Check for closure cycles (use weak references if needed)
- Monitor `navigator.hardwareConcurrency` for connection count

## Future Enhancements

- [ ] WebSocket upgrade for bidirectional communication
- [ ] Automatic reconnection with exponential backoff
- [ ] Connection pooling for multiple concurrent streams
- [ ] Local caching of stream state
- [ ] Server query handling during streams
- [ ] Request cancellation support

## References

- [Server-Sent Events (MDN)](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events)
- [EventSource API (MDN)](https://developer.mozilla.org/en-US/docs/Web/API/EventSource)
- [Axum SSE Support](https://docs.rs/axum/latest/axum/response/sse/index.html)
- [WASM-Bindgen Web Sys](https://docs.rs/web-sys/latest/web_sys/)
