# Loom Web Primitive Components Reference

## Overview
Five new primitive input components implemented for loom-web in parallel, following Leptos best practices with full accessibility and Tailwind styling.

## Components

### 1. Checkbox - Multiple Selection
**File:** `crates/loom-web/src/components/primitives/checkbox.rs`

Multiple checkbox input for selecting zero or more options from a list.

```rust
<Checkbox
    label="I agree to terms"
    checked=agreed_signal
    disabled=false
    error=None
    on_change=move |v| set_agreed(v)
/>
```

**Props:**
- `label: Option<String>` - Label text displayed next to checkbox
- `checked: bool` - Controlled checked state (default: false)
- `disabled: bool` - Disable the checkbox (default: false)
- `error: Option<String>` - Error message displayed below
- `on_change: Option<impl Fn(bool)>` - Callback when state changes
- `class: Option<String>` - Additional CSS classes

**States:** enabled, disabled, checked, unchecked, error

---

### 2. RadioGroup - Single Selection
**File:** `crates/loom-web/src/components/primitives/radio_group.rs`

Single-select radio button group for choosing exactly one option.

```rust
<RadioGroup
    label="Choose one"
    value=selected_signal
    options=vec![
        RadioOption::new("opt1", "Option 1"),
        RadioOption::new("opt2", "Option 2"),
    ]
    disabled=false
    error=None
    on_change=move |v| set_selected(v)
/>
```

**Props:**
- `options: Vec<RadioOption>` - Available options (required)
- `value: String` - Currently selected value (default: empty)
- `label: Option<String>` - Group label
- `disabled: bool` - Disable all options (default: false)
- `error: Option<String>` - Error message
- `on_change: Option<impl Fn(String)>` - Callback on selection
- `class: Option<String>` - Additional CSS classes

**Types:**
```rust
pub struct RadioOption {
    pub value: String,
    pub label: String,
}

impl RadioOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self
}
```

**States:** enabled, disabled, selected, error

---

### 3. Switch - Toggle Boolean
**File:** `crates/loom-web/src/components/primitives/switch.rs`

Visual toggle switch for enabling/disabling features.

```rust
<Switch
    label="Enable notifications"
    checked=enabled_signal
    disabled=false
    error=None
    on_change=move |v| set_enabled(v)
/>
```

**Props:**
- `label: Option<String>` - Label text displayed next to switch
- `checked: bool` - Toggle state (default: false)
- `disabled: bool` - Disable the switch (default: false)
- `error: Option<String>` - Error message
- `on_change: Option<impl Fn(bool)>` - Callback on toggle
- `class: Option<String>` - Additional CSS classes

**Features:**
- Animated toggle with 6px translate
- Smooth color transitions
- role="switch" for accessibility
- Hover effects on enabled state

**States:** on, off, disabled, error

---

### 4. Slider - Range Input
**File:** `crates/loom-web/src/components/primitives/slider.rs`

Draggable range slider for numeric value selection.

```rust
<Slider
    label="Volume"
    value=volume_signal
    min=0.0
    max=100.0
    step=5.0
    disabled=false
    error=None
    show_value=true
    on_change=move |v| set_volume(v)
/>
```

**Props:**
- `label: Option<String>` - Label displayed above slider
- `value: f64` - Current value (default: 0.0)
- `min: f64` - Minimum value (default: 0.0)
- `max: f64` - Maximum value (default: 100.0)
- `step: f64` - Step increment (default: 1.0)
- `disabled: bool` - Disable slider (default: false)
- `error: Option<String>` - Error message
- `show_value: bool` - Display current value (default: false)
- `on_change: Option<impl Fn(f64)>` - Callback on value change
- `class: Option<String>` - Additional CSS classes

**Features:**
- Visual track with filled portion
- Custom styling for webkit and moz
- Min/max labels
- Optional value display
- Full ARIA attributes

**States:** enabled, disabled, error

---

### 5. FileInput - File Upload
**File:** `crates/loom-web/src/components/primitives/file_input.rs`

File upload input with customizable accept filters.

```rust
<FileInput
    label="Upload files"
    accept="image/*"
    multiple=true
    disabled=false
    error=None
    on_change=move |files| handle_upload(files)
/>
```

**Props:**
- `label: Option<String>` - Label displayed above input
- `accept: Option<String>` - File type filter (e.g., "image/*", ".pdf")
- `multiple: bool` - Allow multiple files (default: false)
- `disabled: bool` - Disable input (default: false)
- `error: Option<String>` - Error message
- `on_change: Option<impl Fn(Vec<File>)>` - Callback with selected files
- `class: Option<String>` - Additional CSS classes

**Features:**
- Styled hidden input with overlay
- Dashed border design
- Multiple file support
- Accept filter support
- File list callback

**States:** enabled, disabled, error

---

## Styling

All components use **Tailwind CSS exclusively** with the following palette:

| State | Color | Class |
|-------|-------|-------|
| Primary/Enabled | Blue | `bg-blue-600`, `border-blue-600` |
| Hover | Darker Blue | `hover:bg-blue-700`, `hover:border-blue-500` |
| Disabled | Gray | `bg-gray-100`, `border-gray-300`, `opacity-60` |
| Error | Red | `border-red-500`, `text-red-600` |
| Background | White | `bg-white` |
| Border | Light Gray | `border-gray-200`, `border-gray-300` |

---

## Accessibility

All components include comprehensive accessibility features:

- **ARIA Attributes:** `aria-checked`, `aria-disabled`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax`
- **Semantic HTML:** `<fieldset>`, `<legend>`, `<label>`, `<button role="switch">`
- **Keyboard Support:** Full keyboard navigation via native HTML inputs
- **Error Announcements:** Error states clearly visible with ARIA attributes
- **Screen Reader Friendly:** Proper label associations and semantic structure

---

## Usage Examples

### Form Integration
```rust
#[component]
fn MyForm() -> impl IntoView {
    let (agreed, set_agreed) = create_signal(false);
    let (selected, set_selected) = create_signal(String::new());
    
    view! {
        <div class="space-y-4">
            <Checkbox
                label="I agree to terms"
                checked=agreed
                on_change=move |v| set_agreed(v)
            />
            
            <RadioGroup
                label="Choose preference"
                value=selected
                options=vec![
                    RadioOption::new("opt1", "Option 1"),
                    RadioOption::new("opt2", "Option 2"),
                ]
                on_change=move |v| set_selected(v)
            />
        </div>
    }
}
```

### With Validation
```rust
<Checkbox
    label="Accept terms"
    checked=agreed
    error=if !agreed { Some("Required".into()) } else { None }
    on_change=move |v| set_agreed(v)
/>
```

### Complex Form
```rust
<div class="space-y-6">
    <Slider
        label="Volume"
        value=volume
        min=0.0
        max=100.0
        step=1.0
        show_value=true
        on_change=move |v| set_volume(v)
    />
    
    <RadioGroup
        label="Quality"
        value=quality
        options=vec![
            RadioOption::new("low", "Low"),
            RadioOption::new("med", "Medium"),
            RadioOption::new("high", "High"),
        ]
        on_change=move |v| set_quality(v)
    />
    
    <FileInput
        label="Upload file"
        accept="image/*"
        on_change=move |files| process_files(files)
    />
</div>
```

---

## Styleguide

View all components and their variants at `/styleguide/primitives` in the browser.

The styleguide includes:
- All component variants
- Enabled and disabled states
- Error state demonstrations
- Usage examples
- 36 component instances

---

## File Structure

```
components/primitives/
├── checkbox.rs          (3.1 KB)
├── radio_group.rs       (4.6 KB)
├── switch.rs            (3.4 KB)
├── slider.rs            (5.0 KB)
├── file_input.rs        (4.9 KB)
└── mod.rs               (exports)

routes/styleguide/
└── primitives.rs        (441 lines with examples)
```

---

## Integration Checklist

- ✅ All components exported from `primitives/mod.rs`
- ✅ Imports in styleguide working
- ✅ Format verification passed
- ✅ No breaking changes
- ✅ Comprehensive documentation
- ✅ Ready for form validation integration

---

## Next Steps

1. **View Components:** Visit `/styleguide/primitives` to see all variants
2. **Integrate:** Use in forms with validation systems
3. **Customize:** Adjust Tailwind colors as needed
4. **Test:** Add property-based tests for form patterns
5. **Document:** Add to project docs with form patterns

---

## Version
- Created: December 22, 2025
- Leptos Component Macro: ✅ Used
- Tailwind CSS: ✅ Exclusive
- Accessibility: ✅ Full ARIA
- Format: ✅ Verified (rustfmt)
