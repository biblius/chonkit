use super::{DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_SIZE};
use crate::{core::embedder::Embedder, error::ChonkitError, map_err};

pub use chonkit_embedders::fastembed::remote::RemoteFastEmbedder;

#[async_trait::async_trait]
impl Embedder for RemoteFastEmbedder {
    fn id(&self) -> &'static str {
        "fembed"
    }

    fn default_model(&self) -> (String, usize) {
        (
            String::from(DEFAULT_COLLECTION_MODEL),
            DEFAULT_COLLECTION_SIZE,
        )
    }

    async fn list_embedding_models(&self) -> Result<Vec<(String, usize)>, ChonkitError> {
        Ok(map_err!(self.list_models().await))
    }

    async fn embed(&self, content: &[&str], model: &str) -> Result<Vec<Vec<f64>>, ChonkitError> {
        Ok(map_err!(self.embed(content, model).await))
    }
}
