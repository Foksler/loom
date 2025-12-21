/// Client entry point for Loom Web UI
///
/// Hydrates the server-rendered HTML and starts the client-side application.
#[cfg(feature = "hydrate")]
fn main() {
    // Hydrate islands on the page
    leptos::prelude::hydrate_islands();
}

#[cfg(not(feature = "hydrate"))]
fn main() {
    panic!("This binary requires the 'hydrate' feature. Use `cargo build -p loom-web --features hydrate` to build.")
}
