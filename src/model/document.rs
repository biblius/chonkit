use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug)]
pub enum FileOrDir {
    File(File),
    Dir(File),
}

/// Database model
#[derive(Debug, Serialize)]
pub struct File {
    /// Primary key.
    pub id: uuid::Uuid,

    /// File name.
    pub name: String,

    /// Absolute path to file.
    pub path: String,

    /// Which directory the file belongs to, None if the file is a root directory.
    pub parent: Option<uuid::Uuid>,

    /// File tags.
    pub tags: Option<Vec<String>>,

    /// Utility flag for checking whether the file is a dir
    /// without looking at the fs.
    pub is_dir: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for inserting.
#[derive(Debug)]
pub struct FileInsert<'a> {
    pub name: &'a str,
    pub path: &'a str,
    pub parent: Option<uuid::Uuid>,
    pub tags: Option<Vec<String>>,
    pub is_dir: bool,
}

impl<'a> FileInsert<'a> {
    pub fn new(name: &'a str, path: &'a str, parent: uuid::Uuid, is_dir: bool) -> Self {
        Self {
            name,
            path,
            parent: Some(parent),
            tags: None,
            is_dir,
        }
    }

    pub fn new_root(name: &'a str, path: &'a str) -> Self {
        Self {
            name,
            path,
            parent: None,
            tags: None,
            is_dir: true,
        }
    }
}

/// DTO for updating.
#[derive(Debug)]
pub struct FileUpdate<'a> {
    pub name: Option<&'a str>,
    pub path: Option<&'a str>,
    pub parent: Option<uuid::Uuid>,
    pub tags: Option<Vec<String>>,
}
