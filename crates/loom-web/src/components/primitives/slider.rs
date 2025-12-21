/// Slider component - range input
///
/// Allows users to select a value from a continuous range using a draggable thumb.
/// Supports min/max values, step size, disabled state, and error state.
use crate::prelude::*;

/// Slider component for range selection
///
/// # Props
/// - `label`: Optional label displayed above slider
/// - `value`: Current slider value
/// - `min`: Minimum value
/// - `max`: Maximum value
/// - `step`: Step size for increments
/// - `disabled`: Disabled state
/// - `error`: Optional error message
/// - `show_value`: Whether to display current value
/// - `on_change`: Callback when slider value changes
/// - `class`: Optional additional CSS classes
///
/// # Example
///
/// ```rust
/// view! {
///     <Slider
///         label="Volume"
///         value=volume_signal
///         min=0
///         max=100
///         step=1
///         on_change=move |v| set_volume(v)
///         show_value=true
///     />
/// }
/// ```
#[component]
pub fn Slider(
    /// Optional label text
    #[prop(optional)]
    label: Option<String>,

    /// Current value (controlled)
    #[prop(default = 0.0)]
    value: f64,

    /// Minimum value
    #[prop(default = 0.0)]
    min: f64,

    /// Maximum value
    #[prop(default = 100.0)]
    max: f64,

    /// Step size for increments
    #[prop(default = 1.0)]
    step: f64,

    /// Disabled state
    #[prop(default = false)]
    disabled: bool,

    /// Optional error message
    #[prop(optional)]
    error: Option<String>,

    /// Show the current value next to slider
    #[prop(default = false)]
    show_value: bool,

    /// Callback when slider value changes
    #[prop(optional)]
    on_change: Option<impl Fn(f64) + 'static>,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    let disabled_class = if disabled {
        "opacity-60 cursor-not-allowed"
    } else {
        "cursor-pointer"
    };

    // Calculate the percentage for styling the filled portion
    let percentage = ((value - min) / (max - min)) * 100.0;

    view! {
        <div class=format!("flex flex-col gap-2 {}", class.unwrap_or_default())>
            {label.as_ref().map(|l| {
                view! {
                    <div class="flex items-center justify-between">
                        <label class="text-sm font-medium text-gray-900">{l.clone()}</label>
                        {show_value.then(|| {
                            view! {
                                <span class="text-sm font-medium text-gray-600">{value.round()}</span>
                            }
                        })}
                    </div>
                }
            })}

            <div class=format!(
                "relative w-full h-6 flex items-center {}",
                disabled_class
            )>
                {/* Visual track background */}
                <div class="absolute w-full h-2 bg-gray-200 rounded-full" />

                {/* Filled portion of track */}
                <div
                    class="absolute h-2 bg-blue-600 rounded-full"
                    style=format!("width: {}%; pointer-events: none", percentage)
                />

                {/* Actual input slider */}
                <input
                    type="range"
                    value=value
                    min=min
                    max=max
                    step=step
                    disabled=disabled
                    on:change=move |ev| {
                        if let Some(ref callback) = on_change {
                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                callback(v);
                            }
                        }
                    }
                    class="absolute w-full h-2 bg-transparent rounded-full appearance-none cursor-pointer pointer-events-auto accent-blue-600 [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-blue-600 [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:box-shadow-sm [&::-moz-range-thumb]:w-4 [&::-moz-range-thumb]:h-4 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:bg-white [&::-moz-range-thumb]:border-2 [&::-moz-range-thumb]:border-blue-600 [&::-moz-range-thumb]:cursor-pointer disabled:opacity-60 disabled:cursor-not-allowed"
                    aria-valuemin=min
                    aria-valuemax=max
                    aria-valuenow=value
                    aria-disabled=disabled
                />
            </div>

            <div class="flex items-center justify-between">
                <span class="text-xs text-gray-500">{min}</span>
                <span class="text-xs text-gray-500">{max}</span>
            </div>

            {error.map(|err| {
                view! {
                    <span class="text-xs text-red-600">{err}</span>
                }
            })}
        </div>
    }
}
