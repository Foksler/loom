/// ThreadDetailPage - Display and manage a single thread conversation
///
/// Handles:
/// - Getting thread ID from route params
/// - Rendering thread header + conversation
/// - Mock data display
use crate::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::threads::{Thread, ThreadStatus};

/// Thread detail page component
///
/// Displays a single thread with:
/// - Thread header (title, model, status)
/// - Conversation view (messages)
/// - Mock data for demo
#[component]
pub fn ThreadDetailPage() -> impl IntoView {
    let params = use_params_map();

    // Get thread ID from route params
    let thread_id = Memo::new(move |_| params.with(|p| p.get("id")));

    // Create mock thread data
    let thread_data = Memo::new(move |_| {
        thread_id.get().map(|thread_id| Thread {
            id: thread_id.clone(),
            title: format!("Thread: {}", thread_id),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            model: "gpt-4-turbo".to_string(),
            status: ThreadStatus::Active,
            messages: vec![],
            repository: None,
            tools: vec![],
        })
    });

    view! {
        <div class="h-full flex flex-col p-8">
            {move || {
                thread_data.get().map(|thread| {
                    view! {
                        <div class="flex-1 flex flex-col">
                            <h1 class="text-3xl font-bold mb-4">{thread.title}</h1>
                            <p class="text-gray-600 mb-8">
                                {"Model: "} {thread.model} {" | Status: "} {thread.status.as_str()}
                            </p>
                            {/* Placeholder for conversation view */}
                            <div class="flex-1 bg-gray-50 rounded-lg p-4 overflow-auto">
                                <p class="text-gray-400 text-center py-8">
                                    "Conversation view - mock data"
                                </p>
                            </div>
                        </div>
                    }
                })
            }}
        </div>
    }
}
