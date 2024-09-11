//! Defines application business models.

use serde::{Deserialize, Serialize};
use validify::Validate;

pub mod collection;
pub mod document;

/// Used to obtain paginated lists with a total number of items in
/// the tables.
#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    /// The limit.
    #[validate(range(min = 1.))]
    pub per_page: usize,

    /// The offset.
    #[validate(range(min = 1.))]
    pub page: usize,
}

impl Pagination {
    pub fn new(per_page: usize, page: usize) -> Self {
        Self { per_page, page }
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
            per_page: 10,
            page: 1,
        }
    }
}
