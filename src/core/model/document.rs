use crate::{
    core::{chunk::Chunker, document::parser::ParseConfig},
    error::ChonkitError,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

pub mod config;

/// Holds relevant data for parsing and chunking.
#[derive(Debug, Serialize)]
pub struct DocumentConfig {
    pub id: uuid::Uuid,
    pub name: String,
    pub path: String,
    pub ext: String,
    pub hash: String,
    pub src: String,
    pub chunk_config: Option<Chunker>,
    pub parse_config: Option<ParseConfig>,
}

/// Holds document metadata.
/// Main document model for the `documents` table.
#[derive(Debug, Serialize, Default)]
pub struct Document {
    /// Primary key.
    pub id: uuid::Uuid,

    /// File name.
    pub name: String,

    /// Absolute path to file.
    pub path: String,

    /// File extension.
    pub ext: String,

    /// Content hash.
    pub hash: String,

    /// Content source.
    pub src: String,

    /// Label used to group the file.
    pub label: Option<String>,

    /// File tags.
    pub tags: Option<Vec<String>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// All possible file types chonkit can process.
#[derive(Debug, Clone, Copy)]
pub enum DocumentType {
    /// Encapsulates any files that can be read as strings.
    /// Does not necessarily have to be `.txt`, could be `.json`, `.csv`, etc.
    Text,

    /// Microschlong steaming pile of garbage document.
    Docx,

    /// PDF document.
    Pdf,
}

impl DocumentType {
    pub fn try_from_file_name(name: &str) -> Result<Self, ChonkitError> {
        let Some((_, ext)) = name.rsplit_once('.') else {
            return Err(ChonkitError::UnsupportedFileType(format!(
                "{name} - missing extension"
            )));
        };
        Self::try_from(ext)
    }
}

impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentType::Text => write!(f, "txt"),
            DocumentType::Docx => write!(f, "docx"),
            DocumentType::Pdf => write!(f, "pdf"),
        }
    }
}

impl TryFrom<&str> for DocumentType {
    type Error = ChonkitError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "txt" | "md" | "xml" | "json" => Ok(Self::Text),
            "pdf" => Ok(Self::Pdf),
            "docx" => Ok(Self::Docx),
            _ => Err(ChonkitError::UnsupportedFileType(value.to_owned())),
        }
    }
}

impl TryFrom<String> for DocumentType {
    type Error = ChonkitError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

/// DTO for inserting.
#[derive(Debug)]
pub struct DocumentInsert<'a> {
    pub id: uuid::Uuid,
    pub name: &'a str,
    pub path: &'a str,
    pub hash: &'a str,
    pub ext: DocumentType,
    pub src: &'a str,
    pub label: Option<&'a str>,
    pub tags: Option<Vec<String>>,
}

impl<'a> DocumentInsert<'a> {
    pub fn new(
        name: &'a str,
        path: &'a str,
        ext: DocumentType,
        hash: &'a str,
        src: &'a str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name,
            path,
            ext,
            hash,
            src,
            label: None,
            tags: None,
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }
}

/// DTO for updating.
#[derive(Debug)]
pub struct DocumentUpdate<'a> {
    pub name: Option<&'a str>,
    pub label: Option<&'a str>,
    pub tags: Option<Vec<String>>,
}
