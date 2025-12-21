/// ToolInvocationList component - list of tool calls during execution
use crate::components::primitives::{Badge, BadgeVariant, Card, CardElevation};
use crate::prelude::*;

use super::types::ToolInvocation;

/// ToolInvocationList displays a list of tool invocations during query execution.
#[component]
pub fn ToolInvocationList(
    /// List of tool invocations
    #[prop()]
    invocations: Vec<ToolInvocation>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let total_execution_ms: u32 = invocations.iter().map(|i| i.execution_ms).sum();
    let invocations_len = invocations.len();
    let empty = invocations.is_empty();

    view! {
        <div class=format!(
            "space-y-3 {}",
            class.unwrap_or_default()
        )>
            <div class="flex items-center justify-between">
                <h3 class="text-lg font-semibold text-gray-900">
                    {format!("Tool Invocations ({})", invocations_len)}
                </h3>
                {if !empty {
                    view! {
                        <div class="text-sm text-gray-600">
                            {format!("Total: {}ms", total_execution_ms)}
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>

            {if !empty {
                view! {
                    <div class="space-y-2">
                        {invocations
                            .into_iter()
                            .enumerate()
                            .map(|(idx, invocation)| {
                                view! {
                                    <ToolInvocationItem
                                        invocation=invocation
                                        index=idx
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                }.into_any()
            } else {
                view! {
                    <Card elevation=CardElevation::Sm>
                        <p class="text-gray-500 text-center py-4">
                            "No tool invocations recorded"
                        </p>
                    </Card>
                }.into_any()
            }}
        </div>
    }
}

/// ToolInvocationItem component - single tool invocation card
#[component]
pub fn ToolInvocationItem(
    /// The tool invocation to display
    invocation: ToolInvocation,

    /// Position in list (0-based)
    #[prop(default = 0)]
    index: usize,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let (expanded, set_expanded) = signal(false);

    let args_display = serde_json::to_string_pretty(&invocation.arguments)
        .unwrap_or_else(|_| "Invalid arguments".to_string());
    let result_text = invocation.result.clone();
    let tool_name = invocation.tool_name.clone();

    view! {
        <Card elevation=CardElevation::Sm class=class.unwrap_or_default()>
            <div class="space-y-3">
                <div class="flex items-start justify-between gap-2">
                    <div class="flex-1 min-w-0">
                        <div class="flex items-center gap-2">
                            <span class="inline-block px-2 py-1 bg-blue-100 text-blue-800 text-xs font-medium rounded">
                                {format!("#{}", index + 1)}
                            </span>
                            <h4 class="text-sm font-semibold text-gray-900 truncate">
                                {tool_name}
                            </h4>
                        </div>
                    </div>
                    <Badge variant=BadgeVariant::Green size=Default::default()>
                        {format!("{}ms", invocation.execution_ms)}
                    </Badge>
                </div>

                {if expanded.get() {
                    view! {
                        <div class="space-y-3 border-t border-gray-200 pt-3">
                            <div>
                                <h5 class="text-xs font-semibold text-gray-700 uppercase tracking-wider mb-2">
                                    "Arguments"
                                </h5>
                                <pre class="bg-gray-50 rounded border border-gray-200 p-2 text-xs overflow-x-auto text-gray-700 max-h-48 overflow-y-auto">
                                    {args_display.clone()}
                                </pre>
                            </div>

                            <div>
                                <h5 class="text-xs font-semibold text-gray-700 uppercase tracking-wider mb-2">
                                    "Result"
                                </h5>
                                <pre class="bg-gray-50 rounded border border-gray-200 p-2 text-xs overflow-x-auto text-gray-700 max-h-48 overflow-y-auto">
                                    {result_text.clone()}
                                </pre>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <p class="text-sm text-gray-600 line-clamp-2">
                            {result_text.clone()}
                        </p>
                    }.into_any()
                }}

                <button
                    on:click=move |_| set_expanded.update(|e| *e = !*e)
                    class="text-sm text-blue-600 hover:text-blue-800 font-medium"
                >
                    {if expanded.get() { "▼ Hide details" } else { "▶ Show details" }}
                </button>
            </div>
        </Card>
    }
}
