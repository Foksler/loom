/// ResizablePanels component - Draggable panels with resizable divider
///
/// A composite component that provides two side-by-side resizable panels
/// with mouse drag support, keyboard navigation, and persistent width tracking.
use crate::prelude::*;
use leptos::html;
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;

/// ResizablePanels component - Two resizable side-by-side panels
///
/// Renders two panels with a draggable divider between them.
/// Widths are adjustable via mouse drag or keyboard shortcuts.
///
/// # Props
/// - `left_panel`: Content for the left panel
/// - `right_panel`: Content for the right panel
/// - `initial_width`: Initial left panel width as percentage (default: 50)
/// - `min_width`: Minimum width for each panel (default: 20%)
/// - `resizable`: Enable/disable resize functionality
///
/// # Example
///
/// ```rust
/// view! {
///     <ResizablePanels
///         initial_width=40
///         left_panel=move || {
///             view! { <div>"Left content"</div> }
///         }
///         right_panel=move || {
///             view! { <div>"Right content"</div> }
///         }
///     />
/// }
/// ```
///
/// # Interaction
/// - Click and drag the divider to resize panels
/// - Use arrow keys (when divider focused) to fine-tune width
/// - Left/Up arrows decrease left panel width
/// - Right/Down arrows increase left panel width
#[component]
pub fn ResizablePanels<LF, RF, LFV, RFV>(
    /// Content for left panel
    left_panel: LF,
    /// Content for right panel
    right_panel: RF,
    /// Initial left panel width as percentage (0-100)
    #[prop(default = 50)]
    initial_width: u32,
    /// Minimum width for each panel (percentage)
    #[prop(default = 20)]
    min_width: u32,
    /// Enable resize functionality
    #[prop(default = true)]
    resizable: bool,
    /// Optional CSS class
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView
where
    LF: Fn() -> LFV + 'static,
    RF: Fn() -> RFV + 'static,
    LFV: IntoView + 'static,
    RFV: IntoView + 'static,
{
    let (left_width, set_left_width) = signal(initial_width as f32);
    let (is_dragging, set_is_dragging) = signal(false);

    let container_ref = NodeRef::<html::Div>::new();
    let divider_ref = NodeRef::<html::Div>::new();

    // Handle mouse down on divider
    let on_divider_mouse_down = {
        move |_: MouseEvent| {
            if resizable {
                set_is_dragging.set(true);
            }
        }
    };

    // Handle mouse move during drag
    let on_document_mouse_move = {
        let container_ref = container_ref.clone();
        move |event: MouseEvent| {
            if !is_dragging.get() {
                return;
            }

            if let Some(container) = container_ref.get() {
                // Get bounding rect
                let js_element: &web_sys::Element = container.unchecked_ref();
                let rect: web_sys::DomRect = js_element.get_bounding_client_rect();
                let container_width = rect.width() as f32;

                if container_width > 0.0 {
                    let mouse_x = event.client_x() as f32;
                    let container_left = rect.left() as f32;

                    let relative_x = mouse_x - container_left;
                    let new_width = (relative_x / container_width) * 100.0;

                    // Clamp to min/max widths
                    let clamped = new_width
                        .max(min_width as f32)
                        .min(100.0 - min_width as f32);

                    set_left_width.set(clamped);
                }
            }
        }
    };

    // Handle mouse up to stop dragging
    let on_document_mouse_up = {
        move |_: MouseEvent| {
            set_is_dragging.set(false);
        }
    };

    // Handle keyboard navigation on divider
    let on_divider_key_down = {
        move |event: web_sys::KeyboardEvent| {
            if !resizable {
                return;
            }

            let delta = 2.0; // pixels per key press
            match event.key().as_str() {
                "ArrowLeft" | "ArrowUp" => {
                    event.prevent_default();
                    let new_width = (left_width.get() - delta).max(min_width as f32);
                    set_left_width.set(new_width);
                }
                "ArrowRight" | "ArrowDown" => {
                    event.prevent_default();
                    let new_width = (left_width.get() + delta).min(100.0 - min_width as f32);
                    set_left_width.set(new_width);
                }
                _ => {}
            }
        }
    };

    let right_width = Memo::new(move |_| 100.0 - left_width.get());
    let cursor_class = if resizable { "cursor-col-resize" } else { "" };
    let divider_class = if is_dragging.get() {
        "bg-blue-500"
    } else {
        "bg-gray-300 hover:bg-blue-400"
    };

    view! {
        <div
            node_ref=container_ref
            class=format!(
                "flex h-full overflow-hidden {}",
                class.unwrap_or_default()
            )
            on:mousemove=on_document_mouse_move
            on:mouseup=on_document_mouse_up
            on:mouseleave=on_document_mouse_up
        >
            {/* Left Panel */}
            <div
                class="overflow-auto border-r border-gray-200 bg-white"
                style=format!("width: {}%;", left_width.get())
            >
                {left_panel()}
            </div>

            {/* Resizable Divider */}
            {resizable.then(|| {
                view! {
                    <div
                        node_ref=divider_ref
                        class=format!(
                            "w-1 transition-colors {} {}",
                            divider_class,
                            cursor_class
                        )
                        on:mousedown=on_divider_mouse_down
                        on:keydown=on_divider_key_down
                        tabindex="0"
                        role="separator"
                        aria-orientation="vertical"
                        aria-label="Drag to resize panels"
                    />
                }
            })}

            {/* Right Panel */}
            <div
                class="flex-1 overflow-auto bg-white"
                style=if resizable {
                    format!("width: {}%;", right_width.get())
                } else {
                    "width: 50%;".to_string()
                }
            >
                {right_panel()}
            </div>
        </div>
    }
}
