use super::{ChunkBaseConfig, Chunker, ChunkerError};

/// Default delimiters for the [recursive chunker][Recursive].
const DEFAULT_DELIMS: &[&str] = &["\n\n", "\n", " ", ""];

/// Default delimiters for the [recursive chunker][Recursive] when constructed with
/// [Recursive::markdown].
const MARKDOWN_DELIMS: &[&str] = &[
    "#", "##", "###", "####", "#####", "######", "\n```", "\n---\n", "\n___\n", "\n\n", "\n", " ",
    "",
];

/// A chunker based on langchain's
/// [RecursiveCharacterSplitter](https://dev.to/eteimz/understanding-langchains-recursivecharactertextsplitter-2846).
///
/// Given a default size and set of delimiters, recursively splits the input using the delimiters.
///
/// The default `delims` are : `["\n\n", "\n", " ", ""]`.
///
/// The input is first split into chunks with the first delimiter. For each chunk larger than `size`, split
/// it with the next delimiter in the chain until small enough chunks can be assembled.
#[derive(Debug)]
pub struct Recursive<'a> {
    config: ChunkBaseConfig,
    /// The delimiters to use when splitting.
    pub delims: &'a [&'a str],
}

impl<'delim> Recursive<'delim> {
    pub fn new(size: usize, overlap: usize, delimiters: &'delim [&str]) -> Self {
        Recursive {
            config: ChunkBaseConfig::new(size, overlap),
            delims: delimiters,
        }
    }

    pub fn size(mut self, size: usize) -> Self {
        self.config.size = size;
        self
    }

    pub fn overlap(mut self, overlap: usize) -> Self {
        self.config.overlap = overlap;
        self
    }

    pub fn delimiters(mut self, delimiters: &'delim [&str]) -> Self {
        self.delims = delimiters;
        self
    }

    pub fn markdown() -> Self {
        Self {
            config: ChunkBaseConfig::default(),
            delims: MARKDOWN_DELIMS,
        }
    }

    /// TODO: Overlap
    ///
    /// Chunk the input using this instance's delimiters.
    ///
    /// `input` - The input text to chunk
    ///
    /// `idx` - The current delimiter index.
    ///
    /// `buffer` - A slice in which the current split contents are stored if they are smaller than
    /// this instance's `size'. Must live at least as long as the input.
    ///
    /// `chunks` - Where the final chunks are stored.
    ///
    /// The function initially splits `input` with `delims[idx]`. For each split larger than `size`,
    /// another round of splitting is performed using the next delimiter in `delims`.
    /// In each round, the buffer contents are populated until the next chunk
    /// would cause it to be larger than `size`. When this happens, the current buffer is pushed
    /// into `chunks`.
    ///
    /// Since the buffer is shared between rounds, chunks from the next round's
    /// split will be included in it, maximising the amount of content in the chunk.
    ///
    /// If the chunk is of greater size than allowed and no more delimiters are left,
    /// the chunk will be skipped.
    fn chunk_recursive<'input>(
        &self,
        input: &'input str,
        idx: usize,
        mut buffer: &'input str,
        chunks: &mut Vec<&'input str>,
    ) -> Result<Option<&'input str>, ChunkerError> {
        let Recursive {
            config: ChunkBaseConfig { size, .. },
            delims,
            ..
        } = self;

        if idx >= delims.len() {
            return Ok(None);
        }

        let split: std::str::SplitInclusive<'input, &str> = input.split_inclusive(delims[idx]);

        for chunk in split {
            if buffer.len() + chunk.len() <= *size {
                // Buffer is shared through invocations so use it if not empty
                let buf = if buffer.is_empty() {
                    chunk.as_ptr()
                } else {
                    buffer.as_ptr()
                };

                let buf = std::ptr::slice_from_raw_parts(buf, buffer.len() + chunk.len());

                // SAFETY: We know we are always pointing to something of lifetime 'input
                // and that it lives through each invocation. We are always incrementing
                // the pointer by the chunk length so we are never out of bounds.
                unsafe {
                    buffer = std::str::from_utf8(&*buf)?;
                }

                continue;
            }

            // Can't store current chunk with existing buf
            // If the buf is not empty, add it to the chunks and reset buffer
            if !buffer.is_empty() {
                chunks.push(buffer);

                // Check again and reset loop if it fits, setting the current buffer
                // to the chunk
                if chunk.len() <= *size {
                    buffer = chunk;
                    continue;
                }

                // Otherwise just reset the buffer and do another round
                buffer = "";
            }

            if let Some(buf) = self.chunk_recursive(chunk, idx + 1, buffer, chunks)? {
                buffer = buf;
            }
        }

        // If there's still something at the end of the fn return it
        if !buffer.is_empty() {
            return Ok(Some(buffer));
        }

        Ok(None)
    }
}

impl<'delim> Chunker for Recursive<'delim> {
    fn chunk<'input>(&self, input: &'input str) -> Result<Vec<&'input str>, ChunkerError> {
        let mut splits = vec![];

        if let Some(remainder) = self.chunk_recursive(input, 0, "", &mut splits)? {
            splits.push(remainder);
        }

        println!(
            "Chunked {} chunks, avg chunk size: {}",
            splits.len(),
            if splits.is_empty() {
                0
            } else {
                splits.iter().fold(0, |acc, el| acc + el.len()) / splits.len()
            }
        );

        Ok(splits
            .into_iter()
            .filter(|chunk| !chunk.trim().is_empty())
            .collect())
    }
}

impl Default for Recursive<'_> {
    fn default() -> Self {
        Self {
            config: ChunkBaseConfig::default(),
            delims: DEFAULT_DELIMS,
        }
    }
}

#[cfg(test)]
mod tests {
    const INPUT: &str = r#"
What I Worked On

February 2021

Before college the two main things I worked on, outside of school, were writing and programming. I didn't write essays. I wrote what beginning writers were supposed to write then, and probably still are: short stories. My stories were awful. They had hardly any plot... just characters with strong feelings, which I imagined made them deep.

The first programs I tried writing were on the IBM 1401 that our school district used for what was then called "data processing." This was in 9th grade, so I was 13 or 14. The school district's 1401 happened to be in the basement of our junior high school, and my friend Rich Draves and I got permission to use it. It was like a mini Bond villain's lair down there, with all these alien-looking machines — CPU, disk drives, printer, card reader — sitting up on a raised floor under bright fluorescent lights.
"#;
    use super::*;

    #[test]
    fn recursive_works() {
        let chunker = Recursive::new(100, 50, DEFAULT_DELIMS);
        let mut chunks = vec![];

        chunker
            .chunk_recursive(INPUT.trim(), 0, "", &mut chunks)
            .unwrap();

        for chunk in chunks {
            assert!(chunk.len() <= 100);
        }
    }

    #[test]
    fn recursive_small_input_custom_delims() {
        let input = "Supercalifragilisticexpialadocius";
        let chunker = Recursive::new(5, 0, &["foo"]);
        let chunks = chunker.chunk(input).unwrap();
        assert!(chunks.is_empty());
    }
}
