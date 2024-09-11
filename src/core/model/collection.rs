use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// Used by vector stores.
#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize, Default)]
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

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }
}

/// Vector collection model.
#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize)]
pub struct Collection {
    /// Primary key.
    pub id: Uuid,
    /// Collection name. Unique in combination with provider.
    pub name: String,
    /// Embedding model used for the collection.
    pub model: String,
    /// Embedder provider ID.
    pub embedder: String,
    /// Vector database provider.
    pub provider: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CollectionInsert<'a> {
    pub id: Uuid,
    pub name: &'a str,
    pub model: &'a str,
    pub embedder: &'a str,
    pub provider: &'a str,
}

impl<'a> CollectionInsert<'a> {
    pub fn new(name: &'a str, model: &'a str, embedder: &'a str, provider: &'a str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            model,
            embedder,
            provider,
        }
    }
}

/// Embedding information model.
#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize)]
pub struct Embedding {
    /// Primary key.
    pub id: Uuid,

    /// Which document these embeddings belong to.
    pub document_id: uuid::Uuid,

    /// Collection name.
    pub collection_id: uuid::Uuid,

    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,
}

/// DTO for inserting.
#[derive(Debug)]
pub struct EmbeddingInsert {
    pub id: Uuid,
    pub document_id: Uuid,
    pub collection_id: Uuid,
}

impl EmbeddingInsert {
    pub fn new(document_id: Uuid, collection_id: Uuid) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            document_id,
            collection_id,
        }
    }
}
