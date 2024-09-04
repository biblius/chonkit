use crate::error::ChonkitError;
use serde::{Deserialize, Serialize};
use std::str::Utf8Error;
use thiserror::Error;

mod rec;
mod ssw;
mod sw;

pub use rec::Recursive;
pub use ssw::SnappingWindow;
pub use sw::SlidingWindow;

pub fn chunk(config: ChunkConfig, content: &str) -> Result<Vec<&str>, ChunkerError> {
    match config {
        ChunkConfig::SlidingWindow {
            config: ChunkBaseConfig { size, overlap },
        } => {
            let chunker = SlidingWindow::new(size, overlap);
            let chunks = chunker.chunk(content)?.into_iter().collect::<Vec<_>>();
            Ok(chunks)
        }
        ChunkConfig::SnappingWindow {
            config: ChunkBaseConfig { size, overlap },
            skip_f,
            skip_b,
        } => {
            let chunker = SnappingWindow::new(size, overlap)
                .skip_forward(skip_f)
                .skip_back(skip_b);
            let chunks = chunker.chunk(content)?.into_iter().collect::<Vec<_>>();
            Ok(chunks)
        }
        ChunkConfig::Recursive {
            config: ChunkBaseConfig { size, overlap },
            delimiters,
        } => {
            let delims = delimiters.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            let chunker = Recursive::new(size, overlap, &delims);
            let chunks = chunker.chunk(content)?.into_iter().collect::<Vec<_>>();
            Ok(chunks)
        }
    }
}

/// Chunk all the files in the specified directory. If `out` is provided, the chunks
/// will be written to the given directory.
pub fn prepare_chunks<T: Chunker>(
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

pub trait Chunker {
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
    /// Default chunk size for all chunkers
    pub const DEFAULT_SIZE: usize = 1000;

    /// Default chunk overlap for all character based chunkers
    pub const DEFAULT_OVERLAP: usize = 500;

    pub fn new(size: usize, overlap: usize) -> Self {
        Self { size, overlap }
    }
}

impl Default for ChunkBaseConfig {
    fn default() -> Self {
        Self {
            size: Self::DEFAULT_SIZE,
            overlap: Self::DEFAULT_OVERLAP,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChunkConfig {
    SlidingWindow {
        config: ChunkBaseConfig,
    },
    SnappingWindow {
        config: ChunkBaseConfig,
        skip_f: Vec<String>,
        skip_b: Vec<String>,
    },
    Recursive {
        config: ChunkBaseConfig,
        delimiters: Vec<String>,
    },
}

impl ChunkConfig {
    /// Create a `SlidingWindow` chunker.
    ///
    /// * `size`: Chunk base size.
    /// * `overlap`: Chunk overlap.
    pub fn sw(size: usize, overlap: usize) -> Self {
        Self::SlidingWindow {
            config: ChunkBaseConfig::new(size, overlap),
        }
    }

    /// Create a `SnappingWindow` chunker.
    ///
    /// * `size`: Chunk base size.
    /// * `overlap`: Chunk overlap.
    /// * `skip_f`: Patterns in front of delimiters to not treat as sentence stops.
    /// * `skip_b`: Patterns behind delimiters to not treat as sentence stops.
    pub fn ssw(size: usize, overlap: usize, skip_f: Vec<String>, skip_b: Vec<String>) -> Self {
        Self::SnappingWindow {
            config: ChunkBaseConfig::new(size, overlap),
            skip_f,
            skip_b,
        }
    }

    /// Create a `Recursive` chunker.
    ///
    /// * `size`: Chunk base size.
    /// * `overlap`: Chunk overlap.
    /// * `delimiters`: Delimiters to use to split text.
    pub fn rec(size: usize, overlap: usize, delimiters: Vec<String>) -> Self {
        Self::Recursive {
            config: ChunkBaseConfig::new(size, overlap),
            delimiters,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            ChunkConfig::SlidingWindow { config } => config.size,
            ChunkConfig::SnappingWindow { config, .. } => config.size,
            ChunkConfig::Recursive { config, .. } => config.size,
        }
    }

    pub fn overlap(&self) -> usize {
        match self {
            ChunkConfig::SlidingWindow { config } => config.overlap,
            ChunkConfig::SnappingWindow { config, .. } => config.overlap,
            ChunkConfig::Recursive { config, .. } => config.overlap,
        }
    }
}

impl std::fmt::Display for ChunkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkConfig::SlidingWindow { .. } => write!(f, "SlidingWindow"),
            ChunkConfig::SnappingWindow { .. } => write!(f, "SnappingWindow"),
            ChunkConfig::Recursive { .. } => write!(f, "Recursive"),
        }
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
