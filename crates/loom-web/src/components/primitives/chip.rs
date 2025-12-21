/// Chip component - dismissible tag
///
/// Compact, removable element typically used for selections, tags, or filters.
/// Includes optional icon and close button.
use crate::prelude::*;

/// Chip color variant
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChipVariant {
    /// Gray - default
    Gray,
    /// Blue - primary
    Blue,
    /// Green - success
    Green,
    /// Yellow - warning
    Yellow,
    /// Red - error
    Red,
}

/// Chip component
///
/// # Example
///
/// ```rust
/// let (chips, set_chips) = create_signal(vec!["React", "TypeScript", "Tailwind"]);
/// view! {
///     {move || chips().iter().map(|chip| {
///         let chip_name = chip.to_string();
///         view! {
///             <Chip
///                 variant=ChipVariant::Blue
///                 on_remove=move || {
///                     set_chips(chips().iter().filter(|c| **c != chip_name).collect())
///                 }
///             >
///                 {chip}
///             </Chip>
///         }
///     }).collect_view()}
/// }
/// ```
#[component]
pub fn Chip(
    /// Chip color variant
    #[prop(default = ChipVariant::Gray)]
    variant: ChipVariant,

    /// Chip content/children
    children: Children,

    /// Callback when chip is dismissed
    #[prop(optional)]
    on_remove: Option<Box<dyn Fn() + 'static + Send + Sync>>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let variant_class = match variant {
        ChipVariant::Gray => "bg-gray-200 text-gray-900 hover:bg-gray-300",
        ChipVariant::Blue => "bg-blue-100 text-blue-900 hover:bg-blue-200",
        ChipVariant::Green => "bg-green-100 text-green-900 hover:bg-green-200",
        ChipVariant::Yellow => "bg-yellow-100 text-yellow-900 hover:bg-yellow-200",
        ChipVariant::Red => "bg-red-100 text-red-900 hover:bg-red-200",
    };

    let has_on_remove = on_remove.is_some();
    let on_remove = std::sync::Arc::new(on_remove);

    view! {
        <span
            class=format!(
                "inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full transition-colors {}{}",
                variant_class,
                match &class {
                    Some(c) => format!(" {}", c),
                    None => String::new(),
                }
            )
        >
            {children()}

            <Show when=move || has_on_remove>
                <button
                    class="ml-0.5 text-current opacity-70 hover:opacity-100 focus:outline-none"
                    on:click={
                        let on_remove = on_remove.clone();
                        move |e: web_sys::MouseEvent| {
                            e.stop_propagation();
                            if let Some(ref callback) = on_remove.as_ref() {
                                callback();
                            }
                        }
                    }
                >
                    <svg
                        class="w-4 h-4"
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox="0 0 20 20"
                        fill="currentColor"
                    >
                        <path
                            fill-rule="evenodd"
                            d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                            clip-rule="evenodd"
                        />
                    </svg>
                </button>
            </Show>
        </span>
    }
}
