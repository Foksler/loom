/// StreamingCursor - Blinking cursor for live streaming
///
/// Shows a blinking cursor animation during message streaming.
/// Useful for indicating that the assistant is still typing.
use crate::prelude::*;

/// StreamingCursor component
///
/// # Example
///
/// ```rust
/// view! {
///     <StreamingCursor />
/// }
/// ```
#[component]
pub fn StreamingCursor(
    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let final_class = format!(
        "inline-block w-1 h-5 ml-1 bg-gray-900 animate-pulse{}",
        class
            .as_ref()
            .map(|c| format!(" {}", c))
            .unwrap_or_default()
    );

    view! {
        <span
            class=final_class
            role="status"
            aria-label="Assistant is typing"
        />
    }
}
