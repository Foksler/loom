#!/bin/bash
# Quick fixes for Leptos 0.7 migration
# Run from repo root: bash LEPTOS_0_7_QUICK_FIX.sh

CRATE_PATH="crates/loom-web/src/components"

echo "===== LEPTOS 0.7 QUICK FIX GUIDE ====="
echo ""

echo "1. Find all if/else blocks in view! macros (E0308 errors)"
echo "   These should be replaced with <Show> components"
echo ""
grep -r "view! {" "$CRATE_PATH" --include="*.rs" -A 2 | grep "if " | head -20
echo ""

echo "2. Find all empty view! {} (incomplete conditionals)"
grep -rn "view! {}" "$CRATE_PATH" --include="*.rs" | head -10
echo ""

echo "3. Find handler patterns that might be wrong (E0618 errors)"
echo "   Looking for on_click, on_change, on_input patterns:"
grep -rn "on_\(click\|change\|input\|blur\|focus\)" "$CRATE_PATH" --include="*.rs" | head -15
echo ""

echo "4. Find element .value access (E0599 errors)"
grep -rn "\.value" "$CRATE_PATH" --include="*.rs" | head -10
echo ""

echo "5. Find node_ref usage (E0277 errors)"
grep -rn "node_ref=" "$CRATE_PATH" --include="*.rs" | head -10
echo ""

echo "===== Manual Fixes Required ====="
echo ""
echo "PRIORITY 1: Replace if/else in view! with Show component"
echo "  Pattern: {if COND { view! { ... } } else { view! {} }}"
echo "  Replace: <Show when=move || COND>...</Show>"
echo ""
echo "  Files with most issues:"
cd "$CRATE_PATH" && find . -name "*.rs" -exec sh -c '
  COUNT=$(grep -c "if.*view!" "$1" 2>/dev/null || echo 0)
  if [ "$COUNT" -gt 0 ]; then
    echo "    $1: $COUNT occurrences"
  fi
' _ {} \; | sort -t: -k2 -rn | head -10
echo ""

echo "PRIORITY 2: Fix event handler closures"
echo "  Replace: on_click=handler_signal"
echo "  With:    on_click=move |_| state.set(new_val)"
echo ""

echo "PRIORITY 3: Fix DOM element access"
echo "  Add casting for .value, .get_bounding_client_rect()"
echo "  Pattern: el.unchecked_into::<HtmlInputElement>().value()"
echo ""

echo "PRIORITY 4: Wrap Vec props in RwSignal"
echo "  Pattern: let items_signal = RwSignal::new(items);"
echo "  Usage:   items_signal.get() in closures"
echo ""

echo "To run full check: cargo check -p loom-web"
echo "Error count: $(cargo check -p loom-web 2>&1 | grep -c "^error")"
