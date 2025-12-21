/// Card component - container with padding, border, and shadow
///
/// A flexible card container for organizing content with consistent styling.
/// Supports elevation levels for depth.
use crate::prelude::*;

/// Card elevation/shadow level
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardElevation {
    /// No shadow (flat appearance)
    None,
    /// Subtle shadow
    Sm,
    /// Medium shadow (default)
    Md,
    /// Strong shadow
    Lg,
}

/// Card component
///
/// # Example
///
/// ```rust
/// view! {
///     <Card>"Content here"</Card>
///     <Card elevation=CardElevation::Lg>"Important content"</Card>
/// }
/// ```
#[component]
pub fn Card(
    /// Shadow/elevation level
    #[prop(default = CardElevation::Md)]
    elevation: CardElevation,

    /// Optional background color (default: white)
    #[prop(default = true)]
    white_bg: bool,

    /// Card content
    children: Children,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let elevation_class = match elevation {
        CardElevation::None => "",
        CardElevation::Sm => "shadow-sm",
        CardElevation::Md => "shadow",
        CardElevation::Lg => "shadow-lg",
    };

    let bg_class = if white_bg { "bg-white" } else { "" };

    let final_class = format!(
        "rounded-lg border border-gray-200 p-6 {}{}{}",
        bg_class,
        if !bg_class.is_empty() { " " } else { "" },
        match (elevation_class, &class) {
            ("", None) => String::new(),
            ("", Some(c)) => format!(" {}", c),
            (e, None) => format!(" {}", e),
            (e, Some(c)) => format!(" {} {}", e, c),
        }
    );

    view! {
        <div class=final_class>
            {children()}
        </div>
    }
}
