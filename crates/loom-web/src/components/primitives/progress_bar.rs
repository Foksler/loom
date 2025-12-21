/// ProgressBar component - linear progress indicator
///
/// A horizontal progress bar for showing completion percentage.
/// Supports different sizes and color variants.
use crate::prelude::*;

/// ProgressBar size
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProgressBarSize {
    /// Small height (h-1)
    Sm,
    /// Medium height (h-2, default)
    Md,
    /// Large height (h-3)
    Lg,
}

/// ProgressBar color variant
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProgressBarColor {
    /// Blue (primary, default)
    Primary,
    /// Green (success)
    Success,
    /// Yellow (warning)
    Warning,
    /// Red (danger/error)
    Danger,
    /// Gray (neutral)
    Gray,
}

/// ProgressBar component
///
/// # Example
///
/// ```rust
/// view! {
///     <ProgressBar value=50 />
///     <ProgressBar value=75 color=ProgressBarColor::Success size=ProgressBarSize::Lg />
///     <ProgressBar value=25 color=ProgressBarColor::Warning />
/// }
/// ```
#[component]
pub fn ProgressBar(
    /// Progress percentage (0-100)
    #[prop(default = 0)]
    value: i32,

    /// Size variant
    #[prop(default = ProgressBarSize::Md)]
    size: ProgressBarSize,

    /// Color variant
    #[prop(default = ProgressBarColor::Primary)]
    color: ProgressBarColor,

    /// Show percentage text
    #[prop(default = false)]
    show_label: bool,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let clamped_value = value.max(0).min(100);

    let size_class = match size {
        ProgressBarSize::Sm => "h-1",
        ProgressBarSize::Md => "h-2",
        ProgressBarSize::Lg => "h-3",
    };

    let color_class = match color {
        ProgressBarColor::Primary => "bg-blue-600",
        ProgressBarColor::Success => "bg-green-600",
        ProgressBarColor::Warning => "bg-yellow-600",
        ProgressBarColor::Danger => "bg-red-600",
        ProgressBarColor::Gray => "bg-gray-400",
    };

    let final_class = format!(
        "w-full bg-gray-200 rounded-full overflow-hidden {}{}",
        size_class,
        match &class {
            Some(c) => format!(" {}", c),
            None => String::new(),
        }
    );

    view! {
        <div>
            <div class=final_class>
                <div
                    class=format!("h-full {} transition-all duration-300", color_class)
                    style=format!("width: {}%", clamped_value)
                    role="progressbar"
                    aria-valuenow=clamped_value
                    aria-valuemin="0"
                    aria-valuemax="100"
                />
            </div>
            <Show when=move || show_label>
                <div class="text-sm text-gray-600 mt-1">
                    {format!("{}%", clamped_value)}
                </div>
            </Show>
        </div>
    }
}
