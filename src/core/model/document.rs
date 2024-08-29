use chrono::{DateTime, Utc};
use serde::Serialize;

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

    /// Label used to group the file.
    pub label: Option<String>,

    /// File tags.
    pub tags: Option<Vec<String>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum DocumentType {
    Text,
    Docx,
    Pdf,
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

/// DTO for inserting.
#[derive(Debug)]
pub struct DocumentInsert<'a> {
    pub id: uuid::Uuid,
    pub name: &'a str,
    pub path: &'a str,
    pub ext: &'a str,
    pub label: Option<&'a str>,
    pub tags: Option<Vec<String>>,
}

impl<'a> DocumentInsert<'a> {
    pub fn new(name: &'a str, path: &'a str, ext: &'a str) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name,
            path,
            ext,
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
    pub path: Option<&'a str>,
    pub label: Option<&'a str>,
    pub tags: Option<Vec<String>>,
}
