# Streaming Integration - Quick Start

## Overview

Real-time LLM response streaming is now available via Server-Sent Events (SSE). Stream text deltas, tool calls, and completion events directly to the browser.

## Quick Example

```rust
use loom_web::services::{StreamingManager, StreamEvent};

// Create manager
let manager = StreamingManager::new();

// Start streaming
manager.start_streaming(
    "thread-123".to_string(),
    |event| {
        match event {
            StreamEvent::Chunk { text } => {
                println!("Text: {}", text);
            }
            StreamEvent::Done { full_response } => {
                println!("Complete: {}", full_response);
            }
            StreamEvent::Error(e) => {
                eprintln!("Error: {}", e);
            }
            StreamEvent::ConnectionLost => {
                eprintln!("Lost connection");
            }
            _ => {}
        }
    }
)?;

// Later: stop streaming
manager.stop_streaming("thread-123");
```

## In Leptos Components

```rust
use leptos::prelude::*;
use loom_web::services::{StreamingManager, StreamEvent, StreamingState};

#[component]
fn ChatMessage() -> impl IntoView {
    let manager = StreamingManager::new();
    let streaming_state = use_context::<RwSignal<StreamingState>>()
        .expect("streaming state context");
    let accumulated_text = RwSignal::new(String::new());

    let start_chat = move |thread_id: String| {
        streaming_state.set(StreamingState::Starting {
            thread_id: thread_id.clone(),
        });

        let _ = manager.start_streaming(thread_id, move |event| {
            match event {
                StreamEvent::Chunk { text } => {
                    accumulated_text.update(|s| s.push_str(&text));
                    streaming_state.update(|s| {
                        if let StreamingState::Streaming { partial_message, .. } = s {
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
                    streaming_state.set(StreamingState::Error(
                        "Connection lost".into(),
                    ));
                }
                _ => {}
            }
        });
    };

    view! {
        <div>
            <p>{accumulated_text}</p>
        </div>
    }
}
```

## API Endpoint

The server provides streaming via:

```
POST /api/threads/{thread_id}/stream
Content-Type: application/json

{
  "model": "claude-sonnet",
  "messages": [...],
  "tools": [...]
}
```

**Response Format (SSE):**
```
event: llm
data: {"type":"text_delta","content":"Hello, "}

event: llm
data: {"type":"text_delta","content":"world!"}

event: llm
data: {"type":"completed","response":{...}}
```

## Event Types

### `Chunk { text }`
Received text content from LLM. Accumulate these to build the full response.

```rust
StreamEvent::Chunk { text: "Hello, ".to_string() }
```

### `ToolCallDelta { call_id, tool_name, arguments_fragment }`
Streaming tool call arguments. Accumulate `arguments_fragment` to build complete JSON.

```rust
StreamEvent::ToolCallDelta {
    call_id: "C-123".to_string(),
    tool_name: "read_file".to_string(),
    arguments_fragment: r#"{"path""#.to_string(),
}
```

### `Done { full_response }`
Stream completed. Contains serialized full response including all tool calls.

```rust
StreamEvent::Done {
    full_response: r#"{"message":"...","tool_calls":[...]}"#.to_string(),
}
```

### `Error(String)`
Error occurred during streaming. Check message for details.

```rust
StreamEvent::Error("Rate limit exceeded".to_string())
```

### `ConnectionLost`
SSE connection was lost. Implement reconnection logic.

```rust
StreamEvent::ConnectionLost
```

## StreamingManager API

### Create
```rust
let manager = StreamingManager::new();
```

### Start Streaming
```rust
manager.start_streaming(
    "thread-id".to_string(),
    |event| { /* handle event */ }
)?;
```

### Stop Single Stream
```rust
manager.stop_streaming("thread-id");
```

### Stop All Streams
```rust
manager.stop_all();
```

### Check Status
```rust
let is_active = manager.is_streaming("thread-id");
let count = manager.active_stream_count();
```

## Error Handling

### Connection Errors
Listen for `ConnectionLost` and implement retry logic:

```rust
match event {
    StreamEvent::ConnectionLost => {
        // Wait, then retry
        set_timeout(
            move || {
                let _ = manager.start_streaming(thread_id, on_event);
            },
            1000
        );
    }
    _ => {}
}
```

### Parse Errors
JSON parse failures automatically emit `Error` event with parse details.

### Server Errors
Server error responses are wrapped in `Error(message)`.

## Browser Support

✅ **Fully Supported:**
- Chrome/Chromium 6+
- Firefox 6+
- Safari 5.1+
- Opera 10.6+
- Edge 12+

❌ **Not Supported:**
- Internet Explorer
- Legacy Edge (pre-Chromium)

## Performance Tips

1. **Batch Updates:** Update UI in controlled intervals, not per-event
   ```rust
   let (chunks, set_chunks) = create_signal(Vec::new());
   let batch_update = move |event: StreamEvent| {
       set_chunks.update(|v| {
           if let StreamEvent::Chunk { text } = event {
               v.push(text);
           }
       });
   };
   ```

2. **Throttle Renders:** Use `requestAnimationFrame` for UI updates
3. **Memory:** Store only necessary data, discard intermediate chunks
4. **Concurrent Streams:** Limit to ~5-10 concurrent streams per browser

## Troubleshooting

### "Failed to create EventSource"
- Check endpoint URL is correct
- Verify CORS headers if cross-origin
- Check server is running

### "Event data is not a string"
- Server sending non-string SSE data
- Check server SSE format

### Connection drops unexpectedly
- Network timeout (implement timeout handling)
- Server stream ended (check logs)
- Browser closing tab

### Memory grows while streaming
- Ensure `stop_streaming()` called
- Check closures aren't capturing extra data
- Monitor with browser DevTools

## Testing

All event types have unit tests:

```bash
cargo test --lib services::streaming
```

**Test Coverage:**
- ✅ Event deserialization
- ✅ Manager lifecycle
- ✅ Wire format parsing
- ✅ Event variants

## See Also

- **[Streaming Integration Guide](STREAMING_INTEGRATION_GUIDE.md)** - Comprehensive reference
- **[Implementation Summary](STREAMING_IMPLEMENTATION_SUMMARY.md)** - Technical details
- **[Examples](examples/streaming_example.rs)** - Code examples
- **[Server API](crates/loom-server/src/llm_proxy.rs)** - Backend implementation

## Next Steps

1. Import `StreamingManager` in your component
2. Create manager instance
3. Call `start_streaming()` with thread ID and callback
4. Handle events in callback
5. Call `stop_streaming()` to cleanup

That's it! Real-time streaming is now available in your Loom UI.
