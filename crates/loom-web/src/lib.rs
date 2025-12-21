#![recursion_limit = "512"]

/// Loom Web UI - Leptos SPA
///
/// A single-page application for the Loom AI coding assistant, built with Leptos and Tailwind CSS.
/// Provides a component library (design system) and route-based application shell.
pub mod app;
pub mod components;
pub mod prelude;
pub mod routes;
pub mod server_fns;
pub mod services;

// Re-exports
pub use app::App;
pub use prelude::*;
