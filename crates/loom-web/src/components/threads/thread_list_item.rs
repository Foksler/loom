/// ThreadListItem - Single thread in list/grid view
///
/// Displays a thread summary with title, metadata, and status indicators.
use crate::components::primitives::{Badge, BadgeVariant};
use crate::prelude::*;

use super::{ThreadStatus, ThreadSummary};

/// Single thread list item component
///
/// # Props
/// * `thread` - Thread data to display
/// * `on_click` - Callback when item is clicked
///
/// # Features
/// * Title with truncation
/// * Updated timestamp with relative time
/// * Provider badge
/// * Status indicator
/// * Hover effects
#[component]
pub fn ThreadListItem(
    /// Thread data
    thread: ThreadSummary,

    /// Callback when clicked
    #[prop(optional)]
    on_click: Option<Box<dyn Fn(String) -> () + Send + Sync + 'static>>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let thread_clone = thread.clone();
    let handle_click = move |_| {
        if let Some(ref callback) = on_click {
            callback(thread_clone.id.clone());
        }
    };

    let status_variant = match thread.status {
        ThreadStatus::Active => BadgeVariant::Green,
        ThreadStatus::Archived => BadgeVariant::Gray,
        ThreadStatus::Processing => BadgeVariant::Blue,
    };

    let provider_variant = match thread.provider.as_str() {
        "Claude" => BadgeVariant::Purple,
        "OpenAI" => BadgeVariant::Blue,
        _ => BadgeVariant::Gray,
    };

    // Format relative time
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(thread.updated_at);
    let time_str = if duration.num_seconds() < 60 {
        "Just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    };

    view! {
        <div
            class={format!(
                "bg-white border border-gray-200 rounded-lg p-4 cursor-pointer \
                 hover:shadow-md hover:border-gray-300 transition-all {}",
                class.unwrap_or_default()
            )}
            on:click=handle_click
            role="button"
            tabindex="0"
        >
            {/* Header: Title and Status */}
            <div class="flex items-start justify-between gap-3 mb-3">
                <h3 class="text-sm font-semibold text-gray-900 truncate flex-1">
                    {thread.title.clone()}
                </h3>
                <Badge variant=status_variant>{thread.status.as_str()}</Badge>
            </div>

            {/* Metadata: Provider and Time */}
            <div class="flex items-center justify-between gap-2">
                <Badge variant=provider_variant>{thread.provider.clone()}</Badge>
                <span class="text-xs text-gray-500">{time_str}</span>
            </div>
        </div>
    }
}
