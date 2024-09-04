use crate::error::ChonkitError;
use serde::{Deserialize, Serialize};
use std::str::Utf8Error;
use thiserror::Error;

mod ssw;
mod sw;

pub use ssw::SnappingWindow;
pub use sw::SlidingWindow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Chunker {
    Sliding(SlidingWindow),
    Snapping(SnappingWindow),
}

impl DocumentChunker for Chunker {
    fn chunk<'a>(&self, input: &'a str) -> Result<Vec<&'a str>, ChunkerError> {
        match self {
            Self::Sliding(chunker) => Ok(chunker.chunk(input)?),
            Self::Snapping(chunker) => Ok(chunker.chunk(input)?),
        }
    }
}

impl Chunker {
    /// Create a `SlidingWindow` chunker.
    ///
    /// * `size`: Chunk base size.
    /// * `overlap`: Chunk overlap.
    pub fn sliding(size: usize, overlap: usize) -> Self {
        Self::Sliding(SlidingWindow::new(size, overlap))
    }

    /// Create a `SnappingWindow` chunker.
    ///
    /// * `size`: Chunk base size.
    /// * `overlap`: Chunk overlap.
    /// * `skip_f`: Patterns in front of delimiters to not treat as sentence stops.
    /// * `skip_b`: Patterns behind delimiters to not treat as sentence stops.
    pub fn snapping(size: usize, overlap: usize, skip_f: Vec<String>, skip_b: Vec<String>) -> Self {
        Self::Snapping(
            SnappingWindow::new(size, overlap)
                .skip_forward(skip_f)
                .skip_back(skip_b),
        )
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Sliding(SlidingWindow { config }) => config.size,
            Self::Snapping(SnappingWindow { config, .. }) => config.size,
        }
    }

    pub fn overlap(&self) -> usize {
        match self {
            Self::Sliding(SlidingWindow { config }) => config.overlap,
            Self::Snapping(SnappingWindow { config, .. }) => config.overlap,
        }
    }
}

impl std::fmt::Display for Chunker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sliding(_) => write!(f, "SlidingWindow"),
            Self::Snapping(_) => write!(f, "SnappingWindow"),
        }
    }
}

/// Chunk all the files in the specified directory. If `out` is provided, the chunks
/// will be written to the given directory.
pub fn prepare_chunks<T: DocumentChunker>(
    chunker: &T,
    directory: &str,
    out: Option<&str>,
) -> Result<(), ChonkitError> {
    // TODO: Handle bad out directory

    let entries = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    for entry in entries {
        if entry.path().is_dir() {
            prepare_chunks(chunker, &entry.path().display().to_string(), out)?;
            continue;
        }

        let file = std::fs::read_to_string(entry.path())?;
        let chunks = chunker.chunk(&file)?;

        if let Some(ref out) = out {
            std::fs::write(
                format!(
                    "{}/{}.json",
                    out,
                    entry.path().file_name().unwrap().to_str().unwrap()
                ),
                serde_json::to_string_pretty(&chunks)?,
            )?;
        }
    }

    Ok(())
}

pub trait DocumentChunker {
    fn chunk<'a>(&self, input: &'a str) -> Result<Vec<&'a str>, ChunkerError>;
}

#[derive(Debug, Error)]
pub enum ChunkerError {
    #[error("{0}")]
    Config(String),

    #[error("utf-8: {0}")]
    Utf8(#[from] Utf8Error),
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct ChunkBaseConfig {
    /// Base chunk size.
    pub size: usize,

    /// The overlap per chunk.
    pub overlap: usize,
}

impl ChunkBaseConfig {
    pub fn new(size: usize, overlap: usize) -> Self {
        Self { size, overlap }
    }
}

#[inline(always)]
fn concat<'a>(start_str: &'a str, end_str: &'a str) -> Result<&'a str, Utf8Error> {
    let current_ptr =
        std::ptr::slice_from_raw_parts(start_str.as_ptr(), start_str.len() + end_str.len());
    unsafe { std::str::from_utf8(&*current_ptr) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_sanity() {
        let input = "Hello\nWorld";
        let split = input.split_inclusive('\n').collect::<Vec<_>>();

        let one = split[0];
        let two = split[1];

        assert_eq!(input, concat(one, two).unwrap())
    }
}
