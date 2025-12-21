/// Panel component - lightweight container variant
///
/// A minimal container with optional padding and border.
/// Useful for grouped content with minimal visual weight.
use crate::prelude::*;

/// Panel padding size
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanelPadding {
    /// No padding
    None,
    /// Small padding (0.75rem)
    Sm,
    /// Medium padding (1.5rem)
    Md,
    /// Large padding (2rem)
    Lg,
}

/// Panel component
///
/// # Example
///
/// ```rust
/// view! {
///     <Panel>"Content here"</Panel>
///     <Panel padding=PanelPadding::Lg class="bg-gray-50">"Padded content"</Panel>
/// }
/// ```
#[component]
pub fn Panel(
    /// Padding size
    #[prop(default = PanelPadding::Md)]
    padding: PanelPadding,

    /// Show border
    #[prop(default = true)]
    bordered: bool,

    /// Panel content
    children: Children,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let padding_class = match padding {
        PanelPadding::None => "",
        PanelPadding::Sm => "p-3",
        PanelPadding::Md => "p-6",
        PanelPadding::Lg => "p-8",
    };

    let border_class = if bordered {
        "border border-gray-200"
    } else {
        ""
    };

    let final_class = format!(
        "rounded-md {}{}{}",
        padding_class,
        if !padding_class.is_empty() && !border_class.is_empty() {
            " "
        } else {
            ""
        },
        match (border_class, &class) {
            ("", None) => String::new(),
            ("", Some(c)) => c.to_string(),
            (b, None) => b.to_string(),
            (b, Some(c)) => format!("{} {}", b, c),
        }
    );

    view! {
        <div class=final_class>
            {children()}
        </div>
    }
}
