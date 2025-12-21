# Composite Components Quick Reference

## 🎯 Component Library Overview

Five production-ready composite components built with Leptos and Tailwind CSS.

| Component | Purpose | Example Use |
|-----------|---------|------------|
| **DataTable** | Sortable, paginated tables | User lists, transaction history |
| **KeyValueList** | Key-value pair display | Configuration, metadata, settings |
| **FormSection** | Grouped form fields | Login forms, settings panels |
| **FieldRow** | Label + input wrapper | Individual form fields |
| **ResizablePanels** | Draggable split panes | Code editor, file browser layouts |

---

## 📦 Imports

```rust
use crate::components::layout::{
    Column, DataTable, SortDirection,
    FieldRow, FormSection,
    KeyValueList,
    ResizablePanels,
};
```

---

## 🚀 Quick Start

### DataTable
```rust
let columns = vec![
    Column { 
        id: "name".into(), 
        label: "Name".into(), 
        width: None, 
        sortable: true 
    },
    Column { 
        id: "email".into(), 
        label: "Email".into(), 
        width: None, 
        sortable: true 
    },
];

let rows = vec![
    vec!["Alice".into(), "alice@example.com".into()],
    vec!["Bob".into(), "bob@example.com".into()],
];

view! {
    <DataTable
        columns=columns
        rows=rows
        sortable=true
        pagination=true
        page_size=10
    />
}
```

**Key Props**:
- `columns: Vec<Column>` - Column definitions
- `rows: Vec<Vec<String>>` - Table data
- `sortable: bool` - Enable column sorting
- `pagination: bool` - Show pagination controls
- `page_size: usize` - Rows per page

---

### KeyValueList
```rust
let items = vec![
    ("API Key".into(), "sk-1234567...".into()),
    ("Model".into(), "GPT-4".into()),
    ("Temperature".into(), "0.7".into()),
];

view! {
    <KeyValueList
        items=items
        label_width=Some("120px".into())
        striped=true
        dense=false
    />
}
```

**Key Props**:
- `items: Vec<(String, String)>` - Key-value pairs
- `label_width: Option<String>` - Fixed label column width
- `striped: bool` - Alternating row colors
- `dense: bool` - Compact layout

---

### FormSection + FieldRow
```rust
view! {
    <FormSection
        title="User Profile".into()
        description=Some("Update your account details".into())
        error=None
        disabled=false
    >
        <FieldRow 
            label="Full Name".into()
            required=true
            error=None
            description=Some("Your legal name".into())
            horizontal=false
        >
            <input 
                type="text" 
                class="w-full px-3 py-2 border border-gray-300 rounded"
            />
        </FieldRow>

        <FieldRow 
            label="Email".into()
            required=true
            error=Some("Invalid email format".into())
        >
            <input 
                type="email" 
                class="w-full px-3 py-2 border border-red-500 rounded"
            />
        </FieldRow>
    </FormSection>
}
```

**FormSection Props**:
- `title: String` - Section heading
- `description: Option<String>` - Help text
- `error: Option<String>` - Section-level error
- `disabled: bool` - Disable all children

**FieldRow Props**:
- `label: String` - Field label
- `required: bool` - Show required indicator
- `error: Option<String>` - Field error message
- `description: Option<String>` - Helper text
- `horizontal: bool` - Layout orientation

---

### ResizablePanels
```rust
view! {
    <div style="height: 500px">
        <ResizablePanels
            initial_width=50
            min_width=20
            resizable=true
            left_panel=move || view! {
                <div class="p-6">
                    <h3>"Files"</h3>
                    <FileTree />
                </div>
            }
            right_panel=move || view! {
                <div class="p-6">
                    <h3>"Code"</h3>
                    <CodeEditor />
                </div>
            }
        />
    </div>
}
```

**Key Props**:
- `left_panel: Fn() -> View` - Left panel content
- `right_panel: Fn() -> View` - Right panel content
- `initial_width: u32` - Initial left panel % (0-100)
- `min_width: u32` - Minimum panel width %
- `resizable: bool` - Enable dragging

**Keyboard Support**:
- `←` / `↑` - Decrease left panel width
- `→` / `↓` - Increase left panel width
- Mouse drag divider for continuous resize

---

## 🎨 Styling Notes

### Tailwind Classes Used
- **Colors**: `gray-50` to `gray-900`, `blue-*`, `red-*`
- **Layout**: `flex`, `grid`, `border`, `rounded`
- **Spacing**: `p-*`, `m-*`, `gap-*`
- **Effects**: `shadow`, `hover:`, `transition`

### Custom Styling
All components accept optional `class` prop for appending CSS:

```rust
<DataTable
    columns=cols
    rows=rows
    class=Some("max-w-4xl".into())
/>
```

---

## 🔧 Common Patterns

### Loading State (DataTable)
```rust
let (loading, set_loading) = create_signal(true);

view! {
    {move || if loading.get() {
        view! { <Spinner /> }
    } else {
        view! {
            <DataTable columns=cols rows=rows />
        }
    }}
}
```

### Error Handling (FormSection)
```rust
let (error, set_error) = create_signal::<Option<String>>(None);

view! {
    <FormSection
        title="Submit".into()
        error=error.get()
    >
        {/* form fields */}
    </FormSection>
}
```

### Dynamic Data (KeyValueList)
```rust
let (config, set_config) = create_signal(load_config());

view! {
    <KeyValueList
        items=config.get().to_pairs()
        striped=true
    />
}
```

---

## 📍 File Locations

```
crates/loom-web/src/components/layout/
├── data_table.rs          (278 lines) - Sortable, paginated tables
├── key_value_list.rs      (95 lines)  - Key-value displays
├── form_section.rs        (77 lines)  - Form grouping
├── field_row.rs           (115 lines) - Field wrappers
├── resizable_panels.rs    (195 lines) - Split panes
└── mod.rs                 (14 lines)  - Exports
```

Styleguide examples: `crates/loom-web/src/routes/styleguide/layout.rs`

---

## 🔍 Type Definitions

### Column
```rust
pub struct Column {
    pub id: String,              // Unique identifier
    pub label: String,           // Display text
    pub width: Option<String>,   // CSS width (e.g., "150px")
    pub sortable: bool,          // Allow sorting
}
```

### SortDirection
```rust
pub enum SortDirection {
    Asc,   // Ascending
    Desc,  // Descending
}
```

### DataTableState (Internal)
```rust
pub struct DataTableState {
    pub sort_column: Option<String>,
    pub sort_direction: SortDirection,
    pub current_page: usize,
    pub page_size: usize,
}
```

---

## ✅ Accessibility Features

- ✅ Semantic HTML (`<dl>`, `<dt>`, `<dd>`, `<label>`, `<table>`)
- ✅ ARIA attributes (ResizablePanels divider)
- ✅ Keyboard navigation (Form fields, ResizablePanels)
- ✅ Focus visible states
- ✅ Color contrast compliance
- ✅ Error message associations

---

## 🧪 Testing Tips

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_sorting_works() {
        // Test sort direction toggling
    }

    #[test]
    fn pagination_boundary_conditions() {
        // Test first/last page transitions
    }

    #[test]
    fn form_validation_displays_errors() {
        // Test error message rendering
    }
}
```

---

## 🚦 Performance Considerations

1. **DataTable**: 
   - Sorting is O(n log n) via built-in Vec::sort_by
   - Pagination slicing is O(1)
   - Use `page_size` to limit rendered rows

2. **KeyValueList**:
   - O(n) to render, minimal overhead
   - Efficient for <1000 items

3. **ResizablePanels**:
   - Efficient drag handler
   - Mouse events debounced by browser
   - Minimal rerender on mouse move

---

## 🔗 Integration with Primitives

Components can nest primitives:

```rust
<FormSection title="Settings".into()>
    <FieldRow label="Visibility".into()>
        <Select options=visibility_options />
    </FieldRow>
    <FieldRow label="Notifications".into()>
        <Toggle checked=true />
    </FieldRow>
</FormSection>
```

---

## 📚 Documentation

- Full API docs: See inline comments in `crates/loom-web/src/components/layout/*.rs`
- Examples: `crates/loom-web/src/routes/styleguide/layout.rs`
- Summary: `COMPOSITE_COMPONENTS_SUMMARY.md`

---

## ⚡ Next Steps

1. Add property-based tests with `proptest`
2. Create examples/ directory with standalone demos
3. Add TypeScript type definitions (if needed)
4. Performance profiling with large datasets
5. Mobile responsiveness testing
6. Accessibility audit with WAVE

---

**Last Updated**: Dec 22, 2025
**Version**: 1.0
**Status**: Production Ready ✅
