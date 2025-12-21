/// Button component - primary interactive element
///
/// Supports multiple variants, sizes, and states (disabled, loading).
/// Built entirely with Tailwind CSS.
use crate::prelude::*;

#[cfg(test)]
#[path = "button_test.rs"]
mod button_test;

/// Button visual variant
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonVariant {
    /// Solid brand color, high-emphasis
    Primary,
    /// Muted background, secondary action
    Secondary,
    /// No background, text only
    Ghost,
    /// Red/destructive styling
    Destructive,
}

/// Button size
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonSize {
    /// Small padding and text
    Sm,
    /// Default size
    Md,
    /// Large padding and text
    Lg,
}

/// Button component
///
/// # Example
///
/// ```rust
/// view! {
///     <Button>"Click me"</Button>
///     <Button variant=ButtonVariant::Secondary>"Cancel"</Button>
///     <Button disabled=true>"Disabled"</Button>
/// }
/// ```
#[component]
pub fn Button(
    /// Visual variant
    #[prop(default = ButtonVariant::Primary)]
    variant: ButtonVariant,

    /// Button size
    #[prop(default = ButtonSize::Md)]
    size: ButtonSize,

    /// Disabled state
    #[prop(default = false)]
    disabled: bool,

    /// Loading state (shows spinner)
    #[prop(default = false)]
    loading: bool,

    /// Button label/children
    children: Children,

    /// Optional click handler
    #[prop(optional)]
    on_click: Option<Box<dyn Fn(web_sys::MouseEvent)>>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let variant_class = match variant {
        ButtonVariant::Primary => "bg-blue-600 text-white hover:bg-blue-700 active:bg-blue-800",
        ButtonVariant::Secondary => {
            "bg-gray-200 text-gray-900 hover:bg-gray-300 active:bg-gray-400"
        }
        ButtonVariant::Ghost => "text-gray-700 hover:bg-gray-100 active:bg-gray-200",
        ButtonVariant::Destructive => "bg-red-600 text-white hover:bg-red-700 active:bg-red-800",
    };

    let size_class = match size {
        ButtonSize::Sm => "px-2.5 py-1.5 text-sm font-medium rounded",
        ButtonSize::Md => "px-3.5 py-2 text-sm font-medium rounded-md",
        ButtonSize::Lg => "px-4 py-2.5 text-base font-medium rounded-lg",
    };

    let _disabled_class = if disabled || loading {
        "opacity-60 cursor-not-allowed"
    } else {
        "cursor-pointer"
    };

    let final_class = format!(
        "inline-flex items-center justify-center gap-2 transition-colors {}{}{}{}",
        variant_class,
        " ",
        size_class,
        match &class {
            Some(c) => format!(" {}", c),
            None => String::new(),
        }
    );

    view! {
        <button
            class=final_class
            disabled=disabled || loading
            on:click=move |ev| {
                if let Some(ref handler) = on_click {
                    handler(ev);
                }
            }
        >
            <Show when=move || loading>
                <svg
                    class="w-4 h-4 animate-spin"
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                >
                    <circle
                        class="opacity-25"
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        stroke-width="4"
                    />
                    <path
                        class="opacity-75"
                        fill="currentColor"
                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    />
                </svg>
            </Show>
            {children()}
        </button>
    }
}
