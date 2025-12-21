# Component Architecture & Design System

**Design patterns, composition rules, and extension guidelines**

---

## Table of Contents

1. [Design System Overview](#design-system-overview)
2. [Component Tiers](#component-tiers)
3. [Composition Patterns](#composition-patterns)
4. [Styling Strategy](#styling-strategy)
5. [Variant Patterns](#variant-patterns)
6. [Extension Guidelines](#extension-guidelines)
7. [Anti-Patterns](#anti-patterns)
8. [Performance Considerations](#performance-considerations)

---

## Design System Overview

### Core Principles

1. **Single Responsibility:** Each component has one clear purpose
2. **Composability:** Components combine without conflicts
3. **Consistency:** Shared patterns across all components
4. **Accessibility:** Built-in, not bolted-on
5. **Predictability:** Props and behavior are obvious

### Design Tokens

```rust
// Colors
primary:    #2563EB (blue-600)
secondary:  #6B7280 (gray-500)
error:      #DC2626 (red-600)
warning:    #F59E0B (amber-600)
success:    #059669 (emerald-600)

// Spacing (4px base)
xs:  4px   (0.25rem)
sm:  8px   (0.5rem)
md:  16px  (1rem)
lg:  24px  (1.5rem)
xl:  32px  (2rem)

// Typography
body:      14px / 400
body-bold: 14px / 600
h3:        16px / 600
h2:        20px / 600
h1:        24px / 700

// Border Radius
sm:  4px   (0.25rem)
md:  8px   (0.5rem)
lg:  12px  (0.75rem)
xl:  16px  (1rem)

// Shadows
sm: 0 1px 2px rgba(0,0,0,0.05)
md: 0 4px 6px rgba(0,0,0,0.1)
lg: 0 10px 15px rgba(0,0,0,0.1)
```

---

## Component Tiers

### Tier 1: Primitives (Atomic Components)

**Purpose:** Irreducible UI elements that form the design system foundation.

**Characteristics:**
- No dependencies on other components
- Direct Tailwind CSS styling
- Single, focused responsibility
- Extensive customization via props

**Examples:**
```
Button, TextField, Checkbox, Badge, Card, Spinner
```

**When to create a Tier 1:**
- Element can't be built from simpler components
- Used by multiple Tier 2+ components
- Represents core design system concept

**Structure:**
```rust
// Primitives/example.rs
use leptos::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExampleVariant {
    Primary,
    Secondary,
}

#[component]
pub fn Example(
    #[prop(default = ExampleVariant::Primary)]
    variant: ExampleVariant,
    
    children: Children,
    
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    // Render using Tailwind classes
}
```

---

### Tier 2: Layout Components

**Purpose:** Structure and organize content hierarchically.

**Characteristics:**
- May contain Tier 1 components
- Define spatial relationships
- Enable responsive design
- Provide context for children

**Examples:**
```
AppShell, Panel, FormSection, FieldRow, ResizablePanels
```

**When to create a Tier 2:**
- Solves a structural/layout problem
- Organizes multiple Tier 1 components
- Provides semantic structure

**Composition Pattern:**
```rust
#[component]
pub fn FormSection(
    label: String,
    children: Children,
    #[prop(optional)]
    help_text: Option<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <label class="block text-sm font-medium">{label}</label>
            {help_text.map(|h| view! {
                <p class="text-sm text-gray-600">{h}</p>
            })}
            <div class="space-y-3">
                {children()}
            </div>
        </div>
    }
}
```

---

### Tier 3: Domain Components

**Purpose:** Solve domain-specific problems (chat, query, results).

**Characteristics:**
- Contain Tier 1 and Tier 2 components
- Specialized behavior and styling
- Domain-specific props
- Often include state or event handling

**Examples:**
```
MessageBubble, CodeBlock, ExecutionResult, QueryTimeline
```

**When to create a Tier 3:**
- Solves domain-specific problem
- Reused in multiple screens
- Encapsulates domain logic

---

### Tier 4: Composite Components

**Purpose:** Complete features combining multiple lower tiers.

**Characteristics:**
- Complex interaction patterns
- May include business logic
- State management
- Feature-complete functionality

**Examples:**
```
ThreadList, ConversationView, TabNav, MultiStepForm
```

**When to create a Tier 4:**
- Combines multiple domain components
- Encapsulates feature logic
- Provides complete feature implementation

---

## Composition Patterns

### Pattern 1: Wrapper Pattern

Used when lower tier needs extra behavior.

```rust
// Tier 2 wrapping Tier 1
#[component]
pub fn FormField(
    label: String,
    #[prop(optional)]
    required: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="space-y-1">
            <label class="block text-sm font-medium">
                {label}
                {required && view! { <span class="text-red-600">*</span> }}
            </label>
            {children()}
        </div>
    }
}

// Usage:
view! {
    <FormField label="Email" required=true>
        <TextField/>
    </FormField>
}
```

### Pattern 2: Container Pattern

Higher tier provides structure for lower tiers.

```rust
// Tier 2 containing Tier 1
#[component]
pub fn ButtonGroup(children: Children) -> impl IntoView {
    view! {
        <div class="flex gap-2">
            {children()}
        </div>
    }
}

// Usage:
view! {
    <ButtonGroup>
        <Button>"Save"</Button>
        <Button>"Cancel"</Button>
    </ButtonGroup>
}
```

### Pattern 3: Provider Pattern

Passes context/configuration to children.

```rust
use leptos::*;

#[derive(Clone)]
struct FormContextData {
    disabled: bool,
    is_loading: bool,
}

// Usage:
view! {
    <Provider value=FormContextData { disabled: false, is_loading: false }>
        <FormSection/>
        <TextField/>
        <Button/>
    </Provider>
}
```

### Pattern 4: Slot Pattern

Multiple children slots for flexible layouts.

```rust
#[component]
pub fn Card(
    #[prop(optional)]
    header: Option<Children>,
    
    body: Children,
    
    #[prop(optional)]
    footer: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="border rounded-lg">
            {header.map(|h| view! {
                <div class="border-b px-6 py-4">
                    {h()}
                </div>
            })}
            <div class="px-6 py-4">
                {body()}
            </div>
            {footer.map(|f| view! {
                <div class="border-t px-6 py-4">
                    {f()}
                </div>
            })}
        </div>
    }
}
```

---

## Styling Strategy

### Tailwind-First Approach

**Rule:** Always use Tailwind utilities, never CSS-in-JS.

```rust
// ✅ GOOD: Pure Tailwind
view! {
    <div class="flex gap-4 p-6 bg-white rounded-lg shadow">
        "Content"
    </div>
}

// ❌ AVOID: CSS-in-JS or external CSS
view! {
    <div style="display: flex; gap: 16px;">
        "Content"
    </div>
}
```

### Class Composition

Build CSS class strings systematically:

```rust
#[component]
pub fn Button(
    variant: ButtonVariant,
    size: ButtonSize,
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    // 1. Define variant classes
    let variant_class = match variant {
        ButtonVariant::Primary => "bg-blue-600 text-white",
        ButtonVariant::Secondary => "bg-gray-200 text-gray-900",
    };
    
    // 2. Define size classes
    let size_class = match size {
        ButtonSize::Sm => "px-2 py-1 text-sm",
        ButtonSize::Md => "px-3 py-2 text-base",
    };
    
    // 3. Define state classes
    let state_class = "hover:opacity-90 active:opacity-100 transition-opacity";
    
    // 4. Combine all classes
    let final_class = format!(
        "inline-flex items-center justify-center {} {} {} {}",
        variant_class,
        size_class,
        state_class,
        class.unwrap_or_default()
    );
    
    view! {
        <button class=final_class>
            // Render button
        </button>
    }
}
```

### Responsive Classes

Use Tailwind breakpoint prefixes:

```rust
view! {
    <div class="
        grid
        grid-cols-1      // Mobile
        md:grid-cols-2   // Tablet
        lg:grid-cols-3   // Desktop
        gap-4
        md:gap-6
        lg:gap-8
        p-4
        md:p-6
        lg:p-8
    ">
        // Grid content
    </div>
}
```

---

## Variant Patterns

### Enum-Based Variants

All variants use Rust enums, never strings:

```rust
// ✅ GOOD: Type-safe variants
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Destructive,
    Ghost,
}

#[component]
pub fn Button(
    #[prop(default = ButtonVariant::Primary)]
    variant: ButtonVariant,
    // ...
) -> impl IntoView {
    let class = match variant {
        ButtonVariant::Primary => "bg-blue-600",
        // ...
    };
    // ...
}

// ❌ AVOID: String-based variants
pub fn Button(variant: &str) -> impl IntoView {
    // String matching is error-prone
}
```

### Size Enum Pattern

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

impl ButtonSize {
    pub fn class(&self) -> &'static str {
        match self {
            Self::Sm => "px-2.5 py-1.5 text-sm",
            Self::Md => "px-3.5 py-2 text-base",
            Self::Lg => "px-4 py-2.5 text-lg",
        }
    }
}

#[component]
pub fn Button(
    #[prop(default = ButtonSize::Md)]
    size: ButtonSize,
) -> impl IntoView {
    let size_class = size.class();
    // ...
}
```

### Variant Combinations

When combining variants, ensure orthogonal combinations:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardElevation {
    None,
    Sm,
    Md,
    Lg,
}

#[component]
pub fn Card(
    #[prop(default = CardElevation::Md)]
    elevation: CardElevation,
    
    #[prop(default = true)]
    white_bg: bool,
) -> impl IntoView {
    // elevation and white_bg are independent
    // Any combination is valid
    // ...
}
```

---

## Extension Guidelines

### Adding New Component

**Step 1: Determine Tier**
```
Is it used by multiple other components?  → Tier 1/2
Does it solve a domain problem?            → Tier 3
Does it combine multiple domain components? → Tier 4
```

**Step 2: Create File**
```
crates/loom-web/src/components/{tier}/{component_name}.rs
```

**Step 3: Implement Component**
```rust
/// Brief description of component purpose
///
/// More detailed explanation of when to use it.
use leptos::*;

/// Component variant enum
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ComponentVariant {
    /// Variant description
    Variant1,
    /// Variant description
    Variant2,
}

/// Component implementation
///
/// # Example
///
/// ```rust
/// view! {
///     <Component variant=ComponentVariant::Variant1/>
/// }
/// ```
#[component]
pub fn Component(
    /// Variant selection
    #[prop(default = ComponentVariant::Variant1)]
    variant: ComponentVariant,
    
    /// Component children
    children: Children,
    
    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    // Implementation
}
```

**Step 4: Export from mod.rs**
```rust
pub mod component_name;
pub use component_name::Component;
```

**Step 5: Document in Library Index**
Update `COMPONENT_LIBRARY_COMPLETE.md` with:
- Component name and tier
- Brief description
- Props table
- Example usage
- Related components

### Adding Variant

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Destructive,
    // Add new variant here
    Outline,
}

#[component]
pub fn Button(/* ... */) -> impl IntoView {
    let variant_class = match variant {
        // ...
        ButtonVariant::Outline => "border border-blue-600 text-blue-600",
    };
    // ...
}
```

### Adding Property

```rust
#[component]
pub fn Button(
    // Existing props...
    
    // Add new property with default
    #[prop(default = false)]
    full_width: bool,
    
    #[prop(optional)]
    aria_label: Option<String>,
) -> impl IntoView {
    let width_class = if full_width { "w-full" } else { "" };
    // ...
}
```

---

## Anti-Patterns

### ❌ Anti-Pattern 1: Overly Coupled Components

```rust
// BAD: Component tied to specific data structure
#[component]
pub fn UserCard(user: &User) -> impl IntoView {
    // Can't reuse for other data structures
    // Tightly coupled to User type
}

// GOOD: Generic over data
#[component]
pub fn InfoCard(
    name: String,
    email: String,
    #[prop(optional)]
    avatar_url: Option<String>,
) -> impl IntoView {
    // Can accept any data
    // Loosely coupled
}
```

### ❌ Anti-Pattern 2: Props-Explosion

```rust
// BAD: Too many props
#[component]
pub fn Card(
    border_width: i32,
    border_color: &str,
    shadow_size: i32,
    shadow_color: &str,
    padding_top: i32,
    padding_bottom: i32,
    padding_left: i32,
    padding_right: i32,
    // ...20 more props
) -> impl IntoView {}

// GOOD: Variants + optional overrides
#[component]
pub fn Card(
    #[prop(default = CardElevation::Md)]
    elevation: CardElevation,
    
    #[prop(optional)]
    class: Option<String>, // For advanced customization
) -> impl IntoView {}
```

### ❌ Anti-Pattern 3: Mixing Concerns

```rust
// BAD: Component with business logic
#[component]
pub fn UserForm() -> impl IntoView {
    let (user, set_user) = create_signal(User::default());
    // API calls
    // Validation logic
    // Navigation logic
    // ... 200 lines of code
}

// GOOD: Separate concerns
#[component]
pub fn UserForm(
    initial: User,
    on_submit: impl Fn(User),
    errors: Vec<String>,
) -> impl IntoView {
    // Only render form
    // Parent handles logic
}
```

### ❌ Anti-Pattern 4: Accessibility Violations

```rust
// BAD: No accessibility
<button style="display: flex;">
    <Icon/> // No accessible name
</button>

// GOOD: Accessible
<button aria_label="Delete item">
    <Icon/>
</button>

// Or with visible label
<button>
    <Icon/>
    <span>"Delete"</span>
</button>
```

### ❌ Anti-Pattern 5: Hardcoded Styles

```rust
// BAD: Hardcoded in template
view! {
    <div style="background-color: #2563EB; padding: 16px;">
        "Content"
    </div>
}

// GOOD: Use Tailwind + variants
view! {
    <div class="bg-blue-600 p-4">
        "Content"
    </div>
}
```

---

## Performance Considerations

### 1. Memoization

Use `create_memo` for expensive derived state:

```rust
#[component]
pub fn ComponentList(items: Signal<Vec<Item>>) -> impl IntoView {
    // Without memo: recomputes on every render
    let total = move || items.get().iter().map(|i| i.value).sum::<i32>();
    
    // With memo: only recomputes when items change
    let total = create_memo(move || {
        items.get().iter().map(|i| i.value).sum::<i32>()
    });
    
    view! {
        <p>"Total: " {total}</p>
    }
}
```

### 2. Lazy Components

Use `Suspense` for async components:

```rust
#[component]
pub fn LazyData() -> impl IntoView {
    let data = create_resource(
        || (),
        |_| async { fetch_data().await }
    );
    
    view! {
        <Suspense fallback=move || view! { <Spinner/> }>
            {data.and_then(|d| view! { <DataDisplay data=d/> })}
        </Suspense>
    }
}
```

### 3. Event Delegation

Use event delegation for lists:

```rust
// BAD: Handler on each item
view! {
    {items.get().into_iter().map(|item| view! {
        <div on:click=move |_| handle_click(item.id)>
            {item.name}
        </div>
    }).collect::<Vec<_>>()}
}

// GOOD: Delegate to parent
view! {
    <div on:click=move |e| {
        if let Some(id) = extract_id_from_event(&e) {
            handle_click(id);
        }
    }>
        {items.get().into_iter().map(|item| view! {
            <div data-id=item.id.to_string()>
                {item.name}
            </div>
        }).collect::<Vec<_>>()}
    </div>
}
```

---

## Summary

| Aspect | Guidelines |
|--------|------------|
| **Tiers** | Use appropriate tier for problem; don't skip tiers |
| **Styling** | Tailwind utilities only; no CSS-in-JS |
| **Props** | Use enums for variants; optional for extras |
| **Composition** | Wrap, contain, provide, slot patterns |
| **Variants** | Type-safe enums with sensible defaults |
| **Accessibility** | Built-in, not retrofitted |
| **Performance** | Memoize expensive computations |
| **Naming** | Clear, single responsibility |

---

**Last Updated:** December 22, 2025
