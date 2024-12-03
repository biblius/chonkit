//! Module containing concrete implementations from the [core](crate::core) module.

/// Batch embedder implementation.
pub mod batch;

/// Document storage implementations.
pub mod document;

/// Text embedder implementations.
pub mod embedder;

/// Repository implementations.
pub mod repo;

/// Application state configuration.
pub mod state;

/// Vector database implementations.
pub mod vector;

/// HTTP server implementation.
pub mod server;

#[cfg(test)]
pub mod test;

#[cfg(feature = "auth-vault")]
pub mod auth;
