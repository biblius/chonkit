use std::str::Utf8Error;

mod cursor;
pub mod semantic;
pub mod sliding;
pub mod snapping;

pub use semantic::SemanticWindow;
pub use sliding::SlidingWindow;
pub use snapping::SnappingWindow;

#[derive(Debug, thiserror::Error)]
pub enum ChunkerError {
    #[error("{0}")]
    Config(String),

    #[error("utf-8: {0}")]
    Utf8(#[from] Utf8Error),
}
