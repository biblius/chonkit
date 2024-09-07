use crate::core::model::collection::VectorCollection;
use crate::core::vector::store::VectorStore;
use crate::error::ChonkitError;
use crate::{DEFAULT_COLLECTION_NAME, DEFAULT_COLLECTION_SIZE};
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::with_payload_selector::SelectorOptions;
use qdrant_client::qdrant::{
    CreateCollection, Distance, GetCollectionInfoResponse, PointStruct, SearchParams, SearchPoints,
    UpsertPointsBuilder, VectorParams, VectorsConfig, WithPayloadSelector,
};
use qdrant_client::{Payload, Qdrant, QdrantError};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub fn init(url: &str) -> Qdrant {
    info!("Connecting to qdrant at {url}");
    Qdrant::from_url(url)
        .build()
        .expect("error initialising qdrant")
}

/// Basic Arc wrapper around Qdrant.
#[derive(Clone)]
pub struct QdrantVectorStore {
    q: Arc<Qdrant>,
}

impl VectorStore for QdrantVectorStore {
    fn id(&self) -> &'static str {
        "qdrant"
    }

    async fn list_collections(&self) -> Result<Vec<VectorCollection>, ChonkitError> {
        let collection_names = self
            .q
            .list_collections()
            .await?
            .collections
            .into_iter()
            .map(|col| col.name)
            .collect::<Vec<_>>();

        let mut collections = vec![];

        for name in collection_names {
            let info = self.q.collection_info(&name).await?;
            let size = self.get_collection_size(&info);
            if let Some(size) = size {
                collections.push(VectorCollection::new(name, size));
            }
        }

        Ok(collections)
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

    async fn get_collection(&self, name: &str) -> Result<VectorCollection, ChonkitError> {
        let info = self.q.collection_info(name).await?;
        let Some(size) = self.get_collection_size(&info) else {
            #[cfg(debug_assertions)]
            {
                debug!("{info:?}")
            }
            return Err(ChonkitError::DoesNotExist(format!(
                "Size information for vector collection '{name}'"
            )));
        };
        Ok(VectorCollection::new(name.to_string(), size))
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
}

impl QdrantVectorStore {
    pub fn new(q: Qdrant) -> Self {
        Self { q: Arc::new(q) }
    }

    fn get_collection_size(&self, info: &GetCollectionInfoResponse) -> Option<usize> {
        let config = info
            .result
            .as_ref()?
            .config
            .as_ref()?
            .params
            .as_ref()?
            .vectors_config
            .as_ref()?
            .config
            .as_ref()?;
        match config {
            Config::Params(VectorParams { size, .. }) => Some(*size as usize),
            Config::ParamsMap(pm) => {
                warn!("Found unexpected params map! {pm:?}");
                None
            }
        }
    }
}

impl std::fmt::Debug for QdrantVectorStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QdrantVectorStore {{ q: {{ .. }} }}")
    }
}
