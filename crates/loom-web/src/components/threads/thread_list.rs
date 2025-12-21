/// ThreadList - Grid/list of thread summaries
use crate::prelude::*;

use super::{ThreadListItem, ThreadSummary};

#[cfg(test)]
#[path = "thread_list_test.rs"]
mod thread_list_test;

/// Grid/list component for displaying threads
#[component]
pub fn ThreadList(
    /// List of threads to display
    threads: Vec<ThreadSummary>,

    /// Show loading state
    #[prop(default = false)]
    loading: bool,

    /// Callback when thread is selected
    #[prop(optional)]
    #[allow(dead_code)]
    _on_select: Option<Box<dyn Fn(String) + Send + Sync + 'static>>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let empty = threads.is_empty();

    let loading_skeleton = (0..6)
        .map(|_| {
            view! {
                <div class="bg-white border border-gray-200 rounded-lg p-4">
                    <div class="h-5 w-3/4 bg-gray-200 rounded animate-pulse mb-3" />
                    <div class="h-4 w-full bg-gray-200 rounded animate-pulse mb-2" />
                    <div class="h-4 w-1/2 bg-gray-200 rounded animate-pulse" />
                </div>
            }
        })
        .collect_view();

    let thread_items = threads
        .into_iter()
        .map(|thread| {
            view! { <ThreadListItem thread=thread /> }
        })
        .collect_view();

    view! {
        <div class={format!("w-full {}", class.unwrap_or_default())}>
            <div class="mb-6">
                <input
                    type="text"
                    placeholder="Search threads by title or provider..."
                    class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {if loading {
                    loading_skeleton.into_any()
                } else if empty {
                    view! {
                        <div class="col-span-full">
                            <div class="bg-gray-50 border border-gray-200 rounded-lg p-12 text-center">
                                <h3 class="mt-2 text-sm font-medium text-gray-900">No threads yet</h3>
                                <p class="mt-1 text-sm text-gray-500">Start a new conversation to create a thread.</p>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    thread_items.into_any()
                }}
            </div>
        </div>
    }
}
