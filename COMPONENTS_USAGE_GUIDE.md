# Components Usage Guide

**Practical patterns, best practices, and real-world examples**

---

## Table of Contents

1. [Component Patterns](#component-patterns)
2. [Best Practices](#best-practices)
3. [Common Patterns](#common-patterns)
4. [Accessibility Guidelines](#accessibility-guidelines)
5. [Responsive Design](#responsive-design)
6. [Form Patterns](#form-patterns)
7. [State Management](#state-management)
8. [Troubleshooting](#troubleshooting)

---

## Component Patterns

### Pattern 1: Button Group

**Use when:** Presenting multiple related actions

```rust
use loom_web::components::primitives::{Button, ButtonVariant};
use leptos::*;

#[component]
fn ActionButtons() -> impl IntoView {
    view! {
        <div class="flex gap-2">
            <Button variant=ButtonVariant::Primary>"Save"</Button>
            <Button variant=ButtonVariant::Secondary>"Cancel"</Button>
            <Button variant=ButtonVariant::Destructive>"Delete"</Button>
        </div>
    }
}
```

**Accessibility:** Use `aria-group` to semantically group buttons together.

---

### Pattern 2: Form with Validation

**Use when:** Collecting and validating user input

```rust
use loom_web::components::primitives::{TextField, Button};
use leptos::*;

#[component]
fn LoginForm() -> impl IntoView {
    let (email, set_email) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (errors, set_errors) = create_signal(Vec::new());
    
    let on_submit = move |_| {
        let mut errs = Vec::new();
        
        if email.get().is_empty() {
            errs.push("Email is required".to_string());
        }
        if password.get().len() < 8 {
            errs.push("Password must be 8+ characters".to_string());
        }
        
        if errs.is_empty() {
            // Submit form
        } else {
            set_errors(errs);
        }
    };
    
    view! {
        <form class="space-y-4" on:submit=on_submit>
            <TextField 
                value=email.get()
                on_input=move |v| set_email(v)
                placeholder="your@email.com"
                aria_label="Email address"
            />
            
            <TextField 
                value=password.get()
                on_input=move |v| set_password(v)
                placeholder="Password"
                input_type="password"
                aria_label="Password"
            />
            
            {errors.get().into_iter().map(|e| view! {
                <div class="text-red-600 text-sm">{e}</div>
            }).collect::<Vec<_>>()}
            
            <Button type_="submit">"Sign In"</Button>
        </form>
    }
}
```

**Key Points:**
- Store errors in reactive signal
- Show validation errors immediately
- Use semantic HTML input types
- Provide ARIA labels

---

### Pattern 3: Loading State

**Use when:** Waiting for async operations

```rust
use loom_web::components::primitives::{Button, Spinner};
use leptos::*;

#[component]
fn AsyncButton() -> impl IntoView {
    let (is_loading, set_is_loading) = create_signal(false);
    
    let on_click = move |_| {
        if is_loading.get() { return; }
        
        set_is_loading(true);
        
        // Async operation
        spawn_local(async move {
            // Fetch or process
            set_is_loading(false);
        });
    };
    
    view! {
        <Button loading=is_loading.get() on_click=on_click>
            {if is_loading.get() {
                view! { <Spinner/> }
            } else {
                view! { "Submit" }
            }}
        </Button>
    }
}
```

---

### Pattern 4: Conditional Rendering

**Use when:** Showing/hiding sections based on state

```rust
use loom_web::components::primitives::{Card, CardElevation};
use leptos::*;

#[component]
fn ConditionalContent() -> impl IntoView {
    let (show_details, set_show_details) = create_signal(false);
    
    view! {
        <Card>
            <h2>"User Details"</h2>
            <button on:click=move |_| set_show_details(!show_details.get())>
                {if show_details.get() { "Hide" } else { "Show" }}
            </button>
            
            {show_details.get().then(|| view! {
                <div class="mt-4 space-y-2">
                    <p>"Email: user@example.com"</p>
                    <p>"Phone: +1-555-0123"</p>
                    <p>"Status: Active"</p>
                </div>
            })}
        </Card>
    }
}
```

---

### Pattern 5: List with Selection

**Use when:** Displaying selectable items

```rust
use loom_web::components::primitives::Checkbox;
use leptos::*;

#[component]
fn SelectableList(items: Vec<String>) -> impl IntoView {
    let (selected, set_selected) = create_signal(Vec::<String>::new());
    
    view! {
        <div class="space-y-2">
            {items.into_iter().map(|item| {
                let is_checked = create_memo(move || {
                    selected.get().contains(&item)
                });
                
                view! {
                    <div class="flex items-center gap-2">
                        <Checkbox 
                            checked=is_checked.get()
                            on_change=move |checked| {
                                let mut sel = selected.get();
                                if checked {
                                    sel.push(item.clone());
                                } else {
                                    sel.retain(|i| i != &item);
                                }
                                set_selected(sel);
                            }
                        />
                        <label>{item}</label>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
```

---

### Pattern 6: Nested Card Layout

**Use when:** Creating hierarchical information display

```rust
use loom_web::components::primitives::Card;
use loom_web::components::layout::KeyValueList;

#[component]
fn NestedLayout() -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Card>
                <h3>"Section 1"</h3>
                <KeyValueList/>
            </Card>
            <Card>
                <h3>"Section 2"</h3>
                <KeyValueList/>
            </Card>
        </div>
    }
}
```

---

## Best Practices

### ✅ DO

#### 1. Use Variants Consistently
```rust
// Good: Use component variants
<Button variant=ButtonVariant::Primary>"Save"</Button>

// Avoid: Trying to style with custom classes instead
<Button class="bg-blue-600">"Save"</Button>
```

#### 2. Provide Meaningful Labels
```rust
// Good: Descriptive, accessible labels
<TextField aria_label="Email address" placeholder="your@email.com"/>

// Avoid: Unclear labels
<TextField aria_label="Input" placeholder="?"/>
```

#### 3. Use Semantic Component Nesting
```rust
// Good: Proper tier composition
<Card>
    <FormSection>
        <TextField/>
    </FormSection>
</Card>

// Avoid: Mixing unrelated components
<Card>
    <ChatMessageBubble/>
    <TextField/>
</Card>
```

#### 4. Handle Loading and Error States
```rust
// Good: Explicit state handling
let (is_loading, set_is_loading) = create_signal(false);
let (error, set_error) = create_signal(None::<String>);

view! {
    <Button loading=is_loading.get() disabled=error.get().is_some()>
        "Submit"
    </Button>
    {error.get().map(|e| view! { <div class="text-red-600">{e}</div> })}
}

// Avoid: Silent failures or no feedback
<Button on_click=on_submit>"Submit"</Button>
```

### ❌ DON'T

#### 1. Don't Override Component Styling
```rust
// Avoid: Fighting Tailwind classes
<Button class="!bg-purple-500 !text-yellow-200">
    "Weird colors"
</Button>

// Use proper variants instead
<Button variant=ButtonVariant::Custom>...</Button>
```

#### 2. Don't Nest Too Deeply
```rust
// Avoid: Excessive nesting
<Card>
    <div>
        <div>
            <div>
                <Button/>
            </div>
        </div>
    </div>
</Card>

// Better: Flatten structure
<Card>
    <Button/>
</Card>
```

#### 3. Don't Forget Accessibility
```rust
// Avoid: No labels or ARIA attributes
<input type="checkbox"/>
<Button class="icon-only"/></Button>

// Good: Proper accessibility
<Checkbox aria_label="Accept terms"/>
<Button aria_label="Delete item"><Icon/></Button>
```

#### 4. Don't Use Size Strings
```rust
// Avoid: Using string sizes
<Button size="medium">"Click"</Button>

// Use enums instead
<Button size=ButtonSize::Md>"Click"</Button>
```

---

## Common Patterns

### Dropdown Menu Pattern
```rust
#[component]
fn DropdownMenu() -> impl IntoView {
    let (open, set_open) = create_signal(false);
    
    view! {
        <div class="relative">
            <button on:click=move |_| set_open(!open.get())>
                "Menu ▼"
            </button>
            
            {open.get().then(|| view! {
                <div class="absolute mt-2 bg-white border rounded shadow-lg">
                    <button class="block w-full text-left px-4 py-2 hover:bg-gray-100">
                        "Option 1"
                    </button>
                    <button class="block w-full text-left px-4 py-2 hover:bg-gray-100">
                        "Option 2"
                    </button>
                    <button class="block w-full text-left px-4 py-2 hover:bg-gray-100">
                        "Option 3"
                    </button>
                </div>
            })}
        </div>
    }
}
```

### Tab Navigation Pattern
```rust
#[component]
fn TabNav() -> impl IntoView {
    let (active_tab, set_active_tab) = create_signal("overview");
    
    let tabs = vec![
        ("overview", "Overview"),
        ("details", "Details"),
        ("settings", "Settings"),
    ];
    
    view! {
        <div class="border-b border-gray-200">
            <div class="flex">
                {tabs.iter().map(|(key, label)| {
                    view! {
                        <button
                            class=format!(
                                "px-4 py-2 border-b-2 {}", 
                                if active_tab.get() == key {
                                    "border-blue-600 text-blue-600"
                                } else {
                                    "border-transparent text-gray-600"
                                }
                            )
                            on:click=move |_| set_active_tab(key)
                        >
                            {label}
                        </button>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}
```

### Modal Dialog Pattern
```rust
#[component]
fn ModalExample() -> impl IntoView {
    let (show_modal, set_show_modal) = create_signal(false);
    
    view! {
        <>
            <Button on_click=move |_| set_show_modal(true)>
                "Open Modal"
            </Button>
            
            {show_modal.get().then(|| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center">
                    <div class="bg-white rounded-lg p-6 w-96">
                        <h2>"Confirm Action"</h2>
                        <p>"Are you sure?"</p>
                        <div class="flex gap-2 mt-4">
                            <Button 
                                variant=ButtonVariant::Secondary
                                on_click=move |_| set_show_modal(false)
                            >
                                "Cancel"
                            </Button>
                            <Button on_click=move |_| {
                                // Perform action
                                set_show_modal(false);
                            }>
                                "Confirm"
                            </Button>
                        </div>
                    </div>
                </div>
            })}
        </>
    }
}
```

### Search with Results
```rust
#[component]
fn SearchComponent() -> impl IntoView {
    let (query, set_query) = create_signal(String::new());
    let (results, set_results) = create_signal(Vec::<String>::new());
    
    let on_search = move |v: String| {
        set_query(v.clone());
        // Perform search
        let matching: Vec<_> = vec!["Result 1", "Result 2"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        set_results(matching);
    };
    
    view! {
        <div class="space-y-4">
            <TextField
                value=query.get()
                on_input=on_search
                placeholder="Search..."
            />
            
            <div class="space-y-2">
                {results.get().into_iter().map(|result| view! {
                    <div class="p-3 border rounded hover:bg-gray-50">
                        {result}
                    </div>
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}
```

---

## Accessibility Guidelines

### 1. Keyboard Navigation

All interactive components should be keyboard accessible:

```rust
// Good: Keyboard-accessible button
<button on:click=handler on:keydown=|e| {
    if e.key() == "Enter" || e.key() == " " {
        handler(e.into());
    }
}/>

// Components handle this automatically
<Button on_click=handler/> // Tab-able and Enter-able
```

### 2. ARIA Labels

Provide context for screen readers:

```rust
// Good: Explicit labels
<button aria_label="Close dialog">"✕"</button>
<input aria_label="Search products" placeholder="..."/>

// With associated label
<label for="email">"Email:"</label>
<input id="email" type="email"/>
```

### 3. Color Contrast

Always ensure sufficient contrast:

```rust
// Use component variants (they ensure contrast)
<Button variant=ButtonVariant::Primary>"High contrast text"</Button>

// Check contrast in custom styling
// Ensure text on background has 4.5:1 ratio (AA) or 7:1 (AAA)
```

### 4. Focus Management

Indicate focused elements:

```rust
// Components provide focus styles automatically
<Button/> // Shows focus ring on keyboard tab

// For custom elements, ensure focus-visible styles
<div class="focus:ring-2 focus:ring-offset-2 focus:ring-blue-500">
    Content
</div>
```

### 5. Form Labels

Always label form inputs:

```rust
// Option 1: Associated label (preferred)
<label for="username">"Username:"</label>
<TextField id="username"/>

// Option 2: ARIA label
<TextField aria_label="Username"/>

// Option 3: Helper text
<FormSection label="Username">
    <TextField/>
</FormSection>
```

---

## Responsive Design

### Breakpoint Usage

Loom uses Tailwind breakpoints:

```rust
// Mobile-first: base styles apply to all
// Add larger breakpoint prefixes for wider screens

<div class="
    grid 
    grid-cols-1     // Mobile: 1 column
    md:grid-cols-2  // Tablet: 2 columns
    lg:grid-cols-3  // Desktop: 3 columns
">
    <Card/>
    <Card/>
    <Card/>
</div>
```

### Responsive Font Sizes

```rust
<h1 class="
    text-2xl     // Mobile: 1.5rem
    md:text-3xl  // Tablet: 1.875rem
    lg:text-4xl  // Desktop: 2.25rem
">
    "Responsive Heading"
</h1>
```

### Responsive Spacing

```rust
<div class="
    p-4          // Mobile: 1rem padding
    md:p-6       // Tablet: 1.5rem
    lg:p-8       // Desktop: 2rem
    gap-4
    md:gap-6
    lg:gap-8
">
    Content
</div>
```

---

## Form Patterns

### Multi-Step Form

```rust
#[component]
fn MultiStepForm() -> impl IntoView {
    let (step, set_step) = create_signal(1);
    
    view! {
        <div class="space-y-6">
            {if step.get() == 1 {
                view! {
                    <FormSection label="Personal Info">
                        <TextField placeholder="First Name"/>
                        <TextField placeholder="Last Name"/>
                    </FormSection>
                }
            } else if step.get() == 2 {
                view! {
                    <FormSection label="Contact">
                        <TextField placeholder="Email"/>
                        <TextField placeholder="Phone"/>
                    </FormSection>
                }
            } else {
                view! {
                    <FormSection label="Confirm">
                        <p>"Review your information"</p>
                    </FormSection>
                }
            }}
            
            <div class="flex gap-2">
                {step.get() > 1 && view! {
                    <Button variant=ButtonVariant::Secondary 
                        on_click=move |_| set_step(step.get() - 1)
                    >
                        "Previous"
                    </Button>
                }}
                
                {step.get() < 3 && view! {
                    <Button on_click=move |_| set_step(step.get() + 1)>
                        "Next"
                    </Button>
                }}
                
                {step.get() == 3 && view! {
                    <Button variant=ButtonVariant::Primary>
                        "Submit"
                    </Button>
                }}
            </div>
        </div>
    }
}
```

---

## State Management

### Using Signals

```rust
// Simple reactive state
let (count, set_count) = create_signal(0);

view! {
    <p>"Count: " {count}</p>
    <Button on_click=move |_| set_count(count.get() + 1)>
        "Increment"
    </Button>
}
```

### Using Memos for Derived State

```rust
let (items, set_items) = create_signal(vec!["a", "b", "c"]);

let item_count = create_memo(move || {
    items.get().len()
});

view! {
    <p>"Items: " {item_count}</p>
}
```

---

## Troubleshooting

### Components Not Updating
```rust
// Problem: Value doesn't change when prop changes
let (value, set_value) = create_signal("initial");

// Solution: Use move closures and proper reactivity
view! {
    <TextField 
        value=value.get()
        on_input=move |v| set_value(v)
    />
}
```

### Styling Not Applied
```rust
// Problem: Custom class doesn't show
<Button class="!bg-purple-600"/>

// Solution: Check class is added to final_class, not overridden
// Or use proper component variants instead
<Button variant=ButtonVariant::Custom/>
```

### Events Not Firing
```rust
// Problem: Event handler not called
<Button on_click=on_click/>  // Wrong

// Solution: Use move closures
let on_click = move |_| { /* handler */ };
<Button on_click=on_click/>  // Correct

// Or use inline closure
<Button on_click=move |_| { /* handler */ }/>
```

### Accessibility Issues
```rust
// Problem: Input can't be labeled
<TextField/>

// Solution: Provide ARIA label
<TextField aria_label="Email address"/>

// Or use associated label
<label for="email">"Email:"</label>
<TextField id="email"/>
```

---

## Next Steps

- 📖 See [API Reference](file:///API_REFERENCE_COMPLETE.md) for complete prop documentation
- 🏗️ Review [Architecture Guide](file:///COMPONENT_ARCHITECTURE.md) for design patterns
- 🎨 Visit [/styleguide](/styleguide) for visual examples
- 📚 Check [Component Library](file:///COMPONENT_LIBRARY_COMPLETE.md) for component overview

---

**Last Updated:** December 22, 2025
