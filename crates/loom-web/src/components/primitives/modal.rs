/// Modal component - simplified stub
use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModalSize {
    Sm,
    Md,
    Lg,
    Xl,
}

#[component]
pub fn Modal(
    #[prop(optional)] _open: Option<ReadSignal<bool>>,
    #[prop(optional)] _on_close: Option<Box<dyn Fn() + 'static>>,
    #[prop(optional)] _title: Option<String>,
    #[prop(optional)] _size: Option<ModalSize>,
    #[prop(optional)] _class: Option<String>,
    children: Children,
) -> impl IntoView {
    view! { <div>{children()}</div> }
}
