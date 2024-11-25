use super::ChunkerError;
use tracing::debug;

const SLIDING_WINDOW_DEFAULT_SIZE: usize = 1000;
const SLIDING_WINDOW_DEFAULT_OVERLAP: usize = 200;

/// The most basic of chunkers.
///
/// `size` determines the base amount for every chunk and
/// `overlap` determines how much back and front characters
/// to extend the base with.
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    pub size: usize,
    pub overlap: usize,
}

impl SlidingWindow {
    /// Create a new `SlidingWindow` chunker.
    /// Errors if `overlap` is greater than `size`.
    pub fn new(size: usize, overlap: usize) -> Result<Self, ChunkerError> {
        if overlap > size {
            return Err(ChunkerError::Config(
                "overlap must be less than size".to_string(),
            ));
        }
        Ok(Self { size, overlap })
    }
}

impl Default for SlidingWindow {
    fn default() -> Self {
        Self::new(SLIDING_WINDOW_DEFAULT_SIZE, SLIDING_WINDOW_DEFAULT_OVERLAP)
            .expect("overlap is greater than size")
    }
}

impl SlidingWindow {
    pub fn chunk<'a>(&self, input: &'a str) -> Result<Vec<&'a str>, ChunkerError> {
        let SlidingWindow { size, overlap } = self;

        let input = input.trim();

        if input.is_empty() {
            return Ok(vec![]);
        }

        // Return whole input if it fits
        if input.len() <= size + overlap {
            return Ok(vec![input]);
        }

        let mut chunks = vec![];

        let mut start = 0;
        let mut end = *size;
        let input_size = input.len();

        loop {
            let mut chunk_start = if start == 0 { 0 } else { start - overlap };
            let mut chunk_end = end + overlap;

            // Snap to first char boundary
            while !input.is_char_boundary(chunk_start) {
                chunk_start -= 1;
            }

            // Snap to last char boundary
            while !input.is_char_boundary(chunk_end) && chunk_end < input.len() - 1 {
                chunk_end += 1;
            }

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

    #[tokio::test]
    async fn sliding_window_works() {
        let input = "Sticks and stones may break my bones, but words will never leverage agile frameworks to provide a robust synopsis for high level overviews.";
        let window = SlidingWindow::new(30, 20).unwrap();
        let chunks = window.chunk(input).unwrap();

        assert_eq!(&input[0..50], chunks[0]);
        assert_eq!(&input[10..80], chunks[1]);
        assert_eq!(&input[40..110], chunks[2]);
        assert_eq!(&input[70..], chunks[3]);
    }

    #[tokio::test]
    async fn sliding_window_empty() {
        let input = "";
        let window = SlidingWindow::new(1, 0).unwrap();
        let chunks = window.chunk(input).unwrap();

        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn sliding_window_small_input() {
        let input = "Foobar";
        let window = SlidingWindow::new(30, 20).unwrap();
        let chunks = window.chunk(input).unwrap();

        assert_eq!(input, chunks[0]);
    }
}
