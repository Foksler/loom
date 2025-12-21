/// Badge component - status indicator
///
/// Displays status or category labels with color variants.
/// Compact, non-interactive element typically used for metadata.
use crate::prelude::*;

/// Badge color variant for different statuses
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BadgeVariant {
    /// Gray - default/inactive
    Gray,
    /// Blue - info
    Blue,
    /// Green - success
    Green,
    /// Yellow - warning
    Yellow,
    /// Red - error/danger
    Red,
    /// Purple - custom
    Purple,
}

/// Badge size
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum BadgeSize {
    /// Small badge
    Sm,
    /// Medium badge
    #[default]
    Md,
}

/// Badge component
///
/// # Example
///
/// ```rust
/// view! {
///     <Badge variant=BadgeVariant::Green>"Active"</Badge>
///     <Badge variant=BadgeVariant::Red>"Error"</Badge>
///     <Badge variant=BadgeVariant::Yellow size=BadgeSize::Sm>"Warning"</Badge>
/// }
/// ```
#[component]
pub fn Badge(
    /// Badge color variant
    #[prop(default = BadgeVariant::Gray)]
    variant: BadgeVariant,

    /// Badge size
    #[prop(default = BadgeSize::Md)]
    size: BadgeSize,

    /// Badge content/children
    children: Children,

    /// Optional CSS class
    #[prop(optional)]
    _class: Option<String>,
) -> impl IntoView {
    let variant_class = match variant {
        BadgeVariant::Gray => "bg-gray-100 text-gray-800",
        BadgeVariant::Blue => "bg-blue-100 text-blue-800",
        BadgeVariant::Green => "bg-green-100 text-green-800",
        BadgeVariant::Yellow => "bg-yellow-100 text-yellow-800",
        BadgeVariant::Red => "bg-red-100 text-red-800",
        BadgeVariant::Purple => "bg-purple-100 text-purple-800",
    };

    let size_class = match size {
        BadgeSize::Sm => "px-2 py-1 text-xs font-medium rounded",
        BadgeSize::Md => "px-2.5 py-1.5 text-sm font-medium rounded-md",
    };

    view! {
        <span
            class=format!(
                "inline-flex items-center gap-1.5 {}{}{}",
                variant_class,
                " ",
                size_class
            )
        >
            {children()}
        </span>
    }
}
