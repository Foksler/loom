use crate::components::results::types::{LLMResult, Tab};
/// LLMResultPanel component - Container for displaying LLM execution results
///
/// Provides a tabbed interface for viewing output, errors, logs, and other
/// result metadata with status indicators.
use crate::prelude::*;

/// LLMResultPanel component for displaying full LLM execution results
///
/// # Example
///
/// ```rust
/// view! {
///     <LLMResultPanel
///         result=LLMResult {
///             id: "exec-1".to_string(),
///             status: ExecutionStatus::Success,
///             output: "Build successful".to_string(),
///             errors: vec![],
///             logs: vec!["Starting build...".to_string()],
///             execution_ms: 1500,
///         }
///     />
/// }
/// ```
#[component]
pub fn LLMResultPanel(
    /// Execution result to display
    result: LLMResult,

    /// Optional custom tabs
    #[prop(optional)]
    custom_tabs: Option<Vec<Tab>>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let active_tab = RwSignal::new("output".to_string());

    // Build default tabs
    let mut tabs = vec![Tab {
        id: "output".to_string(),
        label: "Output".to_string(),
        content: result.output.clone(),
    }];

    if !result.errors.is_empty() {
        tabs.push(Tab {
            id: "errors".to_string(),
            label: format!("Errors ({})", result.errors.len()),
            content: result.errors.join("\n"),
        });
    }

    if !result.logs.is_empty() {
        tabs.push(Tab {
            id: "logs".to_string(),
            label: format!("Logs ({})", result.logs.len()),
            content: result.logs.join("\n"),
        });
    }

    // Add custom tabs if provided
    if let Some(custom) = custom_tabs {
        tabs.extend(custom);
    }

    let status_badge = move || {
        let variant = result.status.badge_variant();
        let text = match result.status {
            crate::components::results::types::ExecutionStatus::Success => "Success",
            crate::components::results::types::ExecutionStatus::Error => "Error",
            crate::components::results::types::ExecutionStatus::Warning => "Warning",
            crate::components::results::types::ExecutionStatus::Processing => "Processing",
        };

        view! {
            <crate::components::primitives::Badge variant=variant>
                {text}
            </crate::components::primitives::Badge>
        }
    };

    view! {
        <div class=format!(
            "border border-gray-200 rounded-lg overflow-hidden bg-white{}",
            match &class {
                Some(c) => format!(" {}", c),
                None => String::new(),
            }
        )>
            // Header with metadata
            <div class="border-b border-gray-200 px-6 py-4 bg-gray-50">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-4">
                        {status_badge()}
                        <div class="text-sm text-gray-600">
                            <span class="font-mono text-xs text-gray-500">ID: </span>
                            <code class="text-gray-700">{result.id.clone()}</code>
                        </div>
                    </div>
                    <div class="text-sm text-gray-600">
                        <span class="font-semibold">{result.execution_ms}</span>
                        <span class="text-gray-500">ms</span>
                    </div>
                </div>
            </div>

            // Tab navigation
            <div class="flex border-b border-gray-200 overflow-x-auto bg-gray-50">
                {tabs
                    .iter()
                    .map(|tab| {
                        let tab_id = tab.id.clone();
                        let tab_label = tab.label.clone();
                        let tab_id_click = tab_id.clone();
                        let is_active = move || active_tab.get() == tab_id;

                        view! {
                            <button
                                on:click=move |_| {
                                    active_tab.set(tab_id_click.clone());
                                }
                                class=format!(
                                    "px-4 py-3 text-sm font-medium whitespace-nowrap border-b-2 transition-colors {}",
                                    if is_active() {
                                        "border-blue-600 text-blue-600"
                                    } else {
                                        "border-transparent text-gray-600 hover:text-gray-900"
                                    }
                                )
                            >
                                {tab_label}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>

            // Tab content
            <div class="p-6 overflow-auto max-h-96">
                {tabs
                    .iter()
                    .map(|tab| {
                        let tab_id = tab.id.clone();
                        let content = tab.content.clone();
                        let tab_id_clone = tab_id.clone();

                        view! {
                            <Show when=move || active_tab.get() == tab_id_clone>
                                <pre class="bg-gray-900 text-gray-100 font-mono text-xs p-4 rounded border border-gray-200 overflow-x-auto">
                                    <code>{content.clone()}</code>
                                </pre>
                            </Show>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}
