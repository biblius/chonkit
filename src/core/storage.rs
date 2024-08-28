use std::future::Future;
use uuid::Uuid;

use crate::model::document::File;

pub trait FileStorage {
    type Error;

    /// Store the document to the persistence backend.
    ///
    /// * `name`: The name at which to store the document.
    /// * `url`: The uniform resource locator to the file, e.g. the path if the
    /// backend is fs based, or a URI if remote.
    fn store(
        &self,
        name: &str,
        url: &str,
    ) -> impl Future<Output = Result<File, Self::Error>> + Send;

    /// Read the file contents.
    ///
    /// * `id`:
    fn read(&self, id: Uuid) -> impl Future<Output = Result<String, Self::Error>> + Send;
}
