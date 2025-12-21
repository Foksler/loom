/// Toggle component - on/off switch
///
/// Binary toggle switch with label and disabled state.
/// Built entirely with Tailwind CSS.
use crate::prelude::*;

/// Toggle component
///
/// # Example
///
/// ```rust
/// view! {
///     <Toggle label="Enable notifications" />
///     <Toggle label="Dark mode" checked=true />
///     <Toggle disabled=true label="Locked setting" />
/// }
/// ```
#[component]
pub fn Toggle(
    /// Label text displayed next to the toggle
    #[prop(optional)]
    label: Option<String>,

    /// Checked state
    #[prop(default = false)]
    checked: bool,

    /// Disabled state
    #[prop(default = false)]
    disabled: bool,

    /// Change handler
    #[prop(optional)]
    on_change: Option<impl Fn(bool) + 'static>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let checked_signal = RwSignal::new(checked);

    let container_class = format!(
        "flex items-center gap-3 {}",
        match class {
            Some(c) => c,
            None => String::new(),
        }
    );

    view! {
        <div class=container_class>
            <button
                class=move || {
                    format!(
                        "relative inline-flex h-6 w-11 items-center rounded-full transition-colors {}{}",
                        if checked_signal.get() {
                            "bg-blue-600"
                        } else {
                            "bg-gray-300"
                        },
                        if disabled {
                            " opacity-60 cursor-not-allowed"
                        } else {
                            " cursor-pointer"
                        }
                    )
                }
                disabled=disabled
                on:click=move |_| {
                    let new_state = !checked_signal.get();
                    checked_signal.set(new_state);
                    if let Some(handler) = &on_change {
                        handler(new_state);
                    }
                }
            >
                <span
                    class=move || {
                        format!(
                            "inline-block h-5 w-5 transform rounded-full bg-white transition-transform {}",
                            if checked_signal.get() {
                                "translate-x-5"
                            } else {
                                "translate-x-0"
                            }
                        )
                    }
                />
            </button>
            {label.map(|l| {
                view! {
                    <label class=move || {
                        format!(
                            "text-sm font-medium {}",
                            if disabled {
                                "text-gray-400"
                            } else {
                                "text-gray-700 cursor-pointer"
                            }
                        )
                    }>
                        {l}
                    </label>
                }
            })}
        </div>
    }
}
