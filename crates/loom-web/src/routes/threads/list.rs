/// ThreadListPage - Display and manage all threads
///
/// Handles:
/// - Displaying threads from app state
/// - Loading and error state rendering
/// - Search/filter by title
/// - "New Thread" navigation
use crate::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::components::threads::ThreadList;
use crate::services::state::use_app_state;

/// Thread list page component
///
/// Displays a searchable, filterable list of all threads with:
/// - Loading skeleton states
/// - Error boundary
/// - Search/filter functionality
/// - "New Thread" CTA button
#[component]
pub fn ThreadListPage() -> impl IntoView {
    let (search_filter, set_search_filter) = signal(String::new());
    let navigate = use_navigate();
    let app_state = use_app_state();

    // Filter threads based on search
    let filtered_threads = Memo::new(move |_| {
        let filter = search_filter.get().to_lowercase();
        let threads = app_state.threads.get();

        match threads {
            Some(all_threads) => {
                if filter.is_empty() {
                    all_threads
                } else {
                    all_threads
                        .into_iter()
                        .filter(|t| {
                            t.title.to_lowercase().contains(&filter)
                                || t.provider.to_lowercase().contains(&filter)
                        })
                        .collect()
                }
            }
            _ => vec![],
        }
    });

    let handle_select = {
        let navigate = navigate.clone();
        move |thread_id: String| {
            navigate(&format!("/threads/{}", thread_id), Default::default());
        }
    };

    let handle_new_thread = move |_| {
        navigate("/threads/new", Default::default());
    };

    view! {
        <div class="h-full flex flex-col p-8">
            {/* Header */}
            <div class="flex items-center justify-between mb-8">
                <h1 class="text-3xl font-bold">Threads</h1>
                <button
                    on:click=handle_new_thread
                    class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition"
                >
                    "+ New Thread"
                </button>
            </div>

            {/* Search bar */}
            <div class="mb-8">
                <input
                    type="text"
                    placeholder="Search threads by title or provider..."
                    on:input=move |ev| {
                        set_search_filter.set(event_target_value(&ev));
                    }
                    class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
            </div>

            {/* Thread list */}
            <ThreadList
                threads=filtered_threads.get()
                on_select=Box::new(handle_select)
            />
        </div>
    }
}
