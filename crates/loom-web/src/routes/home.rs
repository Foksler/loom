/// Home page / workspace overview
///
/// Landing page showing:
/// - Quick start
/// - Recent threads
/// - System status
use crate::prelude::*;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100">
            <div class="text-center">
                <h1 class="text-5xl font-bold text-gray-900 mb-4">
                    Welcome to Loom
                </h1>
                <p class="text-xl text-gray-600 mb-8">
                    AI-powered coding assistant
                </p>
                <div class="flex gap-4 justify-center">
                    <a
                        href="/threads"
                        class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition"
                    >
                        View Threads
                    </a>
                    <a
                        href="/styleguide"
                        class="px-6 py-3 bg-gray-200 text-gray-900 rounded-lg hover:bg-gray-300 transition"
                    >
                        Component Gallery
                    </a>
                </div>
            </div>
        </div>
    }
}
