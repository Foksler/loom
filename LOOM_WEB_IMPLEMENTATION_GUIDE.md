# Loom Web UI - Implementation Guide

## Overview

This guide explains the structure, patterns, and next steps for building the Loom Web UI using Leptos and Tailwind CSS.

## Current State

✅ **Completed:**
- Architecture design document (`WEB_UI_ARCHITECTURE.md`)
- Crate scaffold (`loom-web`)
- Module structure (routes, components, services)
- Tailwind configuration
- Basic primitives (Button component)
- Styleguide infrastructure with interactive routes
- Global state management via Context
- API and streaming service skeletons

🚧 **In Progress:**
- Component library implementation
- API integration with server
- Streaming implementation

## Project Structure

```
crates/loom-web/
├── src/
│   ├── lib.rs              # Crate entry
│   ├── app.rs              # Root App component
│   ├── main.rs             # Client hydration entry
│   ├── routes/             # Page-level components
│   │   ├── mod.rs
│   │   ├── home.rs         # Landing page
│   │   ├── threads.rs      # Thread list & detail
│   │   ├── workspace.rs    # Settings
│   │   └── styleguide/     # Component gallery
│   ├── components/         # Reusable UI components
│   │   ├── primitives/     # Design system (buttons, inputs, etc.)
│   │   ├── layout/         # App structure
│   │   ├── chat/           # Chat/conversation components
│   │   ├── query/          # Query bridge visualization
│   │   ├── results/        # Code/diff/file tree
│   │   └── indicators/     # Status/feedback
│   ├── services/           # Application logic
│   │   ├── api.rs          # Server functions
│   │   ├── state.rs        # Global app state
│   │   └── streaming.rs    # SSE/WS handling
│   └── styles/             # Tailwind & CSS
├── tailwind.config.cjs     # Tailwind configuration
├── postcss.config.cjs      # PostCSS configuration
├── Cargo.toml              # Rust dependencies
└── package.json            # npm dependencies (Tailwind, PostCSS)
```

## Development Workflow

### Prerequisites

1. **Rust toolchain**: `rustup` with latest stable + nightly
2. **Node.js**: v16+ (for Tailwind CLI)
3. **cargo-leptos**: `cargo install cargo-leptos`

### Local Development

#### Option 1: Integrated Setup (Recommended)

```bash
cd crates/loom-web
cargo leptos watch
```

This runs:
- Tailwind compiler (watch mode)
- Leptos dev server + SSR compilation
- Hot reload on file changes

#### Option 2: Manual

Terminal 1 - Tailwind:
```bash
cd crates/loom-web
npm install
npm run tailwind:watch
```

Terminal 2 - Leptos:
```bash
cd crates/loom-web
cargo leptos dev
```

### Visiting the App

- **Main app**: http://localhost:3000
- **Styleguide**: http://localhost:3000/styleguide

## Next Steps (Priority Order)

### Phase 1: Foundation (Sprint 1–2)

#### 1.1 Fix Cargo.toml and Dependencies

Currently, `loom-web/Cargo.toml` references Leptos features that may need adjustment. Verify:

```bash
cd crates/loom-web
cargo check
```

If there are errors:
- Check Leptos version compatibility
- Ensure `console_error_panic_hook` is added as a dependency (for error reporting in browser)
- May need to adjust SSR setup for Axum integration

**Action items:**
- [ ] Verify `cargo check` passes
- [ ] Add `console_error_panic_hook` to dependencies if needed
- [ ] Set up SSR binary if using `leptos_axum`

#### 1.2 Primitive Components Library

Create high-quality, reusable primitives:

**Button** ✅ (already done)

**Other primitives to add (in order):**

```
- TextField / TextArea
  └─ File: components/primitives/text_field.rs
  └─ Features: label, placeholder, error state, disabled
  └─ Test: wasm_bindgen_test rendering variants

- Select / MultiSelect
  └─ File: components/primitives/select.rs
  └─ Features: open/close state, keyboard navigation
  └─ Test: arrow key selection, click to toggle

- Toggle / Switch / Checkbox
  └─ File: components/primitives/toggle.rs
  └─ Features: checked state, disabled
  └─ Test: click to toggle state

- Card / Panel / SectionHeader
  └─ File: components/primitives/card.rs
  └─ Features: padding variants, border styles
  └─ Test: rendering with children

- Spinner / Skeleton
  └─ File: components/primitives/spinner.rs
  └─ Features: size variants, animation
  └─ Test: CSS animation classes applied

- ProgressBar
  └─ File: components/primitives/progress.rs
  └─ Features: percentage progress
  └─ Test: width calculation

- Tooltip / Popover / Modal
  └─ File: components/primitives/overlays.rs
  └─ Features: positioning, click-outside close
  └─ Test: event handling

- Badge / Chip / Tag
  └─ File: components/primitives/badge.rs
  └─ Features: variant colors
  └─ Test: rendering with icons
```

**Pattern for each:**

1. Define the component with props
2. Use Tailwind classes for styling
3. Add variants as enums (e.g., `ButtonVariant`, `BadgeVariant`)
4. Export from `components/primitives/mod.rs`
5. Add gallery view to `/styleguide/primitives` page
6. Write property-based tests in a `_test.rs` file

**Example: TextField**

```rust
// components/primitives/text_field.rs
#[component]
pub fn TextField(
    #[prop(optional)] label: Option<String>,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] error: Option<String>,
    #[prop(default = false)] disabled: bool,
    value: String,
    on_change: impl Fn(String) + 'static,
) -> impl IntoView {
    view! {
        <div class="space-y-1">
            {label.map(|l| view! { <label class="text-sm font-medium">{l}</label> })}
            <input
                type="text"
                class="input"
                placeholder=placeholder
                value=value
                on:change=move |e| on_change(event_target_value(&e))
                disabled=disabled
            />
            {error.map(|e| view! { <p class="text-sm text-error">{e}</p> })}
        </div>
    }
}
```

#### 1.3 Composite Components (Layout)

Build composites using primitives:

```
- AppShell ✅ (basic version done)
  └─ Enhance: add sidebar toggles, responsive breakpoints

- ResizablePanels
  └─ File: components/layout/resizable_panels.rs
  └─ Features: drag-to-resize, persist widths
  └─ Test: drag mouse events, computed widths

- DataTable
  └─ File: components/layout/data_table.rs
  └─ Features: sortable columns, pagination
  └─ Test: click to sort, pagination nav

- Form utilities
  └─ File: components/layout/form.rs
  └─ Features: FieldRow, FormSection
  └─ Test: label association
```

#### 1.4 Update Styleguide Pages

For each new primitive/composite:

1. Add a gallery section to the relevant styleguide page
2. Show all variants
3. Include small code snippet
4. Example: `/styleguide/primitives` → show all buttons, text fields, toggles, etc.

### Phase 2: Core Features (Sprint 3–4)

#### 2.1 Thread Management Pages

**File: routes/threads.rs → split into threads/list.rs and threads/detail.rs**

ThreadListPage:
- Fetch threads via `create_resource(get_threads)`
- Display in a list/table component
- "New thread" button

ThreadDetailPage:
- Use route params to get thread ID
- Fetch thread details
- Display metadata (created, last updated, provider, model)
- Show conversation history (placeholder for now)

#### 2.2 Chat Components

Files to create:

```
- components/chat/conversation_view.rs
  └─ Props: messages: Vec<Message>, loading: bool
  └─ Auto-scroll on new messages
  └─ Render each message via MessageBubble

- components/chat/message_bubble.rs
  └─ Props: role (user/assistant/system/tool), content, timestamp
  └─ Render markdown for assistant messages
  └─ Special styling for tool results

- components/chat/message_actions.rs
  └─ Props: message_id
  └─ Actions: copy, quote, retry, fork
  └─ Tooltip on hover

- components/chat/prompt_composer.rs
  └─ Props: on_submit, disabled (while streaming)
  └─ Textarea for input
  └─ Submit button
  └─ Model/temperature controls via PromptToolbar

- components/chat/streaming_cursor.rs
  └─ Blinking cursor indicator
  └─ Show during streaming
```

#### 2.3 Streaming Integration

**File: services/streaming.rs** → Implement actual SSE handler

```rust
#[cfg(target_arch = "wasm32")]
pub fn start_streaming(
    thread_id: String,
    on_event: impl Fn(StreamEvent) + 'static,
) {
    use web_sys::EventSource;

    let url = format!("/api/threads/{}/stream", thread_id);
    
    match EventSource::new(&url) {
        Ok(es) => {
            // Register listeners for 'message' events
            // Parse JSON and emit StreamEvent via on_event
        }
        Err(_) => on_event(StreamEvent::Error("Failed to connect".into())),
    }
}
```

### Phase 3: Query Bridge & Results (Sprint 5)

#### 3.1 Query Bridge Visualization

Files to create:

```
- components/query/query_timeline.rs
  └─ Props: steps: Vec<QueryStep>, current_step: usize
  └─ Render vertical timeline with step cards
  └─ Status icons (pending, running, completed, error)

- components/query/query_step_card.rs
  └─ Props: step (interpret, plan, tool_call, apply)
  └─ Show logs, inputs, outputs
  └─ Expandable details

- components/query/state_machine_trace.rs
  └─ Props: trace: Vec<StateTransition>
  └─ Render state history
  └─ Jump to specific state (future feature)

- components/query/tool_invocation.rs
  └─ Props: tool_name, args, result
  └─ Format args as code
  └─ Show execution time
```

Add styleguide page: `/styleguide/query`

#### 3.2 Results Display

Files to create:

```
- components/results/code_block.rs
  └─ Props: code: String, language: String
  └─ Syntax highlighting (via syntect)
  └─ Copy button
  └─ Language badge

- components/results/diff_view.rs
  └─ Props: before: String, after: String
  └─ Side-by-side or inline diff
  └─ Highlighted additions/deletions

- components/results/file_tree.rs
  └─ Props: files: Vec<FileNode>
  └─ Nested tree structure
  └─ Expandable folders
  └─ Icons for file types

- components/results/result_panel.rs
  └─ Container for showing execution results
  └─ Tab between different result types
```

### Phase 4: Polish & Advanced

- Dark mode / theming (CSS variables, toggle)
- WebSocket upgrade (replace SSE)
- Performance optimizations (code splitting, lazy routes)
- End-to-end tests (Playwright)
- Visual regression testing on styleguide

## Testing Strategy

### Component Tests (Wasm)

**File: components/primitives/button_test.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn button_renders_primary_variant() {
        leptos::leptos_dom::HydrationCtx::reset_id();
        let view = leptos::view! {
            <Button variant=ButtonVariant::Primary>"Click"</Button>
        };
        // Assert generated HTML contains expected classes
    }

    #[wasm_bindgen_test]
    fn button_disabled_has_opacity() {
        // Test disabled styling
    }
}
```

Run tests:
```bash
cargo test --target wasm32-unknown-unknown
```

### Integration Tests (E2E)

Use Playwright to test user flows:

**File: tests/e2e/threads.spec.ts**

```typescript
import { test, expect } from '@playwright/test';

test('user creates a new thread', async ({ page }) => {
    await page.goto('http://localhost:3000/threads');
    await page.click('[data-testid="new-thread-btn"]');
    await page.fill('[data-testid="thread-title"]', 'My Query');
    await page.click('[data-testid="submit"]');
    
    await expect(page.locator('text=My Query')).toBeVisible();
});
```

Run tests:
```bash
npx playwright test
```

### Property-Based Tests

For streaming state logic, use `proptest`:

**File: services/streaming_test.rs**

```rust
proptest! {
    #[test]
    fn streaming_chunks_accumulate(chunks in prop::collection::vec("[a-z]+", 1..10)) {
        let mut buffer = String::new();
        for chunk in chunks {
            buffer.push_str(&chunk);
        }
        // Assert invariant: buffer is concatenation of chunks
    }
}
```

## API Integration (Server Functions)

### Step 1: Define Server Functions

In `services/api.rs`, use `#[server]` macro:

```rust
#[server(GetThreads, "/api")]
pub async fn get_threads() -> Result<Vec<ThreadSummary>, ServerFnError> {
    // This runs on the server
    // Access database, call loom-server endpoints, etc.
    Ok(vec![])
}
```

### Step 2: Implement in loom-server

In `loom-server` binary (or library), register the handler:

```rust
// loom-server/src/api.rs
#[server_fn_handler]
pub async fn get_threads() -> Result<Vec<ThreadSummary>, ServerFnError> {
    let db = /* get database connection */;
    let threads = db.list_threads().await?;
    Ok(threads)
}
```

### Step 3: Call from Components

```rust
let threads = create_resource(
    || (),
    |_| async move { get_threads().await }
);

view! {
    <Suspense fallback=|| "Loading...">
        {move || threads.get().map(|result| {
            match result {
                Ok(list) => view! { /* render threads */ },
                Err(e) => view! { <p class="text-error">{e.to_string()}</p> },
            }
        })}
    </Suspense>
}
```

## Tailwind & Styling

### Design Tokens

Define in `tailwind.config.cjs`:

```javascript
theme: {
  extend: {
    colors: {
      brand: { 500: "#0ea5e9", ... },
      neutral: { 900: "#1f2937", ... },
    },
    spacing: {
      xs: "0.25rem",
      sm: "0.5rem",
      md: "1rem",
      ...
    },
  },
}
```

Use in components:

```rust
view! {
    <button class="px-md py-sm bg-brand-500 text-white rounded-lg">
        Click
    </button>
}
```

### Component Shortcuts

In `styles/input.css`:

```css
@layer components {
  .btn {
    @apply inline-flex items-center gap-2 px-3 py-1.5 rounded-md font-medium transition-colors;
  }

  .btn-primary {
    @apply btn bg-blue-600 text-white hover:bg-blue-700;
  }
}
```

Use:

```rust
view! { <button class="btn btn-primary">Click</button> }
```

## Build & Deploy

### Development Build

```bash
cargo leptos watch
# or
cargo leptos dev
```

### Production Build

```bash
cargo leptos build --release
```

Outputs:
- `target/release/loom-server` (SSR binary)
- `target/site/` (static assets)

### Docker

The existing `loom-server` Docker build can be enhanced to include the web UI:

```dockerfile
FROM rust:latest AS builder
WORKDIR /app
COPY . .

# Install Node for Tailwind
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - && apt-get install -y nodejs

# Build leptos app
RUN cargo leptos build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/loom-server /usr/local/bin/
COPY --from=builder /app/target/site /var/www/loom

EXPOSE 8080
CMD ["loom-server"]
```

## Recommended Tools & Extensions

### VS Code Extensions

- **rust-analyzer** (0.4.x+) - Rust support
- **Leptos** - Syntax highlighting for `view!` macros
- **Tailwind CSS IntelliSense** - Tailwind class completion
- **Thunder Client** or **REST Client** - API testing

### CLI Tools

- **cargo-leptos** - Integrated dev server
- **cargo-watch** - Auto-rebuild on file changes
- **trunk** - Alternative WASM bundler (if not using cargo-leptos)

## Common Patterns

### Reactive Signal Updates

```rust
let count = create_signal(0);

view! {
    <button on:click=move |_| count.set(count.get() + 1)>
        "Increment"
    </button>
    <p>{move || count.get()}</p>
}
```

### Resource with Refetch

```rust
let threads = create_resource(
    || (),
    |_| async move { get_threads().await }
);

view! {
    <button on:click=move |_| threads.refetch()>
        "Refresh"
    </button>
    <Suspense fallback=|| "Loading...">
        {move || threads.get()}
    </Suspense>
}
```

### Context Management

```rust
// Provide
provide_context(my_state);

// Consume
let state = use_context::<MyState>()
    .expect("MyState not provided");
```

### Conditional Rendering

```rust
view! {
    {if show_details {
        view! { <DetailsPanel/> }
    } else {
        view! { <Summary/> }
    }}
}
```

Or using `<Show>`:

```rust
view! {
    <Show when=show_details fallback=|| view! { <Summary/> }>
        <DetailsPanel/>
    </Show>
}
```

## Troubleshooting

### Issue: Leptos macro errors

**Solution**: Ensure you're using the Rust nightly toolchain and latest `leptos` crate.

```bash
rustup install nightly
rustup override set nightly
cargo update
```

### Issue: Tailwind classes not applied

**Solution**: Check that Tailwind config includes your `src/` paths:

```javascript
content: ["./src/**/*.{rs,html}"],
```

Then rebuild CSS:

```bash
npm run tailwind:build
```

### Issue: SSE connection fails

**Solution**: Check server exposes the correct endpoint. Verify CORS headers if on different origin.

### Issue: Hydration mismatch

**Solution**: This happens when server-rendered HTML doesn't match client. Use `HydrationCtx` in tests and ensure no randomness in initial render.

## Resources

- **Leptos Book**: https://leptos.dev
- **Leptos API Docs**: https://docs.rs/leptos
- **Tailwind CSS**: https://tailwindcss.com
- **MDN - Web Events**: https://developer.mozilla.org/en-US/docs/Web/Events
- **Rust Book**: https://doc.rust-lang.org/book/

## Summary

You now have:

1. ✅ Crate scaffold with proper module structure
2. ✅ Tailwind configuration and styling foundation
3. ✅ Button primitive component as example
4. ✅ Styleguide routes for component documentation
5. ✅ Global app state via Context
6. ✅ API and streaming service skeletons

**Next immediate actions:**

1. Verify `cargo check` passes
2. Run `cargo leptos watch` and test basic app
3. Build out remaining primitives (TextField, Select, Toggle, etc.)
4. Add composite layout components (AppShell enhancements, DataTable)
5. Implement thread list and detail pages
6. Connect chat components with streaming

Good luck! 🚀
