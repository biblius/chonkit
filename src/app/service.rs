use document::DocumentService;
use vector::VectorService;

use super::{document::store::FsDocumentStore, embedder::FastEmbedder, repo::pg};

pub mod document;
pub mod vector;

#[derive(Clone)]
pub struct ServiceState {
    pub document: DocumentService,
    pub vector: VectorService,
}

impl ServiceState {
    pub fn new(document: DocumentService, vector: VectorService) -> Self {
        Self { document, vector }
    }

    pub async fn init(repo_url: &str, vector_db_url: &str, upload_path: &str) -> Self {
        let repo = pg::init(repo_url).await;

        #[cfg(feature = "qdrant")]
        let vec_db = crate::app::vector::qdrant::init(vector_db_url);

        #[cfg(feature = "weaviate")]
        let vec_db = crate::app::vector::weaviate::init(vector_db_url);

        let embedder = FastEmbedder;

        let store_document = FsDocumentStore::new(upload_path);

        let service_doc = DocumentService::new(repo.clone(), store_document);
        let service_vec = VectorService::new(repo, vec_db, embedder);

        service_vec.create_default_collection().await;
        service_doc.sync().await.expect("error in sync");

        Self::new(service_doc, service_vec)
    }
}
impl std::fmt::Debug for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceState")
            .field("document", &self.document)
            .field("vector {{ .. }}", &"")
            .finish()
    }
}
