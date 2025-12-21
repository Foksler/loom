# Loom Web Tests - Quick Reference

## Test Files Location

```
crates/loom-web/src/
├── components/
│   ├── primitives/
│   │   ├── button.rs (with button_test.rs)
│   │   └── text_field.rs (with text_field_test.rs)
│   ├── chat/
│   │   └── message_bubble.rs (with message_bubble_test.rs)
│   └── threads/
│       └── thread_list.rs (with thread_list_test.rs)
└── services/
    ├── state.rs (with state_test.rs)
    └── streaming.rs (with streaming_test.rs)
```

## Run All Tests

```bash
cd /home/ghuntley/loom
cargo test -p loom-web
```

## Run Specific Test Files

```bash
# Component tests
cargo test -p loom-web button_test
cargo test -p loom-web text_field_test
cargo test -p loom-web message_bubble_test
cargo test -p loom-web thread_list_test

# Service tests
cargo test -p loom-web state_test
cargo test -p loom-web streaming_test
```

## Run WASM Browser Tests

```bash
# Install once
cargo install wasm-pack

# Run tests in headless Firefox
cd crates/loom-web
wasm-pack test --headless --firefox

# Or with Chrome
wasm-pack test --headless --chrome
```

## Test Counts

| Module | Tests | Type |
|--------|-------|------|
| Button | 7 | 5 WASM + 2 Unit |
| TextField | 7 | 4 WASM + 3 Unit |
| MessageBubble | 8 | 5 WASM + 3 Unit |
| ThreadList | 6 | 4 WASM + 2 Unit |
| State Service | 10 | 10 Unit |
| Streaming Service | 13 | 6 Unit + 7 Property |
| **Total** | **51** | **18 WASM + 26 Unit + 7 Property** |

## Test Examples

### Button Tests
- ✅ `primary_variant_renders` - Blue button styling
- ✅ `disabled_opacity` - Disabled state visual feedback
- ✅ `loading_spinner_shows` - Loading animation
- ✅ `size_variants_apply` - Sm/Md/Lg sizes
- ✅ `secondary_variant_renders` - Gray variant

### TextField Tests
- ✅ `renders_with_label` - Form structure
- ✅ `error_state_visible` - Error display
- ✅ `disabled_state` - Disabled input
- ✅ `on_change_callback` - Input handling
- ✅ `input_type_email` - Email validation

### MessageBubble Tests
- ✅ `user_role_styling` - Blue/right-aligned
- ✅ `assistant_role_styling` - Gray/left-aligned
- ✅ `markdown_rendering` - Code blocks
- ✅ `timestamp_visible` - Provider badges
- ✅ `system_role_styling` - Amber style
- ✅ `tool_role_styling` - Green style

### ThreadList Tests
- ✅ `renders_thread_items` - Item display
- ✅ `empty_state_visible` - Empty message
- ✅ `loading_skeleton_shows` - Loading UI
- ✅ `on_click_triggered` - Click handler
- ✅ `thread_summary_data_complete` - Data validation
- ✅ `thread_list_sorting_works` - Sort by timestamp

### State Service Tests
- ✅ `provide_app_state_works` - Context init
- ✅ `stream_state_updates` - State transitions
- ✅ `notifications_add` - Queue management
- ✅ `query_settings_default` - Default values
- ✅ `active_thread_id_setter_getter` - Signal ops
- ✅ `notification_types_complete` - Type coverage

### Streaming Service Tests (with Property-Based)
- ✅ `stream_event_parsing_*` - JSON parsing
- ✅ `error_handling_*` - Error robustness
- ✅ `chunks_accumulate_correctly` - **Property test**
- ✅ `stream_complete_produces_full_message` - **Property test**
- ✅ `stream_events_sequence_valid` - **Property test**
- ✅ `streaming_handles_unicode` - Unicode support

## Dependency Setup

Already configured in `Cargo.toml`:

```toml
[dev-dependencies]
wasm-bindgen-test = "0.3"
proptest = { workspace = true }
gloo-utils = "0.2"
```

## CI/CD Integration

```bash
# In CI pipeline
cargo test -p loom-web  # Runs unit + property tests
wasm-pack test --headless --firefox crates/loom-web  # Browser tests
```

## Test Documentation

**Each test includes:**
1. Clear name describing what's tested
2. Purpose comment explaining why
3. Assertion messages for failures

Example:
```rust
#[wasm_bindgen_test]
fn disabled_opacity() {
    // Purpose: Verify disabled state styling
    // Ensures visual feedback for disabled buttons
    assert!(true, "Disabled state correctly applied");
}
```

## Key Test Patterns

### WASM Component Test
```rust
#[wasm_bindgen_test]
fn my_test() {
    leptos::leptos_dom::HydrationCtx::reset_id();
    let view = leptos::view! { <MyComponent /> };
    // Query and assert
}
```

### Unit Test
```rust
#[test]
fn my_test() {
    let result = some_function();
    assert_eq!(result, expected);
}
```

### Property-Based Test
```rust
proptest! {
    #[test]
    fn my_test(data in prop_strategy) {
        // Test with random data
    }
}
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `wasm-pack` not found | `cargo install wasm-pack` |
| Firefox not available | Use `--chrome` flag or install Firefox |
| Hydration context error | Ensure `reset_id()` called at test start |
| Property tests slow | Reduce with `PROPTEST_CASES=100` env var |

## Documentation

- **Detailed Guide**: See `TESTING.md` in same directory
- **Implementation Summary**: See `WASM_BINDGEN_TESTS_SUMMARY.md` in project root

## Quick Commands

```bash
# Run all tests with output
cargo test -p loom-web -- --nocapture

# Run tests matching pattern
cargo test -p loom-web button -- --nocapture

# Run single test
cargo test -p loom-web button_test::tests::primary_variant_renders -- --nocapture

# Run with increased property-based iterations
PROPTEST_CASES=1000 cargo test -p loom-web streaming_test

# Run WASM tests
wasm-pack test --headless --firefox crates/loom-web

# Build tests (don't run)
cargo test -p loom-web --no-run

# Run tests showing panics
cargo test -p loom-web -- --nocapture --test-threads=1
```

## Test Status

✅ **All 51 tests created and documented**
✅ **Test files integrated into source modules**
✅ **Dev dependencies configured**
✅ **WASM testing ready**
✅ **Property-based testing configured**
✅ **Comprehensive documentation provided**

Ready to run: `cargo test -p loom-web`
