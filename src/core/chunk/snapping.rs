use super::{concat, ChunkBaseConfig, ChunkerError, DocumentChunker};
use serde::{Deserialize, Serialize};

#[cfg(debug_assertions)]
use tracing::trace;

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
#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        Self {
            config: ChunkBaseConfig::new(1000, 10),
            delimiter: '.',
            // Common urls, abbreviations, file extensions
            skip_forward: Self::DEFAULT_SKIP_F.iter().map(|e| e.to_string()).collect(),
            skip_back: Self::DEFAULT_SKIP_B.iter().map(|e| e.to_string()).collect(),
        }
    }
}

impl SnappingWindow {
    /// Default patterns to skip in front of delimiters.
    /// `___. some text`
    pub const DEFAULT_SKIP_F: &'static [&'static str] = &[
        "com", "org", "net", // Common URL patterns
        "g.", "e.", // Common acronyms (e.g., i.e.)
        "sh", "rs", "js", "json", // Common file extensions
    ];

    /// Default patterns to skip behind delimiters.
    /// `Some text.___` <
    pub const DEFAULT_SKIP_B: &'static [&'static str] = &[
        "www", // Common URL patterns
        "etc", "e.g", "i.e", // Common acronyms
    ];

    pub fn new(size: usize, overlap: usize) -> Self {
        Self {
            config: ChunkBaseConfig::new(size, overlap),
            ..Default::default()
        }
    }

    pub fn delimiter(mut self, delimiter: char) -> Self {
        self.delimiter = delimiter;
        self
    }

    pub fn skip_forward(mut self, skip_forward: Vec<String>) -> Self {
        self.skip_forward = skip_forward;
        self
    }

    pub fn skip_back(mut self, skip_back: Vec<String>) -> Self {
        self.skip_back = skip_back;
        self
    }
}

impl DocumentChunker for SnappingWindow {
    fn chunk<'a>(&self, input: &'a str) -> Result<Vec<&'a str>, ChunkerError> {
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
                if !cursor.finished() {
                    continue;
                }

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

///
/// TODO: Maybe check if we can remove skip_b because fullstops are usually followed by spaces.
/// It would save a lot of work in the long run.

#[derive(Debug)]
struct Cursor<'a> {
    /// Input.
    buf: &'a str,

    /// Total bytes in buf.
    byte_count: usize,

    /// Indexes into buf, the current position of the cursor.
    /// Always gets advanced past the delimiter.
    byte_offset: usize,

    /// How many chars were skipped during advancing.
    char_offset: usize,

    /// Delimiter to split by.
    delim: char,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str, delim: char) -> Self {
        Self {
            buf: input,
            byte_count: byte_count(input),
            byte_offset: 0,
            char_offset: 0,
            delim,
        }
    }

    fn finished(&self) -> bool {
        self.byte_offset == self.byte_count - 1
    }

    fn get_slice(&self) -> &'a str {
        if self.buf.is_empty() || self.finished() {
            return self.buf;
        }
        &self.buf[..self.byte_offset]
    }

    /// Advance the byte_offset until `delim` is found. The byte_offset will be set
    /// to the index following the delim.
    fn advance(&mut self) {
        if self.buf.is_empty() || self.finished() {
            return;
        }

        let mut chars = self.buf.chars().skip(self.char_offset);

        loop {
            let Some(ch) = chars.next() else {
                break;
            };

            self.byte_offset += ch.len_utf8();
            self.char_offset += 1;

            if self.byte_offset == self.byte_count - 1 {
                break;
            }

            if ch != self.delim {
                continue;
            }

            // If we find repeating delimiters, we should
            // continue to the next single one to capture the end
            // of the sentence
            let mut stop = true;

            while chars.next().is_some_and(|ch| ch == self.delim) {
                self.byte_offset += ch.len_utf8();
                self.char_offset += 1;
                stop = false;
            }

            if stop {
                break;
            }

            self.byte_offset += ch.len_utf8();
            self.char_offset += 1;
        }
    }

    fn advance_exact(&mut self, pat: &str) {
        let amt = byte_count(pat);
        if self.byte_offset + amt >= self.byte_count {
            self.byte_offset = self.byte_count - 1;
            self.char_offset = self.buf.chars().count();
            return;
        }
        self.byte_offset += amt;
        self.char_offset += pat.chars().count();
    }

    fn peek_back(&self, pat: &str) -> bool {
        let pat_offset = byte_count(pat);

        if self.byte_offset.saturating_sub(pat_offset) == 0 {
            return false;
        }

        // Skip if we are done.
        if self.finished() {
            return false;
        }

        let mut start = self.byte_offset - 1 - pat_offset;
        let mut end = self.byte_offset - 1;

        snap_back(&mut start, self.buf);
        snap_back(&mut end, self.buf);

        &self.buf[start..end] == pat
    }

    fn peek_forward(&self, pat: &str) -> bool {
        // Skip if we are done.
        if self.byte_offset + byte_count(pat) >= self.byte_count {
            return false;
        }
        let mut end = self.byte_offset + byte_count(pat);
        snap_front(&mut end, self.buf);
        &self.buf[self.byte_offset..end] == pat
    }

    /// TODO: I'm pretty sure that we can only iterate through
    /// one of the skip vectors in this one if we change the implementation
    /// to always advance if it doesn't encounter a character behind a delimiter.
    fn advance_if_peek(&mut self, forward: &[String], back: &[String]) -> bool {
        for s in back {
            if self.peek_back(s) {
                return true;
            }
        }

        for s in forward {
            if self.peek_forward(s) {
                self.advance_exact(s);
                return true;
            }
        }

        false
    }
}

/// Cursor for scanning a string backwards. The `byte_offset` of this cursor is always
/// kept at `delim` points in `buf`.
#[derive(Debug)]
struct CursorRev<'a> {
    /// The str being scanned.
    buf: &'a str,

    byte_count: usize,

    /// The current byte byte offset of the cursor in the str.
    /// Is kept on delimiter when advancing.
    byte_offset: usize,

    /// Total input UTF-8 chars
    char_count: usize,

    /// The current byte byte offset of the cursor in the str.
    char_offset: usize,

    /// The delimiter to snap to
    delim: char,
}

impl<'a> CursorRev<'a> {
    fn new(input: &'a str, delim: char) -> Self {
        Self {
            buf: input,
            byte_count: byte_count(input),
            byte_offset: input.len().saturating_sub(1),
            char_count: input.chars().count(),
            char_offset: input.chars().count(),
            delim,
        }
    }

    fn finished(&self) -> bool {
        self.byte_offset == 0
    }

    fn get_slice(&self) -> &'a str {
        if self.finished() {
            self.buf
        } else {
            let mut start = self.byte_offset + 1;
            snap_front(&mut start, self.buf);
            &self.buf[start..]
        }
    }

    fn advance(&mut self) {
        if self.finished() {
            return;
        }

        self.byte_offset -= self.delim.len_utf8();
        self.char_offset -= 1;

        let mut chars = self
            .buf
            .chars()
            .rev()
            .skip(self.char_count - self.char_offset);

        loop {
            let Some(ch) = chars.next() else {
                self.byte_offset = 0;
                self.char_offset = self.char_count;
                break;
            };

            if self.finished() {
                break;
            }

            self.byte_offset -= ch.len_utf8();
            self.char_offset -= 1;

            if ch != self.delim {
                continue;
            }

            let mut stop = true;

            // Advance until end of delimiter sequence
            while chars.next().is_some_and(|ch| ch == self.delim) {
                self.byte_offset -= ch.len_utf8();
                self.char_offset -= 1;
                stop = false;
            }

            if stop {
                // Since we are at a single fullstop, we want to increment the
                // byte_offset so as not to include it at the start of the slice.
                self.byte_offset += ch.len_utf8();
                self.char_offset += 1;
                break;
            }

            self.byte_offset -= ch.len_utf8();
            self.char_offset -= 1;
        }
    }

    fn peek_back(&self, pat: &str) -> bool {
        // Skip if we are done.
        if self.finished() {
            return false;
        }
        let mut start = self.byte_offset.saturating_sub(byte_count(pat));
        snap_back(&mut start, self.buf);
        &self.buf[start..self.byte_offset] == pat
    }

    fn peek_forward(&self, pat: &str) -> bool {
        let pat_offset = byte_count(pat);

        // Skip if we are done or at the start.
        if self.finished() || self.byte_offset + pat_offset >= self.byte_count {
            return false;
        }

        let mut start = self.byte_offset + 1;
        let mut end = self.byte_offset + 1 + pat_offset;

        snap_front(&mut start, self.buf);
        snap_front(&mut end, self.buf);

        &self.buf[start..end] == pat
    }

    fn advance_if_peek(&mut self, forward: &[String], back: &[String]) -> bool {
        for s in back {
            if self.peek_back(s) {
                self.advance_exact(s);
                return true;
            }
        }

        for s in forward {
            if self.peek_forward(s) {
                return true;
            }
        }

        false
    }

    fn advance_exact(&mut self, pat: &str) {
        let amt = byte_count(pat);
        self.char_offset -= pat.chars().count();
        self.byte_offset = self.byte_offset.saturating_sub(amt);
    }
}

#[inline(always)]
fn byte_count(input: &str) -> usize {
    input.chars().fold(0, |acc, el| acc + el.len_utf8())
}

#[inline(always)]
fn snap_front(i: &mut usize, input: &str) {
    while !input.is_char_boundary(*i) && *i < input.len() {
        *i += 1;
    }
}

#[inline(always)]
fn snap_back(i: &mut usize, input: &str) {
    if *i == 0 {
        return;
    }
    while !input.is_char_boundary(*i) {
        *i -= 1;
    }
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
            .delimiter(delimiter)
            .skip_forward(skip_f.clone())
            .skip_back(skip_b.clone());

        assert_eq!(delimiter, chunker.delimiter);
        assert_eq!(size, chunker.config.size);
        assert_eq!(overlap, chunker.config.overlap);
        assert_eq!(skip_f, chunker.skip_forward);
        assert_eq!(skip_b, chunker.skip_back);
    }

    #[test]
    fn cursor_advances_to_delimiter() {
        let input = "This is such a sentence. One of the sentences in the world. Super wow.";
        let mut cursor = Cursor::new(input, '.');
        let expected = [
            "This is such a sentence.",
            "This is such a sentence. One of the sentences in the world.",
            input,
        ];
        assert!(cursor.get_slice().is_empty());
        for test in expected {
            cursor.advance();
            assert_eq!(test, cursor.get_slice());
        }
    }

    #[test]
    fn cursor_advances_past_repeating_delimiters() {
        let input = "This is such a sentence... One of the sentences in the world. Super wow.";
        let mut cursor = Cursor::new(input, '.');
        let expected = [
            "This is such a sentence... One of the sentences in the world.",
            input,
        ];
        for test in expected {
            cursor.advance();
            assert_eq!(test, cursor.get_slice());
        }
    }

    #[test]
    fn cursor_advances_exact() {
        let input = "This is Sparta my friend";
        let mut cursor = Cursor::new(input, '.');
        let expected = input.split_inclusive(' ');
        let mut buf = String::new();
        for test in expected {
            assert_eq!(&buf, cursor.get_slice());
            cursor.advance_exact(test);
            buf.push_str(test);
        }
    }

    #[test]
    fn cursor_peek_forward() {
        let input = "This. Is. Sentence. etc.";
        let mut cursor = Cursor::new(input, '.');
        let expected = ["This", " Is", " Sentence", " etc"];
        for test in expected {
            assert!(cursor.peek_forward(test));
            cursor.advance();
        }
        assert!(!cursor.peek_forward("etc"));
    }

    #[test]
    fn cursor_peek_back() {
        let input = "This. Is. Sentence. etc.";
        let mut cursor = Cursor::new(input, '.');
        let expected = ["This", " Is", " Sentence"];
        assert!(!cursor.peek_back("This"));
        for test in expected {
            cursor.advance();
            assert!(cursor.peek_back(test));
        }
    }

    #[test]
    fn rev_cursor_advances_to_delimiter() {
        let input = "This is such a sentence. One of the sentences in the world. Super wow.";
        let mut cursor = CursorRev::new(input, '.');
        let expected = [
            " Super wow.",
            " One of the sentences in the world. Super wow.",
            input,
        ];
        for test in expected {
            cursor.advance();
            assert_eq!(test, cursor.get_slice());
        }
    }

    #[test]
    fn rev_cursor_advances_past_repeating_delimiters() {
        let input =
            "This is such a sentence..... Very sentencey. So many.......... words. One of the sentences in the world... Super wow.";
        let mut cursor = CursorRev::new(input, '.');
        let expected = [
            " One of the sentences in the world... Super wow.",
            " So many.......... words. One of the sentences in the world... Super wow.",
            input,
        ];
        for test in expected {
            cursor.advance();
            assert_eq!(test, cursor.get_slice());
        }
    }

    #[test]
    fn rev_cursor_advances_exact() {
        let input = "This is Sparta my friend";
        let mut cursor = CursorRev::new(input, '.');
        let mut buf = String::new();
        let expected = input.split_inclusive(' ');
        for test in expected.into_iter().rev() {
            assert_eq!(&buf, cursor.get_slice());
            cursor.advance_exact(test);
            buf.insert_str(0, test);
        }
    }

    #[test]
    fn rev_cursor_peek_forward() {
        let input = "This. Is. Sentence. etc.";
        let mut cursor = CursorRev::new(input, '.');
        let expected = [" Is", " Sentence", " etc"];
        for test in expected.into_iter().rev() {
            cursor.advance();
            assert!(cursor.peek_forward(test), "{test}");
        }
    }

    #[test]
    fn rev_cursor_peek_back() {
        let input = "This. Is. Sentence. etc.";
        let mut cursor = CursorRev::new(input, '.');
        let expected = ["This", " Is", " Sentence", " etc"];
        assert!(cursor.peek_back("etc"));
        for test in expected.into_iter().rev() {
            assert!(cursor.peek_back(test));
            cursor.advance();
        }
        assert!(!cursor.peek_back("etc"));
    }

    #[test]
    fn snapping_works() {
        let input =
            "I have a sentence. It is not very long. Here is another. Long schlong ding dong.";
        let chunker = SnappingWindow {
            config: ChunkBaseConfig::new(1, 1),
            ..Default::default()
        };
        let expected = [
            "I have a sentence. It is not very long. Here is another.",
            " It is not very long. Here is another. Long schlong ding dong.",
        ];

        let chunks = chunker.chunk(input.trim()).unwrap();
        assert_eq!(2, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }

    #[test]
    fn snapping_skips_back() {
        let input =
            "I have a sentence. It contains letters, words, etc. and it contains more. The most important of which is foobar., because it must be skipped.";
        let chunker = SnappingWindow {
            config: ChunkBaseConfig::new(1, 1),
            skip_back: vec!["etc".to_string(), "foobar".to_string()],
            ..Default::default()
        };
        let expected = [input];

        let chunks = chunker.chunk(input.trim()).unwrap();
        assert_eq!(1, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }

    #[test]
    fn snapping_skips_forward() {
        let input =
            "Go to sentences.org for more words. 50% off on words with >4 syllables. Leverage agile frameworks to provide robust high level overview at agile.com.";

        let chunker = SnappingWindow {
            config: ChunkBaseConfig::new(1, 1),
            skip_forward: vec!["com".to_string(), "org".to_string()],
            ..Default::default()
        };

        let expected = [input];

        let chunks = chunker.chunk(input.trim()).unwrap();
        assert_eq!(1, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }

    #[test]
    fn snapping_skips_common_abbreviations() {
        let input =
            "Words are hard. There are many words in existence, e.g. this, that, etc..., quite a few, as you can see. My opinion, available at nobodycares.com, is that words should convey meaning. Not everyone agrees however, which is why they leverage agile frameworks to provide robust synopses for high level overviews. The lucidity of meaning is, in fact, obscured and ambiguous, therefore the interpretation, i.e. the conveying of units of meaning is less than optimal. Jebem ti boga.";

        let chunker = SnappingWindow {
            config: ChunkBaseConfig::new(1, 1),
            ..Default::default()
        };

        let expected = [
            "Words are hard. There are many words in existence, e.g. this, that, etc..., quite a few, as you can see. My opinion, available at nobodycares.com, is that words should convey meaning.",
            " There are many words in existence, e.g. this, that, etc..., quite a few, as you can see. My opinion, available at nobodycares.com, is that words should convey meaning. Not everyone agrees however, which is why they leverage agile frameworks to provide robust synopses for high level overviews.",
            " My opinion, available at nobodycares.com, is that words should convey meaning. Not everyone agrees however, which is why they leverage agile frameworks to provide robust synopses for high level overviews. The lucidity of meaning is, in fact, obscured and ambiguous, therefore the interpretation, i.e. the conveying of units of meaning is less than optimal.",
            " Not everyone agrees however, which is why they leverage agile frameworks to provide robust synopses for high level overviews. The lucidity of meaning is, in fact, obscured and ambiguous, therefore the interpretation, i.e. the conveying of units of meaning is less than optimal. Jebem ti boga.",
        ];

        let chunks = chunker.chunk(input.trim()).unwrap();
        assert_eq!(4, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }

    #[test]
    fn snapping_table_of_contents() {
        let input =
            "Table of contents:\n1 Super cool stuff\n1.1 Some chonkers in rust\n1.2 Some data for your LLM\n1.3 ??? \n1.4 Profit \n1.4.1 Lambo\nHope you liked the table of contents. See more at content.co.com.";

        let chunker = SnappingWindow {
            config: ChunkBaseConfig::new(1, 1),
            skip_forward: vec![
                "0".to_string(),
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "co".to_string(),
                "com".to_string(),
            ],
            skip_back: vec!["com".to_string()],
            ..Default::default()
        };

        let expected = [input];

        let chunks = chunker.chunk(input.trim()).unwrap();
        assert_eq!(1, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }
}
