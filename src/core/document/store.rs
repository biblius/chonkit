use super::parser::DocumentParser;
use crate::{
    core::{model::document::Document, repo::document::DocumentRepo},
    error::ChonkitError,
};

/// Reads documents' content. Serves as indirection to decouple the documents from their source.
#[async_trait::async_trait]
pub trait DocumentStore {
    fn id(&self) -> &'static str;

    /// Get the content of document located on `path`.
    ///
    /// * `path`: The path to read from.
    async fn read(
        &self,
        document: &Document,
        parser: &(dyn DocumentParser + Sync),
    ) -> Result<String, ChonkitError>;

    /// Delete the document contents from the underlying storage.
    ///
    /// * `path`: The path to the document to delete.
    async fn delete(&self, path: &str) -> Result<(), ChonkitError>;

    /// Write `contents` to the storage implementation.
    /// Returns the absolute path of where the file was written
    /// as the first element in the tuple, and the content hash
    /// as the second.
    ///
    /// * `name`: Document name.
    /// * `content`: What to write.
    async fn write(&self, name: &str, content: &[u8]) -> Result<String, ChonkitError>;

    /// Sync the storage client's contents with the repository.
    ///
    /// * `repo`: Document repository.
    async fn sync(&self, repo: &(dyn DocumentRepo + Sync)) -> Result<(), ChonkitError>;
}
