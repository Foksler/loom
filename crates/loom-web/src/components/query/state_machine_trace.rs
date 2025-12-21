/// StateMachineTrace component - show state transitions over time
use crate::components::primitives::{Badge, BadgeVariant, Card, CardElevation};
use crate::prelude::*;

use super::types::StateTransition;

/// StateMachineTrace displays state transitions during query execution.
#[component]
pub fn StateMachineTrace(
    /// State transitions in order
    #[prop()]
    trace: Vec<StateTransition>,

    /// Whether to show expanded details by default
    #[prop(default = false)]
    expanded: bool,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let (show_details, set_show_details) = signal(expanded);

    let trace_len = trace.len();
    let is_empty = trace.is_empty();

    let trace_items = trace
        .into_iter()
        .enumerate()
        .map(|(idx, transition)| {
            let is_last = idx == trace_len - 1;
            view! {
                <StateTransitionItem
                    transition=transition
                    index=idx
                    is_last=is_last
                    show_details=show_details
                />
            }
        })
        .collect_view();

    view! {
        <div class=format!(
            "space-y-3 {}",
            class.unwrap_or_default()
        )>
            <div class="flex items-center justify-between">
                <h3 class="text-lg font-semibold text-gray-900">
                    {format!("State Transitions ({})", trace_len)}
                </h3>
                <button
                    on:click=move |_| set_show_details.update(|d| *d = !*d)
                    class="text-sm text-blue-600 hover:text-blue-800 font-medium"
                >
                    {if show_details.get() { "Hide All" } else { "Show All" }}
                </button>
            </div>

            {if is_empty {
                view! {
                    <Card elevation=CardElevation::Sm>
                        <p class="text-gray-500 text-center py-4">
                            "No state transitions recorded"
                        </p>
                    </Card>
                }.into_any()
            } else {
                view! {
                    <div class="space-y-0">
                        {trace_items}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// StateTransitionItem component - single state transition
#[component]
pub fn StateTransitionItem(
    /// The state transition to display
    transition: StateTransition,

    /// Position in the trace (0-based)
    #[prop(default = 0)]
    index: usize,

    /// Whether this is the last transition
    #[prop(default = false)]
    is_last: bool,

    /// Whether to show details
    #[prop(into)]
    show_details: Signal<bool>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let timestamp_display = parse_timestamp(&transition.timestamp);
    let time_only = timestamp_display
        .split_whitespace()
        .nth(1)
        .unwrap_or("--:--:--")
        .to_string();

    view! {
        <div class=format!(
            "relative {}",
            class.unwrap_or_default()
        )>
            <Show when=move || !is_last>
                <div class="absolute left-6 top-16 w-1 h-12 bg-gray-200" />
            </Show>

            <Card elevation=CardElevation::Sm>
                <div class="flex gap-4">
                    <div class="flex-shrink-0 w-12 h-12 rounded-full flex items-center justify-center bg-purple-50 border-2 border-purple-500">
                        <div class="text-purple-600 font-bold text-xs text-center">
                            {format!("S{}", index + 1)}
                        </div>
                    </div>

                    <div class="flex-1 min-w-0">
                        <div class="flex items-start justify-between gap-2 flex-wrap">
                            <div class="flex-1">
                                <div class="flex items-center gap-2 flex-wrap">
                                    <span class="inline-block px-2 py-1 bg-gray-100 text-gray-800 text-xs font-medium rounded">
                                        {transition.from_state.clone()}
                                    </span>
                                    <span class="text-gray-400">"→"</span>
                                    <span class="inline-block px-2 py-1 bg-purple-100 text-purple-800 text-xs font-medium rounded">
                                        {transition.to_state.clone()}
                                    </span>
                                </div>
                                <p class="text-sm text-gray-600 mt-1">
                                    {format!("Event: {}", transition.event)}
                                </p>
                            </div>
                            <Badge variant=BadgeVariant::Purple size=Default::default()>
                                {time_only}
                            </Badge>
                        </div>

                        <Show when=move || show_details.get()>
                            <div class="mt-3 border-t border-gray-200 pt-3 space-y-2">
                                <div class="grid grid-cols-2 gap-3 text-sm">
                                    <div>
                                        <span class="text-gray-600">Timestamp:</span>
                                        <p class="text-gray-900 font-mono text-xs">{timestamp_display.clone()}</p>
                                    </div>
                                    <div>
                                        <span class="text-gray-600">Index:</span>
                                        <p class="text-gray-900 font-mono text-xs">{format!("{}", index + 1)}</p>
                                    </div>
                                </div>
                            </div>
                        </Show>
                    </div>
                </div>
            </Card>
        </div>
    }
}

/// Helper function to parse and format timestamp
fn parse_timestamp(ts: &str) -> String {
    ts.split('T')
        .map(|s| s.trim_end_matches('Z'))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        assert_eq!(
            parse_timestamp("2024-01-01T10:00:00Z"),
            "2024-01-01 10:00:00"
        );
    }
}
