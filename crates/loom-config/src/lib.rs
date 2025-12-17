//! Configuration management for Loom AI agent.
//!
//! This crate provides:
//! - XDG Base Directory compliant path resolution
//! - Layered configuration from multiple sources
//! - TOML configuration file parsing
//! - Environment variable overrides
//! - Configuration validation

pub mod error;
pub mod paths;
pub mod layer;
pub mod runtime;
pub mod registry;
pub mod sources;
pub mod validation;

pub use error::ConfigError;
pub use paths::PathsConfig;
pub use layer::ConfigLayer;
pub use runtime::LoomConfig;
pub use registry::ConfigRegistry;
pub use sources::{ConfigSource, Precedence};

/// Load configuration from all sources with default precedence.
pub fn load_config() -> Result<LoomConfig, ConfigError> {
    let paths = paths::resolve_xdg_paths()?;
    let mut registry = ConfigRegistry::new();
    
    registry.register(Box::new(sources::DefaultsSource));
    registry.register(Box::new(sources::FileSource::system()));
    registry.register(Box::new(sources::FileSource::user(&paths)));
    if let Ok(ws) = sources::FileSource::workspace() {
        registry.register(Box::new(ws));
    }
    registry.register(Box::new(sources::EnvSource));
    
    registry.load(paths)
}

/// Load configuration with CLI overrides.
pub fn load_config_with_cli(cli: sources::CliOverrides) -> Result<LoomConfig, ConfigError> {
    let paths = paths::resolve_xdg_paths()?;
    let mut registry = ConfigRegistry::new();
    
    registry.register(Box::new(sources::DefaultsSource));
    registry.register(Box::new(sources::FileSource::system()));
    registry.register(Box::new(sources::FileSource::user(&paths)));
    if let Ok(ws) = sources::FileSource::workspace() {
        registry.register(Box::new(ws));
    }
    registry.register(Box::new(sources::EnvSource));
    registry.register(Box::new(sources::CliSource::new(cli)));
    
    registry.load(paths)
}
