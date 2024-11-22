/// Concrete implementations of the [core] module.
pub mod app;

/// Application starting arguments and configuration.
pub mod config;

/// Core business logic.
pub mod core;

/// Error types.
pub mod error;

/// The name for the default collection created on application startup.
pub const DEFAULT_COLLECTION_NAME: &str = "Chonkit_Default_Collection";
