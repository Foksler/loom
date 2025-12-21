/// Styleguide index / overview page
use crate::prelude::*;

#[component]
pub fn StyleguideIndexPage() -> impl IntoView {
    view! {
        <div class="p-8 max-w-4xl">
            <h1 class="text-4xl font-bold mb-4 text-gray-900">Component Gallery</h1>
            <p class="text-lg text-gray-600 mb-8">
                Loom design system and reusable components, organized by tier.
            </p>

            <div class="grid grid-cols-1 gap-6">
                {/* Primitives card */}
                <a
                    href="/styleguide/primitives"
                    class="block p-6 bg-white rounded-lg border border-gray-200 hover:border-blue-400 hover:shadow-md transition"
                >
                    <h2 class="text-2xl font-bold text-gray-900 mb-2">Primitives</h2>
                    <p class="text-gray-600">
                        Core design system: buttons, inputs, typography, feedback elements.
                    </p>
                </a>

                {/* Chat components card */}
                <a
                    href="/styleguide/chat"
                    class="block p-6 bg-white rounded-lg border border-gray-200 hover:border-blue-400 hover:shadow-md transition"
                >
                    <h2 class="text-2xl font-bold text-gray-900 mb-2">Chat Components</h2>
                    <p class="text-gray-600">
                        Conversation view, messages, composer, streaming indicators.
                    </p>
                </a>

                {/* Query bridge card */}
                <a
                    href="/styleguide/query"
                    class="block p-6 bg-white rounded-lg border border-gray-200 hover:border-blue-400 hover:shadow-md transition"
                >
                    <h2 class="text-2xl font-bold text-gray-900 mb-2">Query Bridge</h2>
                    <p class="text-gray-600">
                        Query timeline, state machine trace, tool invocations.
                    </p>
                </a>

                {/* Results & code card */}
                <a
                    href="/styleguide/results"
                    class="block p-6 bg-white rounded-lg border border-gray-200 hover:border-blue-400 hover:shadow-md transition"
                >
                    <h2 class="text-2xl font-bold text-gray-900 mb-2">Results & Code</h2>
                    <p class="text-gray-600">
                        Code blocks, diff views, file trees, execution results.
                    </p>
                </a>

                {/* Layout card */}
                <a
                    href="/styleguide/layout"
                    class="block p-6 bg-white rounded-lg border border-gray-200 hover:border-blue-400 hover:shadow-md transition"
                >
                    <h2 class="text-2xl font-bold text-gray-900 mb-2">Layout & Shell</h2>
                    <p class="text-gray-600">
                        App shell, resizable panels, navigation structures.
                    </p>
                </a>
            </div>
        </div>
    }
}
