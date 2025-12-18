//! Common configuration primitives for Loom.
//!
//! This crate provides shared types and helpers for configuration across
//! all Loom crates, including:
//!
//! - [`Secret<T>`]: A wrapper type that prevents accidental logging of sensitive values
//!   (re-exported from [`loom_secret`])
//! - [`load_secret_env`]: Helper for loading secrets from environment variables with `*_FILE` support

pub mod env;

// Re-export Secret types from loom-secret for convenience
pub use loom_secret::{Secret, SecretString, REDACTED};

pub use env::{load_secret_env, RequiredSecretError, SecretEnvError};
