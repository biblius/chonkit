//! Defines application business models.

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use validify::{field_err, Validate, ValidationError};

/// Vector collection models.
pub mod collection;

/// Document models.
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
impl<'s, T> utoipa::ToSchema<'s> for List<T>
where
    T: utoipa::ToSchema<'s>,
{
    fn schema() -> (
        &'s str,
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
            utoipa::openapi::RefOr::T(utoipa::openapi::Schema::Object(list_schema)),
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
///
/// `page` defaults to 1 (which results in offset 0).
/// `per_page` defaults to 10.
#[serde_as]
#[cfg_attr(feature = "http", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[derive(Debug, Clone, Copy, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    /// The limit.
    #[serde_as(as = "DisplayFromStr")]
    #[validate(range(min = 1.))]
    per_page: usize,

    /// The offset.
    #[serde_as(as = "DisplayFromStr")]
    #[validate(range(min = 1.))]
    page: usize,
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

/// Used to paginate queries and sort the rows.
#[cfg_attr(feature = "http", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PaginationSort {
    /// See [Pagination].
    #[validate]
    #[serde(flatten)]
    pub pagination: Option<Pagination>,

    /// The column to sort by.
    /// Default: `updated_at`
    // # WARNING
    // Highly important to validate this field since it can be used for SQL injection.
    // Prepared statements do not support placeholders in ORDER BY clauses because they
    // they use column names and not values.
    #[validate(length(min = 1, max = 64))]
    #[validate(custom(ascii_alphanumeric_underscored))]
    pub sort_by: Option<String>,

    /// The direction to sort in.
    /// Default: `DESC`
    pub sort_dir: Option<SortDirection>,
}

impl PaginationSort {
    pub fn new(pagination: Pagination, sort_by: String, sort_dir: SortDirection) -> Self {
        Self {
            pagination: Some(pagination),
            sort_by: Some(sort_by),
            sort_dir: Some(sort_dir),
        }
    }

    pub fn new_default_sort(pagination: Pagination) -> Self {
        Self {
            pagination: Some(pagination),
            sort_by: Some("updated_at".to_string()),
            sort_dir: Some(SortDirection::Desc),
        }
    }

    /// Returns a tuple whose first element is the sort column and
    /// second the sort direction ASC/DESC.
    pub fn to_sort(&self) -> (&str, &str) {
        let direction = match self.sort_dir {
            Some(SortDirection::Asc) => "ASC",
            Some(SortDirection::Desc) | None => "DESC",
        };

        (self.sort_by.as_deref().unwrap_or("updated_at"), direction)
    }

    /// See [Pagination::to_limit_offset].
    pub fn to_limit_offset(&self) -> (i64, i64) {
        self.pagination
            .map(|pagination| pagination.to_limit_offset())
            .unwrap_or(Pagination::default().to_limit_offset())
    }
}

impl Default for PaginationSort {
    fn default() -> Self {
        Self {
            pagination: Some(Pagination::default()),
            sort_by: Some("updated_at".to_string()),
            sort_dir: Some(SortDirection::Desc),
        }
    }
}

#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Deserialize)]
//#[serde(untagged)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

fn ascii_alphanumeric_underscored(s: &str) -> Result<(), ValidationError> {
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(field_err!(
            "ascii_alphanumeric_underscored",
            "parameter must be alphanumeric with underscores [a-z A-Z 0-9 _]"
        ));
    }
    Ok(())
}
