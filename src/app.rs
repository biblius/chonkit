//! Module containing concrete implementations from the [core](crate::core) module.

pub mod batch;
pub mod document;
pub mod embedder;
pub mod repo;
pub mod state;
pub mod vector;

#[cfg(test)]
pub mod test;
