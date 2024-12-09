use super::{DocumentParser, ParseConfig};
use crate::{
    core::model::document::{DocumentType, TextDocumentType},
    error::ChonkitError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextParser {
    config: ParseConfig,
}

impl TextParser {
    pub fn new(config: ParseConfig) -> Self {
        Self { config }
    }
}

impl DocumentParser for TextParser {
    fn parse(&self, input: &[u8]) -> Result<String, ChonkitError> {
        Ok(String::from_utf8_lossy(input).to_string())
    }

    fn dtype(&self) -> DocumentType {
        // TODO
        DocumentType::Text(TextDocumentType::Txt)
    }
}
