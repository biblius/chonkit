use super::{
    concat,
    cursor::{
        byte_count, snap_back, snap_front, Cursor, CursorRev, DEFAULT_SKIP_B, DEFAULT_SKIP_F,
    },
    ChunkBaseConfig, ChunkerError, DocumentChunker,
};
use crate::error::ChonkitError;
use serde::{Deserialize, Serialize};

#[cfg(debug_assertions)]
use tracing::trace;
use validify::Validate;

const SNAPPING_WINDOW_DEFAULT_SIZE: usize = 1000;
const SNAPPING_WINDOW_DEFAULT_OVERLAP: usize = 5;

/// Heuristic chunker for texts intended for humans, e.g. documentation, books, blogs, etc.
///
/// A sliding window that is aware of sentence stops,
///
/// It will attempt to chunk the content according to `size`. Keep in mind it cannot
/// be exact and the chunks will probably be larger, because of the way it searches
/// for delimiters.
///
/// The chunker can also be configured to skip common patterns including the fullstop
/// such as abbreviations (e.g., i.e., etc.) and urls.
///
/// The default delimiter is `'.'`.
/// The default `size` and `overlap` are 1000 and 10.
/// The default forward skips are [SnappingWindow::DEFAULT_SKIP_F].
/// The default backward skips are [SnappingWindow::DEFAULT_SKIP_B].
///
/// Keep in mind the configuration for this chunker is different; The `size` will
/// represent the amount of bytes in the chunk and the `overlap` will represent
/// how many back/forward sentences will be included.
///
/// If the input has a lot of unicode with characters more than 1 byte, a larger `size` is
/// recommended.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SnappingWindow {
    /// Here `size` represents the amount of bytes in the base chunk
    /// while `overlap` will represent the amount of leading/trailing sentences.
    pub config: ChunkBaseConfig,

    /// The delimiter to use to split sentences. At time of writing the most common one is ".".
    pub delimiter: char,

    /// Whenever a delimiter is found, the chunker will look ahead for these sequences
    /// and will skip the delimiter if found, treating it as a regular char.
    ///
    /// Useful for common abbreviations and urls.
    pub skip_forward: Vec<String>,

    /// Whenever a delimiter is found, the chunker will look back for these sequences
    /// and will skip the delimiter if found, treating it as a regular char.
    ///
    /// Useful for common abbreviations and urls.
    pub skip_back: Vec<String>,
}

impl Default for SnappingWindow {
    fn default() -> Self {
        Self::new(
            SNAPPING_WINDOW_DEFAULT_SIZE,
            SNAPPING_WINDOW_DEFAULT_OVERLAP,
        )
        .expect("overlap is greater than size")
        .skip_forward(DEFAULT_SKIP_F.iter().map(|e| e.to_string()).collect())
        .skip_back(DEFAULT_SKIP_B.iter().map(|e| e.to_string()).collect())
    }
}

impl SnappingWindow {
    pub fn new(size: usize, overlap: usize) -> Result<Self, ChunkerError> {
        Ok(Self {
            config: ChunkBaseConfig::new(size, overlap)?,
            delimiter: '.',
            skip_forward: DEFAULT_SKIP_F.iter().map(|e| e.to_string()).collect(),
            skip_back: DEFAULT_SKIP_B.iter().map(|e| e.to_string()).collect(),
        })
    }

    pub fn delimiter(mut self, delimiter: char) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Set the forward skips.
    pub fn skip_forward(mut self, skip_forward: Vec<String>) -> Self {
        self.skip_forward = skip_forward;
        self
    }

    /// Set the backward skips.
    pub fn skip_back(mut self, skip_back: Vec<String>) -> Self {
        self.skip_back = skip_back;
        self
    }

    /// Extend the forward and backward skips.
    pub fn extend_skips(mut self, skip_forward: Vec<String>, skip_back: Vec<String>) -> Self {
        self.skip_forward.extend(skip_forward);
        self.skip_back.extend(skip_back);
        self
    }
}

impl<'a> DocumentChunker<'a> for SnappingWindow {
    type Output = &'a str;

    async fn chunk(&self, input: &'a str) -> Result<Vec<&'a str>, ChonkitError> {
        self.config.validate()?;

        if input.trim().is_empty() {
            return Ok(vec![]);
        }

        let Self {
            config: ChunkBaseConfig { size, overlap },
            delimiter: delim,
            skip_forward,
            skip_back,
        } = self;

        let total_bytes = byte_count(input);

        let mut chunks = vec![];

        let mut cursor = Cursor::new(input, *delim);

        // Cursor position
        let mut start = 1;

        snap_front(&mut start, input);

        let mut chunk = &input[..start];

        loop {
            // Cursor has reached the end.
            if start >= total_bytes {
                if !chunk.is_empty() {
                    chunks.push(chunk)
                }
                break;
            }

            // Advance until delim
            cursor.advance();

            // Check for skips
            if cursor.advance_if_peek(skip_forward, skip_back) {
                continue;
            }

            let piece = &input[start..cursor.byte_offset];

            start += byte_count(piece);

            chunk = concat(chunk, piece)?;

            if byte_count(chunk) < *size {
                // If the cursor is not finished, take another batch.
                if !cursor.finished() {
                    continue;
                }

                // Otherwise, we are at the end of input.
                let prev = &input[..cursor.byte_offset - byte_count(chunk)];
                let prev = previous_chunk(prev, *overlap, *delim, skip_forward, skip_back);
                let chunk_full = concat(prev, chunk)?;
                chunks.push(chunk_full);

                #[cfg(debug_assertions)]
                {
                    trace!(
                        "Added last chunk (full:{}|base:{}|prev:{}), total: {}",
                        chunk_full.len(),
                        chunk.len(),
                        prev.len(),
                        chunks.len()
                    );
                }
                break;
            }

            let prev = &input[..cursor.byte_offset - byte_count(chunk)];
            let next = &input[cursor.byte_offset..];

            let prev = previous_chunk(prev, *overlap, *delim, skip_forward, skip_back);
            let (next, next_offset) = next_chunk(next, *overlap, *delim, skip_forward, skip_back);

            let chunk_full = concat(concat(prev, chunk)?, next)?;

            // Skip the first chunk since its contents will be in the following one.
            if !prev.is_empty() {
                chunks.push(chunk_full);
                #[cfg(debug_assertions)]
                {
                    trace!(
                        "Added chunk (full:{}|base:{}|prev:{}|next:{}), total: {}",
                        chunk_full.len(),
                        chunk.len(),
                        prev.len(),
                        next.len(),
                        chunks.len()
                    );
                }
            }

            start += 1;
            snap_front(&mut start, input);

            if start + next_offset >= total_bytes {
                // Handles case where the full text is chunked
                // and there is no previous chunk
                if chunks.is_empty() {
                    chunks.push(chunk_full);
                    #[cfg(debug_assertions)]
                    {
                        trace!(
                            "Added last chunk (full:{}|base:{}|prev:{}|next:{}), total: {}",
                            chunk_full.len(),
                            chunk.len(),
                            prev.len(),
                            next.len(),
                            chunks.len()
                        );
                    }
                }
                break;
            }

            let mut chunk_start = start - 1;
            snap_back(&mut chunk_start, input);

            chunk = &input[chunk_start..start];
        }

        Ok(chunks)
    }
}

#[inline(always)]
fn previous_chunk<'a>(
    input: &'a str,
    overlap: usize,
    delim: char,
    skip_forward: &[String],
    skip_back: &[String],
) -> &'a str {
    let mut p_cursor = CursorRev::new(input, delim);
    for _ in 0..overlap {
        p_cursor.advance();
        while p_cursor.advance_if_peek(skip_forward, skip_back) {
            p_cursor.advance();
        }
    }
    p_cursor.get_slice()
}

#[inline(always)]
fn next_chunk<'a>(
    input: &'a str,
    overlap: usize,
    delim: char,
    skip_forward: &[String],
    skip_back: &[String],
) -> (&'a str, usize) {
    let mut n_cursor = Cursor::new(input, delim);
    for _ in 0..overlap {
        n_cursor.advance();
        while n_cursor.advance_if_peek(skip_forward, skip_back) {
            n_cursor.advance();
        }
    }
    (n_cursor.get_slice(), n_cursor.byte_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_size() {
        let ch = 'Ü';
        let mut bytes = [0, 0];
        assert_eq!(2, ch.encode_utf8(&mut bytes).len());
    }

    #[test]
    fn constructor() {
        // For lifetime sanity checks
        let skip_f = vec![String::from("foo"), String::from("bar")];
        let skip_b = vec![String::from("foo"), String::from("bar")];
        let size = 1;
        let overlap = 1;
        let delimiter = '!';

        let chunker = SnappingWindow::new(size, overlap)
            .unwrap()
            .delimiter(delimiter)
            .skip_forward(skip_f.clone())
            .skip_back(skip_b.clone());

        assert_eq!(delimiter, chunker.delimiter);
        assert_eq!(size, chunker.config.size);
        assert_eq!(overlap, chunker.config.overlap);
        assert_eq!(skip_f, chunker.skip_forward);
        assert_eq!(skip_b, chunker.skip_back);
    }

    #[tokio::test]
    async fn snapping_works() {
        let input =
            "I have a sentence. It is not very long. Here is another. Long schlong ding dong.";
        let chunker = SnappingWindow::new(1, 1).unwrap();
        let expected = [
            "I have a sentence. It is not very long. Here is another.",
            " It is not very long. Here is another. Long schlong ding dong.",
        ];

        let chunks = chunker.chunk(input.trim()).await.unwrap();
        assert_eq!(2, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }

    #[tokio::test]
    async fn snapping_skips_back() {
        let input =
            "I have a sentence. It contains letters, words, etc. and it contains more. The most important of which is foobar., because it must be skipped.";
        let chunker = SnappingWindow::new(1, 1)
            .unwrap()
            .skip_back(vec!["etc".to_string(), "foobar".to_string()]);
        let expected = [input];

        let chunks = chunker.chunk(input.trim()).await.unwrap();
        assert_eq!(1, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }

    #[tokio::test]
    async fn snapping_skips_forward() {
        let input =
            "Go to sentences.org for more words. 50% off on words with >4 syllables. Leverage agile frameworks to provide robust high level overview at agile.com.";

        let chunker = SnappingWindow::new(1, 1)
            .unwrap()
            .skip_forward(vec!["com".to_string(), "org".to_string()]);
        let expected = [input];

        let chunks = chunker.chunk(input.trim()).await.unwrap();
        assert_eq!(1, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }

    #[tokio::test]
    async fn snapping_skips_common_abbreviations() {
        let input =
            "Words are hard. There are many words in existence, e.g. this, that, etc..., quite a few, as you can see. My opinion, available at nobodycares.com, is that words should convey meaning. Not everyone agrees however, which is why they leverage agile frameworks to provide robust synopses for high level overviews. The lucidity of meaning is, in fact, obscured and ambiguous, therefore the interpretation, i.e. the conveying of units of meaning is less than optimal. Jebem ti boga.";

        let chunker = SnappingWindow::new(1, 1).unwrap();

        let expected = [
            "Words are hard. There are many words in existence, e.g. this, that, etc..., quite a few, as you can see. My opinion, available at nobodycares.com, is that words should convey meaning.",
            " There are many words in existence, e.g. this, that, etc..., quite a few, as you can see. My opinion, available at nobodycares.com, is that words should convey meaning. Not everyone agrees however, which is why they leverage agile frameworks to provide robust synopses for high level overviews.",
            " My opinion, available at nobodycares.com, is that words should convey meaning. Not everyone agrees however, which is why they leverage agile frameworks to provide robust synopses for high level overviews. The lucidity of meaning is, in fact, obscured and ambiguous, therefore the interpretation, i.e. the conveying of units of meaning is less than optimal.",
            " Not everyone agrees however, which is why they leverage agile frameworks to provide robust synopses for high level overviews. The lucidity of meaning is, in fact, obscured and ambiguous, therefore the interpretation, i.e. the conveying of units of meaning is less than optimal. Jebem ti boga.",
        ];

        let chunks = chunker.chunk(input.trim()).await.unwrap();
        assert_eq!(4, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }

    #[tokio::test]
    async fn snapping_table_of_contents() {
        let input =
            "Table of contents:\n1 Super cool stuff\n1.1 Some chonkers in rust\n1.2 Some data for your LLM\n1.3 ??? \n1.4 Profit \n1.4.1 Lambo\nHope you liked the table of contents. See more at content.co.com.";

        let chunker = SnappingWindow::new(1, 1)
            .unwrap()
            .skip_forward(vec![
                "0".to_string(),
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "co".to_string(),
                "com".to_string(),
            ])
            .skip_back(vec!["com".to_string()]);

        let expected = [input];

        let chunks = chunker.chunk(input.trim()).await.unwrap();
        assert_eq!(1, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }

    #[tokio::test]
    async fn snapping_window_empty() {
        let chunker = SnappingWindow::new(1, 1).unwrap();
        let chunks = chunker.chunk("").await.unwrap();
        assert!(chunks.is_empty());
    }
}
