# State Management Integration Guide

## Quick Start

### 1. Initialize State at App Root

```rust
use loom_web::App;

#[component]
pub fn App() -> impl IntoView {
    // Initialize global state - must be called once at root
    provide_app_state();
    
    view! {
        <Router>
            <Routes>
                <Route path="" view=HomePage/>
                <Route path="threads/:id" view=ThreadDetail/>
            </Routes>
        </Router>
    }
}
```

### 2. Use State in Components

```rust
use loom_web::services::{
    use_active_thread, use_notifications, 
    add_notification, NotificationSeverity,
};

#[component]
fn ThreadView() -> impl IntoView {
    // Get reactive signals
    let thread_id = use_active_thread();
    let notifs = use_notifications();
    
    view! {
        <div>
            // Reactive: updates when thread_id changes
            {move || {
                thread_id.get().map(|id| {
                    view! { <p>{id}</p> }
                })
            }}
            
            // Button to add notification
            <button on:click=move |_| {
                add_notification(
                    "Thread loaded!".to_string(),
                    NotificationSeverity::Success
                );
            }>
                "Notify"
            </button>
        </div>
    }
}
```

## Common Integration Patterns

### Pattern 1: Thread Selection

```rust
#[component]
fn ThreadListItem(thread_id: String) -> impl IntoView {
    let active = use_active_thread();
    
    let is_active = move || {
        active.get().map(|id| id == thread_id).unwrap_or(false)
    };
    
    view! {
        <div class=move || if is_active() { "active" } else { "" }>
            <button on:click=move |_| {
                set_active_thread(Some(thread_id.clone()));
            }>
                {thread_id}
            </button>
        </div>
    }
}
```

### Pattern 2: Streaming Response Display

```rust
#[component]
fn StreamingMessage() -> impl IntoView {
    let streaming = use_streaming_state();
    
    view! {
        <div>
            {move || {
                match streaming.get() {
                    StreamingState::Idle => {
                        view! { <p>"Ready"</p> }.into_view()
                    }
                    StreamingState::Starting { thread_id } => {
                        view! { <p>"Starting..."</p> }.into_view()
                    }
                    StreamingState::Streaming { partial_message, .. } => {
                        view! {
                            <div>
                                <p>{partial_message}</p>
                                <span class="loading">█</span>
                            </div>
                        }.into_view()
                    }
                    StreamingState::Error(e) => {
                        view! {
                            <p class="error">{e}</p>
                        }.into_view()
                    }
                }
            }}
        </div>
    }
}
```

### Pattern 3: Query Configuration

```rust
#[component]
fn QuerySettings() -> impl IntoView {
    let settings = use_query_settings();
    
    view! {
        <div class="settings-panel">
            <label>
                "Model:"
                <input
                    type="text"
                    value=move || settings.get().model
                    on:input=move |ev| {
                        update_query_setting(|s| {
                            s.model = event_target_value(&ev);
                        });
                    }
                />
            </label>
            
            <label>
                "Temperature:"
                <input
                    type="range"
                    min="0.0"
                    max="1.0"
                    step="0.1"
                    value=move || settings.get().temperature.to_string()
                    on:input=move |ev| {
                        if let Ok(temp) = event_target_value(&ev).parse::<f32>() {
                            update_query_setting(|s| {
                                s.temperature = temp;
                            });
                        }
                    }
                />
            </label>
            
            <button on:click=move |_| {
                reset_query_settings();
            }>
                "Reset to Defaults"
            </button>
        </div>
    }
}
```

### Pattern 4: Notification System

```rust
#[component]
fn NotificationCenter() -> impl IntoView {
    let notifications = use_notifications();
    
    view! {
        <div class="notifications">
            {move || {
                notifications.get().into_iter().map(|notif| {
                    let id = notif.id.clone();
                    let id_clone = id.clone();
                    
                    view! {
                        <div
                            class=format!(
                                "notification notification-{}",
                                match notif.severity {
                                    NotificationSeverity::Info => "info",
                                    NotificationSeverity::Success => "success",
                                    NotificationSeverity::Warning => "warning",
                                    NotificationSeverity::Error => "error",
                                }
                            )
                        >
                            <p>{notif.message}</p>
                            <button
                                on:click=move |_| {
                                    remove_notification(&id_clone);
                                }
                            >
                                "✕"
                            </button>
                        </div>
                    }
                }).collect::<Vec<_>>()
            }}
        </div>
    }
}
```

### Pattern 5: User Authentication

```rust
#[component]
fn UserMenu() -> impl IntoView {
    let user = use_current_user();
    
    view! {
        <nav>
            {move || {
                match user.get() {
                    Some(u) => {
                        view! {
                            <div class="user-menu">
                                <span>{u.name}</span>
                                <button on:click=move |_| {
                                    logout_user();
                                }>
                                    "Logout"
                                </button>
                            </div>
                        }.into_view()
                    }
                    None => {
                        view! {
                            <button on:click=move |_| {
                                // Navigate to login
                            }>
                                "Login"
                            </button>
                        }.into_view()
                    }
                }
            }}
        </nav>
    }
}
```

### Pattern 6: Async Thread Loading

```rust
#[component]
fn ThreadListWithResource() -> impl IntoView {
    let app_state = use_app_state();
    
    view! {
        <Suspense fallback=move || {
            view! { <div>"Loading threads..."</div> }
        }>
            {move || {
                app_state.threads.get().map(|result| {
                    match result {
                        Ok(threads) => {
                            view! {
                                <ul class="thread-list">
                                    {threads.into_iter().map(|t| {
                                        view! {
                                            <ThreadListItem 
                                                thread_id=t.id.clone()
                                                title=t.title
                                            />
                                        }
                                    }).collect::<Vec<_>>()}
                                </ul>
                            }.into_view()
                        }
                        Err(e) => {
                            view! {
                                <p class="error">"Failed to load threads"</p>
                            }.into_view()
                        }
                    }
                })
            }}
        </Suspense>
    }
}
```

## Error Handling

### Streaming Error

```rust
pub async fn handle_stream(thread_id: String) {
    start_streaming(thread_id);
    
    match stream_from_llm().await {
        Ok(mut stream) => {
            while let Some(chunk) = stream.next().await {
                set_streaming_chunk(chunk);
            }
            complete_streaming();
            add_notification(
                "Stream completed".into(),
                NotificationSeverity::Success
            );
        }
        Err(e) => {
            let msg = format!("Streaming error: {}", e);
            set_streaming_error(msg.clone());
            add_notification(msg, NotificationSeverity::Error);
        }
    }
}
```

### Notification on Error

```rust
pub async fn execute_query() {
    match run_query().await {
        Ok(result) => {
            add_notification(
                "Query executed successfully".into(),
                NotificationSeverity::Success
            );
        }
        Err(e) => {
            add_notification(
                format!("Query failed: {}", e),
                NotificationSeverity::Error
            );
        }
    }
}
```

## Performance Tips

1. **Use Derived Hooks**: Instead of `use_app_state().active_thread_id`, use `use_active_thread()` for cleaner code and better composability.

2. **Minimize State Scope**: Only access the state you need:
   ```rust
   // ✅ Good
   let thread = use_active_thread();
   
   // ❌ Wasteful - accesses entire state
   let state = use_app_state();
   let thread = state.active_thread_id;
   ```

3. **Use Move Closures Correctly**:
   ```rust
   // ✅ Good - signal is captured
   let signal = use_active_thread();
   create_effect(move |_| {
       log!("{:?}", signal.get());
   });
   
   // ❌ Bad - creates closure on every render
   create_effect(move |_| {
       log!("{:?}", use_active_thread().get());
   });
   ```

4. **Batch Updates When Possible**:
   ```rust
   // Better than multiple updates
   update_query_setting(|s| {
       s.model = "gpt-4".into();
       s.temperature = 0.5;
       s.max_tokens = 8192;
   });
   ```

## Testing State

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_notification_creation() {
        let notif = Notification::new(
            "Test".into(),
            NotificationSeverity::Info
        );
        assert!(!notif.id.is_empty());
        assert_eq!(notif.message, "Test");
    }
    
    #[test]
    fn test_query_settings_default() {
        let settings = QuerySettings::default();
        assert_eq!(settings.model, "claude-sonnet");
        assert_eq!(settings.temperature, 0.7);
        assert_eq!(settings.max_tokens, 4096);
    }
}
```

## Troubleshooting

### "AppState not provided" Error
**Problem**: Using `use_app_state()` or derived hooks without calling `provide_app_state()`

**Solution**:
```rust
// In your app root component
provide_app_state();
```

### State Not Updating UI
**Problem**: Signal value changes but UI doesn't update

**Solution**: Make sure you're accessing signals in a reactive context:
```rust
// ✅ Reactive - updates UI
{move || {
    use_active_thread().get().map(|id| view! { <p>{id}</p> })
}}

// ❌ Not reactive - doesn't update
let id = use_active_thread().get();
view! { <p>{id}</p> }
```

### Multiple Signal References
**Problem**: Different parts of code see different state values

**Solution**: Reuse the signal reference:
```rust
// ✅ Good
let signal = use_active_thread();
signal.set(Some("a".into()));
signal.update(|opt| {
    // Will see "a"
});

// ❌ Wrong - different signals
use_active_thread().set(Some("a".into()));
use_active_thread().update(|opt| {
    // Might not see "a"
});
```

## Migration from Old State System

If migrating from an older state system:

1. Call `provide_app_state()` once at app root
2. Replace old signal access with corresponding hooks
3. Update all state mutation to use new helpers
4. Remove old context providers
5. Test streaming and notifications workflows

## Summary

✅ Complete state management system
✅ Type-safe reactive signals
✅ Async resource loading
✅ Notification queuing
✅ Streaming state tracking
✅ Query configuration
✅ User authentication support
✅ Ready for production use
