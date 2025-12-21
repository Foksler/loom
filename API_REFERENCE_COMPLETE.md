# Complete Component API Reference

**Comprehensive prop documentation for all 40+ components**

---

## Table of Contents

1. [Primitives (Tier 1)](#primitives-tier-1)
2. [Layout (Tier 2)](#layout-tier-2)
3. [Chat (Tier 3)](#chat-tier-3)
4. [Query Bridge (Tier 4)](#query-bridge-tier-4)
5. [Results (Tier 5)](#results-tier-5)
6. [Composite (Tier 6)](#composite-tier-6)
7. [Type Definitions](#type-definitions)

---

## Primitives (Tier 1)

### Button

Interactive button element with multiple variants and sizes.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `variant` | `ButtonVariant` | `Primary` | Visual style variant |
| `size` | `ButtonSize` | `Md` | Button size |
| `disabled` | `bool` | `false` | Disable button interaction |
| `loading` | `bool` | `false` | Show loading spinner |
| `children` | `Children` | Required | Button text/content |
| `on_click` | `Option<Fn(MouseEvent)>` | `None` | Click handler |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Variants:**
```rust
pub enum ButtonVariant {
    Primary,      // Solid brand color, high-emphasis
    Secondary,    // Muted background, secondary action
    Ghost,        // No background, text only
    Destructive,  // Red/destructive styling
}
```

**Sizes:**
```rust
pub enum ButtonSize {
    Sm,  // Small padding and text (px-2.5 py-1.5 text-sm)
    Md,  // Default size (px-3.5 py-2 text-sm)
    Lg,  // Large padding and text (px-4 py-2.5 text-base)
}
```

**Example:**
```rust
view! {
    <Button variant=ButtonVariant::Primary size=ButtonSize::Md on_click=handler>
        "Click me"
    </Button>
}
```

---

### TextField

Single-line text input field.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `String` | `""` | Current input value |
| `on_input` | `Fn(String)` | Required | Input change handler |
| `placeholder` | `Option<String>` | `None` | Placeholder text |
| `disabled` | `bool` | `false` | Disable input |
| `input_type` | `&str` | `"text"` | HTML input type |
| `aria_label` | `Option<String>` | `None` | Accessibility label |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
let (value, set_value) = create_signal(String::new());

view! {
    <TextField
        value=value.get()
        on_input=move |v| set_value(v)
        placeholder="Enter text..."
        aria_label="Name input"
    />
}
```

---

### TextArea

Multi-line text input field.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `String` | `""` | Current text value |
| `on_input` | `Fn(String)` | Required | Change handler |
| `placeholder` | `Option<String>` | `None` | Placeholder text |
| `rows` | `Option<usize>` | `4` | Number of visible rows |
| `disabled` | `bool` | `false` | Disable input |
| `aria_label` | `Option<String>` | `None` | Accessibility label |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <TextArea
        value=description.get()
        on_input=move |v| set_description(v)
        rows=6
        placeholder="Enter description..."
    />
}
```

---

### Select

Dropdown selection from predefined options.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `options` | `Vec<(T, String)>` | Required | Options as (value, label) pairs |
| `selected` | `Option<T>` | `None` | Selected value |
| `on_change` | `Fn(T)` | Required | Selection change handler |
| `placeholder` | `Option<String>` | `None` | Placeholder text |
| `disabled` | `bool` | `false` | Disable selection |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
let (selected, set_selected) = create_signal(None::<String>);

view! {
    <Select
        options=vec![
            ("opt1".to_string(), "Option 1".to_string()),
            ("opt2".to_string(), "Option 2".to_string()),
        ]
        selected=selected.get()
        on_change=move |v| set_selected(Some(v))
        placeholder="Choose option..."
    />
}
```

---

### MultiSelect

Multiple value selection component.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `options` | `Vec<(T, String)>` | Required | Available options |
| `selected` | `Vec<T>` | `vec![]` | Selected values |
| `on_change` | `Fn(Vec<T>)` | Required | Change handler |
| `placeholder` | `Option<String>` | `None` | Placeholder text |
| `disabled` | `bool` | `false` | Disable selection |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
let (tags, set_tags) = create_signal(vec![]);

view! {
    <MultiSelect
        options=vec![
            ("rust".to_string(), "Rust".to_string()),
            ("web".to_string(), "Web".to_string()),
        ]
        selected=tags.get()
        on_change=move |v| set_tags(v)
    />
}
```

---

### Checkbox

Single boolean toggle checkbox.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `checked` | `bool` | `false` | Checked state |
| `on_change` | `Fn(bool)` | Required | State change handler |
| `disabled` | `bool` | `false` | Disable checkbox |
| `aria_label` | `Option<String>` | `None` | Accessibility label |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
let (agreed, set_agreed) = create_signal(false);

view! {
    <Checkbox
        checked=agreed.get()
        on_change=move |v| set_agreed(v)
        aria_label="I agree to terms"
    />
    <label>"I agree to the terms and conditions"</label>
}
```

---

### RadioGroup

Grouped radio buttons for single selection.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `options` | `Vec<(T, String)>` | Required | Radio options |
| `selected` | `Option<T>` | `None` | Selected value |
| `on_change` | `Fn(T)` | Required | Selection handler |
| `disabled` | `bool` | `false` | Disable all radios |
| `vertical` | `bool` | `true` | Layout direction |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <RadioGroup
        options=vec![
            ("yes".to_string(), "Yes".to_string()),
            ("no".to_string(), "No".to_string()),
        ]
        selected=answer.get()
        on_change=move |v| set_answer(Some(v))
        vertical=true
    />
}
```

---

### Toggle

Binary toggle switch component.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `active` | `bool` | `false` | Active state |
| `on_change` | `Fn(bool)` | Required | Change handler |
| `disabled` | `bool` | `false` | Disable toggle |
| `size` | `Option<String>` | `"md"` | Size: "sm", "md", "lg" |
| `aria_label` | `Option<String>` | `None` | Accessibility label |

**Example:**
```rust
view! {
    <Toggle
        active=feature_enabled.get()
        on_change=move |v| set_feature_enabled(v)
        aria_label="Enable feature"
        size="md"
    />
}
```

---

### Switch

Animated toggle switch component.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `checked` | `bool` | `false` | Checked state |
| `on_change` | `Fn(bool)` | Required | Change handler |
| `disabled` | `bool` | `false` | Disable switch |
| `label` | `Option<String>` | `None` | Label text |
| `aria_label` | `Option<String>` | `None` | Accessibility label |

**Example:**
```rust
view! {
    <Switch
        checked=dark_mode.get()
        on_change=move |v| set_dark_mode(v)
        label="Dark Mode"
    />
}
```

---

### Slider

Numeric range slider component.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `f64` | `50` | Current value |
| `on_change` | `Fn(f64)` | Required | Change handler |
| `min` | `f64` | `0` | Minimum value |
| `max` | `f64` | `100` | Maximum value |
| `step` | `f64` | `1` | Step increment |
| `disabled` | `bool` | `false` | Disable slider |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <Slider
        value=volume.get()
        on_change=move |v| set_volume(v)
        min=0.0
        max=100.0
        step=1.0
    />
    <p>"Volume: " {volume}</p>
}
```

---

### Card

Container with padding, border, and optional shadow.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `elevation` | `CardElevation` | `Md` | Shadow level |
| `white_bg` | `bool` | `true` | Use white background |
| `children` | `Children` | Required | Card content |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Elevations:**
```rust
pub enum CardElevation {
    None,  // No shadow (flat appearance)
    Sm,    // Subtle shadow
    Md,    // Medium shadow (default)
    Lg,    // Strong shadow
}
```

**Example:**
```rust
view! {
    <Card elevation=CardElevation::Lg>
        <h2>"Card Title"</h2>
        <p>"Card content"</p>
    </Card>
}
```

---

### Badge

Small label with background color for status/tags.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `variant` | `BadgeVariant` | `Primary` | Badge style |
| `size` | `BadgeSize` | `Md` | Badge size |
| `children` | `Children` | Required | Badge text |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Variants:**
```rust
pub enum BadgeVariant {
    Primary,    // Brand color
    Secondary,  // Gray background
    Success,    // Green (success)
    Error,      // Red (error)
    Warning,    // Amber (warning)
}

pub enum BadgeSize {
    Sm,  // Small (px-2 py-1)
    Md,  // Medium (px-3 py-2)
    Lg,  // Large (px-4 py-3)
}
```

**Example:**
```rust
view! {
    <Badge variant=BadgeVariant::Success>"Active"</Badge>
    <Badge variant=BadgeVariant::Error size=BadgeSize::Lg>"Failed"</Badge>
}
```

---

### Chip

Removable label/tag component.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `label` | `String` | Required | Chip text |
| `on_remove` | `Option<Fn()>` | `None` | Remove handler |
| `removable` | `bool` | `true` | Show remove button |
| `selectable` | `bool` | `false` | Allow selection |
| `selected` | `bool` | `false` | Selected state |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <Chip
        label="rust"
        on_remove=Some(move || remove_tag("rust"))
        removable=true
    />
}
```

---

### Spinner

Loading indicator animation.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `size` | `SpinnerSize` | `Md` | Spinner size |
| `color` | `Option<String>` | `None` | Color class |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Sizes:**
```rust
pub enum SpinnerSize {
    Sm,  // Small (w-4 h-4)
    Md,  // Medium (w-6 h-6)
    Lg,  // Large (w-8 h-8)
}
```

**Example:**
```rust
view! {
    <div class="flex gap-4">
        <Spinner size=SpinnerSize::Sm/>
        <Spinner size=SpinnerSize::Md/>
        <Spinner size=SpinnerSize::Lg/>
    </div>
}
```

---

### ProgressBar

Progress indication bar.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `f64` | Required | Progress 0-100 |
| `animated` | `bool` | `true` | Animated stripe |
| `variant` | `Option<String>` | `None` | Color variant |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <ProgressBar value=75.0 animated=true/>
    <p>"75% complete"</p>
}
```

---

## Layout (Tier 2)

### AppShell

Main application layout wrapper with header and content area.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `children` | `Children` | Required | Main content |

**Example:**
```rust
view! {
    <AppShell>
        <main class="flex-1 overflow-auto">
            <Content/>
        </main>
    </AppShell>
}
```

---

### Panel

Flexible container with optional header and collapsible behavior.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `title` | `Option<String>` | `None` | Panel title |
| `collapsible` | `bool` | `false` | Allow collapse |
| `children` | `Children` | Required | Panel content |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <Panel title="Details" collapsible=true>
        <p>"Panel content"</p>
    </Panel>
}
```

---

### FormSection

Group related form inputs with label and help text.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `label` | `String` | Required | Section label |
| `children` | `Children` | Required | Form inputs |
| `help_text` | `Option<String>` | `None` | Helper text |
| `required` | `bool` | `false` | Show required indicator |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <FormSection
        label="Personal Information"
        help_text="Please enter your details"
        required=true
    >
        <TextField placeholder="Name"/>
        <TextField placeholder="Email"/>
    </FormSection>
}
```

---

### FieldRow

Horizontal layout for side-by-side fields with responsive stacking.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `children` | `Children` | Required | Fields to arrange |
| `gap` | `Option<String>` | `"4"` | Gap between fields |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <FieldRow gap="4">
        <TextField placeholder="First Name"/>
        <TextField placeholder="Last Name"/>
    </FieldRow>
}
```

---

### KeyValueList

Display key-value pairs in structured format.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `items` | `Vec<(String, String)>` | Required | Key-value pairs |
| `variant` | `Option<String>` | `None` | Display variant |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <KeyValueList
        items=vec![
            ("Name".to_string(), "John Doe".to_string()),
            ("Email".to_string(), "john@example.com".to_string()),
        ]
    />
}
```

---

### DataTable

Tabular data display with sorting and pagination.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `columns` | `Vec<String>` | Required | Column headers |
| `data` | `Vec<Vec<String>>` | Required | Table rows |
| `sortable` | `bool` | `true` | Enable sorting |
| `paginated` | `bool` | `false` | Enable pagination |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <DataTable
        columns=vec!["Name".to_string(), "Email".to_string()]
        data=vec![
            vec!["John".to_string(), "john@example.com".to_string()],
            vec!["Jane".to_string(), "jane@example.com".to_string()],
        ]
        sortable=true
    />
}
```

---

### ResizablePanels

Two-panel layout with draggable divider for resizing.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `left` | `Children` | Required | Left panel content |
| `right` | `Children` | Required | Right panel content |
| `initial_split` | `f64` | `50.0` | Initial split percentage |
| `min_size` | `f64` | `20.0` | Minimum panel width % |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <ResizablePanels
        initial_split=60.0
        min_size=25.0
    >
        left=view! { <Sidebar/> }
        right=view! { <MainContent/> }
    </ResizablePanels>
}
```

---

## Chat (Tier 3)

### ConversationView

Main container for chat conversation display.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `children` | `Children` | Required | Message elements |
| `auto_scroll` | `bool` | `true` | Auto-scroll to bottom |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

### MessageBubble

Individual message container with role-based styling.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `role` | `&str` | `"user"` | Message role: user/assistant/system |
| `timestamp` | `Option<String>` | `None` | Message timestamp |
| `children` | `Children` | Required | Message content |
| `avatar` | `Option<String>` | `None` | Avatar URL |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <MessageBubble role="user" timestamp="10:30 AM">
        "Hello there!"
    </MessageBubble>
    <MessageBubble role="assistant" timestamp="10:31 AM">
        "Hi! How can I help?"
    </MessageBubble>
}
```

---

### MessageBody

Message content with formatting, code blocks, and markdown.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `content` | `String` | Required | Message text |
| `allow_html` | `bool` | `false` | Allow HTML rendering |
| `code_highlight` | `bool` | `true` | Highlight code blocks |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

### PromptComposer

Text input + send button for message composition.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `placeholder` | `Option<String>` | `None` | Input placeholder |
| `on_submit` | `Fn(String)` | Required | Submit handler |
| `disabled` | `bool` | `false` | Disable input |
| `multiline` | `bool` | `true` | Allow multiple lines |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <PromptComposer
        placeholder="Type a message..."
        on_submit=move |text| send_message(text)
        multiline=true
    />
}
```

---

### StreamingCursor

Animated typing indicator for streaming messages.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `visible` | `bool` | `true` | Show cursor |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

### ChatPlaceholder

Empty state for empty conversation view.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `title` | `Option<String>` | `None` | Placeholder title |
| `suggestions` | `Vec<String>` | `vec![]` | Suggested prompts |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

## Query Bridge (Tier 4)

### QueryTimeline

Timeline visualization of query execution steps.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `steps` | `Vec<TimelineStep>` | Required | Execution steps |
| `current` | `Option<usize>` | `None` | Current step index |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**TimelineStep Type:**
```rust
pub struct TimelineStep {
    pub id: String,
    pub label: String,
    pub status: ExecutionStatus, // Success, Error, Warning, Processing
    pub duration_ms: Option<u64>,
    pub details: Option<String>,
}
```

---

### ToolInvocationList

List display of tool calls and results.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `invocations` | `Vec<ToolCall>` | Required | Tool invocations |
| `collapsible` | `bool` | `true` | Allow collapse |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

### StateMachineTrace

State transition visualization.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `states` | `Vec<StateInfo>` | Required | State information |
| `transitions` | `Vec<Transition>` | Required | State transitions |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

## Results (Tier 5)

### ExecutionResult

Execution status and output display.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `status` | `ExecutionStatus` | Required | Result status |
| `output` | `String` | `""` | Output/stdout |
| `errors` | `Vec<String>` | `vec![]` | Error messages |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**ExecutionStatus Type:**
```rust
pub enum ExecutionStatus {
    Success,    // Green indicator
    Error,      // Red indicator
    Warning,    // Amber indicator
    Processing, // Blue spinner
}
```

---

### CodeBlock

Syntax-highlighted code display.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `code` | `String` | Required | Code content |
| `language` | `Option<String>` | `None` | Language hint |
| `copy_button` | `bool` | `true` | Show copy button |
| `line_numbers` | `bool` | `true` | Show line numbers |
| `collapsible` | `bool` | `false` | Allow collapse |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**Example:**
```rust
view! {
    <CodeBlock
        code="fn main() { println!(\"Hello!\"); }".to_string()
        language="rust"
        copy_button=true
        line_numbers=true
    />
}
```

---

### DiffView

Side-by-side diff visualization.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `old_code` | `String` | Required | Before code |
| `new_code` | `String` | Required | After code |
| `language` | `Option<String>` | `None` | Language hint |
| `unified` | `bool` | `false` | Unified diff format |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

### FileTree

Hierarchical file structure display.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `files` | `Vec<FileNode>` | Required | File structure |
| `expanded_by_default` | `bool` | `false` | Expand folders |
| `on_select` | `Option<Fn(String)>` | `None` | File select handler |
| `class` | `Option<String>` | `None` | Additional CSS classes |

**FileNode Type:**
```rust
pub struct FileNode {
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub children: Vec<FileNode>,
}
```

---

### LLMResultPanel

LLM response display with metadata.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `response` | `String` | Required | LLM response |
| `model` | `Option<String>` | `None` | Model name |
| `tokens` | `Option<(u32, u32)>` | `None` | (input, output) token counts |
| `temperature` | `Option<f64>` | `None` | Temperature used |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

## Composite (Tier 6)

### ThreadList

Scrollable list of conversation threads.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `threads` | `Vec<Thread>` | Required | Thread items |
| `selected` | `Option<String>` | `None` | Selected thread ID |
| `on_select` | `Fn(String)` | Required | Selection handler |
| `on_delete` | `Option<Fn(String)>` | `None` | Delete handler |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

### ThreadListItem

Single thread preview in list.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `id` | `String` | Required | Thread ID |
| `title` | `String` | Required | Thread title |
| `preview` | `String` | Required | Preview text |
| `timestamp` | `String` | Required | Last update time |
| `selected` | `bool` | `false` | Selection state |
| `on_click` | `Fn()` | Required | Click handler |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

### ThreadHeader

Thread title and metadata display.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `title` | `String` | Required | Thread title |
| `subtitle` | `Option<String>` | `None` | Subtitle text |
| `created_at` | `String` | Required | Creation timestamp |
| `actions` | `Option<Children>` | `None` | Action buttons |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

### ThreadMetadataPanel

Detailed thread information display.

**Props:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `thread_id` | `String` | Required | Thread ID |
| `created_at` | `String` | Required | Creation timestamp |
| `updated_at` | `String` | Required | Last updated timestamp |
| `tags` | `Vec<String>` | `vec![]` | Associated tags |
| `class` | `Option<String>` | `None` | Additional CSS classes |

---

## Type Definitions

### ExecutionStatus

```rust
pub enum ExecutionStatus {
    Success,
    Error,
    Warning,
    Processing,
}

impl ExecutionStatus {
    pub fn color_class(&self) -> &'static str {
        match self {
            Self::Success => "bg-green-50 border-green-200 text-green-900",
            Self::Error => "bg-red-50 border-red-200 text-red-900",
            Self::Warning => "bg-amber-50 border-amber-200 text-amber-900",
            Self::Processing => "bg-blue-50 border-blue-200 text-blue-900",
        }
    }
}
```

### ButtonVariant

```rust
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Destructive,
}
```

### BadgeVariant

```rust
pub enum BadgeVariant {
    Primary,
    Secondary,
    Success,
    Error,
    Warning,
}
```

### CardElevation

```rust
pub enum CardElevation {
    None,
    Sm,
    Md,
    Lg,
}
```

### TimelineStep

```rust
pub struct TimelineStep {
    pub id: String,
    pub label: String,
    pub status: ExecutionStatus,
    pub duration_ms: Option<u64>,
    pub details: Option<String>,
}
```

### FileNode

```rust
pub struct FileNode {
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub children: Vec<FileNode>,
}
```

---

## Index by Prop Type

### Signals/Reactive
- Most components accept `Signal<T>` for reactive props
- Use `.get()` to read current value in move closures

### Handlers
- `on_click: Option<Fn(MouseEvent)>`
- `on_input: Fn(String)`
- `on_change: Fn(T)`
- `on_submit: Fn(String)`

### Styling
- All components accept optional `class: Option<String>` for custom classes
- Use Tailwind utilities to extend styles
- Never use `!important` or inline styles

---

**Last Updated:** December 22, 2025
