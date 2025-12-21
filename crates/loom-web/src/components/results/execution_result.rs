use crate::components::results::types::ExecutionStatus;
/// ExecutionResult component - Generic result display with status
///
/// Displays execution status, output, errors, and logs in a structured format.
/// Provides color-coded status indicators and error lists.
use crate::prelude::*;

/// ExecutionResult component for displaying command/execution results
///
/// # Example
///
/// ```rust
/// view! {
///     <ExecutionResult
///         status=ExecutionStatus::Success
///         output="Build completed successfully".to_string()
///         errors=vec![]
///     />
/// }
/// ```
#[component]
pub fn ExecutionResult(
    /// Execution status
    status: ExecutionStatus,

    /// Output/stdout from execution
    #[prop(default = String::new())]
    output: String,

    /// Error messages from execution
    #[prop(default = vec![])]
    errors: Vec<String>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let status_text = match status {
        ExecutionStatus::Success => "Success",
        ExecutionStatus::Error => "Error",
        ExecutionStatus::Warning => "Warning",
        ExecutionStatus::Processing => "Processing",
    };

    let color_class = status.color_class();

    view! {
        <div class=format!(
            "border border-gray-200 rounded-lg overflow-hidden{}",
            match &class {
                Some(c) => format!(" {}", c),
                None => String::new(),
            }
        )>
            // Status header
            <div class=format!("px-4 py-3 border-b border-gray-200 {}", color_class)>
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                        {match status {
                            ExecutionStatus::Success => {
                                view! {
                                    <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                                        <path
                                            fill-rule="evenodd"
                                            d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                                            clip-rule="evenodd"
                                        />
                                    </svg>
                                }.into_view()
                            }
                            ExecutionStatus::Error => {
                                view! {
                                    <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                                        <path
                                            fill-rule="evenodd"
                                            d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                                            clip-rule="evenodd"
                                        />
                                    </svg>
                                }.into_view()
                            }
                            ExecutionStatus::Warning => {
                                view! {
                                    <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                                        <path
                                            fill-rule="evenodd"
                                            d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                                            clip-rule="evenodd"
                                        />
                                    </svg>
                                }.into_view()
                            }
                            ExecutionStatus::Processing => {
                                view! {
                                    <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                                        <path
                                            fill-rule="evenodd"
                                            d="M10 18a8 8 0 100-16 8 8 0 000 16zm.447-4.606a.75.75 0 10-1.06-1.06l-3.5 3.5a.75.75 0 001.06 1.06l3.5-3.5z"
                                            clip-rule="evenodd"
                                        />
                                    </svg>
                                }.into_view()
                            }
                        }}
                        <span class="font-semibold text-sm">{status_text}</span>
                    </div>
                </div>
            </div>

            // Content
            <div class="p-4 space-y-4">
                // Error section
                {
                    let errs = errors.clone();
                    view! {
                        <Show when=move || !errs.is_empty()>
                            <div class="space-y-2">
                                <h3 class="font-semibold text-sm text-gray-900">Errors</h3>
                                <ul class="space-y-1">
                                    {errors
                                        .iter()
                                        .map(|error| {
                                            view! {
                                                <li class="text-sm text-red-700 bg-red-50 px-3 py-2 rounded border border-red-200 flex gap-2">
                                                    <span class="text-red-500 flex-shrink-0">-</span>
                                                    <code>{error.clone()}</code>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            </div>
                        </Show>
                    }
                }

                // Output section
                {
                    let output_check = output.clone();
                    let output_disp = output.clone();
                    view! {
                        <Show
                            when=move || !output_check.is_empty()
                            fallback=|| view! {
                                <p class="text-sm text-gray-500 italic">No output</p>
                            }
                        >
                            {
                                let out_val = output_disp.clone();
                                view! {
                                    <div class="space-y-2">
                                        <h3 class="font-semibold text-sm text-gray-900">Output</h3>
                                        <pre class="bg-gray-900 text-gray-100 p-3 rounded font-mono text-xs overflow-x-auto border border-gray-200">
                                            <code>{out_val}</code>
                                        </pre>
                                    </div>
                                }
                            }
                        </Show>
                    }
                }
            </div>
        </div>
    }
}
