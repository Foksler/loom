//! Loom thread persistence server.
//!
//! This crate provides an HTTP server for persisting and syncing Loom threads
//! to a SQLite database.

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod health;
pub mod llm_proxy;

pub use api::{create_app_state, create_router, AppState};
pub use config::ServerConfig;
pub use db::{GithubInstallation, GithubInstallationInfo, GithubRepo, ThreadRepository};
pub use error::ServerError;
