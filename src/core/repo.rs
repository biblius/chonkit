use serde::{Deserialize, Serialize};
use validify::Validate;

pub mod document;
pub mod vector;

/// Used to obtain paginated lists with a total number of items in
/// the tables.
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

impl<T> std::iter::IntoIterator for List<T> {
    type Item = T;

    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

/// Used to paginate queries.
#[derive(Debug, Clone, Copy, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    /// The offset.
    #[validate(range(min = 1.))]
    pub page: usize,

    /// The limit.
    #[validate(range(min = 1.))]
    pub per_page: usize,
}

impl Pagination {
    pub fn new(page: usize, per_page: usize) -> Self {
        Self { page, per_page }
    }

    /// Returns a tuple whose first element is the LIMIT and second
    /// the OFFSET for the query.
    pub fn to_limit_offset(&self) -> (i64, i64) {
        let Self { page, per_page } = self;
        (*per_page as i64, ((page - 1) * *per_page) as i64)
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 10,
        }
    }
}
