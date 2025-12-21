/// Application shell - main layout container
use crate::prelude::*;

/// Main application shell/layout wrapper
///
/// Provides the basic structure: top nav, sidebar, main content area.
#[component]
pub fn AppShell(children: Children) -> impl IntoView {
    view! {
        <div class="flex flex-col h-screen bg-gray-50">
            {/* Top navigation bar */}
            <header class="h-16 bg-white border-b border-gray-200 flex items-center px-8 shadow-sm">
                <h1 class="text-xl font-bold text-gray-900">Loom</h1>
            </header>

            {/* Main content area */}
            <div class="flex flex-1 overflow-hidden">
                {children()}
            </div>
        </div>
    }
}
