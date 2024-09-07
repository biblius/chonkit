use super::{
    document::store::FsDocumentStore, embedder::FastEmbedder,
    vector::store::qdrant::QdrantVectorStore,
};
use qdrant_client::Qdrant;
use sqlx::PgPool;

use document::DocumentService;
use vector::VectorService;

pub mod document;
pub mod vector;

#[derive(Debug, Clone)]
pub struct ServiceState {
    pub document: DocumentService,
    pub vector: VectorService,
}

impl ServiceState {
    pub fn new(document: DocumentService, vector: VectorService) -> Self {
        Self { document, vector }
    }

    pub async fn init(postgres: PgPool, qdrant: Qdrant, upload_path: &str) -> Self {
        let embedder = FastEmbedder;

        let store_vector = QdrantVectorStore::new(qdrant);
        let store_document = FsDocumentStore::new(upload_path);

        let service_doc = DocumentService::new(postgres.clone(), store_document);
        let service_vec = VectorService::new(postgres, store_vector, embedder);

        service_vec.create_default_collection().await;
        service_doc.sync().await.expect("error in sync");

        Self::new(service_doc, service_vec)
    }
}
