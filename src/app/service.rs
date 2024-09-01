use super::{document::store::FsDocumentStore, embedder::FastEmbedder};
use crate::{
    app::repo::pg::document::PgDocumentRepo,
    core::{
        service::{document::DocumentService, vector::VectorService},
        vector::QdrantVectorStore,
    },
};
use qdrant_client::Qdrant;
use sqlx::PgPool;

type Document = DocumentService<PgDocumentRepo, FsDocumentStore>;
type Vector = VectorService<PgDocumentRepo, QdrantVectorStore, FastEmbedder>;

#[derive(Debug, Clone)]
pub struct ServiceState {
    pub document: Document,
    pub vector: Vector,
}

impl ServiceState {
    pub fn new(document: Document, vector: Vector) -> Self {
        Self { document, vector }
    }

    pub async fn init(pool: PgPool, qdrant: Qdrant) -> Self {
        let embedder = FastEmbedder;
        let store_vector = QdrantVectorStore::new(qdrant);

        let repo_document = PgDocumentRepo::new(pool.clone());

        let service_doc =
            DocumentService::new(repo_document.clone(), FsDocumentStore::new("test_docs"));
        let service_vec = VectorService::new(repo_document, store_vector, embedder);

        service_vec.create_default_collection().await;

        Self::new(service_doc, service_vec)
    }
}
