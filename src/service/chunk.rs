use serde::Deserialize;
use tracing::info;

use crate::{
    llm::chunk::{ChunkConfig, Chunker, ChunkerError, Recursive, SlidingWindow, SnappingWindow},
    model::document::File,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChunkInput {
    SlidingWindow {
        config: ChunkConfig,
    },
    SnappingWindow {
        config: ChunkConfig,
        skip_f: Vec<String>,
        skip_b: Vec<String>,
    },
    Recursive {
        config: ChunkConfig,
        delimiters: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct ChunkService {}

impl ChunkService {
    pub fn chunk<'content>(
        &self,
        config: ChunkInput,
        file: &File,
        content: &'content str,
    ) -> Result<Vec<&'content str>, ChunkerError> {
        match config {
            ChunkInput::SlidingWindow {
                config: ChunkConfig { size, overlap },
            } => {
                info!("Chunking {} with SlidingWindow", file.name);
                let chunker = SlidingWindow::new(size, overlap);
                let chunks = chunker.chunk(content)?.into_iter().collect::<Vec<_>>();
                Ok(chunks)
            }
            ChunkInput::SnappingWindow {
                config: ChunkConfig { size, overlap },
                skip_f,
                skip_b,
            } => {
                info!("Chunking {} with SnappingWindow", file.name);
                let skip_f = skip_f.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                let skip_b = skip_b.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                let chunker = SnappingWindow::new(size, overlap)
                    .skip_forward(&skip_f)
                    .skip_back(&skip_b);
                let chunks = chunker.chunk(content)?.into_iter().collect::<Vec<_>>();
                Ok(chunks)
            }
            ChunkInput::Recursive {
                config: ChunkConfig { size, overlap },
                delimiters,
            } => {
                let delims = delimiters.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                let chunker = Recursive::new(size, overlap, &delims);
                let chunks = chunker.chunk(content)?.into_iter().collect::<Vec<_>>();
                Ok(chunks)
            }
        }
    }
}
