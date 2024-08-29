use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Document {
    /// Primary key.
    pub id: uuid::Uuid,

    /// File name.
    pub name: String,

    /// Absolute path to file.
    pub path: String,

    /// Label used to group the file.
    pub label: Option<String>,

    /// File tags.
    pub tags: Option<Vec<String>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for inserting.
#[derive(Debug)]
pub struct DocumentInsert<'a> {
    pub id: uuid::Uuid,
    pub name: &'a str,
    pub path: &'a str,
    pub label: Option<&'a str>,
    pub tags: Option<Vec<String>>,
}

impl<'a> DocumentInsert<'a> {
    pub fn new(name: &'a str, path: &'a str) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name,
            path,
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
