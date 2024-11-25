use crate::{
    core::{
        document::{
            parser::DocumentParser,
            sha256,
            store::{DocumentStore, DocumentSync},
        },
        model::{
            document::{Document, DocumentInsert, DocumentType},
            Pagination, PaginationSort,
        },
        repo::document::DocumentRepo,
    },
    err,
    error::ChonkitError,
    map_err,
};
use std::{path::PathBuf, str::FromStr, time::Instant};
use tracing::{debug, error, info};

/// Simple FS based implementation of a [DocumentStore](crate::core::document::DocumentStore).
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

    async fn read(
        &self,
        document: &Document,
        parser: &(dyn DocumentParser + Sync),
    ) -> Result<String, ChonkitError> {
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
}

#[async_trait::async_trait]
impl<T> DocumentSync<T> for FsDocumentStore
where
    T: DocumentRepo + Send + Sync,
{
    async fn sync(&self, repo: &T) -> Result<(), ChonkitError> {
        let __start = Instant::now();
        info!("Syncing documents with {}", self.id());

        // Prune
        let documents = repo
            .list(
                PaginationSort::new_default_sort(Pagination::new(10_000, 1)),
                Some(self.id()),
                None,
            )
            .await?;

        for document in documents {
            if let Err(e) = tokio::fs::metadata(&document.path).await {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        info!(
                            "Document '{}' not found in storage, trimming",
                            document.name
                        );
                        repo.remove_by_id(document.id).await?;
                    }
                    _ => return map_err!(Err(e)),
                }
            }
        }

        // Store
        let mut files = map_err!(tokio::fs::read_dir(&self.base).await);

        while let Some(file) = map_err!(files.next_entry().await) {
            let ext = match self.get_extension(file.path()) {
                Ok(ext) => ext,
                Err(e) => {
                    error!("{e}");
                    continue;
                }
            };
            let name = file.file_name().to_string_lossy().to_string();
            let path = file.path().display().to_string();

            let content = map_err!(tokio::fs::read(&path).await);
            let hash = sha256(&content);

            let doc = repo.get_by_path(&path).await?;

            if let Some(Document { id, name, .. }) = doc {
                info!("Document '{name}' already exists ({id})");
                continue;
            }

            let insert = DocumentInsert::new(&name, &path, ext, &hash, self.id());

            match repo.insert(insert).await {
                Ok(Document { id, name, .. }) => info!("Successfully inserted '{name}' ({id})"),
                Err(e) => error!("{e}"),
            }
        }

        info!(
            "Syncing finished for storage '{}', took {}ms",
            self.id(),
            Instant::now().duration_since(__start).as_millis()
        );

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

        let read = store.read(&d, &TextParser::default()).await.unwrap();
        assert_eq!(CONTENT, read);

        store.delete(&path).await.unwrap();

        tokio::fs::remove_dir(DIR).await.unwrap();
    }
}
