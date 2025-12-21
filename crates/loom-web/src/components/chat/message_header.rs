use crate::components::primitives::{Badge, BadgeSize, BadgeVariant};
/// MessageHeader - Metadata above message
///
/// Displays role, timestamp, model info, and token counts
/// as small badges above the message content.
use crate::prelude::*;
use loom_core::message::Role;

/// MessageHeader component
///
/// # Example
///
/// ```rust
/// view! {
///     <MessageHeader
///         role=Role::Assistant
///         model=Some("GPT-4".to_string())
///         tokens=Some(1234)
///     />
/// }
/// ```
#[component]
pub fn MessageHeader(
    /// Message role
    role: Role,

    /// Timestamp of message (ISO format)
    #[prop(optional)]
    timestamp: Option<String>,

    /// Model/provider name
    #[prop(optional)]
    model: Option<String>,

    /// Token count for the message
    #[prop(optional)]
    tokens: Option<u32>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let final_class = format!(
        "flex flex-wrap items-center gap-2 text-xs{}",
        class
            .as_ref()
            .map(|c| format!(" {}", c))
            .unwrap_or_default()
    );

    view! {
        <div class=final_class>
            <Badge variant=match role {
                Role::User => BadgeVariant::Blue,
                Role::Assistant => BadgeVariant::Gray,
                Role::System => BadgeVariant::Yellow,
                Role::Tool => BadgeVariant::Green,
            } size=BadgeSize::Sm>
                {match role {
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                    Role::System => "System",
                    Role::Tool => "Tool",
                }}
            </Badge>

            {model.map(|m| {
                view! {
                    <Badge variant=BadgeVariant::Purple size=BadgeSize::Sm>
                        {m}
                    </Badge>
                }
            })}

            {timestamp.map(|ts| {
                view! {
                    <span class="text-gray-500">{ts}</span>
                }
            })}

            {tokens.map(|t| {
                view! {
                    <Badge variant=BadgeVariant::Gray size=BadgeSize::Sm>
                        {format!("{} tokens", t)}
                    </Badge>
                }
            })}
        </div>
    }
}
