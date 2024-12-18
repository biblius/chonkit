use uuid::Uuid;

use super::parser::Parser;
use crate::{
    core::model::document::{Document, DocumentType},
    error::ChonkitError,
};

#[derive(Debug)]
pub struct DocumentStoreFile {
    pub name: String,
    pub ext: DocumentType,
    pub path: String,
}

/// Manipulates documents' content.
/// Serves as indirection to decouple the documents from their source.
#[async_trait::async_trait]
pub trait DocumentStore {
    fn id(&self) -> &'static str;

    /// Get the content of a document located on `path` and parse it.
    ///
    /// * `document`: Document info.
    /// * `parser`: Parser to use for obtaining the text content.
    async fn read(&self, document: &Document, parser: &Parser) -> Result<String, ChonkitError>;

    /// Delete the document contents from the underlying storage.
    ///
    /// * `path`: The path to the file to delete.
    async fn delete(&self, path: &str) -> Result<(), ChonkitError>;

    /// Write `contents` to the storage implementation.
    /// Returns the absolute path of where the file was written.
    ///
    /// * `name`: File name.
    /// * `content`: What to write.
    async fn write(&self, name: &str, content: &[u8]) -> Result<String, ChonkitError>;

    /// List the documents available in the store.
    async fn list_files(&self) -> Result<Vec<DocumentStoreFile>, ChonkitError>;

    /// Retrieve the raw bytes of a document.
    ///
    /// * `path`: The path to read from.
    async fn get_bytes(&self, path: &str) -> Result<Vec<u8>, ChonkitError>;

    /// Given a slice of documents, return a list of document IDs for files that do not exist.
    /// This is used to prune the repository.
    ///
    /// * `documents`: List of documents to check obtained from the repository.
    async fn filter_non_existing(&self, documents: &[Document]) -> Result<Vec<Uuid>, ChonkitError>;
}
