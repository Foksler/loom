use crate::components::chat::MessageBody;
use crate::components::primitives::{Badge, BadgeVariant};
/// MessageBubble - Individual message with role-based styling
///
/// Displays a single message with role-specific colors, styling,
/// and optional metadata like timestamp or provider info.
use crate::prelude::*;
use loom_core::message::Message;

#[cfg(test)]
#[path = "message_bubble_test.rs"]
mod message_bubble_test;

/// MessageBubble component
///
/// # Example
///
/// ```rust
/// let msg = Message::user("Hello!");
/// view! {
///     <MessageBubble message=msg />
/// }
/// ```
#[component]
pub fn MessageBubble(
    /// Message to display
    message: Message,

    /// Optional provider name (e.g., "GPT-4", "Claude")
    #[prop(optional)]
    provider: Option<String>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    use loom_core::message::Role;
    use leptos::prelude::RwSignal;
    
    let message = RwSignal::new(message);

    let (bg_class, text_class, align_class) = match message.get().role {
        Role::User => ("bg-blue-100", "text-gray-900", "ml-auto mr-0"),
        Role::Assistant => ("bg-gray-100", "text-gray-900", "mr-auto ml-0"),
        Role::System => ("bg-amber-50", "text-amber-900", "mr-auto ml-0"),
        Role::Tool => ("bg-green-50", "text-green-900", "mr-auto ml-0"),
    };

    let role_badge = match message.get().role {
        Role::User => Some("User"),
        Role::Assistant => Some("Assistant"),
        Role::System => Some("System"),
        Role::Tool => Some("Tool"),
    };

    let final_class = format!(
        "flex flex-col gap-2 max-w-xs lg:max-w-md {}{}",
        align_class,
        class
            .as_ref()
            .map(|c| format!(" {}", c))
            .unwrap_or_default()
    );

    view! {
        <div class=final_class>
            <div class="flex items-center gap-2">
                {role_badge.map(|badge| {
                    view! {
                        <Badge variant=match message.get().role {
                            loom_core::message::Role::User => BadgeVariant::Blue,
                            loom_core::message::Role::Assistant => BadgeVariant::Gray,
                            loom_core::message::Role::System => BadgeVariant::Yellow,
                            loom_core::message::Role::Tool => BadgeVariant::Green,
                        }>
                            {badge}
                        </Badge>
                    }
                })}

                {provider.map(|p| {
                    view! {
                        <Badge variant=BadgeVariant::Purple>{p}</Badge>
                    }
                })}
            </div>

            <div class=format!("{} {} rounded-lg p-3 shadow-sm", bg_class, text_class)>
                <MessageBody
                    content=message.get().content.clone()
                    role=message.get().role
                />
            </div>

            <Show when=move || message.get().name.is_some()>
                <div class="text-xs text-gray-500 px-3">
                    "Tool: " {move || message.get().name.clone().unwrap_or_default()}
                </div>
            </Show>
        </div>
    }
}
