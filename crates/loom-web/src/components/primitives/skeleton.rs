/// Skeleton component - placeholder content loader
///
/// A pulsing placeholder skeleton for showing loading states.
/// Useful for content that hasn't loaded yet.
use crate::prelude::*;

/// Skeleton shape variant
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkeletonShape {
    /// Rectangular block (default)
    Block,
    /// Circular (for avatars)
    Circle,
    /// Text line (slightly smaller height)
    Line,
}

/// Skeleton component
///
/// # Example
///
/// ```rust
/// view! {
///     <Skeleton class="h-8 w-32" />
///     <Skeleton shape=SkeletonShape::Circle class="w-10 h-10" />
///     <div class="space-y-2">
///         <Skeleton shape=SkeletonShape::Line class="w-full" />
///         <Skeleton shape=SkeletonShape::Line class="w-5/6" />
///     </div>
/// }
/// ```
#[component]
pub fn Skeleton(
    /// Shape variant
    #[prop(default = SkeletonShape::Block)]
    shape: SkeletonShape,

    /// Optional width class (e.g., "w-32", "w-full")
    #[prop(optional)]
    width: Option<String>,

    /// Optional height class (e.g., "h-8", "h-12")
    #[prop(optional)]
    height: Option<String>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let shape_class = match shape {
        SkeletonShape::Block => "rounded-lg",
        SkeletonShape::Circle => "rounded-full",
        SkeletonShape::Line => "rounded h-4",
    };

    let width_str = width.unwrap_or_else(|| "w-full".to_string());
    let height_str = height.unwrap_or_else(|| match shape {
        SkeletonShape::Block => "h-8".to_string(),
        SkeletonShape::Circle => "h-10 w-10".to_string(),
        SkeletonShape::Line => "h-4".to_string(),
    });

    let final_class = format!(
        "bg-gray-200 animate-pulse {} {} {}{}",
        shape_class,
        width_str,
        height_str,
        match &class {
            Some(c) => format!(" {}", c),
            None => String::new(),
        }
    );

    view! {
        <div class=final_class />
    }
}
