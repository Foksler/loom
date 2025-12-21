/// ThreadHeader - Header for thread detail page
///
/// Displays thread title, status badge, and action menu for thread operations.
use crate::components::primitives::{Badge, BadgeVariant};
use crate::prelude::*;

use super::{Thread, ThreadStatus};

/// Thread actions
#[derive(Clone, Copy, Debug)]
pub enum ThreadAction {
    /// Archive the thread
    Archive,
    /// Delete the thread
    Delete,
    /// Export thread
    Export,
}

/// Header component for thread detail view
///
/// # Props
/// * `thread` - Thread to display
/// * `on_action` - Callback for menu actions
///
/// # Features
/// * Thread title display
/// * Status badge with color coding
/// * Action dropdown menu
/// * Responsive layout
#[component]
pub fn ThreadHeader(
    /// Thread to display
    thread: Thread,

    /// Callback for action menu selections
    #[prop(optional)]
    on_action: Option<Box<dyn Fn(ThreadAction) + 'static>>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    use std::rc::Rc;
    let on_action = Rc::new(on_action);

    let status_variant = match thread.status {
        ThreadStatus::Active => BadgeVariant::Green,
        ThreadStatus::Archived => BadgeVariant::Gray,
        ThreadStatus::Processing => BadgeVariant::Blue,
    };

    view! {
        <header class={format!(
            "bg-white border-b border-gray-200 px-6 py-4 {}",
            class.unwrap_or_default()
        )}>
            <div class="flex items-center justify-between gap-4">
                {/* Title and Status */}
                <div class="flex-1 min-w-0">
                    <h1 class="text-2xl font-bold text-gray-900 truncate">
                        {thread.title}
                    </h1>
                    <p class="text-sm text-gray-500 mt-1">
                        Created on {format_date(&thread.created_at)}
                    </p>
                </div>

                {/* Status Badge */}
                <Badge variant=status_variant>{thread.status.as_str()}</Badge>

                {/* Action Menu Button */}
                <div class="relative group">
                    <button class="px-3 py-2 text-gray-600 hover:text-gray-900">
                        "⋮"
                    </button>

                    {/* Dropdown Menu */}
                    <div class="absolute right-0 mt-2 w-48 bg-white border border-gray-200 \
                                rounded-lg shadow-lg z-10 hidden group-hover:block">
                        {
                            let on_action_archive = on_action.clone();
                            view! {
                                <button
                                    class="block w-full text-left px-4 py-2 hover:bg-gray-50 \
                                           text-sm text-gray-700 first:rounded-t-lg"
                                    on:click=move |_| {
                                        if let Some(ref cb) = (*on_action_archive).as_ref() {
                                            cb(ThreadAction::Archive)
                                        }
                                    }
                                >
                                    "Archive"
                                </button>
                            }
                        }
                        {
                            let on_action_export = on_action.clone();
                            view! {
                                <button
                                    class="block w-full text-left px-4 py-2 hover:bg-gray-50 \
                                           text-sm text-gray-700"
                                    on:click=move |_| {
                                        if let Some(ref cb) = (*on_action_export).as_ref() {
                                            cb(ThreadAction::Export)
                                        }
                                    }
                                >
                                    "Export"
                                </button>
                            }
                        }
                        {
                            let on_action_delete = on_action.clone();
                            view! {
                                <button
                                    class="block w-full text-left px-4 py-2 hover:bg-red-50 \
                                           text-sm text-red-700 rounded-b-lg"
                                    on:click=move |_| {
                                        if let Some(ref cb) = (*on_action_delete).as_ref() {
                                            cb(ThreadAction::Delete)
                                        }
                                    }
                                >
                                    "Delete"
                                </button>
                            }
                        }
                    </div>
                </div>
            </div>
        </header>
    }
}

/// Format date for display
fn format_date(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%B %d, %Y").to_string()
}
