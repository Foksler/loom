/// ThreadMetadataPanel - Sidebar showing thread information
///
/// Displays thread metadata including timestamps, model info,
/// enabled tools, and repository information.
use crate::components::primitives::{Badge, BadgeVariant, SectionHeader, SectionHeaderSize};
use crate::prelude::*;

use super::Thread;

/// Metadata panel component for thread sidebar
///
/// # Props
/// * `thread` - Thread data to display
/// * `expanded` - Whether panel is expanded (default: true)
///
/// # Features
/// * Created/updated timestamps
/// * Model information
/// * Repository path
/// * Tools/capabilities list
/// * Collapsible sections
#[component]
pub fn ThreadMetadataPanel(
    /// Thread to display metadata for
    thread: Thread,

    /// Panel expanded state
    #[prop(default = true)]
    expanded: bool,

    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    view! {
        <aside class={format!(
            "w-64 bg-gray-50 border-l border-gray-200 overflow-y-auto {}",
            class.unwrap_or_default()
        )}>
            {/* Header */}
            <div class="sticky top-0 bg-gray-50 border-b border-gray-200 p-4">
                <h2 class="text-sm font-semibold text-gray-900">Details</h2>
            </div>

            <Show when=move || expanded>
                {
                    let repo = thread.repository.clone();
                    let tools = thread.tools.clone();
                    let created = thread.created_at;
                    let updated = thread.updated_at;
                    let model = thread.model.clone();
                    let repo_check = repo.clone();
                    let tools_check = tools.clone();
                    view! {
                        <div class="p-4 space-y-6">
                            {/* Timestamps Section */}
                            <section>
                                <SectionHeader size=SectionHeaderSize::Sm>
                                    "Timeline"
                                </SectionHeader>
                                    <div class="mt-3 space-y-3 text-sm">
                                        <div>
                                            <p class="text-gray-500">Created</p>
                                            <p class="text-gray-900 font-medium">
                                                {format_full_date(&created)}
                                            </p>
                                        </div>
                                        <div>
                                            <p class="text-gray-500">Last Updated</p>
                                            <p class="text-gray-900 font-medium">
                                                {format_full_date(&updated)}
                                            </p>
                                        </div>
                                    </div>
                                </section>

                                {/* Model Section */}
                                <section>
                                    <SectionHeader size=SectionHeaderSize::Sm>
                                        "Model"
                                    </SectionHeader>
                                    <div class="mt-3">
                                        <div
                                            class="bg-white border border-gray-200 rounded px-3 py-2 \
                                                   text-sm text-gray-900 break-all"
                                        >
                                            <code class="font-mono">{model}</code>
                                        </div>
                                    </div>
                                </section>

                                <Show when=move || repo_check.is_some()>
                                    {
                                        let repo_val = repo.as_ref().map(|r| r.to_string()).unwrap_or_default();
                                        let repo_for_title = repo_val.clone();
                                        view! {
                                            <section>
                                                <SectionHeader size=SectionHeaderSize::Sm>
                                                    "Repository"
                                                </SectionHeader>
                                                <div class="mt-3">
                                                    <div
                                                        class="bg-white border border-gray-200 rounded px-3 py-2 \
                                                               text-sm text-gray-700 break-all"
                                                        title=repo_for_title
                                                    >
                                                        <code class="font-mono text-xs">{repo_val}</code>
                                                    </div>
                                                </div>
                                            </section>
                                        }
                                    }
                                </Show>

                                <Show when=move || !tools_check.is_empty()>
                                    {
                                        let tools_for_view = tools.clone();
                                        view! {
                                            <section>
                                                <SectionHeader size=SectionHeaderSize::Sm>
                                                    "Tools"
                                                </SectionHeader>
                                                <div class="mt-3 flex flex-wrap gap-2">
                                                    {tools_for_view
                                                        .into_iter()
                                                        .map(|tool| {
                                                            view! {
                                                                <Badge variant=BadgeVariant::Blue>
                                                                    {tool}
                                                                </Badge>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            </section>
                                        }
                                    }
                                </Show>

                            {/* Message Count */}
                            <section class="pt-2 border-t border-gray-200">
                                <div class="flex items-center justify-between">
                                    <span class="text-sm text-gray-500">Messages</span>
                                    <span class="text-sm font-semibold text-gray-900">
                                        {thread.messages.len()}
                                    </span>
                                </div>
                            </section>
                        </div>
                    }
                }
            </Show>
        </aside>
    }
}

/// Format date with time for full display
fn format_full_date(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%B %d, %Y at %H:%M").to_string()
}
