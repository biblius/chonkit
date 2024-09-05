//! Http specific DTOs.

use crate::core::model::document::Document;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub(super) struct UploadResult {
    pub documents: Vec<Document>,
    /// Map form keys to errors
    pub errors: HashMap<String, Vec<String>>,
}
