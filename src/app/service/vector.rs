use crate::{
    app::{embedder::FastEmbedder, vector::store::qdrant::QdrantVectorStore},
    core::service::vector::VectorService as Service,
};
use sqlx::PgPool;

pub(in crate::app) type VectorService = Service<PgPool, QdrantVectorStore, FastEmbedder>;

#[cfg(test)]
#[suitest::suite(integration_tests)]
#[suitest::suite_cfg(verbose = true)]
mod vector_service_postgres_qdrant_fastembed {
    use super::VectorService;
    use crate::app::{
        embedder::FastEmbedder,
        test::{init_postgres, init_qdrant, PostgresContainer},
        vector::store::qdrant::QdrantVectorStore,
    };
    use sqlx::PgPool;
    use suitest::before_all;

    #[before_all]
    async fn setup() -> (PgPool, QdrantVectorStore, VectorService, PostgresContainer) {
        let (postgres, pg) = init_postgres().await;
        let (qdrant, _qd_img) = init_qdrant().await;

        let store = QdrantVectorStore::new(qdrant);
        let embedder = FastEmbedder;

        let service = VectorService::new(postgres.clone(), store.clone(), embedder);

        (postgres, store, service, pg)
    }
}
