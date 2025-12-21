use crate::components::results::{
    CodeBlock, DiffView, ExecutionResult, ExecutionStatus, FileNode, FileTree, LLMResult,
    LLMResultPanel,
};
/// Results & code components gallery
use crate::prelude::*;

#[component]
pub fn StyleguideResultsPage() -> impl IntoView {
    // Example file tree
    let file_tree = vec![
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
                FileNode {
                    name: "lib.rs".to_string(),
                    path: "src/lib.rs".to_string(),
                    is_dir: false,
                    children: vec![],
                },
                FileNode {
                    name: "utils.rs".to_string(),
                    path: "src/utils.rs".to_string(),
                    is_dir: false,
                    children: vec![],
                },
            ],
        },
        FileNode {
            name: "Cargo.toml".to_string(),
            path: "Cargo.toml".to_string(),
            is_dir: false,
            children: vec![],
        },
        FileNode {
            name: "README.md".to_string(),
            path: "README.md".to_string(),
            is_dir: false,
            children: vec![],
        },
    ];

    let selected = RwSignal::new(None);

    view! {
        <div class="space-y-8 p-8 max-w-6xl">
            <div>
                <h1 class="text-4xl font-bold mb-2 text-gray-900">Results & Code</h1>
                <p class="text-lg text-gray-600">
                    Components for displaying code, diffs, file trees, and execution results.
                </p>
            </div>

            // CodeBlock examples
            <div class="space-y-4">
                <h2 class="text-2xl font-bold text-gray-900">CodeBlock</h2>
                <p class="text-gray-600">Syntax-highlighted code display with copy button</p>

                <CodeBlock
                    code="fn main() {\n    println!(\"Hello, Loom!\");\n    let result = compute();\n    println!(\"Result: {}\", result);\n}\n\nfn compute() -> i32 {\n    42\n}".to_string()
                    language="rust".to_string()
                    line_numbers=true
                />
            </div>

            // DiffView side-by-side
            <div class="space-y-4">
                <h2 class="text-2xl font-bold text-gray-900">DiffView (Side-by-Side)</h2>
                <p class="text-gray-600">Compare code changes with highlighted additions and deletions</p>

                <DiffView
                    before="fn greet(name: &str) {\n    println!(\"Hello {}\", name);\n}".to_string()
                    after="fn greet(name: &str, age: u32) {\n    println!(\"Hello {}, you are {} years old\", name, age);\n}".to_string()
                    inline=false
                    language="rust".to_string()
                />
            </div>

            // DiffView inline
            <div class="space-y-4">
                <h2 class="text-2xl font-bold text-gray-900">DiffView (Inline)</h2>
                <p class="text-gray-600">Inline diff format for compact comparison</p>

                <DiffView
                    before="import { useState } from 'react';\n\nexport function Counter() {\n  return <div>Counter</div>;\n}".to_string()
                    after="import { useState } from 'react';\n\nexport function Counter() {\n  const [count, setCount] = useState(0);\n  return <div>Count: {count}</div>;\n}".to_string()
                    inline=true
                    language="typescript".to_string()
                />
            </div>

            // FileTree
            <div class="space-y-4">
                <h2 class="text-2xl font-bold text-gray-900">FileTree</h2>
                <p class="text-gray-600">Interactive file structure explorer</p>

                <FileTree
                    files=file_tree
                    selected=selected
                />
                {move || {
                    selected.get().map(|path| {
                        view! {
                            <p class="text-sm text-gray-600 mt-2">
                                Selected: <code class="font-mono bg-gray-100 px-2 py-1 rounded">{path}</code>
                            </p>
                        }
                    })
                }}
            </div>

            // ExecutionResult - Success
            <div class="space-y-4">
                <h2 class="text-2xl font-bold text-gray-900">ExecutionResult</h2>
                <p class="text-gray-600">Status indicators for execution results</p>

                <div class="space-y-4">
                    <div>
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">Success Status</h3>
                        <ExecutionResult
                            status=ExecutionStatus::Success
                            output="✓ Build completed successfully\n✓ All tests passed\n✓ 42 test cases executed".to_string()
                            errors=vec![]
                        />
                    </div>

                    <div>
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">Error Status</h3>
                        <ExecutionResult
                            status=ExecutionStatus::Error
                            output="Build failed with errors:\n\nerror[E0425]: cannot find value `x` in this scope\n  --> src/main.rs:5:13\n   |\n5 |     let y = x + 1;\n  |             ^ not found in this scope".to_string()
                            errors=vec![
                                "Cannot find value `x` in scope".to_string(),
                                "1 error found during compilation".to_string(),
                            ]
                        />
                    </div>

                    <div>
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">Warning Status</h3>
                        <ExecutionResult
                            status=ExecutionStatus::Warning
                            output="Build completed with warnings:\nwarning: unused variable: `unused_var`\n  --> src/main.rs:3:9\n   |\n3 | let unused_var = 42;".to_string()
                            errors=vec![
                                "Unused variable: unused_var".to_string(),
                            ]
                        />
                    </div>
                </div>
            </div>

            // LLMResultPanel
            <div class="space-y-4">
                <h2 class="text-2xl font-bold text-gray-900">LLMResultPanel</h2>
                <p class="text-gray-600">Tabbed container for full execution results</p>

                <LLMResultPanel
                    result=LLMResult {
                        id: "exec-20250101-001".to_string(),
                        status: ExecutionStatus::Success,
                        output: "Generated code executed successfully.\nOutput: Generated 3 functions\nProcessed 150 LOC".to_string(),
                        errors: vec![],
                        logs: vec![
                            "[INFO] Starting code generation...".to_string(),
                            "[INFO] Analyzing context...".to_string(),
                            "[INFO] Generating code...".to_string(),
                            "[INFO] Executing generated code...".to_string(),
                            "[INFO] All tests passed!".to_string(),
                        ],
                        execution_ms: 2500,
                    }
                />
            </div>

            // LLMResultPanel with errors
            <div class="space-y-4">
                <h3 class="text-lg font-semibold text-gray-900">With Errors</h3>
                <LLMResultPanel
                    result=LLMResult {
                        id: "exec-20250101-002".to_string(),
                        status: ExecutionStatus::Error,
                        output: "Code execution failed".to_string(),
                        errors: vec![
                            "Runtime error: Type mismatch in function parameter".to_string(),
                            "Expected String, got i32".to_string(),
                        ],
                        logs: vec![
                            "[DEBUG] Compilation started...".to_string(),
                            "[DEBUG] Type checking...".to_string(),
                            "[ERROR] Type mismatch detected".to_string(),
                        ],
                        execution_ms: 450,
                    }
                />
            </div>
        </div>
    }
}
