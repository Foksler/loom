use crate::components::chat::MessageBubble;
use crate::components::primitives::{Spinner, SpinnerSize};
/// ConversationView - Scrollable container for messages
///
/// Displays a list of messages with auto-scroll to bottom,
/// loading indicator, and empty state handling.
use crate::prelude::*;
use leptos::html;
use leptos::prelude::{Effect, NodeRef, RwSignal};
use loom_core::message::Message;

/// ConversationView component
///
/// # Example
///
/// ```rust
/// let messages = vec![
///     Message::user("Hello"),
///     Message::assistant("Hi there!"),
/// ];
/// view! {
///     <ConversationView messages=messages.into() loading=false />
/// }
/// ```
#[component]
pub fn ConversationView(
    /// Messages to display
    messages: Vec<Message>,

    /// Loading state indicator
    #[prop(default = false)]
    loading: bool,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let node_ref = NodeRef::<html::Div>::new();
    let messages_signal = RwSignal::new(messages);

    // Auto-scroll to bottom when messages change
    Effect::new(move || {
        let _len = messages_signal.get().len();
        if let Some(el) = node_ref.get() {
            el.set_scroll_top(el.scroll_height());
        }
    });

    let final_class = format!(
        "flex flex-col h-full bg-white rounded-lg border border-gray-200 {}",
        class.unwrap_or_default()
    );

    view! {
        <div class=final_class>
            <Show when=move || messages_signal.get().is_empty() && !loading>
                <div class="flex-1 flex items-center justify-center">
                    <div class="text-center">
                        <p class="text-gray-500 text-lg font-medium">No messages yet</p>
                        <p class="text-gray-400 text-sm mt-1">
                            Start a conversation by sending a message
                        </p>
                    </div>
                </div>
            </Show>

            <div
                node_ref=node_ref
                class="flex-1 overflow-y-auto p-4 space-y-4"
                role="log"
                aria-label="Chat messages"
                aria-live="polite"
            >
                {move || {
                    messages_signal.get()
                        .into_iter()
                        .map(|msg| {
                            view! {
                                <MessageBubble message=msg.clone() />
                            }
                        })
                        .collect_view()
                }}

                <Show when=move || loading>
                    <div class="flex items-center gap-2 text-gray-600">
                        <Spinner size=SpinnerSize::Sm />
                        <span class="text-sm">Assistant is typing</span>
                    </div>
                </Show>
            </div>
        </div>
    }
}
