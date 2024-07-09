use chunk::ChunkService;
use document::DocumentService;
use vector::VectorService;

pub mod chunk;
pub mod document;
pub mod vector;

#[derive(Debug, Clone)]
pub struct ServiceState {
    pub document: DocumentService,
    pub vector: VectorService,
    pub chunk: ChunkService,
}

impl ServiceState {
    pub fn new(document: DocumentService, vector: VectorService) -> Self {
        Self {
            document,
            vector,
            chunk: ChunkService {},
        }
    }
}
