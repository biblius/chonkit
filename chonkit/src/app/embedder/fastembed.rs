#[cfg(all(not(debug_assertions), feature = "fe-local", feature = "fe-remote"))]
compile_error!("only one of 'fe-local' or 'fe-remote' can be enabled when compiling");

/// Embedder implementation for fastembed when running it locally.
#[cfg(feature = "fe-local")]
pub mod local;

#[cfg(feature = "fe-remote")]
pub mod remote;

pub const DEFAULT_COLLECTION_MODEL: &str = "Xenova/bge-base-en-v1.5";
pub const DEFAULT_COLLECTION_SIZE: usize = 768;
