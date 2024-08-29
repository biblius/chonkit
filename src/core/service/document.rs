use crate::{
    core::{
        document::store::DocumentStore,
        model::document::{Document, DocumentInsert},
        repo::document::DocumentRepo,
    },
    error::ChonkitError,
};
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
    pub async fn get_content(&self, id: uuid::Uuid) -> Result<String, ChonkitError> {
        let document = self.repo.get_by_id(id).await?;

        let Some(document) = document else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };

        // TODO: Load parsing configuration for documents.

        // self.storage.read(path).await
        todo!()
    }

    /// Insert the document to the repository and write its contents
    /// to the underlying storage implementation.
    pub async fn upload(&self, name: &str, ext: &str, file: &[u8]) -> Result<(), ChonkitError> {
        let path = self.storage.write(name, file).await?;
        let document = DocumentInsert::new(name, &path, ext);
        self.repo.insert(document).await?;
        Ok(())
    }

    // pub async fn sync(&self) -> Result<(), ChonkitError> {
    //     match self.repo.get_by_path(path).await? {
    //         Some(document) => {
    //             info!("'{}' already exists", document.name);
    //             return Ok(());
    //         }
    //         None => {
    //             //let document = DocumentInsert::new(name, &path);
    //             //self.repo.insert(document).await?;
    //         }
    //     }
    //
    //     Ok(())
    // }
}
