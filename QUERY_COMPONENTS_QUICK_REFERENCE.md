# Query Bridge Components - Quick Reference

## Files Location
```
crates/loom-web/src/components/query/
├── types.rs              # Core types (120 lines)
├── query_timeline.rs     # Timeline visualization (293 lines)
├── tool_invocation_list.rs # Tool calls (161 lines)
├── state_machine_trace.rs  # State transitions (184 lines)
└── mod.rs               # Exports
```

## Components at a Glance

| Component | Purpose | Key Props | Status Icons |
|-----------|---------|-----------|--------------|
| `QueryTimeline` | Display query steps in order | `steps`, `current_step` | ◯ ✓ ⚠ ✕ |
| `QueryStepCard` | Individual step card | `step`, `status`, `is_current` | Colored badges |
| `ToolInvocationList` | List of tool calls | `invocations` | Execution time |
| `ToolInvocationItem` | Single tool call | `invocation`, `index` | Result preview |
| `StateMachineTrace` | State transitions | `trace`, `expanded` | Arrow flow → |
| `StateTransitionItem` | Single transition | `transition`, `index` | State labels |

## Type System

### QueryStep
```rust
let step = QueryStep::new("Parse Query")
    .with_description("Parsing input")
    .with_log("Parsing started")
    .with_inputs(json!({"query": "test"}))
    .with_outputs(json!({"tokens": 5}));
```

### ToolInvocation
```rust
let tool = ToolInvocation::new(
    "search",
    json!({"q": "test"}),
    "Found 5 results",
    150  // milliseconds
);
```

### StateTransition
```rust
let transition = StateTransition::new(
    "parsing",           // from
    "executing",         // to
    "2024-01-01T10:00Z", // timestamp
    "parse_complete"     // event
);
```

### StepStatus
```rust
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Error(String),
}
```

## Usage Patterns

### Simple Timeline
```rust
view! {
    <QueryTimeline
        steps=my_steps.into()
        current_step=2
    />
}
```

### With Tool Tracking
```rust
view! {
    <div class="space-y-8">
        <QueryTimeline steps=steps.into() current_step=3 />
        <ToolInvocationList invocations=tools.into() />
    </div>
}
```

### Full Pipeline View
```rust
view! {
    <div class="space-y-8">
        <h1>"Query Execution "</h1>
        <QueryTimeline steps=steps.into() current_step=current />
        <ToolInvocationList invocations=tools.into() />
        <StateMachineTrace trace=transitions.into() />
    </div>
}
```

## Styling

### Colors
- Gray: Pending/neutral
- Blue: Running/info
- Green: Success/complete
- Red: Error/failure
- Purple: State transitions

### States
- **Pending**: Gray circle
- **Running**: Blue spinner
- **Completed**: Green checkmark
- **Error**: Red X mark

## Expandable Features

All cards support toggling details:
- Logs (scrollable)
- Inputs (JSON)
- Outputs (JSON)
- Arguments (JSON)
- Results (code block)
- Timestamps

## Props Summary

### QueryTimeline
```rust
#[component]
pub fn QueryTimeline(
    steps: Vec<QueryStep>,
    #[prop(default = 0)]
    current_step: usize,
    #[prop(optional)]
    class: Option<String>,
)
```

### ToolInvocationList
```rust
#[component]
pub fn ToolInvocationList(
    invocations: Vec<ToolInvocation>,
    #[prop(optional)]
    class: Option<String>,
)
```

### StateMachineTrace
```rust
#[component]
pub fn StateMachineTrace(
    trace: Vec<StateTransition>,
    #[prop(default = false)]
    expanded: bool,
    #[prop(optional)]
    class: Option<String>,
)
```

## Examples in Styleguide

Visit `/styleguide/query` to see:
1. Query Timeline (5 steps, current at 3)
2. Tool Invocations (3 calls, 901ms total)
3. State Transitions (6 states)
4. Early Stage Timeline (showing pending states)

## Common Tasks

### Create a step with all data
```rust
QueryStep::new("Execute Query")
    .with_description("Running LLM inference")
    .with_log("Sending to Claude API")
    .with_log("Streaming response...")
    .with_inputs(json!({ "model": "claude-3", "max_tokens": 1000 }))
    .with_outputs(json!({ "tokens_used": 847, "stop_reason": "end_turn" }))
```

### Create a tool invocation
```rust
ToolInvocation::new(
    "vector_search",
    json!({ "query": "benefits", "top_k": 5 }),
    "Retrieved 5 documents with relevance scores",
    234
)
```

### Create state transitions
```rust
vec![
    StateTransition::new("idle", "parsing", "2024-01-01T10:00:00Z", "query_received"),
    StateTransition::new("parsing", "executing", "2024-01-01T10:00:01Z", "parse_complete"),
]
```

## Imports Required

```rust
use crate::components::query::{
    QueryTimeline, QueryStep, ToolInvocationList, ToolInvocation,
    StateMachineTrace, StateTransition, StepStatus,
};
use serde_json::json;
```

## Total Component Code

- **types.rs**: 140 lines (type definitions)
- **query_timeline.rs**: 293 lines (2 components)
- **tool_invocation_list.rs**: 161 lines (2 components)
- **state_machine_trace.rs**: 184 lines (2 components)
- **Total**: 778 lines of production code

## Performance Characteristics

- Timeline rendering: O(n) where n = number of steps
- Tool list rendering: O(m) where m = number of invocations
- State trace rendering: O(t) where t = number of transitions
- Expand/collapse: O(1) signal update
- Memory: All data structures are Clone-able

## Accessibility

- Semantic HTML (div, section, pre, button)
- Color + icons (not color alone)
- Keyboard navigable (buttons have focus states)
- ARIA labels on spinners
- Proper contrast ratios

## Future Additions (Planned)

- [ ] Click to jump to state
- [ ] Search/filter by name
- [ ] Export as JSON
- [ ] Streaming updates
- [ ] Performance timeline
- [ ] Dark mode variants
- [ ] SVG icons
- [ ] Comparison view
