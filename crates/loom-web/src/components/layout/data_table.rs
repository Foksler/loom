use crate::components::primitives::{Button, ButtonSize, ButtonVariant};
/// DataTable component - Renders tabular data with sortable columns and pagination
///
/// A composite component that combines primitives to display structured tabular data
/// with optional sorting and pagination navigation.
use crate::prelude::*;
use leptos::prelude::RwSignal;

/// Configuration for a table column
#[derive(Clone, Debug)]
pub struct Column {
    /// Unique column identifier
    pub id: String,
    /// Display label for the column header
    pub label: String,
    /// Optional width specification
    pub width: Option<String>,
    /// Whether this column can be sorted
    pub sortable: bool,
}

/// Sort direction
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SortDirection {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

/// DataTable state and props
#[derive(Clone, Debug)]
pub struct DataTableState {
    /// Currently sorted column ID
    pub sort_column: Option<String>,
    /// Sort direction
    pub sort_direction: SortDirection,
    /// Current page (0-based)
    pub current_page: usize,
    /// Items per page
    pub page_size: usize,
}

/// DataTable component - Renders tabular data with sorting and pagination
///
/// # Props
/// - `rows`: Vector of row data (each row is Vec<String>)
/// - `columns`: Vector of column configurations
/// - `sortable`: Enable/disable column sorting
/// - `pagination`: Enable/disable pagination controls
/// - `page_size`: Number of rows per page (default: 10)
///
/// # Example
///
/// ```rust
/// let columns = vec![
///     Column { id: "name".into(), label: "Name".into(), width: None, sortable: true },
///     Column { id: "email".into(), label: "Email".into(), width: None, sortable: true },
/// ];
/// let rows = vec![
///     vec!["Alice".into(), "alice@example.com".into()],
///     vec!["Bob".into(), "bob@example.com".into()],
/// ];
///
/// view! {
///     <DataTable
///         columns=columns
///         rows=rows
///         sortable=true
///         pagination=true
///         page_size=10
///     />
/// }
/// ```
#[component]
pub fn DataTable(
    /// Column definitions
    columns: Vec<Column>,
    /// Row data (each row is a Vec<String>)
    rows: Vec<Vec<String>>,
    /// Enable column sorting
    #[prop(default = true)]
    sortable: bool,
    /// Enable pagination controls
    #[prop(default = true)]
    pagination: bool,
    /// Rows per page
    #[prop(default = 10)]
    page_size: usize,
    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    // State management for sorting and pagination
    let state = RwSignal::new(DataTableState {
        sort_column: None,
        sort_direction: SortDirection::Asc,
        current_page: 0,
        page_size,
    });

    let container_class = format!(
        "overflow-x-auto rounded-lg border border-gray-200 {}",
        class.unwrap_or_default()
    );

    // Sort rows if sorting is enabled
    let rows_for_sort = rows.clone();
    let columns_for_sort = columns.clone();
    let sorted_rows = Memo::new(move |_| {
        let mut sorted = rows_for_sort.clone();

        if sortable {
            if let Some(sort_col) = &state.get().sort_column {
                if let Some(col_idx) = columns_for_sort.iter().position(|c| &c.id == sort_col) {
                    sorted.sort_by(|a, b| {
                        let cmp = a[col_idx].cmp(&b[col_idx]);
                        match state.get().sort_direction {
                            SortDirection::Asc => cmp,
                            SortDirection::Desc => cmp.reverse(),
                        }
                    });
                }
            }
        }
        sorted
    });

    // Paginate rows if pagination is enabled
    let paginated_rows = Memo::new(move |_| {
        let all_rows = sorted_rows.get();
        if pagination {
            let start = state.get().current_page * page_size;
            let end = (start + page_size).min(all_rows.len());
            all_rows[start..end].to_vec()
        } else {
            all_rows
        }
    });

    let rows_for_pages = rows.clone();
    let total_pages = Memo::new(move |_| {
        if pagination {
            rows_for_pages.len().div_ceil(page_size)
        } else {
            1
        }
    });

    let can_prev = Memo::new(move |_| state.get().current_page > 0);
    let can_next =
        Memo::new(move |_| state.get().current_page < total_pages.get().saturating_sub(1));

    view! {
        <div class=container_class>
            {/* Table */}
            <table class="w-full border-collapse">
                <thead>
                    <tr class="bg-gray-50 border-b border-gray-200">
                        {columns.into_iter().map(|col| {
                            let col_id = col.id.clone();
                            let is_sortable = col.sortable && sortable;
                            let is_sorted = state.get().sort_column.as_ref() == Some(&col.id);

                            view! {
                                <th
                                    class=format!(
                                        "px-6 py-3 text-left text-sm font-semibold text-gray-900 {}",
                                        if is_sortable { "cursor-pointer hover:bg-gray-100" } else { "" }
                                    )
                                    style=col.width.map(|w| format!("width: {}", w))
                                    on:click={
                                        let col_id = col_id.clone();
                                        move |_| {
                                            if is_sortable {
                                                let mut new_state = state.get();
                                                if new_state.sort_column.as_ref() == Some(&col_id) {
                                                    new_state.sort_direction = match new_state.sort_direction {
                                                        SortDirection::Asc => SortDirection::Desc,
                                                        SortDirection::Desc => SortDirection::Asc,
                                                    };
                                                } else {
                                                    new_state.sort_column = Some(col_id.clone());
                                                    new_state.sort_direction = SortDirection::Asc;
                                                }
                                                new_state.current_page = 0;
                                                state.set(new_state);
                                            }
                                        }
                                    }
                                >
                                    <div class="flex items-center gap-2">
                                        {col.label}
                                        {is_sorted.then(|| {
                                            let arrow = if state.get().sort_direction == SortDirection::Asc {
                                                "↑"
                                            } else {
                                                "↓"
                                            };
                                            view! { <span class="text-blue-600">{arrow}</span> }
                                        })}
                                    </div>
                                </th>
                            }
                        }).collect::<Vec<_>>()}
                    </tr>
                </thead>
                <tbody>
                    {paginated_rows.get().into_iter().enumerate().map(|(idx, row)| {
                        let bg_class = if idx % 2 == 0 { "bg-white" } else { "bg-gray-50" };
                        view! {
                            <tr class=format!("{} border-b border-gray-200 hover:bg-blue-50 transition-colors", bg_class)>
                                {row.into_iter().map(|cell| {
                                    view! {
                                        <td class="px-6 py-4 text-sm text-gray-700">{cell}</td>
                                    }
                                }).collect::<Vec<_>>()}
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </tbody>
            </table>

            {/* Empty state */}
            {rows.is_empty().then(|| {
                view! {
                    <div class="px-6 py-12 text-center text-gray-500">
                        "No data to display"
                    </div>
                }
            })}

            {/* Pagination controls */}
            {pagination.then(|| {
                view! {
                    <div class="flex items-center justify-between px-6 py-4 bg-gray-50 border-t border-gray-200">
                        <div class="text-sm text-gray-600">
                            {move || {
                                let state_val = state.get();
                                if rows.is_empty() {
                                    "No results".to_string()
                                } else {
                                    let start = state_val.current_page * page_size + 1;
                                    let end = ((state_val.current_page + 1) * page_size).min(rows.len());
                                    format!("Showing {} to {} of {} results", start, end, rows.len())
                                }
                            }}
                        </div>
                        <div class="flex gap-2">
                            <Button
                                variant=ButtonVariant::Secondary
                                size=ButtonSize::Sm
                                disabled=!can_prev.get()
                                on_click=Box::new(move |_| {
                                    let mut new_state = state.get();
                                    new_state.current_page = new_state.current_page.saturating_sub(1);
                                    state.set(new_state);
                                })
                            >
                                "Previous"
                            </Button>
                            <span class="px-3 py-2 text-sm text-gray-700">
                                {move || format!("{} / {}", state.get().current_page + 1, total_pages.get())}
                            </span>
                            <Button
                                variant=ButtonVariant::Secondary
                                size=ButtonSize::Sm
                                disabled=!can_next.get()
                                on_click=Box::new(move |_| {
                                    let mut new_state = state.get();
                                    new_state.current_page = (new_state.current_page + 1).min(total_pages.get() - 1);
                                    state.set(new_state);
                                })
                            >
                                "Next"
                            </Button>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
