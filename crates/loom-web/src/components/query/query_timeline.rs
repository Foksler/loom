/// QueryTimeline component - shows query execution steps
use crate::prelude::*;

use super::types::QueryStep;

/// QueryTimeline displays the steps executed during query processing
///
/// # Example
///
/// ```rust
/// let steps = vec![
///     QueryStep::new("Parse query"),
/// ];
/// view! {
///     <QueryTimeline steps=steps />
/// }
/// ```
#[component]
pub fn QueryTimeline(
    /// Steps in the query execution
    #[prop()]
    steps: Vec<QueryStep>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    view! {
        <div class=format!("space-y-0 {}", class.unwrap_or_default())>
            <div class="text-center text-gray-500 py-8">
                {format!("Query steps: {}", steps.len())}
            </div>
        </div>
    }
}
