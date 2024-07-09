use crate::error::ChonkitError;
use fastembed::{EmbeddingModel, InitOptions, ModelInfo, TextEmbedding};
use qdrant_client::client::Payload;
use qdrant_client::qdrant::with_payload_selector::SelectorOptions;
use qdrant_client::qdrant::{
    CreateCollection, Distance, PointStruct, SearchParams, SearchPoints, UpsertPointsBuilder,
    VectorParams, VectorsConfig, WithPayloadSelector,
};
use qdrant_client::Qdrant;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct VectorService {
    db: PgPool,
    vector_db: Arc<Qdrant>,
}

impl VectorService {
    pub fn new(vector_db: Qdrant, db: PgPool) -> Self {
        Self {
            db,
            vector_db: Arc::new(vector_db),
        }
    }

    pub async fn list_collections(&self) -> Result<Vec<String>, ChonkitError> {
        Ok(self
            .vector_db
            .list_collections()
            .await?
            .collections
            .into_iter()
            .map(|col| col.name)
            .collect())
    }

    /// List all available models in fastembed
    pub fn list_embedding_models(&self) -> Vec<ModelInfo> {
        fastembed::TextEmbedding::list_supported_models()
    }

    pub fn model_for_str(&self, s: &str) -> Option<ModelInfo> {
        self.list_embedding_models()
            .into_iter()
            .find(|model| model.model_code == s)
    }

    /// Create a collection in the vector DB.
    ///
    /// * `model`: Will be used to determine the collection dimensions.
    pub async fn create_collection(
        &self,
        collection_name: &str,
        model: EmbeddingModel,
    ) -> Result<(), ChonkitError> {
        info!("Creating collection '{collection_name}' with embedding model '{model}'");

        let model = TextEmbedding::get_model_info(&model);

        self.vector_db
            .create_collection(CreateCollection {
                collection_name: collection_name.to_string(),
                vectors_config: Some(VectorsConfig {
                    config: Some(qdrant_client::qdrant::vectors_config::Config::Params(
                        VectorParams {
                            size: model.dim as u64,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        },
                    )),
                }),
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    pub async fn embed(&self, content: Vec<&str>, model: EmbeddingModel, collection_name: &str) {
        let embedder = TextEmbedding::try_new(InitOptions {
            model_name: model.clone(),
            show_download_progress: true,
            ..Default::default()
        })
        .unwrap();

        let content_len = content.len();
        // Generate embeddings with the default batch size, 256
        let embeddings = embedder.embed(content.clone(), None).unwrap();

        debug_assert_eq!(embeddings.len(), content_len);

        let points: Vec<PointStruct> = embeddings
            .into_iter()
            .zip(content.iter())
            .map(|(embedding, content)| {
                let mut payload = Payload::new();
                payload.insert("content", content.to_string());
                PointStruct::new(uuid::Uuid::new_v4().to_string(), embedding, payload)
            })
            .collect();

        self.vector_db
            .upsert_points(UpsertPointsBuilder::new(collection_name, points).wait(true))
            .await
            .unwrap();
    }

    pub async fn search(
        &self,
        model: EmbeddingModel,
        query: &str,
        collection_name: String,
    ) -> Vec<String> {
        let embedder = TextEmbedding::try_new(InitOptions {
            model_name: model,
            ..Default::default()
        })
        .unwrap();

        let embeddings = embedder.embed(vec![query], None).unwrap();

        debug_assert!(!embeddings.is_empty());

        let search_points = SearchPoints {
            collection_name,
            vector: embeddings[0].clone(),
            filter: None,
            limit: 5,
            with_payload: Some(WithPayloadSelector {
                selector_options: Some(SelectorOptions::Enable(true)),
            }),
            params: Some(SearchParams::default()),
            ..Default::default()
        };

        let search_result = self.vector_db.search_points(search_points).await.unwrap();

        let result = search_result
            .result
            .into_iter()
            .map(|point| point.payload["content"].to_string());

        dbg!(&result);

        result.collect()
    }
}

impl std::fmt::Debug for VectorService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VectorService {{ db: {:?}, vector_db: {{ ... }} }}",
            self.db,
        )
    }
}
