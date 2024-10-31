//! Defines application business models.

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{openapi::RefOr, ToSchema};
use validify::Validate;

pub mod collection;
pub mod document;

/// Used to obtain paginated lists with a total number of items in
/// the tables.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct List<T> {
    pub total: Option<usize>,
    pub items: Vec<T>,
}

#[cfg(feature = "http")]
impl<'__s, T> ToSchema<'__s> for List<T>
where
    T: ToSchema<'__s>,
{
    fn schema() -> (
        &'__s str,
        utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
    ) {
        let (_, item_schema) = T::schema();

        let list_schema = utoipa::openapi::schema::ObjectBuilder::new()
            .title(Some("List"))
            .property(
                "total",
                utoipa::openapi::schema::ObjectBuilder::new()
                    .title(Some("total"))
                    .schema_type(utoipa::openapi::SchemaType::Integer),
            )
            .property(
                "items",
                utoipa::openapi::schema::ArrayBuilder::new().items(item_schema),
            )
            .build();

        (
            "List",
            RefOr::T(utoipa::openapi::Schema::Object(list_schema)),
        )
    }
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
#[serde_as]
#[cfg_attr(feature = "http", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[derive(Debug, Clone, Copy, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    /// The limit.
    #[serde_as(as = "DisplayFromStr")]
    #[validate(range(min = 1.))]
    pub per_page: usize,

    /// The offset.
    #[serde_as(as = "DisplayFromStr")]
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
