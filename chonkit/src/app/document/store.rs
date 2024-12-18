use crate::{
    core::{
        document::{
            parser::Parser,
            store::{DocumentStore, DocumentStoreFile},
        },
        model::document::{Document, DocumentType},
    },
    err,
    error::ChonkitError,
    map_err,
};
use std::{path::PathBuf, str::FromStr};
use tracing::{debug, info};
use uuid::Uuid;

/// Simple FS based implementation of a [DocumentStore](crate::core::document::store::DocumentStore).
#[derive(Debug, Clone)]
pub struct FsDocumentStore {
    /// The base directory to store the documents in.
    base: PathBuf,
}

impl FsDocumentStore {
    pub fn new(path: &str) -> Self {
        let base = PathBuf::from_str(path)
            .expect("invalid path")
            .canonicalize()
            .expect("unable to canonicalize");

        if !base.is_dir() {
            panic!("not a directory: {path}");
        }

        info!("Initialising fs store at {}", base.display());

        Self { base }
    }

    fn get_extension(&self, pb: PathBuf) -> Result<DocumentType, ChonkitError> {
        if !pb.is_file() {
            return err!(InvalidFileName, "not a file: {}", pb.display());
        }

        let Some(ext) = pb.extension() else {
            return err!(InvalidFileName, "missing extension: {}", pb.display());
        };

        let Some(ext) = ext.to_str() else {
            return err!(InvalidFileName, "extension invalid unicode: {:?}", ext);
        };

        DocumentType::try_from(ext)
    }
}

#[async_trait::async_trait]
impl DocumentStore for FsDocumentStore {
    fn id(&self) -> &'static str {
        "fs"
    }

    async fn read(&self, document: &Document, parser: &Parser) -> Result<String, ChonkitError> {
        debug!("Reading {}", document.path);
        let file = map_err!(tokio::fs::read(&document.path).await);
        parser.parse(&file)
    }

    async fn write(&self, name: &str, file: &[u8]) -> Result<String, ChonkitError> {
        let path = format!("{}/{name}", self.base.display());
        debug!("Writing {path}");
        match tokio::fs::read(&path).await {
            Ok(_) => err!(AlreadyExists, "File '{name}' at {path}"),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    map_err!(tokio::fs::write(&path, file).await);
                    Ok(path)
                }
                _ => Err(map_err!(Err(e))),
            },
        }
    }

    async fn delete(&self, path: &str) -> Result<(), ChonkitError> {
        debug!("Removing {path}");
        Ok(map_err!(tokio::fs::remove_file(path).await))
    }

    async fn list_files(&self) -> Result<Vec<DocumentStoreFile>, ChonkitError> {
        let mut files = vec![];

        let mut entries = map_err!(tokio::fs::read_dir(&self.base).await);

        while let Some(file) = map_err!(entries.next_entry().await) {
            let ext = match self.get_extension(file.path()) {
                Ok(ext) => ext,
                Err(e) => {
                    tracing::error!("{e}");
                    continue;
                }
            };
            let name = file.file_name().to_string_lossy().to_string();
            let path = file.path().display().to_string();

            files.push(DocumentStoreFile { name, path, ext });
        }

        Ok(files)
    }

    async fn get_bytes(&self, path: &str) -> Result<Vec<u8>, ChonkitError> {
        Ok(map_err!(tokio::fs::read(path).await))
    }

    async fn filter_non_existing(&self, documents: &[Document]) -> Result<Vec<Uuid>, ChonkitError> {
        let mut missing = vec![];
        for document in documents {
            if let Err(e) = tokio::fs::metadata(&document.path).await {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        info!("Document '{}' not found in storage", document.name);
                        missing.push(document.id)
                    }
                    _ => return map_err!(Err(e)),
                }
            }
        }
        Ok(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::{DocumentStore, FsDocumentStore};
    use crate::core::{
        document::parser::{text::TextParser, Parser},
        model::document::Document,
    };

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

        let read = store
            .read(&d, &Parser::Text(TextParser::default()))
            .await
            .unwrap();
        assert_eq!(CONTENT, read);

        store.delete(&path).await.unwrap();

        tokio::fs::remove_dir(DIR).await.unwrap();
    }
}
