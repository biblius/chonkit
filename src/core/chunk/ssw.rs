use super::{concat, ChunkBaseConfig, Chunker, ChunkerError};
use tracing::trace;

/// Heuristic chunker for texts intended for humans, e.g. documentation, books, blogs, etc.
///
/// Basically a sliding window which is aware of sentence stops, currently the only stop
/// implemented is the '.' character.
///
/// It will attempt to chunk the content according to `size`. Keep in mind it cannot
/// be exact and the chunks will probably be larger, because of the way it searches
/// for delimiters.
///
/// The chunker can also be configured to skip common patterns including the fullstop
/// such as abbreviations (e.g., i.e., etc.) and urls.
///
/// Keep in mind the configuration for this chunker is different; The `size` will
/// represent the amount of sentences in the chunk and the `overlap` will represent
/// how many back/forward sentences will be included.
#[derive(Debug)]
pub struct SnappingWindow<'skip> {
    /// The config here is semantically different. `size` will represent the
    /// amount of bytes in the base chunk, while `overlap` will represent the amount
    /// of trailing/leading sentences.
    config: ChunkBaseConfig,

    /// The delimiter to use to split sentences. At time of writing the most common one is ".".
    delimiter: char,

    /// Whenever a delimiter is found, the chunker will look ahead for these sequences
    /// and will skip the delimiter if found, basically treating it as a regular char.
    ///
    /// Useful for common abbreviations and urls.
    skip_forward: &'skip [&'skip str],

    /// Whenever a delimiter is found, the chunker will look back for these sequences
    /// and will skip the delimiter if found, basically treating it as a regular char.
    ///
    /// Useful for common abbreviations and urls.
    skip_back: &'skip [&'skip str],
}

impl<'skip> SnappingWindow<'skip> {
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

    pub fn skip_forward(mut self, skip_forward: &'skip [&'skip str]) -> Self {
        self.skip_forward = skip_forward;
        self
    }

    pub fn skip_back(mut self, skip_back: &'skip [&'skip str]) -> Self {
        self.skip_back = skip_back;
        self
    }
}

impl<'skip> Chunker for SnappingWindow<'skip> {
    fn chunk<'a>(&self, input: &'a str) -> Result<Vec<&'a str>, ChunkerError> {
        let Self {
            config: ChunkBaseConfig { size, overlap },
            delimiter: delim,
            skip_forward,
            skip_back,
        } = self;

        let mut chunks = vec![];

        let mut cursor = Cursor::new(input, *delim);

        let mut start = 1;

        snap_front(&mut start, input);

        let mut chunk = &input[..start];

        loop {
            if start >= input.len() {
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

            if chunk.len() < *size {
                continue;
            }

            let prev = &input[..cursor.byte_offset - byte_count(chunk)];
            let next = &input[cursor.byte_offset..];

            let mut p_cursor = CursorRev::new(prev, *delim);
            let mut n_cursor = Cursor::new(next, *delim);

            for _ in 0..*overlap {
                p_cursor.advance();
                while p_cursor.advance_if_peek(skip_forward, skip_back) {
                    p_cursor.advance();
                }

                n_cursor.advance();
                while n_cursor.advance_if_peek(skip_forward, skip_back) {
                    n_cursor.advance();
                }
            }

            let prev = p_cursor.get_slice();
            let next = n_cursor.get_slice();

            let chunk_full = concat(concat(prev, chunk)?, next)?;

            // Skip the first chunk since its contents will be in the following one.
            if !prev.is_empty() {
                chunks.push(chunk_full);
                trace!("Added chunk, total: {}", chunks.len());
            }

            start += 1;

            snap_front(&mut start, input);

            if start + n_cursor.byte_offset >= input.len() {
                // Handles case where the full text is chunked
                // and there is no previous chunk
                if chunks.is_empty() {
                    chunks.push(chunk_full);
                    trace!("Added chunk, total: {}", chunks.len());
                }
                break;
            }

            let mut chunk_start = start - 1;

            snap_front(&mut chunk_start, input);

            chunk = &input[chunk_start..start];
        }

        Ok(chunks)
    }
}

impl Default for SnappingWindow<'_> {
    fn default() -> Self {
        Self {
            config: ChunkBaseConfig::new(500, 10),
            delimiter: '.',
            // Common urls, abbreviations, file extensions
            skip_forward: &["com", "org", "net", "g.", "e.", "sh", "rs", "js", "json"],
            skip_back: &["www", "etc", "e.g", "i.e"],
        }
    }
}

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

    fn get_slice(&self) -> &'a str {
        if self.buf.is_empty() || self.byte_offset == self.byte_count - 1 {
            return self.buf;
        }
        &self.buf[..self.byte_offset]
    }

    /// Advance the byte_offset until `delim` is found. The byte_offset will be set
    /// to the index following the delim.
    fn advance(&mut self) {
        if self.buf.is_empty() || self.byte_offset == self.byte_count - 1 {
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
            self.char_offset += self.buf.chars().count();
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
        if self.byte_offset == self.byte_count - 1 {
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

    fn advance_if_peek(&mut self, forward: &[&str], back: &[&str]) -> bool {
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

    fn get_slice(&self) -> &'a str {
        if self.byte_offset == 0 {
            self.buf
        } else {
            let mut start = self.byte_offset + 1;
            snap_back(&mut start, self.buf);
            &self.buf[start..]
        }
    }

    fn advance(&mut self) {
        if self.byte_offset == 0 {
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

            if self.byte_offset == 0 {
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
        if self.byte_offset == 0 {
            return false;
        }
        let mut start = self.byte_offset.saturating_sub(byte_count(pat));
        snap_back(&mut start, self.buf);
        &self.buf[start..self.byte_offset] == pat
    }

    fn peek_forward(&self, pat: &str) -> bool {
        let pat_offset = byte_count(pat);

        // Skip if we are done or at the start.
        if self.byte_offset == 0 || self.byte_offset + pat_offset >= self.byte_count {
            return false;
        }

        let mut start = self.byte_offset + 1;
        let mut end = self.byte_offset + 1 + pat_offset;

        snap_front(&mut start, self.buf);
        snap_front(&mut end, self.buf);

        &self.buf[start..end] == pat
    }

    fn advance_if_peek(&mut self, forward: &[&str], back: &[&str]) -> bool {
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
        let skip_f = [String::from("foo"), String::from("bar")];
        let skip_f: Vec<_> = skip_f.iter().map(|s| s.as_str()).collect();

        let skip_b = [String::from("foo"), String::from("bar")];
        let skip_b: Vec<_> = skip_b.iter().map(|s| s.as_str()).collect();
        let size = 1;
        let overlap = 1;
        let delimiter = '!';

        let chunker = SnappingWindow::new(size, overlap)
            .delimiter(delimiter)
            .skip_forward(&skip_f)
            .skip_back(&skip_b);

        assert_eq!(delimiter, chunker.delimiter);
        assert_eq!(size, chunker.config.size);
        assert_eq!(overlap, chunker.config.overlap);
        assert_eq!(&skip_f, chunker.skip_forward);
        assert_eq!(&skip_b, chunker.skip_back);
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
    fn ssw_works() {
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
    fn ssw_skips_back() {
        let input =
            "I have a sentence. It contains letters, words, etc. and it contains more. The most important of which is foobar., because it must be skipped.";
        let chunker = SnappingWindow {
            config: ChunkBaseConfig::new(1, 1),
            skip_back: &["etc", "foobar"],
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
    fn ssw_skips_forward() {
        let input =
            "Go to sentences.org for more words. 50% off on words with >4 syllables. Leverage agile frameworks to provide robust high level overview at agile.com.";

        let chunker = SnappingWindow {
            config: ChunkBaseConfig::new(1, 1),
            skip_forward: &["com", "org"],
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
    fn ssw_skips_common_abbreviations() {
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
    fn ssw_table_of_contents() {
        let input =
            "Table of contents:\n1 Super cool stuff\n1.1 Some chonkers in rust\n1.2 Some data for your LLM\n1.3 ??? \n1.4 Profit \n1.4.1 Lambo\nHope you liked the table of contents. See more at content.co.com.";

        let chunker = SnappingWindow {
            config: ChunkBaseConfig::new(1, 1),
            skip_forward: &["0", "1", "2", "3", "4", "co", "com"],
            skip_back: &["com"],
            ..Default::default()
        };

        let expected = [input];

        let chunks = chunker.chunk(input.trim()).unwrap();
        dbg!(&chunks);
        assert_eq!(1, chunks.len());

        for (chunk, test) in chunks.into_iter().zip(expected.into_iter()) {
            assert_eq!(test, chunk);
        }
    }
}
