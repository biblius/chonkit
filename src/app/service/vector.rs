use crate::{
    app::{
        embedder::FastEmbedder, repo::pg::vector::PgVectorRepo,
        vector::store::qdrant::QdrantVectorStore,
    },
    core::service::vector::VectorService as Service,
};

pub(in crate::app) type VectorService = Service<PgVectorRepo, QdrantVectorStore, FastEmbedder>;

#[cfg(test)]
#[suitest::suite(integration_tests)]
mod vector_service_postgres_qdrant_fastembed {
    use super::VectorService;
    use crate::app::{
        embedder::FastEmbedder,
        repo::pg::vector::PgVectorRepo,
        test::{init_postgres, init_qdrant},
        vector::store::qdrant::QdrantVectorStore,
    };
    use suitest::before_all;

    #[before_all]
    async fn setup() -> (PgVectorRepo, QdrantVectorStore, VectorService) {
        let (postgres, _pg_img) = init_postgres().await;
        let (qdrant, _qd_img) = init_qdrant().await;

        let repo = PgVectorRepo::new(postgres.clone());
        let store = QdrantVectorStore::new(qdrant);
        let embedder = FastEmbedder;

        let service = VectorService::new(repo.clone(), store.clone(), embedder);

        (repo, store, service)
    }
}
