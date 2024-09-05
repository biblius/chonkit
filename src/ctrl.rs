//! Controller layer, i.e. interactors.

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "http")]
pub mod http;
