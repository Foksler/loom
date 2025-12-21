/// Select component - dropdown selector
///
/// Single-selection dropdown with support for labels, placeholders, and error states.
/// Built entirely with Tailwind CSS.
use crate::prelude::*;

/// Select option
#[derive(Clone, Debug)]
pub struct SelectOption {
    /// Option label displayed to user
    pub label: String,
    /// Option value
    pub value: String,
}

impl SelectOption {
    /// Create a new select option
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// Select component
///
/// # Example
///
/// ```rust
/// let options = vec![
///     SelectOption::new("Option 1", "opt1"),
///     SelectOption::new("Option 2", "opt2"),
/// ];
///
/// view! {
///     <Select label="Choose one" options=options.clone() />
///     <Select
///         label="Category"
///         options=options.clone()
///         value="opt1"
///         error="Required"
///     />
/// }
/// ```
#[component]
pub fn Select(
    /// Label text displayed above the select
    #[prop(optional)]
    label: Option<String>,

    /// Placeholder text
    #[prop(default = "Select an option")]
    placeholder: &'static str,

    /// Available options
    #[prop(default = vec![])]
    options: Vec<SelectOption>,

    /// Currently selected value
    #[prop(default = "")]
    value: &'static str,

    /// Error message displayed below select
    #[prop(optional)]
    error: Option<String>,

    /// Disabled state
    #[prop(default = false)]
    disabled: bool,

    /// Change handler
    #[prop(optional)]
    on_change: Option<impl Fn(String) + 'static>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let has_error = error.is_some();

    let select_class = format!(
        "w-full px-3 py-2 border rounded-md text-sm transition-colors {}{}",
        if has_error {
            "border-red-500 bg-red-50 text-red-900 focus:outline-none focus:ring-2 focus:ring-red-500"
        } else {
            "border-gray-300 bg-white text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
        },
        if disabled {
            " bg-gray-100 text-gray-500 cursor-not-allowed opacity-60"
        } else {
            ""
        }
    );

    let final_class = match class {
        Some(c) => format!("{} {}", select_class, c),
        None => select_class,
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
            <select
                class=final_class
                disabled=disabled
                on:change=move |ev| {
                    if let Some(handler) = &on_change {
                        handler(event_target_value(&ev));
                    }
                }
            >
                <option value="" disabled={value.is_empty()}>
                    {placeholder}
                </option>
                {options.iter().map(|opt| {
                    view! {
                        <option value=opt.value.clone()>
                            {opt.label.clone()}
                        </option>
                    }
                }).collect::<Vec<_>>()}
            </select>
            {error.map(|e| {
                view! {
                    <p class="text-sm text-red-600">{e}</p>
                }
            })}
        </div>
    }
}
