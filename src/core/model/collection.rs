use crate::{DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_NAME, DEFAULT_COLLECTION_SIZE};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Vector collection model.
#[derive(Debug, Serialize)]
pub struct Collection {
    /// Primary key.
    pub id: uuid::Uuid,

    /// Collection name.
    pub name: String,

    /// Embedding model used when creating the collection.
    /// This field is omitted when syncing from the vector store.
    /// Other models can also be used if their vector size is the same.
    pub model: Option<String>,

    /// Vector size of the embedding model.
    pub size: usize,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for inserting.
#[derive(Debug)]
pub struct CollectionInsert<'a> {
    pub id: uuid::Uuid,
    pub name: &'a str,
    pub model: Option<&'a str>,
    pub size: usize,
}

impl<'a> CollectionInsert<'a> {
    pub fn new(name: &'a str, size: usize) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name,
            size,
            model: None,
        }
    }

    pub fn with_model(mut self, model: &'a str) -> Self {
        self.model = Some(model);
        self
    }
}

impl<'a> Default for CollectionInsert<'a> {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::default(),
            name: DEFAULT_COLLECTION_NAME,
            model: Some(DEFAULT_COLLECTION_MODEL),
            size: DEFAULT_COLLECTION_SIZE,
        }
    }
}
