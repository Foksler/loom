# Leptos 0.7 Migration Guide for loom-web

**Target**: Get loom-web compiling with Leptos 0.7.8  
**Approach**: Systematic API updates without architectural changes  
**Estimated Time**: 4-6 hours  
**Difficulty**: Moderate (mostly straightforward replacements)

## Overview of Changes

Leptos 0.7 made significant API changes to improve ergonomics and follow Rust naming conventions. The changes are mostly 1:1 replacements.

## Migration Checklist

### 1. Signal Creation (Priority: CRITICAL)

**Old API:**
```rust
use leptos::*;
let (value, set_value) = create_signal(initial);
let memo = create_memo(|| computed_value);
let effect = create_effect(|| side_effect());
```

**New API:**
```rust
use leptos::prelude::*;
let (value, set_value) = create_signal(initial);  // Still works with deprecation warning
// OR (recommended):
let value = Signal::new(initial);
let memo = Memo::new(|| computed_value);
let effect = Effect::new(|| side_effect());
```

**Migration Steps for loom-web:**

1. Update imports in all files:
   ```rust
   // Replace: use leptos::*;
   // With: use crate::prelude::*;
   ```

2. In `src/prelude.rs`, ensure these are exported:
   ```rust
   pub use leptos::prelude::*;
   pub use leptos::{component, view, IntoView};
   ```

3. Replace deprecated signal functions:
   - `create_signal(x)` → Keep as-is (works with warning) OR use `Signal::new(x)`
   - `create_rw_signal(x)` → Replace with `RwSignal::new(x)`
   - `create_memo(f)` → Replace with `Memo::new(f)`
   - `create_effect(f)` → Replace with `Effect::new(f)`

**Files to update:**
- `src/services/state.rs` - Uses `create_rw_signal`
- `src/components/**/*.rs` - Many use `create_signal`, `create_memo`

### 2. Resource Creation (Priority: CRITICAL)

**Old API:**
```rust
use leptos::*;
let resource = create_resource(
    || (),
    |_| async { fetch_data().await }
);

// In views:
{move || resource.get().map(|data| view! { ... })}
```

**New API in Leptos 0.7:**
Resource has been replaced with async components or `async_derive`.

**Migration Options:**

**Option A: Use Async Component (Recommended)**
```rust
#[component]
async fn DataComponent() -> impl IntoView {
    let data = fetch_data().await;
    view! { ... }
}
```

**Option B: Use Signal + spawn_local**
```rust
use leptos::prelude::*;

let (data, set_data) = create_signal(None);

// On mount effect:
Effect::new(|| {
    spawn_local(async {
        let result = fetch_data().await;
        set_data(Some(result));
    });
});

view! {
    {move || data.get().map(|d| view! { ... })}
}
```

**Files to update:**
- `src/routes/threads/list.rs` - Line 28: `create_resource`
- `src/routes/threads/detail.rs` - Line 38: `create_resource`
- `src/services/state.rs` - Line 166: `create_resource`

**Recommended approach for these files:**
Remove `create_resource` usage entirely and simplify to use the app state directly.

### 3. Children Type (Priority: HIGH)

**Old API:**
```rust
use leptos::*;

#[component]
pub fn MyComponent(children: Children) -> impl IntoView {
    view! { {children()} }
}
```

**New API:**
```rust
use leptos::prelude::*;

#[component]
pub fn MyComponent(children: Children) -> impl IntoView {
    view! { {children()} }
}
```

**Status**: The new prelude exports `Children` directly, so existing code should work.

**Files to verify:**
- `src/app.rs` - `AppStateProvider` component
- `src/components/layout/app_shell.rs` - `AppShell` component
- Any other components taking `children`

### 4. Router API (Priority: HIGH)

**Old API:**
```rust
use leptos_router::*;

#[derive(Params)]
pub struct MyParams {
    pub id: String,
}

let params = use_params::<MyParams>();
let navigate = use_navigate();
let outlet = <Outlet />;
```

**New API (Leptos 0.7):**
```rust
use leptos_router::params::Params;  // Derive macro location may differ
use leptos_router::*;

#[derive(Params, PartialEq, Clone, Debug)]
pub struct MyParams {
    pub id: String,
}

let params = use_params::<MyParams>();  // Signature unchanged
let navigate = use_navigate();
let outlet = <Outlet />;  // Should still work
```

**Current Status in loom-web:**
- `src/routes/threads/detail.rs` uses `#[derive(Params)]` - verify it compiles
- `use_params()` usage looks correct
- `use_navigate()` usage looks correct
- `Outlet` from `leptos_router` - verify import

**Action needed:**
1. Verify `Params` derive comes from correct location
2. Check if `Outlet` needs to be imported from `leptos_router::components`

### 5. View Type Generics (Priority: MEDIUM)

**Old API:**
```rust
fn render_item<T: Fn() -> View + 'static>(f: T) -> View {
    // ...
}
```

**New API:**
```rust
use leptos::prelude::*;

fn render_item<T: Fn() -> View<impl IntoView> + 'static>(f: T) -> View<impl IntoView> {
    // ...
}

// OR simpler:
fn render_item<T: Fn() -> impl IntoView + 'static>(f: T) -> impl IntoView {
    // ...
}
```

**Files affected:**
- `src/components/layout/resizable_panels.rs` - Line 61-62: `View` type parameters
  ```rust
  // Old:
  LF: Fn() -> View + 'static,
  RF: Fn() -> View + 'static,
  
  // New:
  LF: Fn() -> impl IntoView + 'static,
  RF: Fn() -> impl IntoView + 'static,
  ```

### 6. HTML Elements & Events (Priority: MEDIUM)

**Issue**: HTML element types and event handlers may have moved.

**Old API:**
```rust
use leptos::*;
use web_sys::HtmlInputElement;

let input = create_node_ref::<HtmlInputElement>();
let value = input.get().map(|el| el.value());

// Event handlers:
on:input=move |ev| {
    let val = event_target_value(&ev);
}
```

**New API:**
```rust
use leptos::prelude::*;
use leptos::html::Input;  // HTML element types location may differ
use web_sys::HtmlInputElement;

let input: NodeRef<Input> = NodeRef::new();
let value = input.get().map(|el| el.value());

// Event handlers:
on:input=move |ev| {
    let val = event_target_value(&ev);  // May need proper import
}
```

**Current issues in loom-web:**
- `web_sys::File` not found - check if it needs to be imported differently
- `event_target_value` function - verify it's exported from prelude
- `html::Input`, etc. types - may need qualified paths

### 7. Spawn Local & Async (Priority: MEDIUM)

**Current status**: `spawn_local` should work with proper imports.

**Needed imports:**
```rust
use leptos::prelude::*;
// spawn_local should be available
```

**Check locations:**
- `src/routes/threads/list.rs` - May need `spawn_local`
- `src/routes/threads/detail.rs` - May need `spawn_local`

## Step-by-Step Migration

### Step 1: Update Prelude (DONE ✅)
```bash
# File: src/prelude.rs
# Already created with basic exports
# May need additions based on compilation errors
```

### Step 2: Update All File Imports (DONE ✅)
```bash
# Replace all: use leptos::*;
# With:       use crate::prelude::*;
# Already applied with sed
```

### Step 3: Fix Signal Creation APIs

**Priority files:**
1. `src/services/state.rs`
   - Line 165: `let threads = create_rw_signal(None);` ✅ Already done
   - Line 168-171: Remove `create_rw_signal` calls (already done)

2. Check remaining files for:
   - `create_signal` - Keep as-is (deprecated but works)
   - `create_memo` - Replace with `Memo::new` OR keep (works)
   - `create_effect` - Replace with `Effect::new` OR keep (works)

### Step 4: Fix Resource APIs

**Files:**
1. `src/routes/threads/list.rs`
   - Refactor to use signal-based approach ✅ Already done

2. `src/routes/threads/detail.rs`
   - Refactor to use signal-based approach ✅ Already done

3. `src/services/state.rs`
   - Simplify to not use Resource type ✅ Already done

### Step 5: Fix Router APIs

**Files:**
1. `src/routes/threads/detail.rs`
   - Verify `Params` derive
   - Check `use_params` import
   - Check `use_navigate` import

2. `src/routes/styleguide/mod.rs`
   - Verify `Outlet` import

3. `src/app.rs`
   - Verify router setup

### Step 6: Fix Type Issues

**Files:**
1. `src/components/layout/resizable_panels.rs`
   - Lines 61-62: Replace `View` with `impl IntoView`
   - Fix generic parameters

### Step 7: Fix Web APIs & Events

**Files to check:**
1. `src/components/primitives/file_input.rs`
   - Verify `web_sys::File` import

2. Files using `event_target_value`:
   - `src/components/chat/prompt_composer.rs` (line 84)
   - Any others using event handlers

3. Files using `create_node_ref`:
   - Verify `NodeRef` is properly imported
   - Check type parameter syntax

### Step 8: Verify & Compile

```bash
# After each change group:
cargo check -p loom-web

# Final check:
cargo check -p loom-web
cargo fmt -p loom-web  
cargo clippy -p loom-web -- -D warnings
cargo test -p loom-web
```

## Specific File Changes Needed

### `src/components/layout/resizable_panels.rs`

**Current (Lines 61-62):**
```rust
LF: Fn() -> View + 'static,
RF: Fn() -> View + 'static,
```

**Change to:**
```rust
LF: Fn() -> impl IntoView + 'static,
RF: Fn() -> impl IntoView + 'static,
```

### `src/prelude.rs` (Needs Completion)

**Current:**
```rust
pub use leptos::prelude::*;
pub use leptos::{component, view, IntoView};
pub use leptos_router::Outlet;
pub use web_sys::File;
```

**Needs (probably):**
```rust
pub use leptos::prelude::*;
pub use leptos::{
    component, view, IntoView, RwSignal, Signal, Memo, Effect,
    create_signal, create_rw_signal, create_memo, create_effect,
    create_node_ref, NodeRef, spawn_local, provide_context, use_context,
    Children, html, ev,
};
pub use leptos_router::{Outlet, use_params, use_navigate, Params};
pub use leptos::html::*;
pub use leptos::ev::*;
```

## Testing the Migration

After migration:

```bash
# Check compilation
cargo check -p loom-web

# Check formatting
cargo fmt -p loom-web --check

# Check for warnings (should have 0)
cargo clippy -p loom-web -- -D warnings

# Run tests
cargo test -p loom-web

# Build release
cargo build -p loom-web --release
```

## Common Issues & Solutions

### Issue: "cannot find function `create_signal`"
**Solution**: Ensure `src/prelude.rs` exports it, and files import from prelude

### Issue: "cannot find type `Children`"
**Solution**: Export from prelude, ensure `leptos::prelude::*` is in scope

### Issue: "expected 1 generic argument" for `View`
**Solution**: Replace `View` with `impl IntoView` or `View<T>`

### Issue: "`Params` not found as trait"
**Solution**: Import from `leptos_router` as `#[derive(Params)]`

### Issue: "`spawn_local` not found"
**Solution**: Add to prelude exports from `leptos::prelude`

## Rollout Plan

1. **Phase 1**: Fix core API issues (Signal, RwSignal, etc.)
2. **Phase 2**: Fix router and async APIs
3. **Phase 3**: Fix type system issues
4. **Phase 4**: Final compilation and testing
5. **Phase 5**: Documentation and deployment

## Success Criteria

- [ ] `cargo check -p loom-web` → 0 errors
- [ ] `cargo fmt -p loom-web` → passes
- [ ] `cargo clippy -p loom-web -- -D warnings` → 0 warnings
- [ ] `cargo test -p loom-web` → All tests pass
- [ ] All 50+ components are accessible in styleguide
- [ ] All 5 routes are navigable
- [ ] No console errors in browser

## References

- [Leptos 0.7 Release Notes](https://github.com/leptos-rs/leptos/releases/tag/0.7.0)
- [Leptos Book - Signals](https://book.leptos.dev/15_global_state.html)
- [Leptos Book - Async Data](https://book.leptos.dev/15_async.html)
- [Leptos Book - Routing](https://book.leptos.dev/17_router.html)

## Time Estimate

- Signal API updates: 1-2 hours
- Resource/async refactoring: 1-2 hours  
- Router API fixes: 30-45 minutes
- Type system fixes: 30-45 minutes
- Testing & validation: 1-2 hours
- **Total: 4-6 hours**

---

*This guide was created as part of the Loom Web integration verification process.*
