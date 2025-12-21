use crate::components::primitives::{Button, ButtonVariant};
/// PromptComposer - Text input + submit for prompts
///
/// Multi-line text input for composing messages with send button,
/// keyboard shortcuts, and character counter.
use crate::prelude::*;
use web_sys::KeyboardEvent;

/// PromptComposer component
///
/// # Example
///
/// ```rust
/// let (prompt, set_prompt) = create_signal(String::new());
/// view! {
///     <PromptComposer
///         on_submit=move |text| {
///             log!("Sending: {}", text);
///             set_prompt.set(String::new());
///         }
///         placeholder="Ask me anything..."
///     />
/// }
/// ```
#[component]
pub fn PromptComposer(
    /// Callback when message is submitted
    on_submit: impl Fn(String) + 'static,

    /// Disabled state (prevents submission)
    #[prop(default = false)]
    disabled: bool,

    /// Placeholder text
    #[prop(default = "Type your message...")]
    placeholder: &'static str,

    /// Max characters allowed
    #[prop(default = 4000)]
    max_chars: usize,

    /// Optional CSS class to append
    #[prop(optional)]
    class: Option<String>,
) -> impl IntoView {
    use std::rc::Rc;
    let on_submit = Rc::new(on_submit);

    let (input_value, set_input_value) = signal(String::new());

    let on_submit_keydown = on_submit.clone();
    let handle_keydown = move |ev: KeyboardEvent| {
        if (ev.ctrl_key() || ev.meta_key()) && ev.key() == "Enter" {
            ev.prevent_default();
            let text = input_value.get().trim().to_string();
            if !text.is_empty() && !disabled {
                on_submit_keydown(text);
                set_input_value.set(String::new());
            }
        }
    };

    let char_count = Memo::new(move |_| input_value.get().len());
    let _char_remaining = Memo::new(move |_| max_chars.saturating_sub(char_count.get()));
    let is_over_limit = Memo::new(move |_| char_count.get() > max_chars);

    let final_class = format!(
        "flex flex-col gap-3 p-4 bg-white rounded-lg border border-gray-200{}",
        class
            .as_ref()
            .map(|c| format!(" {}", c))
            .unwrap_or_default()
    );

    view! {
        <div class=final_class>
            <div class="flex flex-col gap-2">
                <textarea
                    class="w-full px-3 py-2 border rounded-md text-sm border-gray-300 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none min-h-24"
                    placeholder=placeholder
                    prop:value=move || input_value.get()
                    on:input=move |ev| {
                        let text = event_target_value(&ev);
                        if text.len() <= max_chars {
                            set_input_value.set(text);
                        }
                    }
                    on:keydown=handle_keydown
                    disabled=disabled
                    rows="4"
                />

                <div class="flex items-center justify-between text-xs text-gray-500">
                    <span>
                        <Show
                            when=move || is_over_limit.get()
                            fallback=move || view! {
                                <span>
                                    {move || format!("{} / {}", char_count.get(), max_chars)}
                                </span>
                            }
                        >
                            <span class="text-red-600 font-medium">
                                "Character limit exceeded"
                            </span>
                        </Show>
                    </span>

                    <span class="text-xs text-gray-400">
                        "Cmd/Ctrl + Enter to send"
                    </span>
                </div>
            </div>

            <div class="flex gap-2 justify-end">
                <Button
                    variant=ButtonVariant::Primary
                    disabled=disabled || input_value.get().trim().is_empty()
                    on_click=Box::new(move |_| {
                        let text = input_value.get().trim().to_string();
                        if !text.is_empty() && !disabled {
                            on_submit(text);
                            set_input_value.set(String::new());
                        }
                    })
                >
                    "Send"
                </Button>
            </div>
        </div>
    }
}
