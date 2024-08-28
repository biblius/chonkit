use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug)]
pub enum FileOrDir {
    File(File),
    Dir(File),
}

#[derive(Debug, Serialize)]
pub struct File {
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
pub struct FileInsert<'a> {
    pub name: &'a str,
    pub path: &'a str,
    pub label: Option<&'a str>,
    pub tags: Option<Vec<String>>,
}

impl<'a> FileInsert<'a> {
    pub fn new(name: &'a str, path: &'a str) -> Self {
        Self {
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
pub struct FileUpdate<'a> {
    pub name: Option<&'a str>,
    pub path: Option<&'a str>,
    pub label: Option<&'a str>,
    pub tags: Option<Vec<String>>,
}
