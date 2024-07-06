use qdrant_client::client::{Payload, QdrantClient};
use qdrant_client::qdrant::{CreateCollection, Distance, VectorParams, VectorsConfig};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use std::future::Future;
use std::sync::Arc;
use tracing::info;

use fastembed::{EmbeddingModel, InitOptions, ModelInfo, TextEmbedding};

use crate::error::ChonkitError;

#[derive(Clone)]
pub struct VectorService {
    db: PgPool,
    vector_db: Arc<QdrantClient>,
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

impl VectorService {
    pub fn new(vector_db: QdrantClient, db: PgPool) -> Self {
        Self {
            db,
            vector_db: Arc::new(vector_db),
        }
    }

    /// List all available models in fastembed
    pub fn list_embedding_models() -> Vec<ModelInfo> {
        fastembed::TextEmbedding::list_supported_models()
    }

    pub async fn init(&self) -> Result<(), ChonkitError> {
        info!("Initialising vectorizer");

        let models = Self::list_embedding_models();

        info!("Creating collections");

        // Ensure a collection exists per embedding model
        for model in models {
            let collection_name = model.model_code.as_str().replace('/', "-");

            match self.vector_db.collection_info(&collection_name).await {
                Ok(_) => info!("Collection {} exists, skipping", model.model_code),
                Err(e) => {
                    self.vector_db
                        .create_collection(&CreateCollection {
                            collection_name,
                            vectors_config: Some(VectorsConfig {
                                config: Some(
                                    qdrant_client::qdrant::vectors_config::Config::Params(
                                        VectorParams {
                                            size: model.dim as u64,
                                            distance: Distance::Cosine.into(),
                                            ..Default::default()
                                        },
                                    ),
                                ),
                            }),
                            ..Default::default()
                        })
                        .await
                        .unwrap_or_else(|_| {
                            panic!("couldn't create collection {}", model.model_code)
                        });
                }
            }
        }

        Ok(())
    }

    pub async fn test_vectors(&self) {
        // With custom InitOptions
        let model = TextEmbedding::try_new(InitOptions {
            model_name: EmbeddingModel::AllMiniLML6V2,
            show_download_progress: true,
            ..Default::default()
        })
        .unwrap();

        let documents = vec![
            "passage: Hello, World!",
            "query: Hello, World!",
            "passage: This is an example passage.",
            // You can leave out the prefix but it's recommended
            "fastembed-rs is licensed under Apache  2.0",
            "hello world",
        ];

        let docs_2 = vec!["", "hello world"];

        // Generate embeddings with the default batch size, 256
        let embeddings = model.embed(documents, None).unwrap();

        let embeddings2 = model.embed(docs_2, None).unwrap();

        // println!("Embeddings test: {}", embeddings2[0].len());
        // println!("Embeddings test (empty): {:?}", embeddings2[0]);
        // println!("Embeddings test (hello world): {:?}", embeddings2[1]);

        let payload: Payload = json!(
            {
                "foo": "Bar",
                "bar": 12,
                "baz": {
                    "qux": "quux"
                }
            }
        )
        .try_into()
        .unwrap();
    }

    pub async fn embed(&self, content: Vec<impl AsRef<str>>) {
        todo!()
    }
}

impl VectorDatabase for QdrantClient {
    async fn list_collections(&self) {
        dbg!(self.list_collections().await.unwrap());
    }
}

#[derive(Debug, Serialize)]
pub struct DocumentPayload {
    pub content: String,
}

pub trait VectorDatabase {
    fn list_collections(&self) -> impl Future<Output = ()>;
}
