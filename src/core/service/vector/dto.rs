use serde::Deserialize;
use validify::Validify;

#[derive(Debug, Deserialize, Validify)]
pub struct CreateCollectionPayload {
    #[validate(length(min = 1))]
    #[modify(trim)]
    pub name: String,

    #[validate(length(min = 1))]
    #[modify(trim)]
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct EmbedPayload {
    pub document_id: uuid::Uuid,
    pub collection_id: uuid::Uuid,
}

#[derive(Debug, Deserialize, Validify)]
pub struct SearchPayload {
    /// The text to search by.
    pub query: String,

    /// The collection to search in.
    #[validate(length(min = 1))]
    #[modify(trim)]
    pub collection: String,

    /// Amount of results to return.
    pub limit: Option<u64>,
}
