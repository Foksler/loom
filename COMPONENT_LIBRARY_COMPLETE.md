# Component Library - Complete Reference

**Last Updated:** December 22, 2025  
**Total Components:** 40+ UI Components across 6 tiers  
**Framework:** Leptos (Rust + WASM)  
**Styling:** Tailwind CSS

---

## 📚 Table of Contents

1. [Overview](#overview)
2. [Component Tiers](#component-tiers)
3. [Quick Start](#quick-start)
4. [Complete Component Index](#complete-component-index)
5. [Navigation Guide](#navigation-guide)
6. [Component Statistics](#component-statistics)

---

## Overview

Loom's component library is a comprehensive design system built with Leptos and Tailwind CSS. Components are organized into 6 tiers, from primitive UI elements to complex feature-specific components.

All components:
- ✅ Support reactive props and event handlers
- ✅ Built entirely with Tailwind CSS (no CSS-in-JS)
- ✅ Follow WCAG 2.1 accessibility guidelines
- ✅ Support responsive design patterns
- ✅ Provide typed enums for variants
- ✅ Include proper documentation and examples

---

## Component Tiers

### 🔷 Tier 1: Primitives (Core Design System)
**15 components** | Foundational UI elements | Direct Tailwind styling

The foundation of all other components. These are low-level, highly reusable elements that form the basis of the design system.

**Components:**
- `Button` - Interactive button with variants
- `TextField` - Text input field
- `TextArea` - Multi-line text input
- `Select` - Dropdown selection
- `MultiSelect` - Multiple value selection
- `Checkbox` - Single toggle checkbox
- `RadioGroup` - Grouped radio buttons
- `Toggle` - Binary toggle switch
- `Switch` - Animated toggle switch
- `Slider` - Numeric range slider
- `Card` - Container with padding and shadow
- `Badge` - Label with background
- `Chip` - Removable label/tag
- `Spinner` - Loading indicator
- `ProgressBar` - Progress visualization

**Key Features:**
- Atomic, single-responsibility components
- No dependencies on other components
- Extensive prop customization
- Standard HTML element wrappers

---

### 🟦 Tier 2: Layout (Structure & Composition)
**8 components** | Application structure | Responsive containers

Layout components provide structure for pages and regions. They establish visual hierarchy and enable responsive design patterns.

**Components:**
- `AppShell` - Main application layout wrapper
- `Panel` - Flexible container with optional title
- `Card` - Elevated container (also in Primitives)
- `FormSection` - Form inputs grouped with labels
- `FieldRow` - Horizontal field layout
- `KeyValueList` - Key-value pair display
- `DataTable` - Tabular data display
- `ResizablePanels` - Draggable panel dividers

**Key Features:**
- Responsive grid and flex layouts
- Semantic HTML structure
- Support for nested compositions
- Mobile-first design patterns

---

### 🟦 Tier 3: Chat Components
**7 components** | Conversation UI | Message rendering

Specialized components for building chat and conversation interfaces. Support streaming, user/assistant roles, and rich message formatting.

**Components:**
- `ConversationView` - Main chat container
- `MessageBubble` - Individual message container
- `MessageHeader` - Message metadata (role, timestamp)
- `MessageBody` - Message content with formatting
- `PromptComposer` - Text input + send button
- `StreamingCursor` - Typing indicator
- `ChatPlaceholder` - Empty state

**Key Features:**
- Streaming message support
- Role-based styling (user/assistant/system)
- Rich text formatting
- Conversation history display

---

### 🟫 Tier 4: Query Bridge Components
**6 components** | Query visualization | State tracing

Components for visualizing query execution, tool invocations, and state transitions. Used in query debugging and monitoring interfaces.

**Components:**
- `QueryTimeline` - Timeline of query execution steps
- `ToolInvocationList` - List of tool calls and results
- `StateMachineTrace` - State transition visualization
- `QueryPlaceholder` - Empty state for query panel

**Key Features:**
- Hierarchical data visualization
- Timeline and sequence display
- State transition animation
- Tool execution details

---

### 🟧 Tier 5: Results Components
**5 components** | Code and execution results | Rich output

Components for displaying execution results, code, diffs, and file trees. Support syntax highlighting and interactive features.

**Components:**
- `ExecutionResult` - Execution status and logs
- `CodeBlock` - Syntax-highlighted code display
- `DiffView` - Side-by-side diff visualization
- `FileTree` - Hierarchical file structure
- `LLMResultPanel` - LLM response and metadata

**Key Features:**
- Syntax highlighting with language detection
- Copy-to-clipboard functionality
- Collapsible code sections
- Error highlighting

---

### 🟪 Tier 6: Composite Components
**9+ components** | Feature-specific | Domain logic

Complex components combining multiple tiers for specific features. May include state management and business logic.

**Components:**
- `ThreadList` - List of conversation threads
- `ThreadListItem` - Single thread item with preview
- `ThreadHeader` - Thread metadata display
- `ThreadMetadataPanel` - Detailed thread information

**Key Features:**
- Integrated state management
- Business logic encapsulation
- Feature-complete functionality
- Domain-specific UI patterns

---

### 🟡 Tier 7: Indicators & Feedback
**3 components** | Status and feedback | User communication

Components for displaying status, loading states, and user feedback.

**Components:**
- `Badge` - Status indicator with color
- `Spinner` - Loading animation
- `ProgressBar` - Progress indication

**Key Features:**
- Status color coding
- Animation support
- Size variants
- Accessibility labels

---

## Quick Start

### Basic Button Usage
```rust
use loom_web::components::primitives::{Button, ButtonVariant};
use leptos::*;

#[component]
fn MyComponent() -> impl IntoView {
    view! {
        <Button>"Click me"</Button>
        <Button variant=ButtonVariant::Secondary>"Cancel"</Button>
        <Button variant=ButtonVariant::Destructive disabled=true>"Delete"</Button>
    }
}
```

### Building with Cards
```rust
use loom_web::components::primitives::{Card, CardElevation};

view! {
    <Card elevation=CardElevation::Lg>
        <h2>"Card Title"</h2>
        <p>"Card content goes here"</p>
    </Card>
}
```

### Chat Interface
```rust
use loom_web::components::chat::{ConversationView, MessageBubble};

view! {
    <ConversationView>
        <MessageBubble role="user" timestamp="10:30 AM">
            "Hello there!"
        </MessageBubble>
        <MessageBubble role="assistant" timestamp="10:31 AM">
            "Hi! How can I help?"
        </MessageBubble>
    </ConversationView>
}
```

---

## Complete Component Index

### Primitives (Tier 1)

#### Button
**File:** `src/components/primitives/button.rs`  
**Variants:** Primary, Secondary, Ghost, Destructive  
**Sizes:** Sm, Md, Lg  
**Features:** Loading state, disabled state, click handler

#### TextField
**File:** `src/components/primitives/text_field.rs`  
**Features:** Placeholder, disabled, validation, helper text

#### TextArea
**File:** `src/components/primitives/text_area.rs`  
**Features:** Rows control, placeholder, disabled state

#### Select
**File:** `src/components/primitives/select.rs`  
**Features:** Option list, selected value, disabled options

#### MultiSelect
**File:** `src/components/primitives/multi_select.rs`  
**Features:** Multiple selections, chip display, removable items

#### Checkbox
**File:** `src/components/primitives/checkbox.rs`  
**Features:** Checked state, indeterminate state, label

#### RadioGroup
**File:** `src/components/primitives/radio_group.rs`  
**Features:** Option list, selected value, vertical/horizontal layout

#### Toggle
**File:** `src/components/primitives/toggle.rs`  
**Features:** Active state, size variants

#### Switch
**File:** `src/components/primitives/switch.rs`  
**Features:** Animated toggle, labeled, disabled state

#### Slider
**File:** `src/components/primitives/slider.rs`  
**Features:** Min/max range, step control, value display

#### Card
**File:** `src/components/primitives/card.rs`  
**Elevations:** None, Sm, Md, Lg  
**Features:** Background color control

#### Badge
**File:** `src/components/primitives/badge.rs`  
**Variants:** Primary, Secondary, Success, Error, Warning  
**Sizes:** Sm, Md, Lg

#### Chip
**File:** `src/components/primitives/chip.rs`  
**Features:** Removable, selectable, custom children

#### Spinner
**File:** `src/components/primitives/spinner.rs`  
**Sizes:** Sm, Md, Lg  
**Features:** Custom color, animation

#### ProgressBar
**File:** `src/components/primitives/progress_bar.rs`  
**Features:** Percentage value, animated, color variants

---

### Layout (Tier 2)

#### AppShell
**File:** `src/components/layout/app_shell.rs`  
**Structure:** Header + Main content  
**Responsive:** Mobile, tablet, desktop

#### Panel
**File:** `src/components/layout/panel.rs`  
**Features:** Title bar, collapsible, custom header

#### FormSection
**File:** `src/components/layout/form_section.rs`  
**Features:** Label, description, input group

#### FieldRow
**File:** `src/components/layout/field_row.rs`  
**Features:** Horizontal layout, responsive stacking

#### KeyValueList
**File:** `src/components/layout/key_value_list.rs`  
**Features:** Key-value pairs, customizable formatting

#### DataTable
**File:** `src/components/layout/data_table.rs`  
**Features:** Sortable columns, pagination, filters

#### ResizablePanels
**File:** `src/components/layout/resizable_panels.rs`  
**Features:** Draggable dividers, persistent sizing

---

### Chat (Tier 3)

#### ConversationView
**File:** `src/components/chat/conversation_view.rs`  
**Features:** Message container, auto-scroll, loading states

#### MessageBubble
**File:** `src/components/chat/message_bubble.rs`  
**Roles:** User, Assistant, System  
**Features:** Timestamp, avatars, metadata

#### MessageHeader
**File:** `src/components/chat/message_header.rs`  
**Features:** Role indicator, timestamp, actions

#### MessageBody
**File:** `src/components/chat/message_body.rs`  
**Features:** Markdown rendering, code blocks, formatting

#### PromptComposer
**File:** `src/components/chat/prompt_composer.rs`  
**Features:** Text input, send button, multiline, file upload

#### StreamingCursor
**File:** `src/components/chat/streaming_cursor.rs`  
**Features:** Animated typing indicator, pulsing text

#### ChatPlaceholder
**File:** `src/components/chat/placeholder.rs`  
**Features:** Empty state, suggested prompts

---

### Query Bridge (Tier 4)

#### QueryTimeline
**File:** `src/components/query/query_timeline.rs`  
**Features:** Step visualization, timing info, status indicators

#### ToolInvocationList
**File:** `src/components/query/tool_invocation_list.rs`  
**Features:** Tool call list, parameters, results

#### StateMachineTrace
**File:** `src/components/query/state_machine_trace.rs`  
**Features:** State transitions, timeline, event details

#### QueryPlaceholder
**File:** `src/components/query/placeholder.rs`  
**Features:** Empty state, loading state

---

### Results (Tier 5)

#### ExecutionResult
**File:** `src/components/results/execution_result.rs`  
**Status:** Success, Error, Warning, Processing  
**Features:** Output display, error lists, status icons

#### CodeBlock
**File:** `src/components/results/code_block.rs`  
**Features:** Syntax highlighting, copy button, language detection

#### DiffView
**File:** `src/components/results/diff_view.rs`  
**Features:** Side-by-side diffs, line numbers, syntax highlighting

#### FileTree
**File:** `src/components/results/file_tree.rs`  
**Features:** Hierarchical display, expand/collapse, icons

#### LLMResultPanel
**File:** `src/components/results/llm_result_panel.rs`  
**Features:** Response display, token count, metadata

---

### Composite (Tier 6)

#### ThreadList
**File:** `src/components/threads/thread_list.rs`  
**Features:** Item list, filtering, pagination

#### ThreadListItem
**File:** `src/components/threads/thread_list_item.rs`  
**Features:** Preview, metadata, status

#### ThreadHeader
**File:** `src/components/threads/thread_header.rs`  
**Features:** Title, actions, status

#### ThreadMetadataPanel
**File:** `src/components/threads/thread_metadata_panel.rs`  
**Features:** Details view, timestamps, tags

---

## Navigation Guide

### By Use Case

**Building Forms:**
1. `FormSection` - Section wrapper
2. `FieldRow` - Horizontal layout
3. `TextField` / `TextArea` - Inputs
4. `Select` / `MultiSelect` - Selections
5. `Button` - Submission

**Creating Chat UI:**
1. `ConversationView` - Main container
2. `MessageBubble` - Message container
3. `MessageBody` - Rich content
4. `PromptComposer` - Input area
5. `StreamingCursor` - Loading state

**Displaying Results:**
1. `ExecutionResult` - Status wrapper
2. `CodeBlock` - Code display
3. `DiffView` - Code changes
4. `FileTree` - File navigation

**Building Layouts:**
1. `AppShell` - Top-level structure
2. `Panel` / `Card` - Content containers
3. `ResizablePanels` - Multi-panel layouts
4. `DataTable` - Data display

---

## Component Statistics

| Tier | Category | Components | Status |
|------|----------|-----------|--------|
| 1 | Primitives | 15 | ✅ Complete |
| 2 | Layout | 8 | ✅ Complete |
| 3 | Chat | 7 | ✅ Complete |
| 4 | Query Bridge | 4 | ✅ Complete |
| 5 | Results | 5 | ✅ Complete |
| 6 | Composite | 4+ | ✅ Complete |
| 7 | Indicators | 3 | ✅ Complete |
| **Total** | **All** | **40+** | **✅ Complete** |

---

## Key Features Across All Components

### 🎨 Design System
- **Consistent spacing:** 4px base unit with Tailwind
- **Color palette:** Gray-900 to Gray-50, brand colors (blue, red, green)
- **Typography:** Semantic font sizes and weights
- **Shadows:** Elevation levels for depth
- **Borders:** Consistent border-radius and colors

### ♿ Accessibility
- **WCAG 2.1 Level AA** compliant
- **Keyboard navigation** support on all interactive elements
- **ARIA labels** and roles where applicable
- **Focus indicators** for keyboard users
- **Screen reader** friendly markup

### 📱 Responsive Design
- **Mobile-first** approach
- **Flexible layouts** that adapt to screen size
- **Touch-friendly** components with adequate spacing
- **Breakpoint support** via Tailwind

### ⚡ Performance
- **Zero runtime CSS-in-JS** (pure Tailwind)
- **Lazy loading** for large component lists
- **Minimal re-renders** via Leptos reactivity
- **No external dependencies** beyond Tailwind

---

## Documentation Links

- 📖 **[Usage Guide](file:///COMPONENTS_USAGE_GUIDE.md)** - How to use components effectively
- 🏗️ **[Architecture Guide](file:///COMPONENT_ARCHITECTURE.md)** - Design patterns and composition
- 🔗 **[API Reference](file:///API_REFERENCE_COMPLETE.md)** - Complete prop documentation
- 🎨 **[Styleguide](file:///STYLEGUIDE_GUIDE.md)** - Visual component showcase at /styleguide

---

## Getting Started

### Installation
Components are in the `loom_web` crate:
```rust
use loom_web::components::primitives::*;
use loom_web::components::layout::*;
use loom_web::components::chat::*;
```

### First Component
```rust
use loom_web::components::primitives::Button;

#[component]
fn App() -> impl IntoView {
    view! {
        <Button>"Hello World"</Button>
    }
}
```

### Next Steps
1. Read the [Usage Guide](file:///COMPONENTS_USAGE_GUIDE.md)
2. Explore the [Styleguide](/styleguide)
3. Review [Architecture patterns](file:///COMPONENT_ARCHITECTURE.md)
4. Check [API Reference](file:///API_REFERENCE_COMPLETE.md) for details

---

## Support & Contribution

### Reporting Issues
- Report bugs in component behavior
- Include example reproduction code
- Specify Leptos and Tailwind versions

### Contributing
- Follow existing patterns in tier-appropriate location
- Add comprehensive documentation
- Include property-based tests
- Update this index

### Code Style
- Use semantic HTML
- Leverage Tailwind utilities
- Document all public props
- Provide example usage in doc comments

---

**Last Updated:** December 22, 2025  
**Maintained By:** Loom Team  
**License:** Project License
