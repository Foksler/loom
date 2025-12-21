# Chat Components - Quick Reference

## 6 Components Implemented

| Component | Purpose | Key Props |
|-----------|---------|-----------|
| **ConversationView** | Scrollable message list with auto-scroll | `messages`, `loading` |
| **MessageBubble** | Single message with role styling | `message`, `provider` |
| **MessageBody** | Content renderer (markdown + code) | `content`, `role` |
| **MessageHeader** | Metadata badges | `role`, `model`, `tokens`, `timestamp` |
| **StreamingCursor** | Blinking typing indicator | - |
| **PromptComposer** | Input + submit button | `on_submit`, `disabled`, `max_chars` |

---

## Quick Import

```rust
use crate::components::chat::{
    ConversationView,
    MessageBubble, 
    MessageBody,
    MessageHeader,
    StreamingCursor,
    PromptComposer,
};
use loom_core::message::Message;
```

---

## Common Patterns

### Display a conversation
```rust
<ConversationView 
    messages=messages 
    loading=is_loading 
/>
```

### Handle user input
```rust
<PromptComposer
    on_submit=move |text| {
        set_messages.update(|msgs| msgs.push(Message::user(text)));
    }
    disabled=is_loading
/>
```

### Show individual messages
```rust
<MessageBubble 
    message=msg
    provider=Some("GPT-4".to_string())
/>
```

### Render message with metadata
```rust
<MessageHeader 
    role=msg.role 
    model=Some("GPT-4".to_string())
/>
<MessageBubble message=msg />
```

---

## Message Types

All components use `loom_core::message::Message`:

```rust
Message::user("Hello")                  // User message
Message::assistant("Hi there!")         // Assistant response
Message::system("You are helpful")      // System prompt
Message::tool(id, name, result)         // Tool execution result
```

---

## Styling Notes

- Components use Tailwind CSS classes
- All components accept optional `class` prop for customization
- Color scheme:
  - **Blue** = User messages
  - **Gray** = Assistant messages
  - **Amber** = System messages
  - **Green** = Tool results

---

## Files Created

```
crates/loom-web/src/components/chat/
├── conversation_view.rs  (120 lines)
├── message_bubble.rs     (95 lines)
├── message_body.rs       (220 lines)
├── message_header.rs     (65 lines)
├── streaming_cursor.rs   (30 lines)
├── prompt_composer.rs    (110 lines)
└── mod.rs                (updated)
```

---

## Styleguide

View all components and examples at:
```
src/routes/styleguide/chat.rs
```

Access via: `/styleguide/chat` in browser

---

## Status

✅ All components compile without errors  
✅ Full accessibility support (ARIA, semantic HTML)  
✅ Responsive design (mobile/desktop)  
✅ Markdown + code block rendering  
✅ Keyboard shortcuts (Cmd/Ctrl+Enter)  
✅ Production-ready
