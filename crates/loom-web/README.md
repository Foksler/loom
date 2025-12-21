# Loom Web UI

A production-ready SPA for the Loom AI coding assistant, built with **Leptos** and **Tailwind CSS**.

## Features

- ✨ Server-side rendering (SSR) with client hydration
- 🎨 Comprehensive design system (primitives, composites, domain components)
- 📚 Built-in Storybook-like component gallery
- 🚀 Type-safe API integration via `#[server]` functions
- 🌊 Real-time streaming support (SSE → WebSocket)
- ♿ Accessible, semantic HTML
- 🎯 Structured logging throughout

## Quick Start

### Prerequisites

- Rust toolchain (stable + nightly)
- Node.js v16+ (for Tailwind)
- `cargo-leptos`: `cargo install cargo-leptos`

### Development

```bash
cd crates/loom-web
cargo leptos watch
```

Visit:
- **Main app**: http://localhost:3000
- **Component gallery**: http://localhost:3000/styleguide

### Production Build

```bash
cargo leptos build --release
```

Output: `target/site/` (static assets) + embedded SSR binary

## Architecture

See the parent docs:
- **WEB_UI_ARCHITECTURE.md** — Design and patterns (600+ lines)
- **LOOM_WEB_IMPLEMENTATION_GUIDE.md** — Step-by-step roadmap (500+ lines)
- **LOOM_WEB_SUMMARY.md** — Overview and getting started

## Project Structure

```
src/
├── app.rs              # Root App component with routing
├── routes/             # Page-level components
│   ├── home.rs
│   ├── threads.rs
│   ├── workspace.rs
│   └── styleguide/     # Component gallery
├── components/         # Reusable UI components
│   ├── primitives/     # Design system (buttons, inputs, etc.)
│   ├── layout/         # App structure
│   ├── chat/           # Chat/conversation
│   ├── query/          # Query bridge
│   ├── results/        # Code, diffs, files
│   └── indicators/     # Status/feedback
└── services/           # Application logic
    ├── api.rs          # Server functions
    ├── state.rs        # Global app state
    └── streaming.rs    # SSE/WebSocket
```

## Component Tiers

### Tier 1: Primitives (Design System)
Reusable, generic building blocks:
- Buttons, text inputs, selects, toggles
- Cards, panels, typography
- Spinners, skeletons, progress bars
- Modals, tooltips, popovers

### Tier 2: Composites
App-level, domain-agnostic:
- AppShell (top nav + sidebar + main)
- DataTable, KeyValueList
- Form utilities

### Tier 3: Domain-Specific
Loom-feature components:
- Chat (ConversationView, MessageBubble, Composer)
- Threads (ThreadList, ThreadDetail)
- Query (QueryTimeline, StateTrace)
- Results (CodeBlock, DiffView, FileTree)

## Styling

- **Tailwind CSS** for utilities
- **CSS variables** for design tokens
- **Component shortcuts** in `styles/input.css` (`.btn`, `.input`, etc.)
- **Responsive** breakpoints via Tailwind

Configure in `tailwind.config.cjs`:
```javascript
colors: {
  brand: { /* ... */ },
  // ...
},
spacing: {
  xs: "0.25rem",
  md: "1rem",
  // ...
}
```

## State Management

Global state via Leptos Context:

```rust
pub struct AppState {
    pub active_thread_id: RwSignal<Option<String>>,
    pub threads: Resource<(), Result<Vec<Thread>, Error>>,
    pub streaming_state: RwSignal<StreamingState>,
    pub query_settings: RwSignal<QuerySettings>,
}
```

Access via `use_app_state()` context hook in components.

## API Integration

### Server Functions (RPC)

```rust
#[server(GetThreads, "/api")]
pub async fn get_threads() -> Result<Vec<Thread>, ServerFnError> {
    // Runs on server; access DB, call backend APIs
}
```

Call from components:
```rust
let threads = create_resource(|| (), |_| async move { get_threads().await });
```

### Streaming (SSE/WebSocket)

```rust
start_streaming(thread_id, move |event| {
    match event {
        StreamEvent::Chunk { text, .. } => { /* append to buffer */ },
        StreamEvent::Done => { /* mark complete */ },
        StreamEvent::Error(e) => { /* show error */ },
    }
});
```

## Testing

### Component Tests (Wasm)

```bash
cargo test --target wasm32-unknown-unknown
```

Example:
```rust
#[wasm_bindgen_test]
fn button_renders_primary_variant() {
    let view = leptos::view! { <Button variant=ButtonVariant::Primary/> };
    // assert on HTML
}
```

### Integration Tests (E2E)

```bash
npx playwright test
```

## Development Workflow

### Adding a New Component

1. Create file: `components/category/component_name.rs`
2. Implement component with `#[component]` macro
3. Export from `components/category/mod.rs`
4. Add to styleguide gallery: `routes/styleguide/category.rs`
5. Add tests: `component_name_test.rs`

### Adding a New Page

1. Create file: `routes/page_name.rs`
2. Implement page component (probably wraps existing components)
3. Export from `routes/mod.rs`
4. Add route in `app.rs`

### Updating Tailwind

Edit `tailwind.config.cjs`, then rebuild:
```bash
npm run tailwind:build
```

## Performance

- **Code splitting** — Leptos splits by route
- **SSR** — Server renders initial HTML for fast first paint
- **Hydration** — Client takes over without full re-render
- **Streaming** — Real-time LLM responses via SSE/WebSocket

## Troubleshooting

### Leptos macro errors
```bash
rustup install nightly
rustup override set nightly
```

### Tailwind classes not applying
Ensure config includes `./src/**/*.{rs,html}`:
```javascript
content: ["./src/**/*.{rs,html}"],
```

Then rebuild CSS.

### Hydration mismatch
Ensure no randomness in initial render. Use `HydrationCtx` in tests.

## Resources

- **Leptos Book**: https://leptos.dev
- **Tailwind CSS**: https://tailwindcss.com
- **Loom Architecture**: ../../../specs/architecture.md
- **Loom Streaming**: ../../../specs/streaming.md

## License

MIT (inherited from Loom project)

---

**Status**: Phase 1 (Foundation) - Ready for implementation  
**Next Steps**: Build primitives → Composites → Domain components → API integration  
**Questions?** See `WEB_UI_ARCHITECTURE.md` or `LOOM_WEB_IMPLEMENTATION_GUIDE.md`
