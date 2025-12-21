/// KeyValueList component - Display key-value pairs in readable format
///
/// A composite component for displaying structured key-value data with semantic HTML
/// and optional alternating row colors.
use crate::prelude::*;

/// KeyValueList component - Display key-value pairs
///
/// Renders key-value pairs in a clean, semantic format with optional
/// visual enhancements like alternating row colors.
///
/// # Props
/// - `items`: Vector of (key, value) string pairs
/// - `label_width`: Optional fixed width for labels (e.g., "150px", "20%")
/// - `dense`: Reduce padding for compact display
/// - `striped`: Enable alternating row colors
///
/// # Example
///
/// ```rust
/// let items = vec![
///     ("Name".into(), "Alice Johnson".into()),
///     ("Email".into(), "alice@example.com".into()),
///     ("Role".into(), "Admin".into()),
/// ];
///
/// view! {
///     <KeyValueList
///         items=items
///         label_width=Some("150px".into())
///         striped=true
///     />
/// }
/// ```
#[component]
pub fn KeyValueList(
    /// Vector of (key, value) pairs
    items: Vec<(String, String)>,
    /// Optional fixed width for label column
    #[prop(optional)]
    label_width: Option<String>,
    /// Reduce padding for compact layout
    #[prop(default = false)]
    dense: bool,
    /// Enable alternating row background colors
    #[prop(default = true)]
    striped: bool,
    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let padding = if dense { "px-4 py-2" } else { "px-6 py-4" };

    let label_style = label_width.map(|w| format!("width: {}", w));

    let items_empty = items.is_empty();
    view! {
        <div class=format!(
            "rounded-lg border border-gray-200 overflow-hidden {}",
            class.unwrap_or_default()
        )>
            <dl class="divide-y divide-gray-200 bg-white">
                {items.into_iter().enumerate().map(|(idx, (key, value))| {
                    let bg_class = if striped && idx % 2 == 1 {
                        "bg-gray-50"
                    } else {
                        "bg-white"
                    };

                    view! {
                        <div class=format!("{} {} hover:bg-blue-50 transition-colors", bg_class, padding)>
                            <div class="flex gap-4">
                                <dt
                                    class="font-semibold text-gray-700 flex-shrink-0"
                                    style=label_style.clone()
                                >
                                    {key}
                                </dt>
                                <dd class="text-gray-900 flex-1 break-words">
                                    {value}
                                </dd>
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </dl>
            {items_empty.then(|| {
                view! {
                    <div class=format!("text-center text-gray-500 {}", padding)>
                        "No items to display"
                    </div>
                }
            })}
        </div>
    }
}
