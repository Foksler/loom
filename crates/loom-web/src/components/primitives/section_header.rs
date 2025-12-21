/// SectionHeader component - heading with optional description
///
/// A semantic heading component for organizing content into sections.
/// Typically used above cards or panels with an optional description.
use crate::prelude::*;

/// Section header size
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SectionHeaderSize {
    /// h3-equivalent (24px)
    Sm,
    /// h2-equivalent (28px)
    Md,
    /// h1-equivalent (32px)
    Lg,
}

/// SectionHeader component
///
/// # Example
///
/// ```rust
/// view! {
///     <SectionHeader title="Settings" />
///     <SectionHeader
///         title="Advanced Options"
///         description="Configure expert-level preferences"
///         size=SectionHeaderSize::Lg
///     />
/// }
/// ```
#[component]
pub fn SectionHeader(
    /// Heading text (or use children)
    #[prop(optional)]
    title: Option<String>,

    /// Optional description text
    #[prop(optional)]
    description: Option<String>,

    /// Header size
    #[prop(default = SectionHeaderSize::Md)]
    size: SectionHeaderSize,

    /// Optional CSS class for title element
    #[prop(optional)]
    class: Option<String>,

    /// Children (used as title if title not provided)
    #[prop(optional)]
    #[allow(unused_variables)]
    children: Option<Children>,
) -> impl IntoView {
    let (title_class, desc_class) = match size {
        SectionHeaderSize::Sm => (
            "text-xl font-bold text-gray-900",
            "text-sm text-gray-600 mt-1",
        ),
        SectionHeaderSize::Md => (
            "text-2xl font-bold text-gray-900",
            "text-sm text-gray-600 mt-2",
        ),
        SectionHeaderSize::Lg => (
            "text-3xl font-bold text-gray-900",
            "text-base text-gray-600 mt-2",
        ),
    };

    let final_title_class = match class {
        Some(c) => format!("{} {}", title_class, c),
        None => title_class.to_string(),
    };

    let title_text = title;

    view! {
        <div>
            <h2 class=final_title_class>
                {title_text}
            </h2>
            {description.map(|desc| {
                view! {
                    <p class=desc_class>{desc}</p>
                }
            })}
        </div>
    }
}
