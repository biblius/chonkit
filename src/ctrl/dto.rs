use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct CreateCollectionPayload {
    pub name: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchPayload {
    pub model: String,
    pub query: String,
    pub collection: String,
    pub limit: u64,
}
