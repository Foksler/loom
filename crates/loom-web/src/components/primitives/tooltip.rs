/// Tooltip component - hover-triggered hint text
///
/// Displays contextual information on hover with configurable positioning.
/// Uses Tailwind for styling and positioning.
use crate::prelude::*;

/// Tooltip position relative to trigger element
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TooltipPosition {
    /// Above the element
    Top,
    /// Below the element
    Bottom,
    /// Left of the element
    Left,
    /// Right of the element
    Right,
}

/// Tooltip component
///
/// # Example
///
/// ```rust
/// view! {
///     <Tooltip text="Click to save" position=TooltipPosition::Top>
///         <Button>"Save"</Button>
///     </Tooltip>
/// }
/// ```
#[component]
pub fn Tooltip(
    /// Tooltip text content
    text: String,

    /// Position relative to trigger
    #[prop(default = TooltipPosition::Top)]
    position: TooltipPosition,

    /// Children (trigger element)
    children: Children,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let (is_hovering, set_hovering) = signal(false);

    let position_class = match position {
        TooltipPosition::Top => "bottom-full mb-2 left-1/2 -translate-x-1/2",
        TooltipPosition::Bottom => "top-full mt-2 left-1/2 -translate-x-1/2",
        TooltipPosition::Left => "right-full mr-2 top-1/2 -translate-y-1/2",
        TooltipPosition::Right => "left-full ml-2 top-1/2 -translate-y-1/2",
    };

    let arrow_class = match position {
        TooltipPosition::Top => "bottom-[-4px] left-1/2 -translate-x-1/2 border-l-4 border-r-4 border-t-4 border-l-transparent border-r-transparent border-t-gray-900",
        TooltipPosition::Bottom => "top-[-4px] left-1/2 -translate-x-1/2 border-l-4 border-r-4 border-b-4 border-l-transparent border-r-transparent border-b-gray-900",
        TooltipPosition::Left => "left-[-4px] top-1/2 -translate-y-1/2 border-t-4 border-b-4 border-l-4 border-t-transparent border-b-transparent border-l-gray-900",
        TooltipPosition::Right => "right-[-4px] top-1/2 -translate-y-1/2 border-t-4 border-b-4 border-r-4 border-t-transparent border-b-transparent border-r-gray-900",
    };

    view! {
        <div
            class=format!(
                "relative inline-block{}",
                match &class {
                    Some(c) => format!(" {}", c),
                    None => String::new(),
                }
            )
            on:mouseenter=move |_| set_hovering.set(true)
            on:mouseleave=move |_| set_hovering.set(false)
        >
            {children()}

            <Show when=move || is_hovering.get()>
                <div
                    class=format!(
                        "absolute z-50 px-2 py-1 text-sm font-medium text-white bg-gray-900 rounded whitespace-nowrap {}",
                        position_class
                    )
                >
                    {text.clone()}
                    <div class=format!("absolute {}", arrow_class)></div>
                </div>
            </Show>
        </div>
    }
}
