use super::{
    document::store::FsDocumentStore, embedder::FastEmbedder, repo::pg::vector::PgVectorRepo,
    vector::store::qdrant::QdrantVectorStore,
};
use crate::app::repo::pg::document::PgDocumentRepo;
use document::DocumentService;
use qdrant_client::Qdrant;
use sqlx::PgPool;
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

    pub async fn init(pool: PgPool, qdrant: Qdrant, upload_path: &str) -> Self {
        let embedder = FastEmbedder;

        let store_vector = QdrantVectorStore::new(qdrant);
        let store_document = FsDocumentStore::new(upload_path);

        let repo_document = PgDocumentRepo::new(pool.clone());
        let repo_vector = PgVectorRepo::new(pool.clone());

        let service_doc = DocumentService::new(repo_document, store_document);
        let service_vec = VectorService::new(repo_vector, store_vector, embedder);

        service_vec.create_default_collection().await;

        Self::new(service_doc, service_vec)
    }
}
