use std::str::Utf8Error;

mod cursor;
pub mod semantic;
pub mod sliding;
pub mod snapping;

pub use semantic::SemanticWindow;
pub use sliding::SlidingWindow;
pub use snapping::SnappingWindow;

#[inline(always)]
fn concat<'a>(start_str: &'a str, end_str: &'a str) -> Result<&'a str, ChunkerError> {
    let current_ptr =
        std::ptr::slice_from_raw_parts(start_str.as_ptr(), start_str.len() + end_str.len());
    Ok(unsafe { std::str::from_utf8(&*current_ptr) }?)
}

#[derive(Debug, thiserror::Error)]
pub enum ChunkerError {
    #[error("{0}")]
    Config(String),

    #[error("utf-8: {0}")]
    Utf8(#[from] Utf8Error),
}
