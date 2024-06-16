use chrono::{DateTime, Utc};

/// Database model
#[derive(Debug)]
pub struct File {
    /// DB ID.
    pub id: uuid::Uuid,

    /// File name
    pub name: String,

    /// Absolute path to file
    pub path: String,

    /// Which directory the file belongs to, None if in root
    pub parent: Option<uuid::Uuid>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
