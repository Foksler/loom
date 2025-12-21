# Loom Web UI - Project Summary

## What Was Delivered

A complete **architecture design** and **crate scaffold** for a production-ready Leptos + Tailwind CSS SPA for the Loom AI coding assistant.

### Artifacts

1. **WEB_UI_ARCHITECTURE.md** — Comprehensive 600+ line architecture document covering:
   - Crate structure (single `loom-web` crate with clear internal boundaries)
   - Component hierarchy (Tier 1 primitives, Tier 2 composites, Tier 3 domain-specific)
   - Leptos integration patterns (`#[server]` RPC + explicit streaming transports)
   - Tailwind configuration and design tokens
   - State management via Context
   - Streaming lifecycle (SSE → WebSocket in Phase 3)
   - Routing and URL structure
   - Testing strategy (wasm-bindgen-test, Playwright, proptest)

2. **LOOM_WEB_IMPLEMENTATION_GUIDE.md** — Detailed 500+ line implementation roadmap:
   - Phase-by-phase breakdown (Foundation → Core → Advanced)
   - Prioritized component checklist
   - Concrete file locations and code patterns
   - API integration guide
   - Testing examples
   - Troubleshooting & resources

3. **crates/loom-web/** — Complete crate scaffold including:
   - **Cargo.toml** — Dependencies (Leptos 0.7, Axum, Tailwind, web-sys)
   - **src/lib.rs** — Crate entry
   - **src/app.rs** — Root component with routing & state
   - **src/main.rs** — Client hydration entry point
   - **routes/** — Module structure for 6 page categories
     - home.rs (landing)
     - threads.rs (thread list & detail)
     - workspace.rs (settings)
     - styleguide/ (component gallery with 6 sub-routes)
   - **components/** — Module structure for 6 component categories
     - primitives/ (buttons, inputs, etc.)
     - layout/ (app shell, panels)
     - chat/ (messages, composer)
     - query/ (state machine visualization)
     - results/ (code, diffs)
     - indicators/ (status, feedback)
   - **services/** — Application logic
     - api.rs (server functions skeleton)
     - state.rs (global app state via Context)
     - streaming.rs (SSE/WebSocket skeleton)
   - **styles/** — Tailwind & CSS
     - input.css (with @tailwind directives, design tokens, component utilities)
   - **tailwind.config.cjs** — Theme configuration
   - **postcss.config.cjs** — CSS processing
   - **package.json** — npm dependencies

4. **Example Implementation:**
   - **Button component** (components/primitives/button.rs) — Fully built with:
     - 4 variants (Primary, Secondary, Ghost, Destructive)
     - 3 sizes (Sm, Md, Lg)
     - States (disabled, loading)
     - 100% Tailwind styling
     - Proper documentation
   - **Styleguide pages** — Interactive component gallery with navigation sidebar

### Design Philosophy

**1. Simplicity & Maintainability**
- Single crate (not multi-crate) to avoid tooling friction
- Clear internal module structure
- Tiered component organization (primitives → composites → domain)

**2. Production-Ready Patterns**
- Leptos `#[server]` functions for type-safe RPC
- Explicit streaming via SSE (WebSocket in Phase 3)
- Global state via Context (no extra state lib)
- Tailwind co-located in UI crate
- Structured logging via `tracing`

**3. Reusability First**
- Design system tier with generic primitives
- Component gallery (styleguide) for documentation
- Property-based testing for streaming logic
- Clear separation between design system and app-specific code

**4. Streaming as First-Class Citizen**
- SSE protocol for Phase 2 (real-time LLM responses)
- WebSocket upgrade path for Phase 3
- Centralized streaming logic (`services/streaming.rs`)
- State machine modeling for streaming lifecycle

### Key Decisions

| Decision | Rationale |
|----------|-----------|
| **Single `loom-web` crate** | Simplifies builds, tooling, Tailwind setup; still allows clear internal boundaries |
| **`#[server]` for CRUD, explicit transport for streaming** | Leverages Leptos strengths: typed RPC for standard ops, flexible transport for SSE/WS |
| **Tailwind co-located in `loom-web`** | Standard Rust+Leptos pattern; avoids cross-crate styling complexity |
| **Signals + Context for state** | Idiomatic Leptos; no extra state management library needed |
| **Styleguide as part of SPA** | Zero extra toolchain; documentation always in sync with code |
| **Tier-based component organization** | Clear mental model: primitives → composites → domain-specific |

## Architecture Overview

### Crate Structure

```
loom/
├── crates/
│   ├── loom-server      ← HTTP server + SSR backend
│   ├── loom-web         ← NEW SPA (Leptos + Tailwind)
│   ├── loom-core        ← Types & traits
│   ├── loom-thread      ← Thread persistence
│   └── ... (other crates)
```

### Component Hierarchy

```
Tier 1: Primitives (Design System)
├── Buttons, Inputs, Selects, Toggles
├── Cards, Panels, Typography
├── Feedback (Spinner, Skeleton, Progress, Toast)
└── Overlays (Modal, Tooltip, Popover)

Tier 2: Composites (App-level)
├── AppShell, ResizablePanels
├── DataTable, KeyValueList
└── Form utilities (FormSection, FieldRow)

Tier 3: Domain-Specific (Loom features)
├── Chat (ConversationView, MessageBubble, Composer)
├── Threads (ThreadList, ThreadDetail, Metadata)
├── Query (QueryTimeline, StateTrace, ToolInvocations)
└── Results (CodeBlock, DiffView, FileTree)
```

### Data Flow

```
┌─────────────────┐
│   Components    │
└────────┬────────┘
         │
         ├─→ [#server functions] → loom-server (CRUD, metadata)
         │
         └─→ [streaming service] → /api/threads/:id/stream (SSE)
                                   /proxy/anthropic/stream
                                   /proxy/openai/stream
```

## Getting Started

### 1. Verify Build

```bash
cd /home/ghuntley/loom
cargo check --workspace
```

Should succeed with `loom-web` included.

### 2. Run Development

```bash
cd crates/loom-web
cargo leptos watch
```

Visit http://localhost:3000 to see:
- Home page (landing)
- /threads (thread management)
- /styleguide (component gallery)

### 3. Explore Styleguide

The styleguide is built into the SPA:
- `/styleguide` — Overview
- `/styleguide/primitives` — Button variants (Button component already implemented)
- `/styleguide/chat` — Chat components (coming soon)
- `/styleguide/query` — Query bridge (coming soon)
- `/styleguide/results` — Code & results (coming soon)
- `/styleguide/layout` — App layout (coming soon)

### 4. Next Implementation Steps

**Immediate (this sprint):**
1. Verify `cargo check` & `cargo leptos watch` work
2. Implement remaining primitives (TextField, Select, Toggle, etc.)
3. Add primitive components to styleguide
4. Build composite layout components

**Following sprint:**
1. Thread list & detail pages
2. Chat components (ConversationView, MessageBubble)
3. Streaming integration (SSE handler)

**Future:**
1. Query bridge visualization
2. Code & diff components
3. Dark mode / theming
4. WebSocket upgrade
5. E2E tests & visual regression

## Design Patterns

### Component Pattern

```rust
#[component]
pub fn MyComponent(
    #[prop(default = SomeValue)] variant: MyVariant,
    #[prop(optional)] on_event: Option<impl Fn() + 'static>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="...">
            {children()}
        </div>
    }
}
```

**Key points:**
- Props are function parameters with `#[prop]` attributes
- Use enums for variants (e.g., `ButtonVariant`, `BadgeVariant`)
- Callbacks are optional function props
- Always support `Children`

### State Management Pattern

```rust
// Global state
provide_context(AppState { ... });

// Consume in components
let app = use_app_state();
let threads = app.threads; // Resource
let active = app.active_thread_id; // RwSignal

view! {
    <Suspense fallback=|| "Loading...">
        {move || threads.get().map(|items| { /* render */ })}
    </Suspense>
}
```

### Streaming Pattern

```rust
let (state, set_state) = create_signal(StreamingState::Idle);

let handle_submit = move |prompt| {
    set_state(StreamingState::Starting);
    start_streaming(thread_id, move |event| {
        match event {
            StreamEvent::Chunk { text, .. } => {
                set_state.update(|s| { /* append text */ });
            }
            StreamEvent::Done => set_state(StreamingState::Idle),
            StreamEvent::Error(e) => set_state(StreamingState::Error(e)),
        }
    });
};
```

## Integration with loom-server

The `loom-web` crate is integrated into `loom-server` via:

1. **SSR rendering** — `leptos_axum` handler serves `<App/>` on `/`
2. **Server functions** — `#[server]` handlers in `loom-server` endpoints
3. **Streaming endpoints** — `/api/threads/:id/stream`, `/proxy/*/stream`
4. **Static assets** — Compiled CSS, JS served from `/assets`

See `WEB_UI_ARCHITECTURE.md` section 9 for detailed setup.

## Testing

### Component Tests

```bash
cargo test --target wasm32-unknown-unknown
```

Tests are co-located in `_test.rs` files using `wasm-bindgen-test`.

### Integration Tests (E2E)

```bash
npx playwright test
```

Tests in `tests/e2e/` using Playwright.

## Performance Considerations

1. **Code splitting** — Leptos automatically code-splits by route
2. **Lazy loading** — Components can use `Suspense` for async data
3. **Streaming** — Real-time responses avoid blocking on full LLM output
4. **Hydration** — Server renders, client resumes without full re-render

## Resources & Documentation

- **Architecture**: WEB_UI_ARCHITECTURE.md (600+ lines, comprehensive)
- **Implementation**: LOOM_WEB_IMPLEMENTATION_GUIDE.md (500+ lines, step-by-step)
- **Leptos Docs**: https://leptos.dev (official book & API)
- **Tailwind Docs**: https://tailwindcss.com (utility classes)
- **Loom Architecture**: specs/architecture.md (system overview)
- **Streaming Design**: specs/streaming.md (SSE format & patterns)

## File Tree

```
crates/loom-web/
├── src/
│   ├── lib.rs
│   ├── app.rs
│   ├── main.rs
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── home.rs
│   │   ├── threads.rs
│   │   ├── workspace.rs
│   │   └── styleguide/
│   │       ├── mod.rs
│   │       ├── index.rs
│   │       ├── primitives.rs
│   │       ├── chat.rs
│   │       ├── query.rs
│   │       ├── results.rs
│   │       └── layout.rs
│   ├── components/
│   │   ├── mod.rs
│   │   ├── primitives/
│   │   │   ├── mod.rs
│   │   │   └── button.rs ✅
│   │   ├── layout/
│   │   │   ├── mod.rs
│   │   │   └── app_shell.rs ✅
│   │   ├── chat/
│   │   ├── query/
│   │   ├── results/
│   │   └── indicators/
│   ├── services/
│   │   ├── mod.rs
│   │   ├── api.rs ✅
│   │   ├── state.rs ✅
│   │   └── streaming.rs ✅
│   └── styles/
│       └── input.css ✅
├── tailwind.config.cjs ✅
├── postcss.config.cjs ✅
├── Cargo.toml ✅
├── package.json ✅
└── README.md (to be added)
```

✅ = Complete/Ready

## Success Metrics

You'll know this is working when:

1. ✅ `cargo check --workspace` passes
2. ✅ `cargo leptos watch` runs without errors
3. ✅ http://localhost:3000 shows the landing page
4. ✅ http://localhost:3000/styleguide shows component gallery
5. ✅ Button component displays all 4 variants in `/styleguide/primitives`
6. ✅ Tailwind classes apply correctly to components
7. ✅ Hot reload works (changes reflect without manual refresh)

## Next Owner Actions

1. **Review architecture** — Read WEB_UI_ARCHITECTURE.md
2. **Verify build** — Run `cargo check --workspace`
3. **Test local dev** — Run `cargo leptos watch`
4. **Implement primitives** — Start with TextField, Select, Toggle
5. **Build styleguide** — Add gallery views for new components
6. **Connect API** — Implement server functions for thread CRUD
7. **Add streaming** — Implement SSE handler in services/streaming.rs
8. **Build chat** — Create ConversationView and related components

---

**Created**: Dec 22, 2025  
**Status**: ✅ Ready for implementation  
**Next Milestone**: Phase 1 (Foundation) completion  
**Estimated Effort**: 2-3 weeks for full UI (if following phase breakdown)
