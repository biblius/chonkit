/// Embedder implementation for fastembed when running it locally.
#[cfg(feature = "fe-local")]
pub mod local;

/// Embedder implementation for running fastembed on a remote
/// machine supporting CUDA. Uses a reqwest client to connect
/// to a machine running feserver.
#[cfg(feature = "fe-remote")]
pub mod remote;
