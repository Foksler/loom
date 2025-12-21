/// Layout & shell components gallery
use crate::components::{
    layout::{Column, DataTable, FieldRow, FormSection, KeyValueList, ResizablePanels},
    threads::{
        mock_data::{mock_thread, mock_threads},
        ThreadHeader, ThreadList, ThreadMetadataPanel,
    },
};
use crate::prelude::*;

#[component]
pub fn StyleguideLayoutPage() -> impl IntoView {
    let threads = mock_threads();

    view! {
        <div class="p-8 max-w-5xl">
            <h1 class="text-4xl font-bold mb-4 text-gray-900">Layout & Shell</h1>
            <p class="text-lg text-gray-600 mb-8">
                Components for app shell, resizable panels, and navigation structures.
            </p>

            {/* ThreadList Component */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-3 text-gray-900">Thread List</h2>
                <p class="text-gray-600 mb-4">
                    Responsive grid of thread summaries with search and filtering.
                </p>
                <div class="bg-white border border-gray-200 rounded-lg p-6">
                    <ThreadList threads=threads.clone() loading=false />
                </div>
            </section>

            {/* ThreadHeader Component */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-3 text-gray-900">Thread Header</h2>
                <p class="text-gray-600 mb-4">
                    Header for thread detail page with title editing and action menu.
                </p>
                <div class="border border-gray-200 rounded-lg overflow-hidden">
                    <ThreadHeader thread=mock_thread() />
                </div>
            </section>

            {/* ThreadMetadataPanel Component */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-3 text-gray-900">Thread Metadata Panel</h2>
                <p class="text-gray-600 mb-4">
                    Sidebar showing thread information, timestamps, model, tools, and repository.
                </p>
                <div class="flex border border-gray-200 rounded-lg overflow-hidden bg-white">
                    <div class="flex-1 p-6">
                        <p class="text-gray-600">Thread detail view goes here</p>
                    </div>
                    <ThreadMetadataPanel thread=mock_thread() expanded=true />
                </div>
            </section>

            {/* ThreadList Loading State */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-3 text-gray-900">Thread List (Loading)</h2>
                <p class="text-gray-600 mb-4">
                    Thread list with loading skeleton state.
                </p>
                <div class="bg-white border border-gray-200 rounded-lg p-6">
                    <ThreadList threads=threads.clone() loading=true />
                </div>
            </section>

            {/* DataTable Component */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-3 text-gray-900">DataTable</h2>
                <p class="text-gray-600 mb-4">
                    Tabular data display with sortable columns and pagination.
                </p>
                <div class="bg-white border border-gray-200 rounded-lg p-6">
                    {move || {
                        let columns = vec![
                            Column {
                                id: "name".into(),
                                label: "Name".into(),
                                width: None,
                                sortable: true,
                            },
                            Column {
                                id: "email".into(),
                                label: "Email".into(),
                                width: None,
                                sortable: true,
                            },
                            Column {
                                id: "role".into(),
                                label: "Role".into(),
                                width: None,
                                sortable: false,
                            },
                        ];
                        let rows = vec![
                            vec!["Alice Johnson".into(), "alice@example.com".into(), "Admin".into()],
                            vec!["Bob Smith".into(), "bob@example.com".into(), "User".into()],
                            vec!["Carol White".into(), "carol@example.com".into(), "Moderator".into()],
                            vec!["David Brown".into(), "david@example.com".into(), "User".into()],
                        ];

                        view! {
                            <DataTable
                                columns=columns
                                rows=rows
                                sortable=true
                                pagination=true
                                page_size=2
                            />
                        }
                    }}
                </div>
            </section>

            {/* KeyValueList Component */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-3 text-gray-900">KeyValueList</h2>
                <p class="text-gray-600 mb-4">
                    Display structured key-value data in readable format.
                </p>
                <div class="bg-white border border-gray-200 rounded-lg p-6">
                    {move || {
                        let items = vec![
                            ("Model".into(), "GPT-4 Turbo".into()),
                            ("Temperature".into(), "0.7".into()),
                            ("Max Tokens".into(), "2048".into()),
                            ("API Key".into(), "sk-...xxxxx".into()),
                        ];

                        view! {
                            <KeyValueList
                                items=items
                                label_width="150px".into()
                                striped=true
                            />
                        }
                    }}
                </div>
            </section>

            {/* FormSection & FieldRow Components */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-3 text-gray-900">FormSection & FieldRow</h2>
                <p class="text-gray-600 mb-4">
                    Group form fields with labels, descriptions, and error handling.
                </p>
                <div class="bg-white border border-gray-200 rounded-lg p-6 max-w-md">
                    <FormSection
                        title="Account Settings".into()
                        description="Update your account information".into()
                    >
                        <FieldRow label="Username".into() required=true>
                            <input
                                type="text"
                                placeholder="Enter username"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md"
                            />
                        </FieldRow>
                        <FieldRow
                            label="Email".into()
                            required=true
                        >
                            <input
                                type="email"
                                placeholder="your@email.com"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md"
                            />
                        </FieldRow>
                        <FieldRow
                            label="Password".into()
                            required=true
                            error="Password too short (min 8 characters)".into()
                        >
                            <input
                                type="password"
                                placeholder="Enter password"
                                class="w-full px-3 py-2 border border-red-500 rounded-md"
                            />
                        </FieldRow>
                    </FormSection>
                </div>
            </section>

            {/* ResizablePanels Component */}
            <section class="mb-12">
                <h2 class="text-2xl font-bold mb-3 text-gray-900">ResizablePanels</h2>
                <p class="text-gray-600 mb-4">
                    Two draggable resizable side-by-side panels.
                </p>
                <div class="bg-white border border-gray-200 rounded-lg overflow-hidden" style="height: 300px;">
                    <ResizablePanels
                        initial_width=50
                        left_panel=move || {
                            view! {
                                <div class="p-6">
                                    <h3 class="font-semibold text-gray-900 mb-2">"Left Panel"</h3>
                                    <p class="text-gray-600 text-sm">
                                        "Drag the divider to resize. Left/Right arrows to fine-tune."
                                    </p>
                                    <p class="text-gray-600 text-sm mt-4">
                                        "This panel contains content that can be scrolled independently."
                                    </p>
                                </div>
                            }
                        }
                        right_panel=move || {
                            view! {
                                <div class="p-6">
                                    <h3 class="font-semibold text-gray-900 mb-2">"Right Panel"</h3>
                                    <p class="text-gray-600 text-sm">
                                        "Panels are responsive and maintain minimum width constraints."
                                    </p>
                                </div>
                            }
                        }
                    />
                </div>
            </section>
        </div>
    }
}
