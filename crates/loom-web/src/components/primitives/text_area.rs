/// TextArea component - multi-line text input
///
/// Supports labels, placeholders, error states, and disabled state.
/// Built entirely with Tailwind CSS.
use crate::prelude::*;

/// TextArea component
///
/// # Example
///
/// ```rust
/// view! {
///     <TextArea label="Message" placeholder="Enter your message" />
///     <TextArea
///         label="Description"
///         error="Too long"
///         value="some text"
///         rows=5
///     />
///     <TextArea disabled=true label="Disabled field" />
/// }
/// ```
#[component]
pub fn TextArea(
    /// Label text displayed above the input
    #[prop(optional)]
    label: Option<String>,

    /// Placeholder text
    #[prop(default = "")]
    placeholder: &'static str,

    /// Error message displayed below input
    #[prop(optional)]
    error: Option<String>,

    /// Current input value
    #[prop(default = "")]
    #[allow(unused_variables)]
    value: &'static str,

    /// Number of rows
    #[prop(default = 4)]
    rows: u32,

    /// Disabled state
    #[prop(default = false)]
    disabled: bool,

    /// Change handler
    #[prop(optional)]
    on_input: Option<impl Fn(String) + 'static>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let has_error = error.is_some();

    let textarea_class = format!(
        "w-full px-3 py-2 border rounded-md text-sm transition-colors resize-vertical {}{}",
        if has_error {
            "border-red-500 bg-red-50 text-red-900 placeholder-red-400 focus:outline-none focus:ring-2 focus:ring-red-500"
        } else {
            "border-gray-300 bg-white text-gray-900 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
        },
        if disabled {
            " bg-gray-100 text-gray-500 cursor-not-allowed opacity-60"
        } else {
            ""
        }
    );

    let final_class = match class {
        Some(c) => format!("{} {}", textarea_class, c),
        None => textarea_class,
    };

    view! {
        <div class="flex flex-col gap-2">
            {label.map(|l| {
                view! {
                    <label class="block text-sm font-medium text-gray-700">
                        {l}
                    </label>
                }
            })}
            <textarea
                class=final_class
                placeholder=placeholder
                rows=rows.to_string()
                disabled=disabled
                on:input=move |ev| {
                    if let Some(handler) = &on_input {
                        handler(event_target_value(&ev));
                    }
                }
            />
            {error.map(|e| {
                view! {
                    <p class="text-sm text-red-600">{e}</p>
                }
            })}
        </div>
    }
}
