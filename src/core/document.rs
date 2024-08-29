use crate::error::ChonkitError;
use docx_rs::read_docx;
use std::{future::Future, path::PathBuf, str::FromStr};

/// Load and parse a PDF file from the given buffer.
///
/// * `source`: The buffer.
pub fn load_pdf(source: &[u8]) -> Result<String, ChonkitError> {
    let document = lopdf::Document::load_mem(source)?;
    let text = crate::core::parse::pdf::parse(document)?;
    Ok(text)
}

/// Load and parse a DOCX file from the given buffer.
///
/// * `source`: The buffer.
pub fn load_docx(source: &[u8]) -> Result<String, ChonkitError> {
    let document = read_docx(source)?;
    let text = crate::core::parse::docx::parse(document)?;
    Ok(text)
}

/// Reads documents' content. Serves as indirection to decouple the documents from their source.
pub trait DocumentStore {
    /// Get the content of document located on `path`.
    ///
    /// * `path`: The path to read from.
    fn read(&self, path: &str) -> impl Future<Output = Result<String, ChonkitError>> + Send;

    /// Delete the document contents from the underlying storage.
    ///
    /// * `path`: The path to the document to delete.
    fn delete(&self, path: &str) -> impl Future<Output = Result<(), ChonkitError>> + Send;

    /// Write `contents` to the storage implementation.
    /// Returns the absolute path of where the file was written.
    ///
    /// * `name`: Document name.
    /// * `content`: What to write.
    fn write(
        &self,
        name: &str,
        content: &str,
    ) -> impl Future<Output = Result<String, ChonkitError>> + Send;
}

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
}

impl DocumentStore for FsDocumentStore {
    async fn read(&self, path: &str) -> Result<String, ChonkitError> {
        Ok(tokio::fs::read_to_string(path).await?)
    }

    async fn write(&self, name: &str, content: &str) -> Result<String, ChonkitError> {
        let path = format!("{}/{name}", self.base.display());
        tokio::fs::write(&path, content).await?;
        Ok(path)
    }

    async fn delete(&self, path: &str) -> Result<(), ChonkitError> {
        Ok(tokio::fs::remove_file(path).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::{DocumentStore, FsDocumentStore};

    const DIR: &str = "__fs_doc_store_tests";
    const CONTENT: &str = "Hello world.";

    #[tokio::test]
    async fn works() {
        tokio::fs::create_dir(DIR).await.unwrap();

        let store = FsDocumentStore::new(DIR);

        let path = store.write("foo", CONTENT).await.unwrap();
        let file = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(CONTENT, file);

        let read = store.read(&path).await.unwrap();
        assert_eq!(CONTENT, read);

        store.delete(&path).await.unwrap();

        tokio::fs::remove_dir(DIR).await.unwrap();
    }
}
