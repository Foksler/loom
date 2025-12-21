# Chat Components - Usage Examples

## Complete Chat Application Example

```rust
// In your page component
use leptos::*;
use loom_core::message::Message;
use crate::components::chat::{ConversationView, PromptComposer, MessageBubble};

#[component]
pub fn ChatPage() -> impl IntoView {
    let (messages, set_messages) = create_signal(vec![
        Message::system("You are a helpful Rust coding assistant"),
        Message::assistant("Hello! I'm here to help you with Rust. What would you like to know?"),
    ]);
    let (loading, set_loading) = create_signal(false);

    let handle_submit = move |text: String| {
        // Add user message
        set_messages.update(|msgs| {
            msgs.push(Message::user(text.clone()));
        });
        
        set_loading.set(true);
        
        // Call API (pseudo-code)
        spawn_local(async move {
            // let response = api::send_message(text).await;
            // set_messages.update(|msgs| {
            //     msgs.push(Message::assistant(response));
            // });
            set_loading.set(false);
        });
    };

    view! {
        <div class="h-screen flex flex-col bg-white">
            {/* Header */}
            <div class="border-b border-gray-200 px-6 py-4">
                <h1 class="text-2xl font-bold text-gray-900">Loom Chat</h1>
                <p class="text-sm text-gray-500">Rust Coding Assistant</p>
            </div>

            {/* Messages Container */}
            <div class="flex-1 overflow-hidden p-4">
                <ConversationView 
                    messages=messages.get()
                    loading=loading.get()
                />
            </div>

            {/* Input Area */}
            <div class="border-t border-gray-200 p-4">
                <PromptComposer
                    on_submit=handle_submit
                    disabled=loading.get()
                    placeholder="Ask about Rust..."
                    max_chars=4000
                />
            </div>
        </div>
    }
}
```

---

## Component-by-Component Examples

### 1. ConversationView - Standalone

```rust
view! {
    <div class="h-96">
        <ConversationView 
            messages=vec![
                Message::user("What's the closure syntax?"),
                Message::assistant("Closures in Rust use `||` syntax..."),
            ]
            loading=false
        />
    </div>
}
```

### 2. MessageBubble - Individual Messages

```rust
view! {
    <div class="space-y-4">
        {/* User message */}
        <MessageBubble 
            message=Message::user("How do I use iterators?")
            provider=Some("You".to_string())
        />

        {/* Assistant response */}
        <MessageBubble 
            message=Message::assistant(
                "Iterators in Rust provide a functional way to process sequences. \n\nExample:\n```rust\nlet nums = vec![1, 2, 3];\nlet doubled: Vec<i32> = nums.iter()\n    .map(|x| x * 2)\n    .collect();\n```"
            )
            provider=Some("GPT-4".to_string())
        />

        {/* System message */}
        <MessageBubble 
            message=Message::system("Context window: 4096 tokens")
        />

        {/* Tool result */}
        <MessageBubble 
            message=Message::tool(
                "search_001",
                "docs_search",
                "Found 5 docs about iterators"
            )
        />
    </div>
}
```

### 3. MessageBody - Content Rendering

```rust
view! {
    <div class="space-y-4 p-4">
        {/* Plain text */}
        <MessageBody 
            content="This is plain text content".to_string()
            role=Role::Assistant
        />

        {/* With markdown */}
        <MessageBody 
            content="Here is **bold**, *italic*, and `code` text".to_string()
            role=Role::Assistant
        />

        {/* With code block */}
        <MessageBody 
            content="Here's a function:\n\n```rust\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n```\n\nThis adds two numbers.".to_string()
            role=Role::Assistant
        />

        {/* With link */}
        <MessageBody 
            content="Check [the docs](https://rust-lang.org) for more info".to_string()
            role=Role::Assistant
        />
    </div>
}
```

### 4. MessageHeader - Metadata Display

```rust
view! {
    <div class="space-y-3">
        {/* Basic header */}
        <MessageHeader role=Role::User />

        {/* With model info */}
        <MessageHeader 
            role=Role::Assistant
            model=Some("GPT-4 Turbo".to_string())
        />

        {/* With all metadata */}
        <MessageHeader 
            role=Role::Assistant
            timestamp=Some("2025-12-22 14:30:45".to_string())
            model=Some("Claude 3".to_string())
            tokens=Some(2341)
        />

        {/* System message header */}
        <MessageHeader 
            role=Role::System
            timestamp=Some("2025-12-22 14:25:00".to_string())
        />
    </div>
}
```

### 5. StreamingCursor - Typing Indicator

```rust
view! {
    <div class="p-4 bg-gray-50 rounded">
        <p class="text-gray-900">
            "The assistant is responding"
            <StreamingCursor />
        </p>
    </div>
}
```

Or in a message context:

```rust
view! {
    <div class="bg-gray-100 rounded-lg p-3">
        <p class="text-gray-900">
            "I'm thinking about your question"
            <StreamingCursor />
        </p>
    </div>
}
```

### 6. PromptComposer - Input Handling

```rust
view! {
    <div class="p-4">
        <PromptComposer
            on_submit=move |text| {
                console::log_1(&format!("User sent: {}", text).into());
                // Handle message...
            }
            placeholder="Type your question..."
            disabled=false
            max_chars=2000
        />
    </div>
}
```

With loading state:

```rust
let (is_loading, set_is_loading) = create_signal(false);

view! {
    <PromptComposer
        on_submit=move |text| {
            set_is_loading.set(true);
            // Send message asynchronously
            spawn_local(async move {
                // await response
                set_is_loading.set(false);
            });
        }
        disabled=is_loading.get()
        placeholder="Ask me anything..."
    />
}
```

---

## Advanced Patterns

### Pattern 1: Custom Message Styling

```rust
view! {
    <div class="flex flex-col gap-2">
        <MessageBubble 
            message=msg
            class=Some("shadow-lg border-2 border-blue-400".to_string())
        />
    </div>
}
```

### Pattern 2: Stream Response Handling

```rust
let handle_stream = move |text: String| {
    set_messages.update(|msgs| msgs.push(Message::user(text)));
    
    let assistant_idx = messages.get().len();
    set_messages.update(|msgs| {
        msgs.push(Message::assistant(String::new()));
    });
    
    set_loading.set(true);
    
    spawn_local(async move {
        // Stream response chunks
        let mut response = String::new();
        // while let Some(chunk) = stream.next().await {
        //     response.push_str(&chunk);
        //     set_messages.update(|msgs| {
        //         if let Some(msg) = msgs.get_mut(assistant_idx) {
        //             msg.content = response.clone();
        //         }
        //     });
        // }
        set_loading.set(false);
    });
};
```

### Pattern 3: Paginated History

```rust
#[component]
pub fn ChatHistory() -> impl IntoView {
    let (page, set_page) = create_signal(0);
    let messages_per_page = 20;
    
    let paginated = create_memo(move || {
        let offset = page.get() * messages_per_page;
        all_messages.get()[offset..].to_vec()
    });
    
    view! {
        <div class="flex flex-col gap-4">
            <div class="flex-1">
                <ConversationView 
                    messages=paginated.get()
                    loading=false
                />
            </div>
            
            <div class="flex gap-2 justify-between">
                <button on:click=move |_| set_page(page.get().saturating_sub(1))>
                    "Previous"
                </button>
                <button on:click=move |_| set_page(page.get() + 1)>
                    "Next"
                </button>
            </div>
        </div>
    }
}
```

### Pattern 4: Message Filtering

```rust
view! {
    <ConversationView
        messages=messages.get()
            .into_iter()
            .filter(|msg| msg.role != Role::System)
            .collect()
        loading=is_loading.get()
    />
}
```

---

## Accessibility Examples

### Screen Reader Friendly

```rust
view! {
    <div 
        role="region"
        aria-label="Chat conversation"
    >
        <ConversationView 
            messages=messages.get()
            loading=loading.get()
        />
    </div>
}
```

### Keyboard Navigation

The components support:
- `Tab` - Navigate between messages and input
- `Shift+Tab` - Reverse navigation
- `Cmd/Ctrl+Enter` - Submit message in composer
- `Enter` - Newline in composer
- `Space` - Activate buttons

---

## Styling Customization

All components accept the `class` prop for custom styling:

```rust
view! {
    <ConversationView 
        messages=messages
        loading=false
        class=Some("bg-slate-900 text-white".to_string())
    />
}
```

Override default Tailwind classes:

```rust
view! {
    <PromptComposer
        on_submit=handle_submit
        class=Some("bg-gradient-to-r from-blue-500 to-purple-600".to_string())
    />
}
```

---

## Testing Tips

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_rendering() {
        let msg = Message::user("Test message");
        assert_eq!(msg.content, "Test message");
    }

    #[test]
    fn test_role_badges() {
        let roles = vec![Role::User, Role::Assistant, Role::System, Role::Tool];
        for role in roles {
            let header = MessageHeader { role, /* ... */ };
            // Assert rendering
        }
    }
}
```

---

## Performance Considerations

### Large Message Lists
- Use `ConversationView` which handles scrolling efficiently
- Consider pagination for 1000+ messages
- Memoize message filtering with `create_memo`

### Streaming Content
- Update message content in-place rather than recreating
- Use `create_effect` to batch updates
- Consider debouncing rapid updates

### Code Highlighting
- MessageBody pre-parses markdown on render
- Syntax highlighting is CSS-based (no JavaScript)
- Code blocks are lazy-rendered

---

## Common Issues & Solutions

| Issue | Solution |
|-------|----------|
| Messages not scrolling | Ensure ConversationView parent has fixed height |
| Markdown not rendering | Check content uses proper syntax (e.g., \*\*bold\*\*) |
| Character limit not working | Pass `max_chars` prop to PromptComposer |
| Keyboard shortcut not firing | Ensure PromptComposer is focused |
| Custom styles not applying | Use `class` prop, not inline styles |

---

## Next Steps

1. Copy examples into your application
2. Connect to your LLM API backend
3. Add message persistence (database/localStorage)
4. Implement message editing/deletion
5. Add rich media support (images, files)
6. Customize colors and branding
