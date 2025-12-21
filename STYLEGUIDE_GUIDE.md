# Styleguide Navigation & Best Practices

**How to use the interactive component styleguide at `/styleguide`**

---

## Table of Contents

1. [What is the Styleguide?](#what-is-the-styleguide)
2. [Accessing the Styleguide](#accessing-the-styleguide)
3. [Navigation](#navigation)
4. [Component Sections](#component-sections)
5. [Adding Examples](#adding-examples)
6. [Best Practices](#best-practices)
7. [Visual Regression Testing](#visual-regression-testing)
8. [Performance Tips](#performance-tips)

---

## What is the Styleguide?

The styleguide is an **interactive, live component gallery** built into the Loom web application at `/styleguide`.

**Purpose:**
- Visual reference for all components
- Live, interactive examples
- Copy-paste ready code snippets
- Design system documentation
- QA and testing reference

**Built with:**
- Leptos (reactive framework)
- Tailwind CSS (styling)
- Live Rust code examples

---

## Accessing the Styleguide

### From Browser

1. Start the Loom web server:
   ```bash
   cd crates/loom-web
   cargo run
   ```

2. Navigate to styleguide:
   ```
   http://localhost:3000/styleguide
   ```

### From Code

In your Leptos components, link to styleguide:

```rust
view! {
    <a href="/styleguide" target="_blank">
        "View in Styleguide ↗"
    </a>
}
```

---

## Navigation

### Main Categories

The styleguide is organized by component tier:

| Section | URL | Components |
|---------|-----|-----------|
| **Primitives** | `/styleguide/primitives` | Button, TextField, Checkbox, Badge, etc. |
| **Layout** | `/styleguide/layout` | AppShell, Panel, FormSection, ResizablePanels |
| **Chat** | `/styleguide/chat` | MessageBubble, PromptComposer, ConversationView |
| **Query Bridge** | `/styleguide/query` | QueryTimeline, ToolInvocationList, StateMachineTrace |
| **Results** | `/styleguide/results` | CodeBlock, DiffView, ExecutionResult, FileTree |
| **Home** | `/styleguide` | Overview and quick links |

### Sidebar Navigation

The styleguide includes a sidebar with:
- **Quick links** to each section
- **Search** functionality
- **Favorites** (click star to favorite)
- **Settings** (dark mode, zoom level)

---

## Component Sections

### Typical Component Example

Each component in the styleguide includes:

```
┌─────────────────────────────────────────┐
│ Component Name                          │
│ Brief description of component purpose  │
├─────────────────────────────────────────┤
│                                         │
│ ╔═ Live Example ═════════════════════╗  │
│ ║                                    ║  │
│ ║ Interactive, clickable component   ║  │
│ ║                                    ║  │
│ ╚════════════════════════════════════╝  │
│                                         │
├─────────────────────────────────────────┤
│ Props Table                             │
│ ┌─────────────────────────────────────┐ │
│ │ Prop | Type | Default | Description│ │
│ └─────────────────────────────────────┘ │
├─────────────────────────────────────────┤
│ Code Example (copy-paste ready)         │
│ ┌─────────────────────────────────────┐ │
│ │ <Button variant=Primary>            │ │
│ │   "Click me"                        │ │
│ │ </Button>                           │ │
│ └─────────────────────────────────────┘ │
├─────────────────────────────────────────┤
│ Variants (if applicable)                │
│ ┌─────────────────────────────────────┐ │
│ │ Primary | Secondary | Ghost | etc.  │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

### Interactive Features

**Click Actions:**
- Hover over elements to see interactive states
- Click buttons to see click handlers work
- Enter text in inputs to test behavior
- Adjust sliders and selections

**Code Copying:**
- Click "Copy" button to copy example code
- Automatically copies to clipboard
- Includes necessary imports

**Variant Switching:**
- Click variant pills to swap examples
- Live updates show different renderings
- Useful for comparing variants side-by-side

---

## Adding Examples

### Step 1: Create Example Component

In the styleguide route file:

```rust
// File: src/routes/styleguide/primitives.rs

#[component]
fn ButtonExamples() -> impl IntoView {
    view! {
        <div class="space-y-8">
            // Example 1: Basic Button
            <div class="space-y-4">
                <h3 class="text-lg font-semibold">"Basic Button"</h3>
                <div class="p-4 border rounded bg-gray-50">
                    <Button>"Click me"</Button>
                </div>
                <CodeBlock
                    code=r#"<Button>"Click me"</Button>"#.to_string()
                    language="rust"
                />
            </div>
            
            // Example 2: Button Variants
            <div class="space-y-4">
                <h3 class="text-lg font-semibold">"Button Variants"</h3>
                <div class="p-4 border rounded bg-gray-50 flex gap-2">
                    <Button variant=ButtonVariant::Primary>"Primary"</Button>
                    <Button variant=ButtonVariant::Secondary>"Secondary"</Button>
                    <Button variant=ButtonVariant::Ghost>"Ghost"</Button>
                    <Button variant=ButtonVariant::Destructive>"Delete"</Button>
                </div>
                <CodeBlock
                    code=r#"<Button variant=ButtonVariant::Primary>"Primary"</Button>
<Button variant=ButtonVariant::Secondary>"Secondary"</Button>
<Button variant=ButtonVariant::Ghost>"Ghost"</Button>
<Button variant=ButtonVariant::Destructive>"Delete"</Button>"#.to_string()
                    language="rust"
                />
            </div>
        </div>
    }
}
```

### Step 2: Add to Styleguide Routes

In `src/routes/styleguide/mod.rs`:

```rust
pub mod primitives;
pub mod layout;
pub mod chat;
pub mod query;
pub mod results;

use leptos::*;

#[component]
pub fn StyleguideRoute() -> impl IntoView {
    let (active_section, set_active_section) = create_signal("primitives");
    
    view! {
        <div class="flex gap-8">
            // Sidebar navigation
            <aside class="w-48 border-r p-4">
                <button
                    class=active_section.get() == "primitives" && "bg-blue-100"
                    on:click=move |_| set_active_section("primitives")
                >
                    "Primitives"
                </button>
                // More sections...
            </aside>
            
            // Main content
            <main class="flex-1 p-8">
                {match active_section.get() {
                    "primitives" => view! { <primitives::ButtonExamples/> },
                    // More sections...
                    _ => view! { <div/> },
                }}
            </main>
        </div>
    }
}
```

---

## Best Practices

### ✅ DO

#### 1. Provide Complete Examples
```rust
// Good: Full, working example
view! {
    <div class="p-4 border rounded">
        <h3>"Login Form"</h3>
        <FormSection label="Email">
            <TextField placeholder="your@email.com"/>
        </FormSection>
        <FormSection label="Password">
            <TextField input_type="password" placeholder="Password"/>
        </FormSection>
        <Button>"Sign In"</Button>
    </div>
}
```

#### 2. Show All Important Variants
```rust
// Good: All variants visible
view! {
    <div class="space-y-4">
        <Button variant=ButtonVariant::Primary>"Primary"</Button>
        <Button variant=ButtonVariant::Secondary>"Secondary"</Button>
        <Button variant=ButtonVariant::Ghost>"Ghost"</Button>
        <Button variant=ButtonVariant::Destructive>"Delete"</Button>
    </div>
}
```

#### 3. Demonstrate State Changes
```rust
// Good: Interactive state changes
let (count, set_count) = create_signal(0);

view! {
    <div class="space-y-4">
        <Button on_click=move |_| set_count(count.get() + 1)>
            "Clicked " {count} " times"
        </Button>
    </div>
}
```

#### 4. Include Proper Spacing
```rust
// Good: Breathing room between examples
view! {
    <div class="space-y-12">
        <ComponentExample1/>
        <ComponentExample2/>
        <ComponentExample3/>
    </div>
}
```

### ❌ DON'T

#### 1. Don't Mix Unrelated Components
```rust
// Avoid: Confusing example
view! {
    <MessageBubble/>
    <TextField/>
    <DataTable/>
}

// Better: Focused example
view! {
    <MessageBubble/>
    <MessageBubble/>
    <PromptComposer/>
}
```

#### 2. Don't Use Placeholder Data
```rust
// Avoid: Lorem ipsum
<Card>"Lorem ipsum dolor sit amet..."</Card>

// Better: Semantic example data
<Card>"User Settings"</Card>
```

#### 3. Don't Forget Error States
```rust
// Good: Show success and error states
view! {
    <div class="space-y-4">
        <ExecutionResult status=ExecutionStatus::Success output="Build successful"/>
        <ExecutionResult status=ExecutionStatus::Error errors=vec!["Compilation failed"]/>
    </div>
}
```

#### 4. Don't Make Examples Too Complex
```rust
// Avoid: Overly complicated example
view! {
    <div class="grid grid-cols-12 gap-4">
        {/* 50 lines of component nesting */}
    </div>
}

// Better: Simple, focused
view! {
    <Card>
        <Button>"Simple example"</Button>
    </Card>
}
```

---

## Visual Regression Testing

### Manual Testing

1. **Baseline comparison:** Open styleguide, take screenshots
2. **Compare across browsers:** Chrome, Firefox, Safari
3. **Test responsive:** Resize window, check mobile view
4. **Check color contrast:** Use accessibility tools
5. **Verify interactive states:** Hover, click, focus

### Automated Testing

Use Percy or similar for visual regression:

```bash
# Run visual regression tests
cargo test --features visual-test

# Update baseline if changes are intentional
cargo test --features visual-test -- --update-baseline
```

### Testing Checklist

- [ ] Component renders correctly
- [ ] All variants display properly
- [ ] Responsive design works (mobile/tablet/desktop)
- [ ] Hover states visible
- [ ] Focus states visible (keyboard navigation)
- [ ] Disabled states clear
- [ ] Loading states animated
- [ ] Color contrast sufficient (WCAG AA)
- [ ] Text is readable
- [ ] No layout shifts
- [ ] Responsive spacing maintained

---

## Performance Tips

### 1. Lazy Load Examples

For styleguides with many components, use `Suspense`:

```rust
#[component]
fn LazyComponentExample() -> impl IntoView {
    let data = create_resource(|| (), |_| async {
        // Load example data
    });
    
    view! {
        <Suspense fallback=move || view! { <Spinner/> }>
            {data.and_then(|d| view! {
                <ComponentExample data=d/>
            })}
        </Suspense>
    }
}
```

### 2. Memoize Complex Examples

```rust
let example = create_memo(move || {
    let items = create_large_dataset();
    view! {
        <DataTable data=items/>
    }
});
```

### 3. Split Sections

Keep sections manageable:
- Each section: max 500 lines
- Max 10 examples per section
- Use tabs for variant switching

---

## Styleguide Maintenance

### Adding New Component

1. Create component in `src/components/`
2. Add examples to styleguide section
3. Include all variants and use cases
4. Update styleguide index
5. Add to documentation

### Updating Existing Component

1. Update component source
2. Update styleguide examples
3. Update props table in example
4. Test all variants render correctly
5. Update main documentation

### Deprecating Component

1. Mark as deprecated in code comments
2. Add deprecation notice in styleguide
3. Link to replacement component
4. Set removal date (6 months minimum)
5. Add migration guide

---

## External Resources

### Component Inspiration
- [Headless UI](https://headlessui.com/) - Unstyled, accessible components
- [Shadcn/ui](https://ui.shadcn.com/) - Copy-paste component library
- [Material Design](https://material.io/components) - Design system reference

### Accessibility References
- [WAI-ARIA Practices](https://www.w3.org/WAI/ARIA/apg/)
- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [MDN Accessibility](https://developer.mozilla.org/en-US/docs/Web/Accessibility)

### Design System References
- [Design Tokens](https://www.designtokens.org/)
- [System Design Handbook](https://www.designsystemshq.com/)
- [Component Naming](https://www.smashingmagazine.com/2014/11/how-to-scale-responsive-design/)

---

## Quick Links

- 📖 [Component Library Index](file:///COMPONENT_LIBRARY_COMPLETE.md)
- 📚 [Usage Guide](file:///COMPONENTS_USAGE_GUIDE.md)
- 🏗️ [Architecture Guide](file:///COMPONENT_ARCHITECTURE.md)
- 🔗 [API Reference](file:///API_REFERENCE_COMPLETE.md)
- 🎨 [Live Styleguide](/styleguide)

---

**Last Updated:** December 22, 2025
