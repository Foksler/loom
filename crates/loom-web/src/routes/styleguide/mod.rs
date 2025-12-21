pub mod chat;
/// Styleguide / component gallery routes
///
/// Interactive, Storybook-like documentation of all UI components
/// organized by tier (primitives, composites, domain-specific).
pub mod index;
pub mod layout;
pub mod primitives;
pub mod query;
pub mod results;

pub use chat::StyleguideChatPage;
pub use index::StyleguideIndexPage;
pub use layout::StyleguideLayoutPage;
pub use primitives::StyleguidePrimitivesPage;
pub use query::StyleguideQueryPage;
pub use results::StyleguideResultsPage;

use crate::prelude::*;
use leptos_router::nested_router::Outlet;

/// Outer layout for styleguide pages
///
/// Provides navigation sidebar and main content area.
#[component]
pub fn StyleguideLayout() -> impl IntoView {
    view! {
        <div class="flex h-screen bg-gray-50">
            {/* Styleguide sidebar */}
            <aside class="w-64 bg-white border-r border-gray-200 p-6 overflow-auto">
                <h2 class="text-lg font-bold mb-6 text-gray-900">Component Gallery</h2>
                <nav class="space-y-2">
                    <a
                        href="/styleguide"
                        class="block px-4 py-2 rounded hover:bg-gray-100 text-gray-700"
                    >
                        Overview
                    </a>
                    <a
                        href="/styleguide/primitives"
                        class="block px-4 py-2 rounded hover:bg-gray-100 text-gray-700"
                    >
                        Primitives
                    </a>
                    <a
                        href="/styleguide/chat"
                        class="block px-4 py-2 rounded hover:bg-gray-100 text-gray-700"
                    >
                        Chat Components
                    </a>
                    <a
                        href="/styleguide/query"
                        class="block px-4 py-2 rounded hover:bg-gray-100 text-gray-700"
                    >
                        Query Bridge
                    </a>
                    <a
                        href="/styleguide/results"
                        class="block px-4 py-2 rounded hover:bg-gray-100 text-gray-700"
                    >
                        Results & Code
                    </a>
                    <a
                        href="/styleguide/layout"
                        class="block px-4 py-2 rounded hover:bg-gray-100 text-gray-700"
                    >
                        Layout & Shell
                    </a>
                </nav>
            </aside>

            {/* Main content */}
            <main class="flex-1 overflow-auto">
                <Outlet/>
            </main>
        </div>
    }
}
