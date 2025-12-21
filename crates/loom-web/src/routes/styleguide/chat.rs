use crate::components::chat::{
    ConversationView, MessageBody, MessageBubble, MessageHeader, PromptComposer, StreamingCursor,
};
/// Chat components gallery
use crate::prelude::*;
use loom_core::message::{Message, Role};
use web_sys::console;

#[component]
pub fn StyleguideChatPage() -> impl IntoView {
    let sample_messages = vec![
        Message::user("Hello! Can you help me with Rust?"),
        Message::assistant("Of course! I'd be happy to help with Rust. What would you like to know?"),
        Message::user("How do I create a vector?"),
        Message::assistant("You can create a vector in Rust using the `vec!` macro or `Vec::new()`. Here's an example:\n\n```rust\n// Using vec! macro\nlet v = vec![1, 2, 3];\n\n// Using Vec::new()\nlet mut v: Vec<i32> = Vec::new();\nv.push(1);\nv.push(2);\n```\n\nThe `vec!` macro is more convenient when you know the initial values."),
        Message::tool("search_docs", "search_rust_docs", "Found 3 relevant docs about Vec"),
    ];

    view! {
        <div class="p-8 max-w-6xl">
            <h1 class="text-4xl font-bold mb-4 text-gray-900">Chat Components</h1>
            <p class="text-lg text-gray-600 mb-8">
                Components for conversation view, messages, composer, and streaming indicators.
            </p>

            {/* ConversationView Gallery */}
            <section class="mb-12">
                <h2 class="text-2xl font-semibold mb-4 text-gray-800">ConversationView</h2>
                <p class="text-gray-600 mb-4">
                    Scrollable container displaying a conversation with auto-scroll to bottom
                </p>

                <div class="bg-white p-6 rounded-lg border border-gray-200">
                    <div class="h-96 border border-gray-200 rounded-lg">
                        <ConversationView messages=sample_messages.clone() loading=false />
                    </div>
                </div>

                <div class="mt-6 bg-gray-50 p-4 rounded-lg">
                    <h3 class="font-mono text-sm font-semibold text-gray-900 mb-2">Usage</h3>
                    <pre class="bg-gray-900 text-gray-100 p-3 rounded text-xs overflow-x-auto"><code>{r#"<ConversationView
    messages=messages
    loading=false
/>"#}</code></pre>
                </div>
            </section>

            {/* MessageBubble Gallery */}
            <section class="mb-12">
                <h2 class="text-2xl font-semibold mb-4 text-gray-800">MessageBubble</h2>
                <p class="text-gray-600 mb-4">
                    Individual message with role-based styling
                </p>

                <div class="space-y-6">
                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">User Message</h3>
                        <div class="space-y-4">
                            <MessageBubble
                                message=Message::user("Hello! This is a user message")
                                provider="You".to_string()
                            />
                        </div>
                    </div>

                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">Assistant Message</h3>
                        <div class="space-y-4">
                            <MessageBubble
                                message=Message::assistant("This is an assistant response with **bold** and *italic* text")
                                provider="GPT-4".to_string()
                            />
                        </div>
                    </div>

                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">System Message</h3>
                        <div class="space-y-4">
                            <MessageBubble
                                message=Message::system("You are a helpful coding assistant")
                            />
                        </div>
                    </div>

                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">Tool Message</h3>
                        <div class="space-y-4">
                            <MessageBubble
                                message=Message::tool("search_001", "web_search", "Found 5 relevant results")
                            />
                        </div>
                    </div>
                </div>
            </section>

            {/* MessageBody Gallery */}
            <section class="mb-12">
                <h2 class="text-2xl font-semibold mb-4 text-gray-800">MessageBody</h2>
                <p class="text-gray-600 mb-4">
                    Renders content with markdown and code block support
                </p>

                <div class="space-y-4">
                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">Plain Text</h3>
                        <div class="bg-gray-50 p-4 rounded">
                            <MessageBody
                                content="This is plain text".to_string()
                                role=Role::Assistant
                            />
                        </div>
                    </div>

                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">Markdown Text</h3>
                        <div class="bg-gray-50 p-4 rounded">
                            <MessageBody
                                content="This has **bold** text, *italic* text, and `inline code`".to_string()
                                role=Role::Assistant
                            />
                        </div>
                    </div>

                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">Code Block</h3>
                        <div class="bg-gray-50 p-4 rounded">
                            <MessageBody
                                content="Here's a function:\n\n```rust\nfn greet(name: &str) {\n    println!(\"Hello, {}!\", name);\n}\n```\n\nThis prints a greeting.".to_string()
                                role=Role::Assistant
                            />
                        </div>
                    </div>
                </div>
            </section>

            {/* MessageHeader Gallery */}
            <section class="mb-12">
                <h2 class="text-2xl font-semibold mb-4 text-gray-800">MessageHeader</h2>
                <p class="text-gray-600 mb-4">
                    Metadata badges for role, model, timestamp, and token count
                </p>

                <div class="space-y-4">
                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">Basic</h3>
                        <MessageHeader role=Role::Assistant />
                    </div>

                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">With Model & Tokens</h3>
                        <MessageHeader
                            role=Role::Assistant
                            model="GPT-4".to_string()
                            tokens=1234
                        />
                    </div>

                    <div class="bg-white p-6 rounded-lg border border-gray-200">
                        <h3 class="text-sm font-semibold text-gray-700 mb-4">With Timestamp</h3>
                        <MessageHeader
                            role=Role::User
                            timestamp="2025-12-22 14:30".to_string()
                            model="Web User".to_string()
                        />
                    </div>
                </div>
            </section>

            {/* StreamingCursor Gallery */}
            <section class="mb-12">
                <h2 class="text-2xl font-semibold mb-4 text-gray-800">StreamingCursor</h2>
                <p class="text-gray-600 mb-4">
                    Blinking cursor for live message streaming
                </p>

                <div class="bg-white p-6 rounded-lg border border-gray-200">
                    <p class="text-gray-900">
                        "Assistant is typing" <StreamingCursor />
                    </p>
                </div>
            </section>

            {/* PromptComposer Gallery */}
            <section class="mb-12">
                <h2 class="text-2xl font-semibold mb-4 text-gray-800">PromptComposer</h2>
                <p class="text-gray-600 mb-4">
                    Text input with submit button and keyboard shortcuts
                </p>

                <div class="bg-white p-6 rounded-lg border border-gray-200">
                    <PromptComposer
                        on_submit=|text| {
                            console::log_1(&format!("Message submitted: {}", text).into());
                        }
                        placeholder="Type your message here..."
                        disabled=false
                    />
                </div>

                <div class="mt-6 bg-gray-50 p-4 rounded-lg">
                    <h3 class="font-mono text-sm font-semibold text-gray-900 mb-2">Features</h3>
                    <ul class="text-sm text-gray-700 space-y-1">
                        <li>"• Submit with Cmd/Ctrl + Enter or Send button"</li>
                        <li>"• Character counter (max 4000)"</li>
                        <li>"• Disabled state support"</li>
                        <li>"• Empty state validation"</li>
                    </ul>
                </div>
            </section>
        </div>
    }
}
