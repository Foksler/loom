# Results Components Quick Reference

## Component Locations
```
crates/loom-web/src/components/results/
├── CodeBlock         (code_block.rs:48)      - Syntax-highlighted code
├── DiffView          (diff_view.rs:29)       - Code diff viewer
├── FileTree          (file_tree.rs:16)       - File explorer
├── ExecutionResult   (execution_result.rs:21) - Status & output
└── LLMResultPanel    (llm_result_panel.rs:26) - Tabbed result view
```

## Quick Usage

### CodeBlock
```rust
use crate::components::results::CodeBlock;

view! {
    <CodeBlock
        code="fn main() {}".to_string()
        language="rust".to_string()
        line_numbers=true
    />
}
```

### DiffView
```rust
use crate::components::results::DiffView;

view! {
    <DiffView
        before="old".to_string()
        after="new".to_string()
        inline=false
        language="rust".to_string()
    />
}
```

### FileTree
```rust
use crate::components::results::{FileTree, FileNode};

let selected = create_rw_signal(None);
let files = vec![
    FileNode {
        name: "src".to_string(),
        path: "src".to_string(),
        is_dir: true,
        children: vec![
            FileNode {
                name: "main.rs".to_string(),
                path: "src/main.rs".to_string(),
                is_dir: false,
                children: vec![],
            },
        ],
    },
];

view! {
    <FileTree files=files selected=selected />
}
```

### ExecutionResult
```rust
use crate::components::results::{ExecutionResult, ExecutionStatus};

view! {
    <ExecutionResult
        status=ExecutionStatus::Success
        output="Build complete".to_string()
        errors=vec![]
    />
}
```

### LLMResultPanel
```rust
use crate::components::results::{LLMResultPanel, LLMResult, ExecutionStatus};

view! {
    <LLMResultPanel
        result=LLMResult {
            id: "exec-1".to_string(),
            status: ExecutionStatus::Success,
            output: "Generated code".to_string(),
            errors: vec![],
            logs: vec!["Starting...".to_string()],
            execution_ms: 1500,
        }
    />
}
```

## Type Reference

### ExecutionStatus
```rust
pub enum ExecutionStatus {
    Success,    // Green
    Error,      // Red
    Warning,    // Yellow
    Processing, // Blue (animated spinner)
}
```

### LLMResult
```rust
pub struct LLMResult {
    pub id: String,              // Unique ID
    pub status: ExecutionStatus, // Status indicator
    pub output: String,          // Main output
    pub errors: Vec<String>,     // Error messages
    pub logs: Vec<String>,       // Log lines
    pub execution_ms: u32,       // Execution time
}
```

### FileNode
```rust
pub struct FileNode {
    pub name: String,            // File/folder name
    pub path: String,            // Full path
    pub is_dir: bool,            // Directory flag
    pub children: Vec<FileNode>, // Child nodes
}

impl FileNode {
    pub fn new(name: String, path: String, is_dir: bool) -> Self
    pub fn add_child(self, child: FileNode) -> Self
    pub fn extension(&self) -> Option<&str>
    pub fn icon(&self) -> &'static str  // Returns emoji icon
}
```

### Tab
```rust
pub struct Tab {
    pub id: String,      // Tab ID
    pub label: String,   // Display label
    pub content: String, // Content text
}
```

## Component Features Matrix

| Component | Copy Button | Line Numbers | Dark Theme | Expandable | Status Badge | Tabs | Interactive |
|-----------|-------------|--------------|------------|-----------|--------------|------|-------------|
| CodeBlock | ✓ | ✓ | ✓ | - | - | - | ✓ |
| DiffView | - | ✓ | ✓ | - | - | - | - |
| FileTree | - | - | - | ✓ | - | - | ✓ |
| ExecutionResult | - | - | ✓ | - | ✓ | - | - |
| LLMResultPanel | - | - | ✓ | - | ✓ | ✓ | ✓ |

## Props Reference

### CodeBlock Props
- `code: String` - Source code
- `language: String` (default: "text") - Language identifier
- `line_numbers: bool` (default: true) - Show line numbers
- `class: Option<String>` - Extra CSS classes

### DiffView Props
- `before: String` - Original code
- `after: String` - Modified code
- `inline: bool` (default: false) - Inline mode
- `language: String` (default: "text") - Language
- `class: Option<String>` - Extra CSS classes

### FileTree Props
- `files: Vec<FileNode>` - File structure
- `selected: RwSignal<Option<String>>` - Selection state
- `class: Option<String>` - Extra CSS classes

### ExecutionResult Props
- `status: ExecutionStatus` - Status indicator
- `output: String` (default: "") - Output text
- `errors: Vec<String>` (default: []) - Error list
- `class: Option<String>` - Extra CSS classes

### LLMResultPanel Props
- `result: LLMResult` - Execution result
- `custom_tabs: Option<Vec<Tab>>` - Additional tabs
- `class: Option<String>` - Extra CSS classes

## Color Mapping

### Status Colors
- **Success**: `bg-green-100` + `text-green-800`
- **Error**: `bg-red-100` + `text-red-800`
- **Warning**: `bg-yellow-100` + `text-yellow-800`
- **Processing**: `bg-blue-100` + `text-blue-800`

### Code Display
- Background: `bg-gray-900`
- Text: `text-gray-100`
- Font: `font-mono` (monospace)
- Line numbers: `bg-gray-100` + `text-gray-600`

## Icons in FileTree
- 📁 Folder (directory)
- 🦀 Rust file (.rs)
- ⚙️ JavaScript/TypeScript (.js, .ts, .tsx, .jsx)
- 📋 JSON (.json)
- 📄 Markdown/Text (.md, .txt)
- 🎨 HTML/CSS (.html, .css)
- 📄 Default (unknown)

## Imports
```rust
use crate::components::results::{
    CodeBlock, DiffView, FileTree, ExecutionResult, LLMResultPanel,
    ExecutionStatus, LLMResult, FileNode, Tab, CodeBlockProps,
};
```

## Testing & Examples
See `crates/loom-web/src/routes/styleguide/results.rs` for:
- CodeBlock with Rust code
- DiffView side-by-side examples
- DiffView inline examples
- FileTree with selection
- ExecutionResult status variants
- LLMResultPanel with/without errors

## Compilation
✓ All components compile without errors  
✓ No compiler warnings for component code  
✓ Full type safety with Rust type system  
✓ Leptos reactivity via RwSignal  
✓ Tailwind CSS styling  

## Performance Notes
- FileTree: O(n) rendering, efficient on expansion/collapse
- CodeBlock: Handles large files well, copy is async
- DiffView: Linear comparison, good for small-medium diffs
- LLMResultPanel: Lazy tab rendering (tabs not visible = not rendered)
- ExecutionResult: Simple rendering, minimal overhead

## Accessibility
- Semantic HTML structure
- Alt text on SVG icons
- Proper heading hierarchy
- Color + icons for status (not color alone)
- Keyboard navigation on interactive elements
