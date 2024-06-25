use crate::{
    document::{self, db::DocumentDb, FileInsert, FileOrDir},
    error::ChonkitError,
    vector_db::VectorService,
};
use async_recursion::async_recursion;
use std::path::Path;
use tracing::{info, trace, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct State {
    pub documents: DocumentService,
    pub vectorizer: VectorService,
}

#[derive(Debug, Clone)]
pub struct DocumentService {
    pub db: DocumentDb,
}

impl DocumentService {
    pub fn new(db: DocumentDb) -> Self {
        Self { db }
    }

    pub async fn init(&self, roots: &[&Path]) -> Result<(), ChonkitError> {
        // Trim any files and directories no longer on fs

        self.sync().await?;
        for root in roots {
            self.process_root(root).await?;
        }

        Ok(())
    }

    /// Read a file's contents from the fs based on its database ID.
    ///
    /// * `id`: The database ID of the file.
    pub async fn get_file(&self, id: Uuid) -> Result<FileOrDir, ChonkitError> {
        let file = self.db.get_file(id).await?;

        let Some(file) = file else {
            return Err(ChonkitError::NotFound(id.to_string()));
        };

        if file.is_dir {
            return Ok(FileOrDir::Dir(file));
        }

        Ok(FileOrDir::File(file))
    }

    pub async fn get_file_contents(&self, path: &str) -> Result<String, ChonkitError> {
        Ok(tokio::fs::read_to_string(path).await?)
    }

    /// Remove any non-existent files from the database.
    pub async fn sync(&self) -> Result<(), ChonkitError> {
        let file_paths = self.db.get_all_file_paths().await?;

        for path in file_paths {
            if let Err(e) = tokio::fs::metadata(&path).await {
                warn!("Error while reading file {path}, trimming");
                trace!("Error: {e}");
                self.db.remove_file_by_path(&path).await?;
            }
        }

        Ok(())
    }

    async fn process_root(&self, path: &Path) -> Result<(), ChonkitError> {
        let root = Self::path_str(path)?;

        info!("Scanning root {root}");

        let root_file = match self.db.get_file_by_path(&root).await? {
            Some(file) => file,
            None => {
                // Insert root if it does not exist
                let root_name = Self::get_valid_name(path)?;
                let file = FileInsert::new_root(root_name, &root);
                self.db.insert_file(file).await?
            }
        };

        let mut files = tokio::fs::read_dir(root).await?;

        while let Some(entry) = files.next_entry().await? {
            let path = entry.path().canonicalize()?;
            if entry.path().is_dir() {
                self.process_directory(&path, root_file.id).await?;
            } else {
                self.process_file(&path, root_file.id).await?;
            }
        }

        Ok(())
    }

    #[async_recursion]
    async fn process_directory(
        &self,
        path: &Path,
        parent_id: uuid::Uuid,
    ) -> Result<(), ChonkitError> {
        let name = Self::get_valid_name(path)?;
        let path = Self::path_str(path)?;

        info!("Scanning {path}");

        // Search for existing
        let directory = match self.db.get_file_by_path(&path).await? {
            Some(dir) => dir,
            None => {
                info!("Inserting {path}");
                let file = FileInsert::new(name, &path, parent_id, true);
                self.db.insert_file(file).await?
            }
        };

        let mut entries = tokio::fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.path().is_dir() {
                self.process_directory(&entry.path(), directory.id).await?;
            } else {
                self.process_file(&entry.path(), directory.id).await?;
            }
        }

        Ok(())
    }

    async fn process_file(&self, path: &Path, parent_id: uuid::Uuid) -> Result<(), ChonkitError> {
        let name = Self::get_valid_name(path)?;
        let path = Self::path_str(path)?;

        info!("Processing {path}");

        match self.db.get_file_by_path(&path).await? {
            Some(file) => {
                info!("{} exists, skipping", file.name);
            }
            None => {
                let file = FileInsert::new(name, &path, parent_id, false);
                self.db.insert_file(file).await?;
            }
        }

        Ok(())
    }

    fn path_str(path: &Path) -> Result<String, ChonkitError> {
        Ok(path.display().to_string())
    }

    fn get_valid_name(path: &Path) -> Result<&str, ChonkitError> {
        let name = path.file_name().ok_or_else(|| {
            ChonkitError::InvalidFileName(format!("{}: unsupported file name", path.display()))
        })?;
        name.to_str()
            .ok_or_else(|| ChonkitError::InvalidFileName(format!("{name:?}: not valid utf-8")))
    }
}
