use crate::{DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_NAME};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Vector collection model.
#[derive(Debug, Serialize)]
pub struct Collection {
    pub id: uuid::Uuid,
    pub name: String,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for inserting.
#[derive(Debug)]
pub struct CollectionInsert<'a> {
    pub id: uuid::Uuid,
    pub name: &'a str,
    pub model: &'a str,
}

impl<'a> CollectionInsert<'a> {
    pub fn new(name: &'a str, model: &'a str) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name,
            model,
        }
    }
}

impl<'a> Default for CollectionInsert<'a> {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::default(),
            name: DEFAULT_COLLECTION_NAME,
            model: DEFAULT_COLLECTION_MODEL,
        }
    }
}
