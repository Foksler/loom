# Leptos 0.7 Migration Status

## Summary
Loom-web requires significant updates for Leptos 0.7 compatibility. While the `#[component]` macro and view! syntax mostly remain compatible, the signal and routing APIs have changed substantially.

## ✅ Completed Fixes

### Prelude Updates
- ✅ Added `leptos::html` to prelude for NodeRef support
- ✅ Added `leptos::task::spawn_local` for async operations

### Deprecated Function Replacements
- ✅ `create_effect` → `Effect::new()` (conversation_view.rs)
- ⚠️ `create_signal` → `signal()` (deprecated but functional)
- ⚠️ `create_rw_signal` → `RwSignal::new()` (needs rollout across codebase)
- ⚠️ `create_memo` → `Memo::new()` (needs parameter change)

### Router Updates
- ✅ Fixed imports: `leptos_router::hooks::use_params_map`
- ✅ Added Outlet component import

### View! Macro Conditionals
- ✅ Replaced `if/else` blocks with `<Show>` component (conversation_view.rs, message_bubble.rs, message_body.rs)
- ✅ Fixed incompatible view! types in conditional renders

### Signal Conversion
- ✅ Converted Vec<Message> to RwSignal in ConversationView
- ✅ Converted Message to RwSignal in MessageBubble
- ✅ Updated signal getter calls (.get())

## 🔴 Critical Remaining Issues

### 1. Signal API Pattern (~50+ instances)
**Problem:** `signal()` vs `RwSignal::new()` inconsistency
```rust
// Old (0.6)
let (read, write) = create_signal(value);

// New (0.7)
let signal = RwSignal::new(value);  // for reactive updates
let signal = Memo::new(|| value);   // for derived values
```
**Files affected:**
- data_table.rs (WriteSignal usage as function - needs refactoring)
- All components with event handlers storing state
- resizable_panels.rs (closure tracking state)

### 2. Event Handler Props API
**Problem:** Props for event handlers have changed
```rust
// Old: on_click: Option<impl Fn(MouseEvent)>
// New: on_click: impl Fn(MouseEvent) + 'static
```
**Symptoms:**
- `expected function, found WriteSignal<bool>`
- `expected 1 argument, but takes 0`
**Files:**
- primitives/ (form inputs, buttons)
- All interactive components

### 3. Node Reference API
**Problem:** `node_ref` attribute syntax changed
```rust
// Old: node_ref=ref_
// New: node_ref=node_ref
```
**Issues:**
- `no method named 'r#ref'`
- `trait bound IntoAttributeValue not satisfied`
**Files:**
- resizable_panels.rs
- form components

### 4. HTML Element Methods
**Problem:** Some web-sys methods not exposed via leptos::html
```rust
// Available in web_sys but not in leptos::html element wrapper
el.get_bounding_client_rect()
el.value  // for input elements
```
**Files:**
- resizable_panels.rs
- text_field.rs, textarea.rs

### 5. Breadcrumbs Component
**Problem:** Struct field mismatch
```rust
#[derive(Clone)]
pub struct BreadcrumbItem {
    // has no 'on_click' field
}
```
**Fix needed:** Remove on_click assignments or add field

### 6. Conditional Rendering Type Mismatches
**Remaining issues:** Still have some `if/else` returning incompatible view types
**Files:**
- data_table.rs
- Several layout components

## 🔧 Migration Path

### Phase 1: Signal API (HIGH PRIORITY)
1. Replace all `create_signal` → `signal()`
2. Replace all `create_rw_signal` → `RwSignal::new()`
3. Update all signal destructuring patterns
4. Convert write signal callbacks to RwSignal.set() pattern

### Phase 2: Event Handlers
1. Audit all event handler Props 
2. Remove Option wrappers for required handlers
3. Update handler function signatures

### Phase 3: DOM Bindings
1. Fix node_ref attribute syntax
2. Use explicit web_sys casts for element methods
3. Update form value access patterns

### Phase 4: Final Type Fixes
1. Replace remaining if/else conditionals with Show
2. Fix match arm type mismatches
3. Verify all closures capture correctly

## 📋 Files Requiring Major Work

High Priority (blocking build):
- [ ] crates/loom-web/src/components/layout/data_table.rs (7+ errors)
- [ ] crates/loom-web/src/components/primitives/ (10+ errors across files)
- [ ] crates/loom-web/src/components/layout/resizable_panels.rs (5+ errors)
- [ ] crates/loom-web/src/routes/styleguide/ (nested routing)

Medium Priority:
- [ ] All form components (text_field, textarea, etc.)
- [ ] Interactive components (modal, popover, etc.)
- [ ] Results/query components

## 💡 Key API Changes

### Signal Creation
```rust
// Before
let (value, set_value) = create_signal(initial);
let (value, set_value) = create_rw_signal(initial);

// After
let signal = RwSignal::new(initial);
let signal = signal(initial);  // shorthand
```

### Signal Usage
```rust
// Before
value()        // in view macros
set_value(new_val)

// After  
signal.get()   // read
signal.set(new_val)  // write
```

### Node References
```rust
// Before
let node_ref = create_node_ref::<html::Input>();

// After
let node_ref = NodeRef::<html::Input>::new();
// Use in view!: node_ref=node_ref
```

### Show Component
```rust
// Before
{if condition { view! { ... } } else { view! {} }}

// After
<Show when=move || condition>
  ...
</Show>
```

## Testing Notes
- Once compiled, test routing (params extraction)
- Verify form inputs capture and update values
- Check event handler firing
- Test Show/Suspense component rendering

## References
- https://docs.rs/leptos/0.7/leptos/
- Leptos Changelog: 0.6 → 0.7 breaking changes
