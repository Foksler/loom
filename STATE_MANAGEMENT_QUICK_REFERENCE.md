# State Management Quick Reference

## One-Liner Setup
```rust
#[component]
fn App() -> impl IntoView {
    provide_app_state();
    // ... components
}
```

## Get Any State Signal
```rust
use_app_state().active_thread_id        // RwSignal<Option<String>>
use_app_state().threads                 // Resource<...>
use_app_state().streaming_state         // RwSignal<StreamingState>
use_app_state().query_settings          // RwSignal<QuerySettings>
use_app_state().current_user            // RwSignal<Option<User>>
use_app_state().notifications           // RwSignal<Vec<Notification>>
```

## Or Use Specialized Hooks
```rust
use_active_thread()      // Direct access to thread ID signal
use_streaming_state()    // Direct access to streaming signal  
use_notifications()      // Direct access to notifications signal
use_query_settings()     // Direct access to settings signal
use_current_user()       // Direct access to user signal
```

## Common Patterns

### Read Current Value
```rust
let thread_id = use_active_thread();
let current = thread_id.get();  // Option<String>
```

### Watch for Changes
```rust
let thread_id = use_active_thread();
create_effect(move |_| {
    if let Some(id) = thread_id.get() {
        log!("Thread changed to: {}", id);
    }
});
```

### Update State
```rust
// Method 1: Set new value
use_active_thread().set(Some("thread_123".to_string()));

// Method 2: Update in place
use_notifications().update(|notifs| {
    notifs.push(Notification::new(
        "Done!".to_string(),
        NotificationSeverity::Success
    ));
});
```

### In Views (Reactive)
```rust
#[component]
fn StatusBar() -> impl IntoView {
    let streaming = use_streaming_state();
    
    view! {
        <div>
            {move || match streaming.get() {
                StreamingState::Idle => "Ready",
                StreamingState::Starting { .. } => "Starting...",
                StreamingState::Streaming { .. } => "Streaming...",
                StreamingState::Error(e) => &e,
            }}
        </div>
    }
}
```

## Notification Workflow
```rust
// Show info
add_notification("Loading...".into(), NotificationSeverity::Info);

// Show success  
add_notification("Done!".into(), NotificationSeverity::Success);

// Show warning
add_notification("Check this".into(), NotificationSeverity::Warning);

// Show error
add_notification("Failed!".into(), NotificationSeverity::Error);

// Clear all
clear_notifications();

// Remove specific
remove_notification("notification-id");
```

## Streaming Workflow
```rust
// Start streaming for a thread
start_streaming("thread_123".into());

// Receive chunks and add to buffer
set_streaming_chunk("Hello ".into());
set_streaming_chunk("world".into());

// Handle error
set_streaming_error("Connection lost".into());

// Finish
complete_streaming();
```

## Query Settings Workflow
```rust
// Change single setting
update_query_setting(|s| {
    s.temperature = 0.5;
});

// Change multiple
update_query_setting(|s| {
    s.model = "gpt-4".to_string();
    s.max_tokens = 8192;
    s.tools_enabled = false;
});

// Reset to defaults
reset_query_settings();

// Direct access to current settings
let settings = use_query_settings().get();
println!("Model: {}", settings.model);
```

## User Management
```rust
// Set user
set_current_user(Some(User {
    id: "user_123".into(),
    name: "Alice".into(),
    email: "alice@example.com".into(),
}));

// Check if logged in
let is_logged_in = use_current_user().get().is_some();

// Logout (clears user + notifications)
logout_user();
```

## Thread Management
```rust
// Set active thread
set_active_thread(Some("thread_456".into()));

// Access in view
view! {
    {move || {
        use_active_thread().get().map(|id| {
            view! { <ThreadView id=id /> }
        })
    }}
}

// Deselect
set_active_thread(None);
```

## Access Thread List
```rust
#[component]
fn ThreadList() -> impl IntoView {
    let app_state = use_app_state();
    
    view! {
        <Suspense>
            {move || {
                app_state.threads.get().map(|result| {
                    match result {
                        Ok(threads) => {
                            view! {
                                <ul>
                                    {threads.iter().map(|t| {
                                        view! { <li>{&t.title}</li> }
                                    }).collect::<Vec<_>>()}
                                </ul>
                            }.into_view()
                        },
                        Err(e) => {
                            view! { <p>"Error loading threads"</p> }.into_view()
                        }
                    }
                })
            }}
        </Suspense>
    }
}
```

## Types Reference

### StreamingState
```rust
Idle                                           // No streaming
Starting { thread_id: String }                 // Initializing
Streaming {                                    // Active
    thread_id: String,
    partial_message: String,
    start_time: f64,
}
Error(String)                                  // Failed
```

### QuerySettings
```rust
QuerySettings {
    model: String,                  // LLM identifier
    temperature: f32,               // 0.0 (deterministic) to 1.0 (creative)
    max_tokens: u32,               // Max response length
    system_prompt: Option<String>,  // Guide LLM behavior
    tools_enabled: bool,            // Allow tool use
}
```

### NotificationSeverity
```rust
Info      // Informational
Success   // Success message  
Warning   // Warning alert
Error     // Error notification
```

### User
```rust
User {
    id: String,     // Unique identifier
    name: String,   // Display name
    email: String,  // Email address
}
```

### Notification (auto-created)
```rust
Notification {
    id: String,                    // Auto UUID
    message: String,               // Content
    severity: NotificationSeverity, // Type
    timestamp: f64,               // Unix timestamp
}
```

## Tips & Tricks

**Don't** store signals in local variables - they're Clone but create multiple refs
```rust
// ❌ Wrong - different signal each access
let s1 = use_active_thread();
let s2 = use_active_thread();
s1.set(Some("a".into()));
// s2 might not see the change

// ✅ Right - reuse the signal
let signal = use_active_thread();
signal.set(Some("a".into()));
```

**Do** use `.get()` to extract values, not `.clone()`
```rust
// ✅ Correct
let val = use_active_thread().get();

// ❌ Clones the signal, not the value
let val = use_active_thread().clone();
```

**Do** use `create_effect` for side effects based on state
```rust
create_effect(move |_| {
    if use_active_thread().get().is_some() {
        // Do something when thread changes
    }
});
```

**Don't** call hooks inside loops or conditions
```rust
// ❌ Wrong
for i in 0..10 {
    let state = use_app_state();  // Invalid hook call
}

// ✅ Right
let state = use_app_state();
for i in 0..10 {
    // Use state inside loop
}
```
