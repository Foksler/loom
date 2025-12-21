/// DiffView component - Side-by-side or inline diff display
///
/// Shows differences between two code blocks with highlighted additions
/// and deletions. Supports both inline and side-by-side layouts.
use crate::prelude::*;

/// DiffView component for displaying code differences
///
/// # Example
///
/// ```rust
/// view! {
///     <DiffView
///         before="fn old() {}".to_string()
///         after="fn new() { println!(\"hello\"); }".to_string()
///         inline=false
///         language="rust".to_string()
///     />
/// }
/// ```
#[component]
pub fn DiffView(
    /// Original code
    before: String,

    /// Modified code
    after: String,

    /// Display as inline diff (true) or side-by-side (false)
    #[prop(default = false)]
    inline: bool,

    /// Language for syntax highlighting
    #[prop(default = "text".to_string())]
    language: String,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let before_lines: Vec<String> = before.lines().map(|s| s.to_string()).collect();
    let after_lines: Vec<String> = after.lines().map(|s| s.to_string()).collect();

    if inline {
        (view! {
            <div class=format!(
                "border border-gray-200 rounded-lg overflow-hidden{}",
                match &class {
                    Some(c) => format!(" {}", c),
                    None => String::new(),
                }
            )>
                // Header
                <div class="bg-gray-50 border-b border-gray-200 px-4 py-3">
                    <span class="px-2.5 py-1 text-xs font-semibold text-gray-700 bg-gray-200 rounded">
                        {language.to_uppercase()}
                    </span>
                    <span class="ml-3 text-sm text-gray-600">Inline Diff</span>
                </div>

                // Inline diff content
                <div class="bg-gray-900 text-gray-100 font-mono text-sm overflow-x-auto">
                    <table class="w-full border-collapse">
                        <tbody>
                            {(0..before_lines.len().max(after_lines.len()))
                                .map(|i| {
                                    let before_line = before_lines.get(i).map(|s| s.clone());
                                    let after_line = after_lines.get(i).map(|s| s.clone());

                                    let (is_same, is_deleted, is_added) = match (&before_line, &after_line) {
                                        (Some(b), Some(a)) if b == a => (true, false, false),
                                        (Some(_), _) => (false, true, false),
                                        (_, Some(_)) => (false, false, true),
                                        _ => (false, false, false),
                                    };
                                    
                                    let content = if is_same {
                                        view! {
                                            <tr class="border-b border-gray-700">
                                                <td class="w-1/2 px-4 py-2 bg-gray-800">
                                                    <span class="text-gray-500">- </span>
                                                    <code>{before_line.clone().unwrap_or_default()}</code>
                                                </td>
                                                <td class="w-1/2 px-4 py-2 bg-gray-800">
                                                    <span class="text-gray-500">- </span>
                                                    <code>{after_line.clone().unwrap_or_default()}</code>
                                                </td>
                                            </tr>
                                        }.into_any()
                                    } else if is_deleted {
                                        view! {
                                            <tr class="border-b border-gray-700 bg-red-950">
                                                <td class="w-1/2 px-4 py-2 text-red-300">
                                                    <span class="text-red-500">- </span>
                                                    <code>{before_line.clone().unwrap_or_default()}</code>
                                                </td>
                                                <td class="w-1/2 px-4 py-2 bg-gray-800"></td>
                                            </tr>
                                        }.into_any()
                                    } else if is_added {
                                        view! {
                                            <tr class="border-b border-gray-700 bg-green-950">
                                                <td class="w-1/2 px-4 py-2 bg-gray-800"></td>
                                                <td class="w-1/2 px-4 py-2 text-green-300">
                                                    <span class="text-green-500">+ </span>
                                                    <code>{after_line.clone().unwrap_or_default()}</code>
                                                </td>
                                            </tr>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <tr></tr>
                                        }.into_any()
                                    };

                                    content
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>
        }).into_any()
    } else {
        // Side-by-side view
        (view! {
            <div class=format!(
                "border border-gray-200 rounded-lg overflow-hidden{}",
                match &class {
                    Some(c) => format!(" {}", c),
                    None => String::new(),
                }
            )>
                // Header
                <div class="bg-gray-50 border-b border-gray-200 px-4 py-3 flex justify-between">
                    <div class="flex-1">
                        <span class="px-2.5 py-1 text-xs font-semibold text-gray-700 bg-red-100 rounded">
                            BEFORE
                        </span>
                    </div>
                    <div class="flex-1 text-right">
                        <span class="px-2.5 py-1 text-xs font-semibold text-gray-700 bg-green-100 rounded">
                            AFTER
                        </span>
                    </div>
                </div>

                // Side-by-side content
                <div class="flex overflow-auto bg-gray-900">
                    // Before side
                    <div class="flex-1 border-r border-gray-700 min-w-0">
                        <pre class="p-4 text-gray-100 font-mono text-sm whitespace-pre-wrap break-words">
                            {before_lines
                                .iter()
                                .enumerate()
                                .map(|(i, line)| {
                                    let line_clone = line.clone();
                                    view! {
                                        <div class="h-6 hover:bg-red-950 transition-colors">
                                            <span class="text-gray-500 mr-2">{i + 1}</span>
                                            <code class="text-red-300">{line_clone}</code>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </pre>
                    </div>

                    // After side
                     <div class="flex-1 min-w-0">
                         <pre class="p-4 text-gray-100 font-mono text-sm whitespace-pre-wrap break-words">
                             {after_lines
                                 .iter()
                                 .enumerate()
                                 .map(|(i, line)| {
                                     let line_clone = line.clone();
                                     view! {
                                         <div class="h-6 hover:bg-green-950 transition-colors">
                                             <span class="text-gray-500 mr-2">{i + 1}</span>
                                             <code class="text-green-300">{line_clone}</code>
                                         </div>
                                     }
                                 })
                                 .collect_view()}
                         </pre>
                     </div>
                </div>
            </div>
        }).into_any()
    }
}
