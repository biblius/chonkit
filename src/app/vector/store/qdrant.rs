use crate::core::repo::vector::VectorRepo;
use crate::core::vector::store::VectorStore;
use crate::error::ChonkitError;
use crate::{DEFAULT_COLLECTION_NAME, DEFAULT_COLLECTION_SIZE};
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::with_payload_selector::SelectorOptions;
use qdrant_client::qdrant::{
    CreateCollection, Distance, PointStruct, SearchParams, SearchPoints, UpsertPointsBuilder,
    VectorParams, VectorsConfig, WithPayloadSelector,
};
use qdrant_client::{Payload, Qdrant, QdrantError};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Basic Arc wrapper around Qdrant.
#[derive(Clone)]
pub struct QdrantVectorStore {
    q: Arc<Qdrant>,
}

impl QdrantVectorStore {
    pub fn new(q: Qdrant) -> Self {
        Self { q: Arc::new(q) }
    }
}

impl VectorStore for QdrantVectorStore {
    async fn list_collections(&self) -> Result<Vec<String>, ChonkitError> {
        Ok(self
            .q
            .list_collections()
            .await?
            .collections
            .into_iter()
            .map(|col| col.name)
            .collect())
    }

    async fn create_collection(&self, name: &str, size: u64) -> Result<(), ChonkitError> {
        let config = VectorsConfig {
            config: Some(Config::Params(VectorParams {
                size,
                distance: Distance::Cosine.into(),
                ..Default::default()
            })),
        };

        let res = self
            .q
            .create_collection(CreateCollection {
                collection_name: name.to_string(),
                vectors_config: Some(config),
                ..Default::default()
            })
            .await?;

        debug_assert!(res.result);

        Ok(())
    }

    async fn delete_collection(&self, name: &str) -> Result<(), ChonkitError> {
        self.q.delete_collection(name).await?;
        Ok(())
    }

    async fn create_default_collection(&self) {
        let result = self
            .create_collection(DEFAULT_COLLECTION_NAME, DEFAULT_COLLECTION_SIZE as u64)
            .await;

        match result {
            Ok(_) => info!("Created default collection '{DEFAULT_COLLECTION_NAME}'."),
            Err(ChonkitError::Qdrant(QdrantError::ResponseError { status }))
                if matches!(status.code(), tonic::Code::AlreadyExists) =>
            {
                info!("Default collection '{DEFAULT_COLLECTION_NAME}' already exists.");
            }
            Err(e) => panic!("{e}"),
        }
    }

    async fn query(
        &self,
        search: Vec<f32>,
        collection: &str,
        limit: u64,
    ) -> Result<Vec<String>, ChonkitError> {
        let search_points = SearchPoints {
            collection_name: collection.to_string(),
            vector: search,
            filter: None,
            limit,
            with_payload: Some(WithPayloadSelector {
                selector_options: Some(SelectorOptions::Enable(true)),
            }),
            params: Some(SearchParams::default()),
            ..Default::default()
        };

        let search_result = self.q.search_points(search_points).await.unwrap();

        let result = search_result
            .result
            .into_iter()
            .map(|point| point.payload["content"].to_string());

        Ok(result.collect())
    }

    async fn store(
        &self,
        content: Vec<String>,
        vectors: Vec<Vec<f32>>,
        collection: &str,
    ) -> Result<(), ChonkitError> {
        debug!("Storing vectors to {collection}");

        debug_assert_eq!(
            content.len(),
            vectors.len(),
            "Content length is different from embeddings!"
        );

        let points: Vec<PointStruct> = vectors
            .into_iter()
            .zip(content.iter())
            .map(|(embedding, content)| {
                let mut payload = Payload::new();
                payload.insert("content", content.to_string());
                PointStruct::new(uuid::Uuid::new_v4().to_string(), embedding, payload)
            })
            .collect();

        self.q
            .upsert_points(UpsertPointsBuilder::new(collection, points).wait(true))
            .await
            .unwrap();

        Ok(())
    }

    async fn sync(&self, repo: &(impl VectorRepo + Sync)) -> Result<(), ChonkitError> {
        let collection_list = self.list_collections().await?;
        let mut collections = vec![];
        for collection in collection_list {
            let info = self.q.collection_info(&collection).await?;
            let info = info
                .result
                .unwrap()
                .config
                .unwrap()
                .params
                .unwrap()
                .vectors_config
                .unwrap()
                .config
                .unwrap();

            match info {
                Config::Params(VectorParams { size, .. }) => collections.push((collection, size)),
                Config::ParamsMap(pm) => {
                    warn!("Found unexpected params map! {pm:?}");
                }
            }
        }

        dbg!(collections);
        Ok(())
    }
}

impl std::fmt::Debug for QdrantVectorStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QdrantVectorStore {{ q: {{ .. }} }}")
    }
}
