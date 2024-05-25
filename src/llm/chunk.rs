use serde::{Deserialize, Serialize};
use std::str::Utf8Error;
use thiserror::Error;

mod seq;
mod ssw;
mod sw;

pub use seq::Recursive;
pub use ssw::SnappingWindow;
pub use sw::SlidingWindow;

pub trait Chunker {
    fn chunk<'a>(&self, input: &'a str) -> Result<Vec<Chunk<'a>>, ChunkerError>;
}

#[derive(Debug, Error)]
pub enum ChunkerError {
    #[error("{0}")]
    Config(String),

    #[error("utf-8: {0}")]
    Utf8(#[from] Utf8Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chunk<'a> {
    pub content: &'a str,
}

impl<'a> Chunk<'a> {
    pub fn new(content: &'a str) -> Self {
        Chunk { content }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct ChunkConfig {
    /// Base chunk size.
    pub size: usize,

    /// The overlap per chunk.
    pub overlap: usize,
}

impl ChunkConfig {
    /// Default chunk size for all chunkers
    pub const DEFAULT_SIZE: usize = 1000;

    /// Default chunk overlap for all character based chunkers
    pub const DEFAULT_OVERLAP: usize = 500;

    pub fn new(size: usize, overlap: usize) -> Self {
        Self { size, overlap }
    }
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            size: Self::DEFAULT_SIZE,
            overlap: Self::DEFAULT_OVERLAP,
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

    pub const INPUT: &str = r#"
What I Worked On

February 2021

Before college the two main things I worked on, outside of school, were writing and programming. I didn't write essays. I wrote what beginning writers were supposed to write then, and probably still are: short stories. My stories were awful. They had hardly any plot... just characters with strong feelings, which I imagined made them deep.

The first programs I tried writing were on the IBM 1401 that our school district used for what was then called "data processing." This was in 9th grade, so I was 13 or 14. The school district's 1401 happened to be in the basement of our junior high school, and my friend Rich Draves and I got permission to use it. It was like a mini Bond villain's lair down there, with all these alien-looking machines — CPU, disk drives, printer, card reader — sitting up on a raised floor under bright fluorescent lights.
"#;

    #[test]
    fn pointer_sanity() {
        let input = "Hello\nWorld";
        let split = input.split_inclusive('\n').collect::<Vec<_>>();

        let one = split[0];
        let two = split[1];

        assert_eq!(input, concat(one, two).unwrap())
    }
}
