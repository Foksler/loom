# Loom Web UI Architecture

## Overview

This document describes the architecture of the `loom-web` SPA (Single Page Application) built with **Leptos** and **Tailwind CSS**. The UI is served by `loom-server` and provides the primary interface for interacting with the Loom AI coding assistant.

## Crate Structure

### Workspace Layout

```
/loom
  /crates
    /loom-server         # Backend HTTP server + SSR
    /loom-web            # New Leptos SPA
    /loom-types          # (Future) Shared DTOs
    /loom-core           # Core types
    /... (existing)
```

### `loom-web` Internal Structure

```
loom-web/
  src/
    main.rs                    # Client entry point (hydration)
    server.rs                  # SSR entry point (Leptos-Axum)
    app.rs                     # <App/> root component
    routes/
      mod.rs
      home.rs                  # Landing page / workspace view
      threads/
        mod.rs
        list.rs                # Thread list page
        detail.rs              # Thread detail + conversation
      workspace.rs             # Workspace overview
      styleguide/              # Storybook-like component gallery
        mod.rs
        index.rs               # Styleguide index
        primitives.rs          # Buttons, inputs, typography
        chat.rs                # Message components
        query.rs               # Query bridge components
        results.rs             # Code, diff, file tree
    components/
      primitives/              # Design system components
        mod.rs
        button.rs
        input.rs
        select.rs
        toggle.rs
        card.rs
        spinner.rs
        skeleton.rs
        progress_bar.rs
        tooltip.rs
        modal.rs
        toast.rs
      layout/                  # App structure
        mod.rs
        app_shell.rs
        top_nav.rs
        sidebar.rs
        resizable_panels.rs
      navigation/              # Navigation components
        mod.rs
        nav_item.rs
        tabs.rs
        breadcrumbs.rs
      chat/                    # Loom-domain: conversation
        mod.rs
        conversation_view.rs
        message_bubble.rs
        message_header.rs
        message_body.rs
        streaming_cursor.rs
        message_actions.rs
        prompt_composer.rs
        prompt_toolbar.rs
      threads/                 # Loom-domain: thread management
        mod.rs
        thread_list.rs
        thread_list_item.rs
        thread_header.rs
        thread_metadata.rs
      query/                   # Loom-domain: query bridge
        mod.rs
        query_timeline.rs
        query_step_card.rs
        tool_invocation.rs
        state_machine_trace.rs
      results/                 # Loom-domain: execution results
        mod.rs
        result_panel.rs
        code_block.rs
        diff_view.rs
        file_tree.rs
      indicators/              # Status & feedback
        mod.rs
        badge.rs
        chip.rs
        status_icon.rs
    services/
      mod.rs
      api.rs                   # #[server] functions & API wrappers
      streaming.rs             # SSE/WS streaming logic
      state.rs                 # Global app state via Context
    styles/
      input.css                # @tailwind directives + design tokens
  tailwind.config.cjs          # Tailwind configuration
  postcss.config.cjs           # PostCSS configuration
  Cargo.toml
  package.json                 # For Tailwind/PostCSS tooling
```

## Design System

### Component Hierarchy

#### **Tier 1: Primitives** (Design System)

Pure, reusable building blocks with no Loom-specific logic:

- **Typography & Layout**
  - `Card`, `Panel`, `SectionHeader`
  - `Stack` (vertical), `Inline` (horizontal) — optional wrappers
- **Inputs**
  - `Button` (primary, secondary, ghost, destructive, icon)
  - `TextField` (with label, help, error states)
  - `TextArea`
  - `Select`, `MultiSelect`
  - `Toggle`, `Switch`, `Checkbox`, `RadioGroup`
  - `Slider` (temperature, top_p, etc.)
- **Feedback & Overlays**
  - `Spinner`, `Skeleton`, `ProgressBar`
  - `Tooltip`, `Popover`
  - `ModalDialog`
  - `Toast` / `Notification` (global)
- **Navigation**
  - `SidebarNav`, `TopNavBar`, `Tabs`, `Breadcrumbs`

#### **Tier 2: Composite Components** (App-level)

- **App Shell**
  - `AppShell` (arranges layout regions)
  - `ResizablePanels` (drag-to-resize regions)
- **Data Display**
  - `DataTable`
  - `KeyValueList`
  - `Tag`, `Chip`, `Badge`
- **Form Patterns**
  - `FormSection`, `FieldRow`

#### **Tier 3: Loom-Domain Components** (Feature-specific)

- **Chat & Messages**
  - `ConversationView`
  - `MessageBubble` (user/assistant/system/tool variants)
  - `MessageHeader`, `MessageBody`
  - `StreamingCursor` (blinking cursor for live tokens)
  - `MessageActions`
- **Composer & Query Input**
  - `PromptComposer` (textarea, shortcuts, submit)
  - `PromptToolbar` (model, temperature, system prompt)
- **Threads & Workspace**
  - `ThreadList`, `ThreadListItem`
  - `ThreadHeader`
  - `ThreadMetadataPanel`
- **Query Bridge & State Machine**
  - `QueryTimeline` (steps: interpret → plan → execute)
  - `QueryStepCard`
  - `ToolInvocationList`, `ToolInvocationItem`
  - `StateMachineTrace` (status timeline)
- **Results & Code**
  - `LLMResultPanel`
  - `CodeBlock` (syntax highlighting, copy)
  - `DiffView` (side-by-side or inline diff)
  - `FileTree`

### Styling Approach

- **Tailwind CSS**: Co-located in `loom-web/tailwind.config.cjs`.
- **CSS Variables for tokens**: Theme colors, spacing, radii defined as CSS custom properties.
- **Component classes**: Minimal component abstractions (e.g., `.btn`, `.input`) for heavily-reused patterns.
- **Utility-first**: Prefer inline Tailwind utilities in component templates.

## Data Access & Integration

### API Integration Pattern

#### Non-Streaming Operations: `#[server]` RPC

Define server functions in `services/api.rs`:

```rust
#[server(GetThreads, "/api")]
pub async fn get_threads() -> Result<Vec<ThreadSummary>, ServerFnError> {
    // Implementation in loom-server
}

#[server(CreateThread, "/api")]
pub async fn create_thread(title: String) -> Result<Thread, ServerFnError> {
    // ...
}
```

Call from components:

```rust
let threads = create_resource(
    || (),
    |_| async move { get_threads().await }
);

view! {
    <Suspense fallback=|| "Loading...">
        {move || threads.get().map(|t| /* render threads */)}
    </Suspense>
}
```

**Benefits:**
- Type-safe client/server boundary.
- Automatic serialization/deserialization.
- Seamless SSR + hydration integration.

#### Streaming Operations: Explicit Transport (SSE → WebSocket)

Keep streaming in `services/streaming.rs`:

```rust
pub enum StreamEvent {
    Chunk { message_id: String, text: String },
    Done { final_response: LlmResponse },
    Error(String),
}

pub fn start_streaming(
    thread_id: ThreadId,
    on_event: impl Fn(StreamEvent) + 'static,
) {
    use web_sys::EventSource;
    // Construct URL, attach listeners
}
```

Use in components:

```rust
let (state, set_state) = create_signal(StreamState::Idle);

let handle_submit = move |prompt: String| {
    set_state(StreamState::Streaming {
        buffer: String::new(),
    });
    start_streaming(thread_id, move |event| {
        match event {
            StreamEvent::Chunk { text, .. } => {
                set_state.update(|s| {
                    if let StreamState::Streaming { buffer } = s {
                        buffer.push_str(&text);
                    }
                });
            }
            StreamEvent::Done { .. } => set_state(StreamState::Idle),
            StreamEvent::Error(e) => set_state(StreamState::Error(e)),
        }
    });
};
```

### State Management

#### Global State via Context

`services/state.rs`:

```rust
#[derive(Clone)]
pub struct AppState {
    pub active_thread_id: RwSignal<Option<ThreadId>>,
    pub threads: Resource<(), Result<Vec<ThreadSummary>, ServerFnError>>,
    pub streaming_state: RwSignal<StreamingState>,
    pub query_settings: RwSignal<QuerySettings>,
}

pub fn provide_app_state(cx: Scope) {
    let threads = create_resource(|| (), |_| async move { get_threads().await });
    let active_thread_id = create_rw_signal(cx, None);
    // ... initialize other signals

    provide_context(cx, AppState {
        active_thread_id,
        threads,
        streaming_state,
        query_settings,
    });
}

pub fn use_app_state(cx: Scope) -> AppState {
    use_context::<AppState>(cx).expect("AppState not provided")
}
```

Wrap in `<App/>`:

```rust
#[component]
pub fn App() -> impl IntoView {
    view! {
        <AppStateProvider>
            <AppShell>
                <Router>
                    <Routes>
                        // Routes...
                    </Routes>
                </Router>
            </AppShell>
        </AppStateProvider>
    }
}
```

#### Per-Route/Component State

- Route components maintain their own `Resource`s (e.g., thread detail fetches current thread).
- Component-local state stays in `Signal`s (e.g., form input, UI toggles).
- Cross-cutting state (active thread, streaming state) lives in `AppState`.

### Streaming Request Lifecycle

Model as a state enum:

```rust
pub enum StreamingState {
    Idle,
    Starting,
    Streaming {
        thread_id: ThreadId,
        partial_message: String,
    },
    Error(String),
}
```

**Flow:**
1. User submits prompt in `PromptComposer` → set state to `Starting`.
2. API call triggered; streaming begins → update to `Streaming { partial_message: "..." }`.
3. Each chunk appends to `partial_message` via `Signal::update`.
4. On completion or error → revert to `Idle` and refresh the thread's messages.

## Routing

### Route Map

```
/                               # Workspace overview / landing
/threads                        # Thread list
/threads/:id                    # Thread detail + conversation
/threads/:id/query-bridge       # Query bridge visualization (optional phase 2)
/workspace                      # Settings, tools, providers
/styleguide                     # Component gallery index
/styleguide/primitives          # Buttons, inputs, typography
/styleguide/chat                # Message components
/styleguide/query               # Query bridge components
/styleguide/results             # Code, diff, file tree
/styleguide/layout              # Layout & shell variants
```

Implement with `leptos_router`:

```rust
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes>
                <Route path="" view=|| view! { <HomePage/> }/>
                <Route path="threads" view=|| view! { <ThreadsPage/> }/>
                <Route path="threads/:id" view=|| view! { <ThreadDetailPage/> }/>
                <Route path="styleguide/*" view=|| view! { <StyleguideLayout/> }/>
                // ...
            </Routes>
        </Router>
    }
}
```

## Development Workflow

### Local Development

#### Option 1: Integrated (Recommended)

Use `cargo-leptos` for a unified dev loop:

```bash
cargo leptos watch
```

This runs:
- Tailwind CLI (watch mode).
- Leptos build (client + server, watch mode).
- Axum server (auto-restart on code changes).

#### Option 2: Manual

```bash
# Terminal 1: Tailwind
npm run tailwind:watch

# Terminal 2: Leptos + Axum
cargo leptos dev

# Terminal 3: Optional — Storybook-like standalone server (future)
cargo leptos serve
```

### Styleguide Development

The `/styleguide` routes provide an interactive component gallery:

- Navigate to `/styleguide/primitives` to see all button variants.
- Modify component props via URL query params or local controls (if added).
- Take screenshots for visual regression testing.
- Treat styleguide changes as a commit requirement when updating components.

### Build for Production

```bash
cargo leptos build --release
```

Outputs:
- `loom-server` binary with embedded SSR.
- `loom-web` static assets (JS, CSS) in a dist folder.
- Configured for production: minified JS/CSS, optimized images.

## Testing Strategy

### Component Testing (Wasm)

Use `wasm-bindgen-test` + Leptos test utilities:

```rust
#[wasm_bindgen_test]
fn button_renders_primary_variant() {
    leptos::leptos_dom::HydrationCtx::reset_id();
    let view = leptos::view! {
        <Button variant=ButtonVariant::Primary>Submit</Button>
    };
    // Assert on generated HTML or use query selectors
    // Example: assert_eq!(view.innerHTML.contains("btn-primary"), true);
}
```

Test organization:
- Place component tests in `components/**/*_test.rs`.
- Test primitives extensively (rendering, variants, accessibility).
- Test domain components with mock data and state.

### Integration Testing (E2E)

Use **Playwright** or **Cypress** to test user flows:

```javascript
// tests/e2e/threads.spec.ts
test('user creates a new thread', async ({ page }) => {
    await page.goto('/threads');
    await page.click('[data-testid="new-thread-btn"]');
    await page.fill('[data-testid="thread-title"]', 'My Query');
    await page.click('[data-testid="submit"]');
    
    // Assert new thread appears in list
    await expect(page.locator('text=My Query')).toBeVisible();
});
```

Test the styleguide visually:
- Run tests against `/styleguide/*` routes.
- Capture screenshots and compare against baseline (visual regression).

### Property-Based Testing (Streaming)

Test streaming state machines with `proptest`:

```rust
proptest! {
    #[test]
    fn streaming_chunks_accumulate(chunks in prop::collection::vec("[a-z]+", 1..10)) {
        let mut state = StreamingState::Idle;
        for chunk in chunks {
            // Apply chunk to state
            update_streaming_state(&mut state, chunk);
            // Assert invariant: accumulated text is concatenation
        }
    }
}
```

## Tailwind & Styling

### Configuration

**`tailwind.config.cjs`:**

```javascript
module.exports = {
  content: [
    "./src/**/*.{rs,html}",
  ],
  theme: {
    extend: {
      colors: {
        brand: {
          DEFAULT: "hsl(var(--color-brand))",
          dark: "hsl(var(--color-brand-dark))",
        },
        neutral: {
          DEFAULT: "hsl(var(--color-neutral))",
        },
      },
      spacing: {
        xs: "0.25rem",
        sm: "0.5rem",
        md: "1rem",
        lg: "1.5rem",
        xl: "2rem",
      },
      borderRadius: {
        xs: "0.25rem",
        sm: "0.375rem",
        md: "0.5rem",
        lg: "0.75rem",
      },
    },
  },
};
```

**`styles/input.css`:**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

/* Design tokens */
:root {
  --color-brand: 220 90% 56%;
  --color-brand-dark: 220 90% 40%;
  --color-neutral: 220 13% 40%;
  --radius-md: 0.5rem;
}

/* Component shortcuts */
.btn {
  @apply inline-flex items-center justify-center px-3 py-1.5 
         rounded-md text-sm font-medium transition-colors;
}

.btn-primary {
  @apply btn bg-brand text-white hover:bg-brand-dark;
}

.input {
  @apply w-full px-3 py-2 rounded-md border border-neutral/20 
         placeholder-neutral/40 focus:outline-none focus:ring-2 
         focus:ring-brand;
}
```

### Design Token Consistency

- Define colors, spacing, radii as **CSS variables** for easy theming.
- Use Tailwind's `theme.extend` to reference them.
- Document tokens in `/styleguide/primitives`.

## Integration with `loom-server`

### Server-Side Setup

In `loom-server/src/lib.rs` or `main.rs`:

1. **Serve SSR:**

```rust
use leptos_axum::LeptosRoutes;

let app = Router::new()
    .leptos_routes(leptos_options, routes(cx), || { view! { <App/> } })
    .fallback_service(ServeDir::new("dist").not_found_service(ServeDir::new("dist/index.html")))
    .into_make_service();
```

2. **API Endpoints:** Implement `#[server]` function handlers in `loom-server`:

```rust
// loom-server/src/api.rs
#[server(GetThreads, "/api")]
pub async fn get_threads() -> Result<Vec<ThreadSummary>, ServerFnError> {
    // Query SQLite, fetch threads
    let threads = db::list_threads().await?;
    Ok(threads)
}
```

3. **Streaming Endpoints:** Expose SSE/WebSocket endpoints:

```rust
// Example: GET /api/threads/:id/stream -> SSE
// Existing: /proxy/anthropic/stream, /proxy/openai/stream
```

## Phases & Roadmap

### Phase 1: Foundation (Sprint 1–2)

- [x] Design architecture document (this file).
- [ ] Create `loom-web` crate scaffold.
- [ ] Set up Leptos + Tailwind + build tooling.
- [ ] Build Tier 1 primitives (button, input, card, etc.).
- [ ] Build Tier 2 (app shell, data table).
- [ ] Implement basic routing.

### Phase 2: Core Features (Sprint 3–4)

- [ ] Thread list & detail pages.
- [ ] `ConversationView` + `PromptComposer`.
- [ ] Streaming integration (SSE for LLM responses).
- [ ] Basic query bridge visualization (`QueryTimeline`).
- [ ] Styleguide / component gallery.

### Phase 3: Polish & Advanced (Sprint 5+)

- [ ] Dark mode / theming support.
- [ ] WebSocket upgrade (persistent connections).
- [ ] Advanced state machine visualization.
- [ ] E2E tests + visual regression.
- [ ] Performance optimization (code splitting, lazy loading).

## Example Component

### Primitive: Button

**File:** `components/primitives/button.rs`

```rust
use leptos::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Destructive,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

#[component]
pub fn Button(
    #[prop(default = ButtonVariant::Primary)] variant: ButtonVariant,
    #[prop(default = ButtonSize::Md)] size: ButtonSize,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] loading: bool,
    children: Children,
    #[prop(optional)] on_click: Option<impl Fn(web_sys::MouseEvent) + 'static>,
) -> impl IntoView {
    let variant_class = match variant {
        ButtonVariant::Primary => "bg-brand text-white hover:bg-brand-dark",
        ButtonVariant::Secondary => "bg-neutral/10 text-neutral hover:bg-neutral/20",
        ButtonVariant::Ghost => "text-neutral hover:bg-neutral/10",
        ButtonVariant::Destructive => "bg-red-600 text-white hover:bg-red-700",
    };

    let size_class = match size {
        ButtonSize::Sm => "px-2 py-1 text-sm",
        ButtonSize::Md => "px-3 py-1.5 text-sm",
        ButtonSize::Lg => "px-4 py-2 text-base",
    };

    view! {
        <button
            class=format!(
                "btn {} {} {} transition-colors {}",
                variant_class,
                size_class,
                if disabled { "opacity-50 cursor-not-allowed" } else { "" },
                if loading { "opacity-70" } else { "" }
            )
            disabled=disabled
            on_click=on_click
        >
            {if loading {
                view! { <Spinner size=SpinnerSize::Sm/> }
            } else {
                view! { {children()} }
            }}
        </button>
    }
}
```

### Styleguide: Button Gallery

**File:** `routes/styleguide/primitives.rs`

```rust
#[component]
pub fn StyleguidePrimitives() -> impl IntoView {
    view! {
        <div class="p-8 max-w-4xl mx-auto">
            <h1 class="text-3xl font-bold mb-8">Primitives</h1>

            <section class="mb-12">
                <h2 class="text-2xl font-semibold mb-4">Button</h2>
                <div class="space-y-4">
                    <div class="flex gap-4">
                        <Button variant=ButtonVariant::Primary>"Primary"</Button>
                        <Button variant=ButtonVariant::Secondary>"Secondary"</Button>
                        <Button variant=ButtonVariant::Ghost>"Ghost"</Button>
                        <Button variant=ButtonVariant::Destructive>"Destructive"</Button>
                    </div>
                    <div class="flex gap-4">
                        <Button size=ButtonSize::Sm>"Small"</Button>
                        <Button size=ButtonSize::Md>"Medium"</Button>
                        <Button size=ButtonSize::Lg>"Large"</Button>
                    </div>
                </div>
            </section>

            // ... other primitives
        </div>
    }
}
```

## References

- **Leptos Book**: https://leptos.dev
- **Tailwind CSS**: https://tailwindcss.com
- **Loom Architecture**: See `specs/architecture.md`
- **Streaming Design**: See `specs/streaming.md`
