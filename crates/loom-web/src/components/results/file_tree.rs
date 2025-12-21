use crate::components::results::types::FileNode;
use crate::prelude::*;

/// FileTree component for exploring nested file structures
#[component]
pub fn FileTree(
    /// Tree structure to display
    files: Vec<FileNode>,

    /// Currently selected file path
    #[prop(into)]
    selected: RwSignal<Option<String>>,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    view! {
        <div class=format!(
            "border border-gray-200 rounded-lg bg-white overflow-hidden{}",
            class.map(|c| format!(" {}", c)).unwrap_or_default()
        )>
            <div class="overflow-y-auto max-h-96">
                <ul class="list-none p-0 m-0">
                    {files
                        .into_iter()
                        .map(|node| {
                            view! {
                                <FileTreeNode
                                    node=node
                                    selected=selected
                                    depth=0
                                />
                            }
                        })
                        .collect_view()}
                </ul>
            </div>
        </div>
    }
}

/// Internal file tree node
#[component]
fn FileTreeNode(
    node: FileNode,
    selected: RwSignal<Option<String>>,
    #[prop(default = 0)] depth: usize,
) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let node_path = node.path.clone();
    let node_name = node.name.clone();
    let is_dir = node.is_dir;
    let children_list = node.children.clone();
    let icon = node.icon().to_string();

    let node_path_click = node_path.clone();
    let handle_click = move |_| {
        selected.set(Some(node_path_click.clone()));
    };

    let handle_toggle = move |_| {
        if is_dir {
            expanded.update(|e| *e = !*e);
        }
    };

    let node_path_selected = node_path.clone();
    let is_selected = move || selected.get().as_ref() == Some(&node_path_selected);

    let padding = depth * 20;
    let has_children = !children_list.is_empty();

    let children_view = if expanded.get() && is_dir && has_children {
        view! {
            <ul class="list-none p-0 m-0">
                {children_list
                    .into_iter()
                    .map(|child| {
                        view! {
                            <FileTreeNode
                                node=child
                                selected=selected
                                depth=depth + 1
                            />
                        }
                    })
                    .collect_view()}
            </ul>
        }
        .into_any()
    } else {
        let _: () = view! { <></> };
        ().into_any()
    };

    view! {
        <li class="list-none">
            <div
                on:click=handle_click
                style=format!("padding-left: {}px", padding)
                class=format!(
                    "h-8 px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 transition-colors {}",
                    if is_selected() {
                        "bg-blue-50 border-l-2 border-blue-500"
                    } else {
                        "border-l-2 border-transparent"
                    }
                )
            >
                {if is_dir {
                    view! {
                        <button
                            on:click=handle_toggle
                            class="inline-flex items-center justify-center w-5 h-5 text-gray-600"
                        >
                            <svg
                                class=format!("w-4 h-4 transition-transform {}", if expanded.get() { "rotate-90" } else { "" })
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                            >
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                            </svg>
                        </button>
                    }.into_any()
                } else {
                    view! { <div class="w-5 h-5"></div> }.into_any()
                }}

                <span class="text-lg flex-shrink-0">{icon}</span>
                <span class="text-sm text-gray-900 truncate">{node_name}</span>
            </div>

            {children_view}
        </li>
    }
}
