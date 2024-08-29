use crate::{
    core::{
        document::DocumentStore,
        model::document::{Document, DocumentInsert},
        repo::document::DocumentRepo,
    },
    error::ChonkitError,
};
use tracing::info;
use uuid::Uuid;

/// # CORE
/// High level operations for document management.
#[derive(Debug, Clone)]
pub struct DocumentService<R, S> {
    pub repo: R,
    pub storage: S,
}

impl<R, S> DocumentService<R, S>
where
    R: DocumentRepo,
    S: DocumentStore,
{
    pub fn new(repo: R, storage: S) -> Self {
        Self { repo, storage }
    }

    /// Get the metadata for a document.
    ///
    /// * `id`: The ID of the document.
    pub async fn get_metadata(&self, id: Uuid) -> Result<Document, ChonkitError> {
        let file = self.repo.get_by_id(id).await?;

        let Some(file) = file else {
            return Err(ChonkitError::NotFound(id.to_string()));
        };

        Ok(file)
    }

    /// Get document content.
    ///
    /// * `path`: Where to read from.
    pub async fn get_content(&self, path: &str) -> Result<String, ChonkitError> {
        self.storage.read(path).await
    }

    /// Insert the document to the repository and write its contents
    /// to the underlying storage implementation.
    ///
    /// * `name`: Document name.
    /// * `content`: Document contents.
    pub async fn upload(&self, name: &str, content: &str) -> Result<(), ChonkitError> {
        let path = self.storage.write(name, content).await?;
        let document = DocumentInsert::new(name, &path);
        self.repo.insert(document).await?;
        Ok(())
    }

    pub async fn register(&self, path: &str) -> Result<(), ChonkitError> {
        info!("Registering document at path: {path}");

        match self.repo.get_by_path(path).await? {
            Some(file) => {
                info!("'{}' exists, skipping", file.name);
            }
            None => {
                //let document = DocumentInsert::new(name, &path);
                //self.repo.insert(document).await?;
            }
        }

        Ok(())
    }
}
