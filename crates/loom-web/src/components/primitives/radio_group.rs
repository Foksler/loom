/// RadioGroup component - single selection from options
///
/// Allows users to select exactly one option from a set of choices.
/// Supports disabled state and error state.
use crate::prelude::*;

/// Single radio button option
#[derive(Clone, Debug)]
pub struct RadioOption {
    /// The value for this option
    pub value: String,
    /// Display label for this option
    pub label: String,
}

impl RadioOption {
    /// Create a new radio option
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// RadioGroup component for single selection
///
/// # Props
/// - `options`: Vector of available options
/// - `value`: Currently selected value
/// - `disabled`: Disable all options
/// - `error`: Optional error message
/// - `label`: Optional group label
/// - `on_change`: Callback when selection changes
/// - `class`: Optional additional CSS classes
///
/// # Example
///
/// ```rust
/// view! {
///     <RadioGroup
///         options=vec![
///             RadioOption::new("option1", "Option 1"),
///             RadioOption::new("option2", "Option 2"),
///         ]
///         value=selected_signal
///         on_change=move |v| set_selected(v)
///     />
/// }
/// ```
#[component]
pub fn RadioGroup(
    /// List of available options
    options: Vec<RadioOption>,

    /// Currently selected value
    #[prop(default = String::new())]
    value: String,

    /// Optional group label
    #[prop(optional)]
    label: Option<String>,

    /// Disable all options
    #[prop(default = false)]
    disabled: bool,

    /// Optional error message
    #[prop(optional)]
    error: Option<String>,

    /// Callback when selection changes
    #[prop(optional)]
    on_change: Option<impl Fn(String) + 'static>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    use std::rc::Rc;
    let on_change = Rc::new(on_change);
    let has_error = error.is_some();

    view! {
        <fieldset class=format!("flex flex-col gap-2 {}", class.unwrap_or_default())>
            {label.as_ref().map(|l| {
                view! {
                    <legend class="text-sm font-medium text-gray-900">{l.clone()}</legend>
                }
            })}

            <div class="space-y-2">
                {options
                    .into_iter()
                    .map(|option| {
                        let is_checked = value == option.value;

                        view! {
                            <label class=format!(
                                "flex items-center gap-2 {} select-none",
                                if disabled { "cursor-not-allowed opacity-60" } else { "cursor-pointer" }
                            )>
                                <input
                                     type="radio"
                                     name="radio-group"
                                     value=option.value.clone()
                                     checked=is_checked
                                     disabled=disabled
                                     on:change={
                                         let on_change_cb = on_change.clone();
                                         move |ev| {
                                             if let Some(ref callback) = *on_change_cb {
                                                 callback(event_target_value(&ev));
                                             }
                                         }
                                     }
                                     class=format!(
                                         "w-4 h-4 border-2 transition-colors {} {}",
                                         if is_checked {
                                             "bg-blue-600 border-blue-600"
                                         } else {
                                             "bg-white border-gray-300 hover:border-blue-500"
                                         },
                                         if has_error { "border-red-500" } else { "" }
                                     )
                                     aria-checked=is_checked
                                     aria-disabled=disabled
                                 />
                                <span class="text-sm font-medium text-gray-700">{option.label}</span>
                            </label>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>

            {error.map(|err| {
                view! {
                    <span class="text-xs text-red-600">{err}</span>
                }
            })}
        </fieldset>
    }
}
