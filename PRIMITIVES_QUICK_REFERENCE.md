# Loom-Web Primitives Quick Reference

## Import Statement
```rust
use loom_web::components::primitives::*;
```

## Components Overview

| Component | Purpose | Key Props | Event |
|-----------|---------|-----------|-------|
| `TextField` | Single-line text input | `label`, `error`, `disabled` | `on_input: String` |
| `TextArea` | Multi-line text input | `label`, `rows`, `error` | `on_input: String` |
| `Select` | Single-select dropdown | `options`, `value`, `error` | `on_change: String` |
| `MultiSelect` | Multi-select dropdown | `options`, `value: Vec` | `on_change: Vec<String>` |
| `Toggle` | Binary on/off switch | `checked`, `label` | `on_change: bool` |

---

## Component Examples

### TextField
```rust
view! {
    // Basic
    <TextField label="Username" placeholder="Enter username" />
    
    // With error
    <TextField 
        label="Email"
        error="Invalid format"
        value="bad@"
        input_type="email"
    />
    
    // Disabled
    <TextField disabled=true label="Read-only" />
    
    // With handler
    <TextField 
        label="Search"
        on_input=move |val| set_search(val)
    />
}
```

### TextArea
```rust
view! {
    // Basic
    <TextArea label="Message" placeholder="Your message" />
    
    // Custom rows
    <TextArea label="Description" rows=5 />
    
    // With error
    <TextArea 
        label="Feedback"
        error="Max 500 chars"
        rows=4
    />
    
    // With handler
    <TextArea 
        label="Notes"
        on_input=move |val| set_notes(val)
    />
}
```

### Select
```rust
view! {
    // Basic
    <Select
        label="Category"
        options=vec![
            SelectOption::new("Option 1", "opt1"),
            SelectOption::new("Option 2", "opt2"),
        ]
    />
    
    // With value
    <Select
        label="Status"
        value="active"
        options=vec![
            SelectOption::new("Active", "active"),
            SelectOption::new("Inactive", "inactive"),
        ]
    />
    
    // With error
    <Select
        label="Required field"
        error="Must select an option"
        options=options_vec.clone()
    />
    
    // With handler
    <Select
        label="Choice"
        options=options_vec.clone()
        on_change=move |val| set_choice(val)
    />
}
```

### MultiSelect
```rust
view! {
    // Basic
    <MultiSelect
        label="Tags"
        options=vec![
            MultiSelectOption::new("Tag 1", "tag1"),
            MultiSelectOption::new("Tag 2", "tag2"),
            MultiSelectOption::new("Tag 3", "tag3"),
        ]
    />
    
    // With selected values
    <MultiSelect
        label="Permissions"
        value=vec!["read".to_string(), "write".to_string()]
        options=vec![
            MultiSelectOption::new("Read", "read"),
            MultiSelectOption::new("Write", "write"),
            MultiSelectOption::new("Delete", "delete"),
        ]
    />
    
    // With handler
    <MultiSelect
        label="Selected tags"
        options=tags.clone()
        on_change=move |vals| set_tags(vals)
    />
}
```

### Toggle
```rust
view! {
    // Basic
    <Toggle label="Enable notifications" />
    
    // With initial state
    <Toggle label="Dark mode" checked=true />
    
    // Disabled
    <Toggle label="Premium feature" disabled=true />
    
    // With handler
    <Toggle 
        label="Notifications"
        checked=notifications_enabled
        on_change=move |checked| set_notifications(checked)
    />
}
```

---

## State Management Examples

### Using Signals with TextField
```rust
let (name, set_name) = create_signal(String::new());

view! {
    <TextField
        label="Name"
        value=&name.get()
        on_input=move |val| set_name(val)
    />
    <p>"Hello, " {name}</p>
}
```

### Using Signals with Select
```rust
let (category, set_category) = create_signal("opt1".to_string());

view! {
    <Select
        label="Category"
        value=&category.get()
        options=options_vec.clone()
        on_change=move |val| set_category(val)
    />
    <p>"Selected: " {category}</p>
}
```

### Using Signals with MultiSelect
```rust
let (tags, set_tags) = create_signal(vec![]);

view! {
    <MultiSelect
        label="Tags"
        value=tags.get()
        options=available_tags.clone()
        on_change=move |vals| set_tags(vals)
    />
    <p>"Count: " {move || tags.get().len()}</p>
}
```

### Using Signals with Toggle
```rust
let (enabled, set_enabled) = create_signal(false);

view! {
    <Toggle 
        label="Enable feature"
        checked=enabled.get()
        on_change=move |checked| set_enabled(checked)
    />
    {move || if enabled.get() {
        view! { <p>"Feature is enabled!"</p> }
    } else {
        view! { <p>"Feature is disabled"</p> }
    }}
}
```

---

## Common Props Across Components

### All Input Components
- `label: Option<String>` - Label text displayed above input
- `error: Option<String>` - Error message (red styling)
- `disabled: bool` - Disable user interaction
- `class: Option<String>` - Additional CSS classes

### Styling States
- **Default**: Gray border, blue focus ring
- **Error**: Red border, red-50 background, red text
- **Disabled**: Gray background, reduced opacity, no cursor pointer

---

## Styling with Tailwind

### Default Styling
- Border: gray-300
- Focus ring: blue-500
- Text: gray-900
- Label: gray-700

### Error Styling  
- Border: red-500
- Background: red-50
- Text: red-900
- Message: red-600 (smaller)

### Disabled Styling
- Background: gray-100
- Opacity: 60%
- Cursor: not-allowed

---

## Form Integration Example

```rust
#[component]
fn MyForm() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (email, set_email) = create_signal(String::new());
    let (category, set_category) = create_signal("cat1".to_string());
    let (message, set_message) = create_signal(String::new());
    let (newsletter, set_newsletter) = create_signal(false);

    view! {
        <form class="space-y-4">
            <TextField
                label="Username"
                placeholder="Enter username"
                on_input=move |val| set_username(val)
            />
            
            <TextField
                label="Email"
                input_type="email"
                placeholder="user@example.com"
                on_input=move |val| set_email(val)
            />
            
            <Select
                label="Category"
                value=&category.get()
                options=vec![
                    SelectOption::new("Category 1", "cat1"),
                    SelectOption::new("Category 2", "cat2"),
                ]
                on_change=move |val| set_category(val)
            />
            
            <TextArea
                label="Message"
                placeholder="Your message here..."
                on_input=move |val| set_message(val)
            />
            
            <Toggle
                label="Subscribe to newsletter"
                checked=newsletter.get()
                on_change=move |checked| set_newsletter(checked)
            />
            
            <button class="mt-4">Submit</button>
        </form>
    }
}
```

---

## File Locations

- **Component Files**: `crates/loom-web/src/components/primitives/`
  - `text_field.rs`
  - `text_area.rs`
  - `select.rs`
  - `multi_select.rs`
  - `toggle.rs`

- **Exports**: `crates/loom-web/src/components/primitives/mod.rs`

- **Gallery**: `crates/loom-web/src/routes/styleguide/primitives.rs`

---

## Next Steps

1. **View in Styleguide**: Navigate to `/styleguide/primitives` to see all components
2. **Copy Examples**: Use the examples above as templates
3. **Customize**: Extend with additional CSS classes via the `class` prop
4. **Integrate**: Add to your forms and workflows

---

## Tips & Tricks

### Custom Styling
```rust
<TextField 
    label="Custom styled"
    class="border-2 border-purple-500"
/>
```

### Form Validation
```rust
let error = if email.get().contains('@') { 
    None 
} else { 
    Some("Invalid email".to_string()) 
};

view! {
    <TextField label="Email" error=error />
}
```

### Conditional Rendering
```rust
<Select
    label="Category"
    disabled=loading.get()
    options=if loading.get() { vec![] } else { options.clone() }
/>
```

### Handler Composition
```rust
<TextField
    label="Search"
    on_input=move |val| {
        set_search_term(val.clone());
        trigger_search(val);
    }
/>
```
