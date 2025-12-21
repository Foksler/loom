/// Spinner component - animated loading indicator
///
/// A rotating circle spinner for indicating loading states.
/// Available in multiple sizes with customizable color.
use crate::prelude::*;

/// Spinner size
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpinnerSize {
    /// Extra small (w-3 h-3)
    Xs,
    /// Small (w-4 h-4)
    Sm,
    /// Medium (w-6 h-6, default)
    Md,
    /// Large (w-8 h-8)
    Lg,
    /// Extra large (w-12 h-12)
    Xl,
}

/// Spinner color variant
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpinnerColor {
    /// Blue (primary color)
    Primary,
    /// Gray
    Gray,
    /// White (for dark backgrounds)
    White,
}

/// Spinner component
///
/// # Example
///
/// ```rust
/// view! {
///     <Spinner />
///     <Spinner size=SpinnerSize::Lg color=SpinnerColor::Primary />
/// }
/// ```
#[component]
pub fn Spinner(
    /// Spinner size
    #[prop(default = SpinnerSize::Md)]
    size: SpinnerSize,

    /// Spinner color
    #[prop(default = SpinnerColor::Primary)]
    color: SpinnerColor,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let (width_class, height_class) = match size {
        SpinnerSize::Xs => ("w-3", "h-3"),
        SpinnerSize::Sm => ("w-4", "h-4"),
        SpinnerSize::Md => ("w-6", "h-6"),
        SpinnerSize::Lg => ("w-8", "h-8"),
        SpinnerSize::Xl => ("w-12", "h-12"),
    };

    let color_class = match color {
        SpinnerColor::Primary => "text-blue-600",
        SpinnerColor::Gray => "text-gray-400",
        SpinnerColor::White => "text-white",
    };

    let final_class = format!(
        "{} {} animate-spin {}",
        width_class,
        height_class,
        match &class {
            Some(c) => format!("{}", c),
            None => String::new(),
        }
    );

    view! {
        <svg
            class=final_class
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            role="status"
            aria-label="Loading"
        >
            <circle
                class=format!("opacity-25 {}", color_class)
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                stroke-width="4"
            />
            <path
                class=format!("opacity-75 {}", color_class)
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            />
        </svg>
    }
}
