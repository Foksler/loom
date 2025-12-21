/// Breadcrumbs component - navigation path
///
/// Displays hierarchical navigation path showing the user's location in the site.
/// Clickable items for navigation between levels.
use crate::prelude::*;

/// Single breadcrumb item
#[derive(Clone, Debug, PartialEq)]
pub struct BreadcrumbItem {
    /// Display label
    pub label: String,
    /// Optional href for navigation
    pub href: Option<String>,
}

impl BreadcrumbItem {
    /// Create a new breadcrumb item
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: None,
        }
    }

    /// Create a breadcrumb item with a link
    pub fn with_href(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: Some(href.into()),
        }
    }
}

/// Breadcrumbs component
///
/// # Example
///
/// ```rust
/// let items = vec![
///     BreadcrumbItem::with_href("Home", "/"),
///     BreadcrumbItem::with_href("Products", "/products"),
///     BreadcrumbItem::new("Electronics"),
/// ];
///
/// view! {
///     <Breadcrumbs items=items />
/// }
/// ```
#[component]
pub fn Breadcrumbs(
    /// Array of breadcrumb items
    items: Vec<BreadcrumbItem>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    view! {
        <nav
            class=format!(
                "flex items-center gap-2 text-sm{}",
                match &class {
                    Some(c) => format!(" {}", c),
                    None => String::new(),
                }
            )
            aria-label="breadcrumb"
        >
            <ol class="flex items-center gap-2">
                {items
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| {
                        let is_last = idx == items.len() - 1;
                        let label = item.label.clone();
                        let href = item.href.clone();
                        let href_clone = href.clone();
                        let label_for_span = label.clone();

                        view! {
                            <li class="flex items-center gap-2">
                                <Show
                                    when=move || href_clone.is_some()
                                    fallback=move || {
                                        let lbl = label_for_span.clone();
                                        view! {
                                            <span class=if is_last {
                                                "text-gray-900 font-medium"
                                            } else {
                                                "text-gray-700"
                                            }>
                                                {lbl}
                                            </span>
                                        }
                                    }
                                >
                                    {
                                        let lbl = label.clone();
                                        let href_val = href.clone();
                                        view! {
                                            <a
                                                href=href_val.unwrap_or_default()
                                                class="text-blue-600 hover:text-blue-800 hover:underline transition-colors"
                                            >
                                                {lbl}
                                            </a>
                                        }
                                    }
                                </Show>

                                <Show when=move || !is_last>
                                    <span class="text-gray-400">/</span>
                                </Show>
                            </li>
                        }
                    })
                    .collect_view()}
            </ol>
        </nav>
    }
}
