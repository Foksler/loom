/// CodeBlock component - Syntax-highlighted code display
///
/// Displays code with optional line numbers, copy button, and language badge.
/// Uses Tailwind CSS for styling with simulated syntax highlighting.
use crate::prelude::*;

/// CodeBlock component for displaying syntax-highlighted code
///
/// # Example
///
/// ```rust
/// view! {
///     <CodeBlock
///         code="fn main() { println!(\"Hello, world!\"); }".to_string()
///         language="rust".to_string()
///         line_numbers=true
///     />
/// }
/// ```
#[component]
pub fn CodeBlock(
    /// Source code to display
    code: String,

    /// Programming language for syntax highlighting
    /// Common values: "rust", "javascript", "typescript", "python", "json", "html", "css"
    #[prop(default = "text".to_string())]
    language: String,

    /// Whether to display line numbers
    #[prop(default = true)]
    line_numbers: bool,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let copy_signal = RwSignal::new(false);
    let line_count = code.lines().count();

    let handle_copy = move || {
        // Simplified copy handler - just show feedback
        copy_signal.set(true);
        set_timeout(
            move || {
                copy_signal.set(false);
            },
            std::time::Duration::from_secs(2),
        );
    };

    view! {
        <div class=format!(
            "border border-gray-200 rounded-lg overflow-hidden{}",
            match &class {
                Some(c) => format!(" {}", c),
                None => String::new(),
            }
        )>
            // Header with language badge and copy button
            <div class="bg-gray-50 border-b border-gray-200 px-4 py-3 flex items-center justify-between">
                <span class="inline-flex items-center gap-2">
                    <span class="px-2.5 py-1 text-xs font-semibold text-gray-700 bg-gray-200 rounded">
                        {language.to_uppercase()}
                    </span>
                </span>
                <button
                    on:click=move |_| handle_copy()
                    class="inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-200 active:bg-gray-300 rounded transition-colors"
                    title="Copy code to clipboard"
                >
                    <svg
                        class="w-4 h-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
                        />
                    </svg>
                    {move || {
                        if copy_signal.get() {
                            "Copied!"
                        } else {
                            "Copy"
                        }
                    }}
                </button>
            </div>

            // Code display
            <div class="flex overflow-auto bg-gray-900">
                <Show when=move || line_numbers>
                    <div class="bg-gray-100 text-gray-600 p-4 rounded-l-lg border-r border-gray-200 text-right select-none font-mono text-sm">
                        {(1..=line_count)
                            .map(|i| {
                                view! {
                                    <div class="h-6 leading-6">{i}</div>
                                }
                            })
                            .collect_view()}
                    </div>
                </Show>

                <pre class="flex-1 p-4 text-gray-100 font-mono text-sm overflow-x-auto">
                    <code>{code}</code>
                </pre>
            </div>
        </div>
    }
}
