/// FileInput component - file upload input
///
/// Allows users to select one or multiple files for upload.
/// Supports accept filters, disabled state, and error state.
use crate::prelude::*;
use leptos::html;

/// FileInput component for file uploads
///
/// # Props
/// - `label`: Optional label displayed above input
/// - `accept`: Optional file type filter (e.g., "image/*", ".pdf")
/// - `multiple`: Allow selecting multiple files
/// - `disabled`: Disabled state
/// - `error`: Optional error message
/// - `on_change`: Callback with selected files
/// - `class`: Optional additional CSS classes
///
/// # Example
///
/// ```rust
/// view! {
///     <FileInput
///         label="Upload avatar"
///         accept="image/*"
///         multiple=false
///         on_change=move |files| handle_upload(files)
///     />
/// }
/// ```
#[component]
pub fn FileInput(
    /// Optional label text
    #[prop(optional)]
    label: Option<String>,

    /// Optional file type filter (e.g., "image/*", ".pdf,.doc")
    #[prop(optional)]
    accept: Option<String>,

    /// Allow selecting multiple files
    #[prop(default = false)]
    multiple: bool,

    /// Disabled state
    #[prop(default = false)]
    disabled: bool,

    /// Optional error message
    #[prop(optional)]
    error: Option<String>,

    /// Callback when files are selected
    #[prop(optional)]
    on_change: Option<impl Fn(Vec<web_sys::File>) + 'static>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let disabled_class = if disabled {
        "opacity-60 cursor-not-allowed"
    } else {
        "cursor-pointer"
    };

    let has_error = error.is_some();

    let input_ref = NodeRef::<html::Input>::new();

    view! {
        <div class=format!("flex flex-col gap-2 {}", class.unwrap_or_default())>
            {label.as_ref().map(|l| {
                view! {
                    <label class="text-sm font-medium text-gray-900">{l.clone()}</label>
                }
            })}

            <div
                class=format!(
                    "relative inline-block w-full max-w-xs {}",
                    if disabled { "opacity-60 cursor-not-allowed" } else { "" }
                )
            >
                {/* Hidden actual input */}
                <input
                    node_ref=input_ref
                    type="file"
                    multiple=multiple
                    disabled=disabled
                    accept=accept.clone().unwrap_or_default()
                    on:change=move |_ev| {
                        if let Some(ref callback) = on_change {
                            if let Some(input) = input_ref.get() {
                                if let Some(files) = input.files() {
                                    let mut file_vec = Vec::new();
                                    for i in 0..files.length() {
                                        if let Some(file) = files.get(i) {
                                            file_vec.push(file);
                                        }
                                    }
                                    callback(file_vec);
                                }
                            }
                        }
                    }
                    class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                    aria-disabled=disabled
                />

                {/* Styled button label */}
                <label
                    class=format!(
                        "flex items-center justify-center gap-2 px-4 py-2.5 border-2 border-dashed border-gray-300 rounded-lg bg-white transition-colors {} {}",
                        disabled_class,
                        if has_error {
                            "border-red-500"
                        } else {
                            "hover:border-blue-400 hover:bg-blue-50"
                        }
                    )
                >
                    <svg
                        class="w-4 h-4 text-gray-600"
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M12 4v16m8-8H4"
                        />
                    </svg>
                    <span class="text-sm font-medium text-gray-700">
                        {if multiple { "Choose files" } else { "Choose file" }}
                    </span>
                </label>
            </div>

            {error.map(|err| {
                view! {
                    <span class="text-xs text-red-600">{err}</span>
                }
            })}
        </div>
    }
}
