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
