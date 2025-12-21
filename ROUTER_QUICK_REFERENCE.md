# Leptos 0.7 Router API Quick Reference

## Fixed Issues Summary

| Issue | Status | Files |
|-------|--------|-------|
| `use_params_map()` usage | ✓ Fixed | threads/detail.rs |
| `use_navigate()` import | ✓ Fixed | threads/list.rs |
| `Outlet` component import | ✓ Fixed | styleguide/mod.rs |
| `ParentRoute` nested routes | ✓ Fixed | app.rs |

## Code Patterns - Leptos 0.7

### Route Parameters (No Typed Params)
```rust
// ✓ Use untyped params map
use leptos_router::hooks::use_params_map;

let params = use_params_map();
let thread_id = Memo::new(move || params.with(|p| p.get("id").cloned()));
```

### Navigation
```rust
// ✓ use_navigate() takes no context
use leptos_router::hooks::use_navigate;

let navigate = use_navigate();
navigate(&format!("/threads/{}", id), Default::default());
```

### Nested Routes - Parent Layout
```rust
// ✓ Use ParentRoute for parent layouts
<ParentRoute path=path!("/styleguide") view=StyleguideLayout>
    <Route path=path!("") view=StyleguideIndexPage/>
    <Route path=path!("primitives") view=StyleguidePrimitivesPage/>
</ParentRoute>
```

### Outlet in Parent Component
```rust
// ✓ Outlet from nested_router submodule
use leptos_router::nested_router::Outlet;

#[component]
pub fn StyleguideLayout() -> impl IntoView {
    view! {
        <div class="flex h-screen">
            <Outlet/>
        </div>
    }
}
```

## Import Locations

```rust
// Route components
use leptos_router::components::{ParentRoute, Route, Router, Routes};

// Navigation hooks
use leptos_router::hooks::{use_navigate, use_params_map, use_location};

// Outlet component
use leptos_router::nested_router::Outlet;
```

## No-Go Patterns (0.6 Style)

```rust
// ❌ DON'T - use_params with context (0.6 style)
let params = use_params::<ThreadParams>(cx);

// ❌ DON'T - navigate with context (0.6 style)  
navigate(cx, "/path");

// ❌ DON'T - Outlet from root (0.6 style)
use leptos_router::Outlet;

// ❌ DON'T - Nested Route with children syntax
<Route path="styleguide">
    <Route path="primitives" />
</Route>
```

## Verification Commands

```bash
# Check for old patterns
grep -r "use_params::<" crates/loom-web/src
grep -r "navigate(cx" crates/loom-web/src

# Check new patterns
grep -r "use_params_map()" crates/loom-web/src
grep -r "use_navigate()" crates/loom-web/src
```

## Related Issues Fixed
- Router API incompatibility with Leptos 0.7
- Navigation hook signatures
- Outlet component location
- Nested route definitions
