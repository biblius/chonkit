use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// Used by vector stores.
#[derive(Debug, Serialize)]
pub struct VectorCollection {
    /// Unique collection name.
    pub name: String,

    /// Collection vector size
    pub size: usize,
}

impl VectorCollection {
    pub fn new(name: String, size: usize) -> Self {
        Self { name, size }
    }
}

/// Vector collection model.
#[derive(Debug, Serialize)]
pub struct Collection {
    /// Primary key.
    pub name: String,
    /// Embedding model used for the collection.
    pub model: String,
    /// Embedder ID.
    pub embedder: String,
    /// Vector database source.
    pub src: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CollectionInsert<'a> {
    pub name: &'a str,
    pub model: &'a str,
    pub embedder: &'a str,
    pub src: &'a str,
}

impl<'a> CollectionInsert<'a> {
    pub fn new(name: &'a str, model: &'a str, embedder: &'a str, src: &'a str) -> Self {
        Self {
            name,
            model,
            embedder,
            src,
        }
    }
}

/// Embedding information model.
#[derive(Debug, Serialize)]
pub struct Embedding {
    /// Primary key.
    pub id: Uuid,

    /// Which document these embeddings belong to.
    pub document_id: uuid::Uuid,

    /// Collection name.
    pub collection: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for inserting.
#[derive(Debug)]
pub struct EmbeddingInsert<'a> {
    pub id: Uuid,
    pub document_id: Uuid,
    pub collection: &'a str,
}

impl<'a> EmbeddingInsert<'a> {
    pub fn new(document_id: Uuid, collection: &'a str) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            document_id,
            collection,
        }
    }
}
