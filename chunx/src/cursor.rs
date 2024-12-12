use std::{iter::Peekable, str::Chars};

/// Default patterns to skip in front of delimiters.
/// `___. some text`
pub(super) const DEFAULT_SKIP_F: &[&str] = &[
    "com", "org", "net", // Common URL patterns
    "g.", "e.", // Common acronyms (e.g., i.e.)
    "sh", "rs", "js", "json", // Common file extensions
];

/// Default patterns to skip behind delimiters.
/// `Some text.___` <
pub(super) const DEFAULT_SKIP_B: &[&str] = &[
    "www", // Common URL patterns
    "etc", "e.g", "i.e", // Common acronyms
];

#[derive(Debug)]
pub(super) struct Cursor<'a> {
    /// Input.
    pub buf: &'a str,

    /// Total bytes in buf.
    pub byte_count: usize,

    /// Indexes into buf, the current position of the cursor.
    /// Always gets advanced past the delimiter.
    pub byte_offset: usize,

    /// Delimiter to split by.
    pub delim: char,
    pub chars: Peekable<Chars<'a>>,
}

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str, delim: char) -> Self {
        Self {
            buf: input,
            chars: input.chars().peekable(),
            byte_count: byte_count(input),
            byte_offset: 0,
            delim,
        }
    }

    pub fn finished(&self) -> bool {
        self.byte_offset == self.byte_count - 1
    }

    pub fn get_slice(&self) -> &'a str {
        if self.buf.is_empty() || self.finished() {
            return self.buf;
        }
        &self.buf[..self.byte_offset]
    }

    /// Advance the byte_offset until `delim` is found. The byte_offset will be set
    /// to the index following the delim.
    pub fn advance(&mut self) {
        if self.buf.is_empty() || self.finished() {
            return;
        }

        loop {
            let Some(ch) = self.chars.next() else {
                break;
            };

            self.byte_offset += ch.len_utf8();

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

            while let Some(ch) = self.chars.peek().cloned() {
                if ch == self.delim {
                    self.chars.next();
                    self.byte_offset += ch.len_utf8();
                    stop = false;
                } else {
                    break;
                }
            }

            if stop {
                break;
            }
        }
    }

    pub fn advance_exact(&mut self, pat: &str) {
        for ch in pat.chars() {
            self.chars.next();
            self.byte_offset += ch.len_utf8();
        }
    }

    pub fn peek_back(&self, pat: &str) -> bool {
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

    pub fn peek_forward(&self, pat: &str) -> bool {
        if self.byte_offset + byte_count(pat) >= self.byte_count {
            return false;
        }
        let mut end = self.byte_offset + byte_count(pat);
        snap_front(&mut end, self.buf);
        &self.buf[self.byte_offset..end] == pat
    }

    pub fn advance_if_peek(&mut self, forward: &[String], back: &[String]) -> bool {
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
pub(super) struct CursorRev<'a> {
    /// The str being scanned.
    pub buf: &'a str,

    pub byte_count: usize,

    /// The current byte byte offset of the cursor in the str.
    /// Is kept on delimiter when advancing.
    pub byte_offset: usize,

    /// Total input UTF-8 chars
    pub char_count: usize,

    /// The current byte byte offset of the cursor in the str.
    pub char_offset: usize,

    /// The delimiter to snap to
    pub delim: char,
}

impl<'a> CursorRev<'a> {
    pub fn new(input: &'a str, delim: char) -> Self {
        Self {
            buf: input,
            byte_count: byte_count(input),
            byte_offset: input.len().saturating_sub(1),
            char_count: input.chars().count(),
            char_offset: input.chars().count(),
            delim,
        }
    }

    pub fn finished(&self) -> bool {
        self.byte_offset == 0
    }

    pub fn get_slice(&self) -> &'a str {
        if self.finished() {
            self.buf
        } else {
            let mut start = self.byte_offset + 1;
            snap_front(&mut start, self.buf);
            &self.buf[start..]
        }
    }

    pub fn advance(&mut self) {
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

    pub fn peek_back(&self, pat: &str) -> bool {
        // Skip if we are done.
        if self.finished() {
            return false;
        }
        let mut start = self.byte_offset.saturating_sub(byte_count(pat));
        snap_back(&mut start, self.buf);
        &self.buf[start..self.byte_offset] == pat
    }

    pub fn peek_forward(&self, pat: &str) -> bool {
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

    pub fn advance_if_peek(&mut self, forward: &[String], back: &[String]) -> bool {
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

    pub fn advance_exact(&mut self, pat: &str) {
        let amt = byte_count(pat);
        self.char_offset -= pat.chars().count();
        self.byte_offset = self.byte_offset.saturating_sub(amt);
    }
}

/// Count the number of bytes in the input.
/// Assumes that the input is valid UTF-8
#[inline(always)]
pub(super) fn byte_count(input: &str) -> usize {
    input.chars().fold(0, |acc, el| acc + el.len_utf8())
}

/// Snap the index to the first char boundary to the right.
#[inline(always)]
pub(super) fn snap_front(i: &mut usize, input: &str) {
    while !input.is_char_boundary(*i) && *i < input.len() {
        *i += 1;
    }
}

/// Snap the index to the first char boundary to the left.
#[inline(always)]
pub(super) fn snap_back(i: &mut usize, input: &str) {
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
            dbg!(cursor.get_slice(), cursor.byte_offset);
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
}
