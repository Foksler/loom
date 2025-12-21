# Query Bridge Visualization Components Implementation

## Overview

Implemented a complete set of visualization components for the query bridge state machine/LLM processing pipeline. These components enable users to observe and understand how queries flow through the system, from initial parsing through LLM execution to final response formatting.

## Architecture

```
components/query/
├── mod.rs                      # Module exports
├── types.rs                    # Type definitions
├── query_timeline.rs           # Timeline visualization
├── tool_invocation_list.rs     # Tool call tracking
├── state_machine_trace.rs      # State transition history
└── placeholder.rs              # Existing placeholder
```

## Components Implemented

### 1. **QueryTimeline & QueryStepCard**
- **File**: [query_timeline.rs](file:///home/ghuntley/loom/crates/loom-web/src/components/query/query_timeline.rs)
- **Purpose**: Vertical timeline showing query execution progress
- **Features**:
  - Step cards with status indicators (pending, running, completed, error)
  - Animated spinner for running steps
  - Color-coded status badges
  - Timeline connector lines between steps
  - Expandable details (logs, inputs/outputs)
  - JSON pretty-printing for structured data
  
**Usage**:
```rust
let steps = vec![
    QueryStep::new("Parse Query")
        .with_description("Parsing user input")
        .with_log("Query received"),
];
view! {
    <QueryTimeline steps=steps.into() current_step=0 />
}
```

### 2. **ToolInvocationList & ToolInvocationItem**
- **File**: [tool_invocation_list.rs](file:///home/ghuntley/loom/crates/loom-web/src/components/query/tool_invocation_list.rs)
- **Purpose**: Display all tool calls made during query execution
- **Features**:
  - Individual cards per tool invocation
  - Execution time display and total time calculation
  - Tool name with numbered badge
  - Expandable arguments (JSON) and result view
  - Execution metrics highlighted
  - Empty state when no tools were invoked

**Usage**:
```rust
let invocations = vec![
    ToolInvocation::new(
        "search",
        json!({"query": "test"}),
        "Found 5 results",
        150
    ),
];
view! {
    <ToolInvocationList invocations=invocations.into() />
}
```

### 3. **StateMachineTrace & StateTransitionItem**
- **File**: [state_machine_trace.rs](file:///home/ghuntley/loom/crates/loom-web/src/components/query/state_machine_trace.rs)
- **Purpose**: Track and display state transitions in the query pipeline
- **Features**:
  - Numbered state circles with borders
  - State transition arrows
  - Event names that triggered transitions
  - Timestamp display (ISO 8601 format)
  - Expandable details showing full timestamp and index
  - Batch show/hide for all transitions
  - Timeline connector lines

**Usage**:
```rust
let trace = vec![
    StateTransition::new("idle", "parsing", "2024-01-01T10:00:00Z", "query_received"),
    StateTransition::new("parsing", "executing", "2024-01-01T10:00:01Z", "parse_complete"),
];
view! {
    <StateMachineTrace trace=trace.into() expanded=false />
}
```

## Type Definitions

### QueryStep
```rust
pub struct QueryStep {
    pub name: String,
    pub description: Option<String>,
    pub logs: Vec<String>,
    pub inputs: Option<serde_json::Value>,
    pub outputs: Option<serde_json::Value>,
}
```
**Builder Methods**:
- `new(name)` - Create step
- `with_description(desc)` - Add description
- `with_log(msg)` - Add log message
- `with_inputs(json)` - Set input data
- `with_outputs(json)` - Set output data

### ToolInvocation
```rust
pub struct ToolInvocation {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: String,
    pub execution_ms: u32,
}
```

### StateTransition
```rust
pub struct StateTransition {
    pub from_state: String,
    pub to_state: String,
    pub timestamp: String,        // ISO 8601 format
    pub event: String,
}
```

### StepStatus (Enum)
```rust
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Error(String),
}
```

## Design System Integration

### Primitives Used
- **Card**: Container for all major components with elevation support
- **Badge**: Status indicators (Green=Success, Red=Error, Blue=Info, etc.)
- **Spinner**: Running state indicator (animated)
- **Custom styling**: Tailwind CSS for all layout and theming

### Color Coding
- **Gray**: Pending/neutral states
- **Blue**: Info/running states
- **Green**: Success/completed states
- **Red**: Error states
- **Purple**: State transitions
- **Yellow**: Warnings (available)

## Styleguide Integration

Added comprehensive examples to [styleguide/query.rs](file:///home/ghuntley/loom/crates/loom-web/src/routes/styleguide/query.rs) with:

1. **Query Timeline Demo**
   - 5-step query pipeline showing various stages
   - Current step at index 3 (Tool Invocation)
   - Expandable details on each step

2. **Tool Invocations Demo**
   - 3 sample tool calls (search, get_document, summarize)
   - Varying execution times (89ms, 245ms, 567ms)
   - Total execution time: 901ms

3. **State Machine Trace Demo**
   - 6-step state transition pipeline
   - Complete flow from idle → parsing → routing → executing → tools → formatting → complete
   - Realistic timestamps with 1-second intervals

4. **Early Stage Timeline Example**
   - Shows timeline at execution step 1 for comparison
   - Demonstrates pending and running states

## Key Features

### 1. **Expandable Details**
- Click "Show details" to expand logs, inputs, outputs
- Code blocks with syntax highlighting (via Tailwind styling)
- Max-height with scroll for large content
- Smooth toggle animation

### 2. **Status Indicators**
- Pending: Gray circle
- Running: Blue circle with spinner
- Completed: Green circle with checkmark
- Error: Red circle with X mark

### 3. **Timeline Visualization**
- Vertical connector lines between steps
- Left-aligned visual hierarchy
- Card-based layout for readability
- Clear separation of concerns

### 4. **Performance Metrics**
- Execution time per tool invocation
- Total execution time summary
- Millisecond precision

### 5. **Data Display**
- JSON pretty-printing for structured data
- Mono-space font for code/results
- Truncation for long content
- Scrollable containers for overflow

## Module Exports

**From `components/query/mod.rs`**:
```rust
pub use types::*;
pub use query_timeline::*;
pub use tool_invocation_list::*;
pub use state_machine_trace::*;
```

**Publicly exported types and components**:
- `QueryStep`
- `ToolInvocation`
- `StateTransition`
- `StepStatus`
- `QueryTimeline`
- `QueryStepCard`
- `ToolInvocationList`
- `ToolInvocationItem`
- `StateMachineTrace`
- `StateTransitionItem`

## Tailwind CSS Classes Used

- **Layout**: `flex`, `flex-col`, `flex-wrap`, `gap-*`, `space-*`
- **Sizing**: `w-*`, `h-*`, `px-*`, `py-*`, `max-h-*`
- **Colors**: `text-*`, `bg-*`, `border-*`
- **Typography**: `font-bold`, `font-semibold`, `font-mono`, `text-sm`, etc.
- **Styling**: `rounded-*`, `border`, `shadow-*`, `line-clamp-*`, `overflow-*`
- **States**: `hover:`, `disabled:`

## Testing Coverage

### Types Module
- `QueryStep` builder pattern validation
- `StepStatus` Display trait implementation
- `StateTransition` timestamp parsing utility

### Component Features
- Expandable/collapsible states
- Empty state handling
- Large content rendering
- Signal-based reactivity

## Files Modified/Created

1. ✅ Created: `components/query/types.rs` (140 lines)
2. ✅ Created: `components/query/query_timeline.rs` (293 lines)
3. ✅ Created: `components/query/tool_invocation_list.rs` (161 lines)
4. ✅ Created: `components/query/state_machine_trace.rs` (184 lines)
5. ✅ Modified: `components/query/mod.rs` (exports)
6. ✅ Modified: `routes/styleguide/query.rs` (114 lines of examples)

**Total Lines Added**: ~892 lines of production code + examples

## Build Verification

- ✅ No syntax errors in new components
- ✅ Proper imports and trait bounds
- ✅ Type-safe structures with serde support
- ✅ Leptos component compatibility
- ✅ CSS class names valid (Tailwind)
- ✅ Unicode handling (quoted strings for symbols)

## Future Enhancements

1. **Jump to State**: Click state transitions to navigate timeline
2. **Search/Filter**: Filter steps by status or name
3. **Export**: Download execution trace as JSON
4. **Streaming Updates**: Real-time step completion via WebSocket
5. **Diff View**: Compare multiple query executions
6. **Performance Analysis**: Visual profiling of execution times
7. **Custom Icons**: Replace text symbols with SVG icons
8. **Dark Mode**: Theme variants for dark UI

## Integration Guide

### Basic Usage
```rust
use crate::components::query::*;

#[component]
fn QueryViewer(id: String) -> impl IntoView {
    let steps = vec![/* ... */];
    let invocations = vec![/* ... */];
    let trace = vec![/* ... */];
    
    view! {
        <div class="space-y-8">
            <QueryTimeline steps=steps.into() current_step=2 />
            <ToolInvocationList invocations=invocations.into() />
            <StateMachineTrace trace=trace.into() />
        </div>
    }
}
```

### Data Source Integration
```rust
// From API response
let response = fetch_query_execution(id).await?;
let timeline = response.steps.into();
let invocations = response.tool_calls.into();
let trace = response.state_transitions.into();
```

## Styling Notes

All components use:
- **Base**: Card with shadow and border
- **Spacing**: Consistent gap/padding (px-2/4/6, py-1/2/3)
- **Typography**: Roboto/system fonts, with monospace for code
- **Responsiveness**: Flex layout adapts to container width
- **Accessibility**: Semantic HTML, proper color contrast

## Conclusion

The Query Bridge Visualization Components provide a comprehensive, type-safe interface for displaying LLM execution pipelines. The modular design allows easy integration with existing Loom UI, while the rich feature set enables deep inspection of query processing stages.
