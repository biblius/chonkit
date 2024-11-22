#[cfg(any(feature = "fe-local", feature = "fe-remote"))]
pub mod fastembed;

#[cfg(feature = "openai")]
pub mod openai;
