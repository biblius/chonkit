#[cfg(all(not(debug_assertions), feature = "fe-local", feature = "fe-remote"))]
compile_error!("only one of 'fe-local' or 'fe-remote' can be enabled when compiling");

#[cfg(all(not(feature = "fe-remote"), feature = "fe-local"))]
pub use local::FastEmbedder;

#[cfg(all(test, feature = "fe-local"))]
pub use local::init_single;

#[cfg(feature = "fe-remote")]
pub use remote::FastEmbedder;

/// Embedder implementation for fastembed when running it locally.
#[cfg(feature = "fe-local")]
mod local;

/// Embedder implementation for running fastembed on a remote
/// machine supporting CUDA.
#[cfg(feature = "fe-remote")]
mod remote;

const DEFAULT_COLLECTION_MODEL: &str = "Xenova/bge-base-en-v1.5";
const DEFAULT_COLLECTION_SIZE: usize = 768;
