/// Checkbox component - multiple selection input
///
/// Allows users to select or deselect one or more options.
/// Supports checked state, disabled state, and error state.
use crate::prelude::*;

/// Checkbox component for multiple selections
///
/// # Props
/// - `label`: Optional label displayed next to checkbox
/// - `checked`: Whether checkbox is checked (controlled)
/// - `disabled`: Disabled state
/// - `error`: Optional error message
/// - `on_change`: Callback when checkbox state changes
/// - `class`: Optional additional CSS classes
///
/// # Example
///
/// ```rust
/// view! {
///     <Checkbox
///         label="I agree"
///         checked=checked_signal
///         on_change=move |v| set_checked(v)
///     />
/// }
/// ```
#[component]
pub fn Checkbox(
    /// Optional label text
    #[prop(optional)]
    label: Option<String>,

    /// Checked state (controlled)
    #[prop(default = false)]
    checked: bool,

    /// Disabled state
    #[prop(default = false)]
    disabled: bool,

    /// Optional error message to display below
    #[prop(optional)]
    error: Option<String>,

    /// Callback when checkbox state changes
    #[prop(optional)]
    on_change: Option<impl Fn(bool) + 'static>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let disabled_class = if disabled {
        "opacity-60 cursor-not-allowed"
    } else {
        "cursor-pointer"
    };

    let base_checkbox_class = "w-4 h-4 rounded border-2 transition-colors";
    let checkbox_class = if disabled {
        format!("{} bg-gray-100 border-gray-300", base_checkbox_class)
    } else if checked {
        format!("{} bg-blue-600 border-blue-600", base_checkbox_class)
    } else {
        format!(
            "{} bg-white border-gray-300 hover:border-blue-500",
            base_checkbox_class
        )
    };

    let error_class = if error.is_some() {
        "border-red-500"
    } else {
        ""
    };

    let _has_error = error.is_some();

    view! {
        <div class=format!("flex flex-col gap-1 {}", class.unwrap_or_default())>
            <label class=format!("flex items-center gap-2 {} select-none", disabled_class)>
                <input
                    type="checkbox"
                    checked=checked
                    disabled=disabled
                    on:change=move |ev| {
                        if let Some(ref callback) = on_change {
                            callback(event_target_checked(&ev));
                        }
                    }
                    class=format!("{} {}", checkbox_class, error_class)
                    aria-checked=checked
                    aria-disabled=disabled
                />
                {label.as_ref().map(|l| {
                    view! {
                        <span class="text-sm font-medium text-gray-700">{l.clone()}</span>
                    }
                })}
            </label>
            {error.map(|err| {
                view! {
                    <span class="text-xs text-red-600">{err}</span>
                }
            })}
        </div>
    }
}
