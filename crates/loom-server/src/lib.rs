//! Loom thread persistence server.
//!
//! This crate provides an HTTP server for persisting and syncing Loom threads
//! to a SQLite database.

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod health;

pub use api::create_router;
pub use config::ServerConfig;
pub use db::ThreadRepository;
pub use error::ServerError;
