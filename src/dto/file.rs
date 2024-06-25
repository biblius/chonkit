use serde::Serialize;

use crate::document::File;

#[derive(Debug, Serialize)]
pub struct FileResponse {
    content: String,
    meta: File,
}

impl From<(File, String)> for FileResponse {
    fn from(value: (File, String)) -> Self {
        Self {
            content: value.1,
            meta: value.0,
        }
    }
}
