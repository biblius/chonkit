use super::{ChunkConfig, Chunker, ChunkerError};
use tracing::debug;

/// The most basic of chunkers.
///
/// `size` determines the base amount for every chunk and
/// `overlap` determines how much back and front characters
/// to extend the base with.
#[derive(Debug, Default)]
pub struct SlidingWindow {
    config: ChunkConfig,
}

impl SlidingWindow {
    pub fn from_config(config: ChunkConfig) -> Result<Self, ChunkerError> {
        Ok(Self { config })
    }

    pub fn new(size: usize, overlap: usize) -> Self {
        Self {
            config: ChunkConfig::new(size, overlap),
        }
    }
}

impl Chunker for SlidingWindow {
    fn chunk<'a>(&self, input: &'a str) -> Result<Vec<&'a str>, ChunkerError> {
        let SlidingWindow {
            config: ChunkConfig { size, overlap },
        } = self;

        let input = input.trim();

        if input.is_empty() {
            return Ok(vec![]);
        }

        if input.len() <= size + overlap {
            return Ok(vec![input]);
        }

        let mut chunks = vec![];

        let mut start = 0;
        let mut end = *size;
        let input_size = input.len();

        loop {
            let chunk_start = if start == 0 { 0 } else { start - overlap };
            let chunk_end = end + overlap;

            if chunk_end > input_size {
                let chunk = &input[chunk_start..input_size];
                chunks.push(chunk);
                break;
            }

            let chunk = &input[chunk_start..chunk_end];
            chunks.push(chunk);

            start = end;
            end += size;
        }

        debug!(
            "Chunked {} chunks, avg chunk size: {}",
            chunks.len(),
            chunks.iter().fold(0, |acc, el| acc + el.len()) / chunks.len()
        );

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliding_window_works() {
        let input = "Sticks and stones may break my bones, but words will never leverage agile frameworks to provide a robust synopsis for high level overviews.";
        let window = SlidingWindow::new(30, 20);
        let chunks = window.chunk(input).unwrap();

        assert_eq!(&input[0..50], chunks[0]);
        assert_eq!(&input[10..80], chunks[1]);
        assert_eq!(&input[40..110], chunks[2]);
        assert_eq!(&input[70..], chunks[3]);
    }

    #[test]
    fn sliding_window_empty() {
        let input = "";
        let window = SlidingWindow::new(1, 0);
        let chunks = window.chunk(input).unwrap();

        assert!(chunks.is_empty());
    }

    #[test]
    fn sliding_window_small_input() {
        let input = "Foobar";
        let window = SlidingWindow::new(30, 20);
        let chunks = window.chunk(input).unwrap();

        assert_eq!(input, chunks[0]);
    }
}
