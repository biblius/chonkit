use std::{path::PathBuf, str::FromStr};

use tracing::{error, info};

use crate::{
    core::{
        document::{parser::DocumentParser, store::DocumentStore},
        model::document::{Document, DocumentInsert, DocumentType},
        repo::document::DocumentRepo,
    },
    error::ChonkitError,
};

/// Simple FS based implementation of a [DocumentStore](crate::core::document::DocumentStore).
#[derive(Debug, Clone)]
pub struct FsDocumentStore {
    /// The base directory to store the documents in.
    base: PathBuf,
}

impl FsDocumentStore {
    pub fn new(path: &str) -> Self {
        Self {
            base: PathBuf::from_str(path)
                .expect("invalid path")
                .canonicalize()
                .expect("unable to canonicalize"),
        }
    }

    fn get_extension(&self, pb: PathBuf) -> Result<DocumentType, ChonkitError> {
        if !pb.is_file() {
            return Err(ChonkitError::InvalidFileName(format!(
                "not a file: {}",
                pb.display()
            )));
        }

        let Some(ext) = pb.extension() else {
            return Err(ChonkitError::InvalidFileName(format!(
                "missing extension: {}",
                pb.display()
            )));
        };

        let Some(ext) = ext.to_str() else {
            return Err(ChonkitError::InvalidFileName(format!(
                "extension invalid unicode: {:?}",
                ext
            )));
        };

        match ext {
            "pdf" => Ok(DocumentType::Pdf),
            "docx" => Ok(DocumentType::Docx),
            "txt" | "json" | "md" | "xml" | "html" => Ok(DocumentType::Text),
            ext => Err(ChonkitError::UnsupportedFileType(ext.to_string())),
        }
    }
}

impl DocumentStore for FsDocumentStore {
    async fn read(
        &self,
        document: &Document,
        parser: impl DocumentParser + Send,
    ) -> Result<String, ChonkitError> {
        let file = tokio::fs::read(&document.path).await?;
        parser.parse(&file)
    }

    async fn write(&self, name: &str, file: &[u8]) -> Result<String, ChonkitError> {
        let path = format!("{}/{name}", self.base.display());
        tokio::fs::write(&path, file).await?;
        Ok(path)
    }

    async fn delete(&self, path: &str) -> Result<(), ChonkitError> {
        Ok(tokio::fs::remove_file(path).await?)
    }

    async fn sync(&self, repo: &(impl DocumentRepo + Sync)) -> Result<(), ChonkitError> {
        let mut files = tokio::fs::read_dir(&self.base).await?;
        while let Some(file) = files.next_entry().await? {
            let ext = match self.get_extension(file.path()) {
                Ok(ext) => ext,
                Err(e) => {
                    error!("{e}");
                    continue;
                }
            };
            let name = file.file_name().to_string_lossy().to_string();
            let path = file.path().display().to_string();

            let doc = repo.get_by_path(&path).await?;

            if let Some(Document { id, path, .. }) = doc {
                info!("Document '{path}' already exists ({id})");
                continue;
            }

            let insert = DocumentInsert::new(&name, &path, ext);

            match repo.insert(insert).await {
                Ok(Document { id, path, .. }) => info!("Successfully inserted '{path}' ({id})"),
                Err(e) => error!("{e}"),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{DocumentStore, FsDocumentStore};
    use crate::core::{document::parser::text::TextParser, model::document::Document};

    const DIR: &str = "__fs_doc_store_tests";
    const CONTENT: &str = "Hello world.";

    #[tokio::test]
    async fn works() {
        tokio::fs::create_dir(DIR).await.unwrap();

        let store = FsDocumentStore::new(DIR);

        let d = Document {
            name: "foo".to_string(),
            path: format!("{DIR}/foo"),
            ..Default::default()
        };

        let path = store.write(&d.name, CONTENT.as_bytes()).await.unwrap();

        let file = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(CONTENT, file);

        let read = store.read(&d, TextParser::default()).await.unwrap();
        assert_eq!(CONTENT, read);

        store.delete(&path).await.unwrap();

        tokio::fs::remove_dir(DIR).await.unwrap();
    }
}
