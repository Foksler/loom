# Leptos 0.7 Migration - Priority Fix Checklist

## Status: ~221 errors remaining (down from 224)

### Error Breakdown by Category
- **E0308** (123): Type mismatches - incompatible if/else, match arms, signals
- **E0277** (30): Trait bound issues - IntoAttributeValue, AttributeValue
- **E0599** (26): Missing methods - .value, .get_bounding_client_rect(), .r#ref()
- **E0618** (20): Expected function, found WriteSignal - event handler patterns
- **E0593** (11): Closure argument mismatches - event handlers taking wrong args
- **E0382** (7): Moved values - Vec/items moved in closures
- **E0283/E0282** (3): Type inference issues

## ✅ Fixes Already Applied

1. ✅ Prelude: Added `html`, `spawn_local`
2. ✅ Deprecated: `create_effect` → `Effect::new()`
3. ✅ Routing: `use_params_map`, Outlet imports
4. ✅ Conditionals: if/else → `<Show>` in conversation_view, message_bubble, message_body
5. ✅ Signals: Vec<Message> and Message converted to RwSignal
6. ✅ data_table: signal() → RwSignal::new()
7. ✅ resizable_panels: Generic bounds fixed

## 🔥 Critical Fixes Needed (Quick Wins)

### 1. Replace All if/else in view! with Show Component
**Pattern:** 123 E0308 errors mostly from if/else returning different view types
```rust
// BEFORE (wrong)
{if condition {
    view! { <Div>"Content"</Div> }
} else {
    view! {}
}}

// AFTER (correct)
<Show when=move || condition>
    <div>"Content"</div>
</Show>
```

**Files to scan:**
```bash
grep -r "view! {" crates/loom-web/src --include="*.rs" | grep -A2 "if " | head -50
```

### 2. Fix Event Handler Props
**Problem:** E0618 - WriteSignal used where function expected
**Pattern:** Event handlers can't be Option or WriteSignal directly

```rust
// BEFORE  
#[prop(optional)]
on_click: Option<impl Fn(MouseEvent)>,

// AFTER - make prop non-optional or use Signal
on_click: impl Fn(MouseEvent) + 'static,
```

**Common files:** primitives/* (button, text_field, etc.)

### 3. Move All Vec Props to Signals
**Problem:** E0382 - items/messages moved in closures
```rust
// BEFORE
items: Vec<T>,

// AFTER
items: Vec<T>,
// Then in component: let items_signal = RwSignal::new(items);
```

### 4. Fix Node Reference Usage
**Problem:** E0277, E0599 - node_ref attribute issues
```rust
// OLD (0.6)
let node_ref = create_node_ref::<html::Input>();

// NEW (0.7)
let input_ref = NodeRef::<html::Input>::new();
// In view: node_ref=input_ref (not node_ref=ref_)

// To access value:
if let Some(el) = input_ref.get() {
    let val = el.value();  // or cast to web_sys::HtmlInputElement
}
```

### 5. Fix Web-sys Element Access
**Problem:** E0599 - Methods like .value, .get_bounding_client_rect() not found
```rust
// Solution: Use wasm_bindgen casting
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlTextAreaElement, HtmlDivElement};

if let Some(el) = input_ref.get() {
    if let Ok(input) = el.unchecked_into::<HtmlInputElement>() {
        let val = input.value();
    }
}
```

## 📋 Systematic Fix Order

### Phase 1: Type Mismatches (E0308) - ~100 errors
```bash
cd crates/loom-web
cargo check 2>&1 | grep "E0308" | head -10
# For each: Replace if/else blocks with <Show>
```

**Commands to help:**
```bash
# Find all if/else in view! macros
grep -n "view! {" src/components -r | grep -B1 "if "

# Find all view! {} (empty views)
grep -n "view! {}" src/components -r
```

### Phase 2: Event Handler Closures (E0593, E0618) - ~30 errors
- Audit all component event handler props
- Ensure closures take correct number of arguments
- Remove Option wrappers or provide defaults

### Phase 3: DOM Access Issues (E0599, E0277) - ~55 errors
- Fix all node_ref attribute usage
- Add web_sys casting for element methods
- Update form value access

### Phase 4: Signal Pattern Consolidation - remaining errors
- Ensure all state uses RwSignal or proper Memo
- Fix any remaining closure captures
- Verify signal getter/setter patterns

## 🎯 High-Impact Components to Fix First

**These affect many other components:**
1. primitives/button.rs - button click handlers
2. primitives/text_field.rs - input value access
3. primitives/modal.rs - show/hide boolean state
4. layout/data_table.rs - sorting/pagination state (DONE)
5. chat/conversation_view.rs - message list (DONE)

**Then fix domain-specific:**
6. chat/* - all chat components
7. results/* - code block, file tree
8. threads/* - thread list, detail
9. layout/* - remaining layout components

## 💾 Files by Error Count (Top 10)

Run: `cargo check 2>&1 | grep "^error" | cut -d: -f1 | sort | uniq -c | sort -rn | head -20`

This will show which files have most errors.

## 🧪 Testing After Fixes

```bash
# Build check
cargo check -p loom-web

# Run tests
cargo test -p loom-web

# If web tests exist
wasm-pack test --headless --firefox crates/loom-web
```

## References for 0.6 → 0.7 Changes

Key breaking changes:
- Signal destructuring removed: `let (val, set_val) = ...` → RwSignal
- Memo requires parameter: `Memo::new(|prev| ...)`
- create_node_ref deprecated: use `NodeRef::new()`
- Node attributes: `node_ref=` instead of just reference
- Event handler signatures changed
- Show component replaces conditional views
- For component iterator syntax updated

See: https://docs.rs/leptos/0.7/leptos/

## Notes

- 0.7 has better performance and more idiomatic Rust patterns
- Typed builders for components are more strict
- Macro hygiene improvements might reveal naming issues
- Focus on compilable state, then runtime testing
