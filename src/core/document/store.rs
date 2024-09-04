use super::parser::DocumentParser;
use crate::{
    core::{model::document::Document, repo::document::DocumentRepo},
    error::ChonkitError,
};
use std::future::Future;

/// Reads documents' content. Serves as indirection to decouple the documents from their source.
pub trait DocumentStore {
    /// Get the content of document located on `path`.
    ///
    /// * `path`: The path to read from.
    fn read(
        &self,
        document: &Document,
        parser: impl DocumentParser + Send,
    ) -> impl Future<Output = Result<String, ChonkitError>> + Send;

    /// Delete the document contents from the underlying storage.
    ///
    /// * `path`: The path to the document to delete.
    fn delete(&self, path: &str) -> impl Future<Output = Result<(), ChonkitError>> + Send;

    /// Write `contents` to the storage implementation.
    /// Returns the absolute path of where the file was written
    /// as the first element in the tuple, and the content hash
    /// as the second.
    ///
    /// * `name`: Document name.
    /// * `content`: What to write.
    fn write(
        &self,
        name: &str,
        content: &[u8],
    ) -> impl Future<Output = Result<String, ChonkitError>> + Send;

    /// Sync the storage client's contents with the repository.
    ///
    /// * `repo`: Document repository.
    fn sync(
        &self,
        repo: &(impl DocumentRepo + Sync),
    ) -> impl Future<Output = Result<(), ChonkitError>> + Send;
}
