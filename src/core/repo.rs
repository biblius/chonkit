use serde::{Deserialize, Serialize};

pub mod document;

#[derive(Debug, Serialize)]
pub struct List<T> {
    pub total: Option<usize>,
    pub items: Vec<T>,
}

impl<T> List<T> {
    pub fn new(total: Option<usize>, items: Vec<T>) -> Self {
        Self { total, items }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Pagination {
    pub page: usize,
    pub per_page: usize,
}
