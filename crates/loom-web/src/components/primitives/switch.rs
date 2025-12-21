/// Switch component - toggle boolean state
///
/// A visual toggle switch for enabling/disabling a feature.
/// Supports disabled state and error state.
use crate::prelude::*;

/// Switch component for toggling boolean state
///
/// # Props
/// - `label`: Optional label displayed next to switch
/// - `checked`: Whether switch is on (controlled)
/// - `disabled`: Disabled state
/// - `error`: Optional error message
/// - `on_change`: Callback when toggle state changes
/// - `class`: Optional additional CSS classes
///
/// # Example
///
/// ```rust
/// view! {
///     <Switch
///         label="Enable notifications"
///         checked=enabled_signal
///         on_change=move |v| set_enabled(v)
///     />
/// }
/// ```
#[component]
pub fn Switch(
    /// Optional label text
    #[prop(optional)]
    label: Option<String>,

    /// Toggle state (controlled)
    #[prop(default = false)]
    checked: bool,

    /// Disabled state
    #[prop(default = false)]
    disabled: bool,

    /// Optional error message to display below
    #[prop(optional)]
    error: Option<String>,

    /// Callback when toggle state changes
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

    let bg_class = if checked {
        "bg-blue-600"
    } else {
        "bg-gray-300"
    };

    let toggle_position = if checked {
        "translate-x-6"
    } else {
        "translate-x-1"
    };

    view! {
        <div class=format!("flex flex-col gap-1 {}", class.unwrap_or_default())>
            <label class=format!("flex items-center gap-3 {} select-none", disabled_class)>
                <button
                    type="button"
                    role="switch"
                    aria-checked=checked
                    aria-disabled=disabled
                    disabled=disabled
                    on:click=move |_| {
                        if !disabled {
                            if let Some(ref callback) = on_change {
                                callback(!checked);
                            }
                        }
                    }
                    class=format!(
                        "relative inline-flex h-6 w-11 items-center rounded-full transition-colors {} {}",
                        bg_class,
                        if disabled {
                            "cursor-not-allowed"
                        } else {
                            "hover:opacity-90"
                        }
                    )
                >
                    <span
                        class=format!(
                            "inline-block h-4 w-4 transform rounded-full bg-white transition-transform {}",
                            toggle_position
                        )
                    />
                </button>

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
