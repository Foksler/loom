/// MultiSelect component - multi-selection dropdown
///
/// Multi-selection dropdown with support for labels and error states.
/// Built entirely with Tailwind CSS.
use crate::prelude::*;

/// Multi-select option
#[derive(Clone, Debug)]
pub struct MultiSelectOption {
    /// Option label displayed to user
    pub label: String,
    /// Option value
    pub value: String,
}

impl MultiSelectOption {
    /// Create a new multi-select option
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// MultiSelect component
///
/// # Example
///
/// ```rust
/// let options = vec![
///     MultiSelectOption::new("Option 1", "opt1"),
///     MultiSelectOption::new("Option 2", "opt2"),
///     MultiSelectOption::new("Option 3", "opt3"),
/// ];
///
/// view! {
///     <MultiSelect label="Choose options" options=options.clone() />
///     <MultiSelect
///         label="Tags"
///         options=options.clone()
///         value=vec!["opt1".to_string(), "opt2".to_string()]
///     />
/// }
/// ```
#[component]
pub fn MultiSelect(
    /// Label text displayed above the select
    #[prop(optional)]
    label: Option<String>,

    /// Available options
    #[prop(default = vec![])]
    options: Vec<MultiSelectOption>,

    /// Currently selected values
    #[prop(default = vec![])]
    value: Vec<String>,

    /// Error message displayed below select
    #[prop(optional)]
    error: Option<String>,

    /// Disabled state
    #[prop(default = false)]
    disabled: bool,

    /// Change handler
    #[prop(optional)]
    on_change: Option<impl Fn(Vec<String>) + 'static>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let has_error = error.is_some();
    let value_signal = RwSignal::new(value);

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
                multiple
                disabled=disabled
                on:change=move |ev| {
                    let selected: Vec<String> = event_target_value(&ev)
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
                    value_signal.set(selected.clone());
                    if let Some(handler) = &on_change {
                        handler(selected);
                    }
                }
            >
                {options.iter().map(|opt| {
                    let opt_value = opt.value.clone();
                    let is_selected = value_signal.get().contains(&opt_value);
                    view! {
                        <option value=opt.value.clone() selected=is_selected>
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
