# Loom Web - Testing Guide

This document provides comprehensive instructions for running tests in the loom-web crate, including both traditional unit tests and WebAssembly-specific browser tests.

## Test Organization

Tests are organized by component/module and colocated with source code using the `_test.rs` pattern:

### Component Tests (WASM Browser Tests)

These tests use `wasm-bindgen-test` to test components in a browser environment:

- **Button Component** - `src/components/primitives/button_test.rs`
  - Tests: primary variant rendering, disabled state, loading spinner, size variants
  - Coverage: 7 tests (5 WASM + 2 unit)

- **TextField Component** - `src/components/primitives/text_field_test.rs`
  - Tests: label rendering, error state display, disabled state, input callbacks, email type
  - Coverage: 7 tests (4 WASM + 3 unit)

- **MessageBubble Component** - `src/components/chat/message_bubble_test.rs`
  - Tests: user/assistant/system/tool role styling, markdown rendering, provider info
  - Coverage: 8 tests (5 WASM + 3 unit)

- **ThreadList Component** - `src/components/threads/thread_list_test.rs`
  - Tests: thread item rendering, empty state, loading skeleton, click callbacks
  - Coverage: 6 tests (4 WASM + 2 unit)

### Service Tests (Unit + Property-Based)

These tests verify service logic and streaming behavior:

- **State Service** - `src/services/state_test.rs`
  - Tests: AppState initialization, context usage, streaming state transitions, notifications
  - Coverage: 10 tests (traditional unit tests)
  - Key tests:
    - State context provider validation
    - StreamingState machine transitions (Idle → Starting → Streaming → Idle)
    - Notification queue management and uniqueness

- **Streaming Service** - `src/services/streaming_test.rs`
  - Tests: event parsing, error handling, chunk accumulation (property-based)
  - Coverage: 13 tests (6 unit + 7 property-based)
  - Key tests:
    - Text chunk parsing and accumulation
    - Tool call delta event handling
    - Completion event processing
    - Unicode content preservation
    - Property-based tests with proptest for robustness

## Running Tests

### Run All Tests

```bash
# Run all tests (default to native/unit tests)
cargo test -p loom-web

# Run with detailed output
cargo test -p loom-web -- --nocapture
```

### Run Tests for Specific Module

```bash
# Test state service
cargo test -p loom-web state_test -- --nocapture

# Test streaming service
cargo test -p loom-web streaming_test -- --nocapture
```

### Run WASM Browser Tests

WASM tests require additional setup and must be compiled to WebAssembly:

```bash
# Install wasm-pack (one-time)
cargo install wasm-pack

# Run WASM tests in browser (requires headless browser)
cd crates/loom-web
wasm-pack test --headless --firefox

# Or with Chrome
wasm-pack test --headless --chrome
```

**Note:** Browser testing requires Firefox or Chrome to be installed. For CI environments, headless browsers are recommended.

### Run Property-Based Tests

Property-based tests use `proptest` to validate behavior across many random inputs:

```bash
# Run property-based tests with verbose output
cargo test -p loom-web streaming_test::tests::chunks_accumulate_correctly -- --nocapture

# Property tests run 256 cases by default (configurable via PROPTEST_CASES env var)
PROPTEST_CASES=1000 cargo test -p loom-web streaming_test -- --nocapture
```

## Test Coverage Metrics

### Total Tests: 51

| Category | Count | Type |
|----------|-------|------|
| Button Component | 7 | WASM + Unit |
| TextField Component | 7 | WASM + Unit |
| MessageBubble Component | 8 | WASM + Unit |
| ThreadList Component | 6 | WASM + Unit |
| State Service | 10 | Unit |
| Streaming Service | 13 | Unit + Property-Based |
| **Total** | **51** | |

### Coverage by Test Type

- **WASM Browser Tests**: 18 tests (component rendering, DOM interaction)
- **Unit Tests**: 24 tests (logic validation, state transitions)
- **Property-Based Tests**: 7 tests (streaming accumulation, event sequences)
- **Integration-Style Tests**: 2 tests (context provider patterns)

## Test Patterns

### WASM Component Tests

```rust
#[wasm_bindgen_test]
fn component_renders_correctly() {
    // Reset Leptos hydration context
    leptos::leptos_dom::HydrationCtx::reset_id();

    // Create the component
    let view = leptos::view! {
        <MyComponent prop="value" />
    };

    // Query DOM
    let container = gloo_utils::window()
        .document()
        .expect("window.document exists")
        .create_element("div")
        .expect("create_element succeeds");

    // Assert structure
    let element = container.query_selector("button");
    assert!(element.is_ok(), "Element should render");
}
```

### Unit Tests

```rust
#[test]
fn validation_logic_works() {
    // Purpose: Verify specific logic without browser
    // Non-WASM tests for pure Rust logic
    
    let result = some_function();
    assert_eq!(result, expected_value, "Assertion message");
}
```

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn accumulation_works(
        chunks in proptest::collection::vec("[a-z]+", 1..10)
    ) {
        // Test accumulates chunks correctly for ANY valid input sequence
        let result = accumulate(chunks);
        assert!(!result.is_empty());
    }
}
```

## Dependencies

Test dependencies in `Cargo.toml`:

```toml
[dev-dependencies]
wasm-bindgen-test = "0.3"  # WASM browser testing
proptest = "1.0"            # Property-based testing
gloo-utils = "0.2"          # Browser utilities for tests
```

## Test Documentation

Each test includes three documentation layers:

1. **Test Name**: Descriptive function name (e.g., `primary_variant_renders`)
2. **Purpose Comment**: Why the test exists and what it validates
3. **Assertion Messages**: Clear failure messages for debugging

Example:

```rust
#[wasm_bindgen_test]
fn disabled_opacity() {
    // Purpose: Verify that disabled state applies opacity reduction
    // Ensures disabled buttons are visually distinct and non-interactive
    
    // ... setup code ...
    
    assert!(
        button.is_ok(),
        "Disabled button should still render in DOM"
    );
}
```

## Continuous Integration

For CI/CD pipelines:

```bash
# Run all unit tests
make test

# Run linting and tests
make check

# Full validation
make check && wasm-pack test --headless --firefox crates/loom-web
```

## Troubleshooting

### WASM Test Setup Issues

**Issue**: `wasm-pack` not found
```bash
# Install wasm-pack
cargo install wasm-pack
```

**Issue**: WebDriver errors in CI
```bash
# Firefox already running
killall firefox
# Try headless Chrome instead
wasm-pack test --headless --chrome
```

### Test Failures

**Leptos Hydration Context Issues**
- Ensure `leptos::leptos_dom::HydrationCtx::reset_id()` is called first
- This resets internal Leptos state between tests

**Missing Mock Data**
- Component tests create minimal mock objects
- For real data testing, add integration tests

## Adding New Tests

When adding new component tests:

1. Create `{component}_test.rs` in same directory
2. Add `#[path = "{component}_test.rs"] mod {component}_test;` to component file
3. Document purpose and assertions
4. Follow established patterns (WASM vs Unit)

Example:

```rust
#[cfg(test)]
#[path = "my_component_test.rs"]
mod my_component_test;
```

Then in `my_component_test.rs`:

```rust
#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    fn my_component_renders() {
        // Purpose: ...
        // Test code...
    }
}
```

## Future Improvements

- [ ] Screenshot testing for visual regression
- [ ] End-to-end tests with Playwright
- [ ] Performance benchmarks with criterion
- [ ] Coverage reporting with tarpaulin
- [ ] Snapshot testing for component output

## References

- [wasm-bindgen-test Documentation](https://docs.rs/wasm-bindgen-test/)
- [Proptest Book](https://docs.rs/proptest/1.0.0/proptest/)
- [Leptos Testing Guide](https://leptos.dev/01_getting_started.html)
- [wasm-pack Documentation](https://rustwasm.org/docs/wasm-pack/)
