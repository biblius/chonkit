use super::{
    document::store::FsDocumentStore, embedder::FastEmbedder, repo::pg::vector::PgVectorRepo,
    vector::store::qdrant::QdrantVectorStore,
};
use crate::{
    app::repo::pg::document::PgDocumentRepo,
    core::service::{document::DocumentService, vector::VectorService},
};
use qdrant_client::Qdrant;
use sqlx::PgPool;

type Document = DocumentService<PgDocumentRepo, FsDocumentStore>;
type Vector = VectorService<PgVectorRepo, QdrantVectorStore, FastEmbedder>;

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
        let store_document = FsDocumentStore::new("test_docs");

        let repo_document = PgDocumentRepo::new(pool.clone());
        let repo_vector = PgVectorRepo::new(pool.clone());

        let service_doc = Document::new(repo_document, store_document);
        let service_vec = Vector::new(repo_vector, store_vector, embedder);

        service_vec.create_default_collection().await;

        Self::new(service_doc, service_vec)
    }
}
