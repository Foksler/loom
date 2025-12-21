/// Client entry point for Loom Web UI
///
/// Hydrates the server-rendered HTML and starts the client-side application.
use crate::prelude::*;

#[cfg(feature = "hydrate")]
use loom_web::App;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use loom_web::App;

    #[cfg(feature = "hydrate")]
    {
        console_error_panic_hook::set_once();
        leptos::mount_to_body(|| view! { <App/> });
    }
}

#[cfg(not(feature = "hydrate"))]
fn main() {
    panic!("This is a WebAssembly binary. Use `cargo leptos build` to build.")
}
