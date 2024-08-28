use crate::{
    db::document::DocumentDB,
    error::ChonkitError,
    model::document::{Document, DocumentInsert},
};
use std::path::Path;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DocumentService {
    pub db: DocumentDB,
}

impl DocumentService {
    pub fn new(db: DocumentDB) -> Self {
        Self { db }
    }

    /// Read a file's contents from the fs based on its database ID.
    ///
    /// * `id`: The database ID of the file.
    pub async fn get_file(&self, id: Uuid) -> Result<Document, ChonkitError> {
        let file = self.db.get_file(id).await?;

        let Some(file) = file else {
            return Err(ChonkitError::NotFound(id.to_string()));
        };

        Ok(file)
    }

    pub async fn get_file_contents(&self, path: &str) -> Result<String, ChonkitError> {
        Ok(tokio::fs::read_to_string(path).await?)
    }

    async fn store_file(&self, path: &str) -> Result<(), ChonkitError> {
        info!("Processing {path}");

        match self.db.get_file_by_path(&path).await? {
            Some(file) => {
                info!("'{}' exists, skipping", file.name);
            }
            None => {
                // let file = DocumentInsert::new(name, &path, parent_id, false);
                // self.db.insert_file(file).await?;
            }
        }

        Ok(())
    }
}
