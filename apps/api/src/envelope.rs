use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    /// The page returned, 1-based.
    #[schema(example = 1)]
    pub page: u32,
    /// Items per page for this endpoint.
    #[schema(example = 30)]
    pub page_size: u32,
    /// Number of pages, at least 1.
    #[schema(example = 4)]
    pub total_pages: u32,
    /// Number of items across all pages. In a search section it stops at 10,000.
    #[schema(example = 104)]
    pub total_items: u32,
}

#[derive(Serialize, ToSchema)]
#[schema(
    description = "A page of items under `data`, with its place in the whole list under `pagination`."
)]
pub struct ListEnvelope<T> {
    /// The items of this page.
    pub data: Vec<T>,
    /// Where this page sits in the list.
    pub pagination: Pagination,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "The requested item under `data`.")]
pub struct ItemEnvelope<T> {
    /// The requested item.
    pub data: T,
}

pub fn build_pagination(page: u32, page_size: u32, total_items: u32) -> Pagination {
    Pagination {
        page,
        page_size,
        total_pages: std::cmp::max(1, total_items.div_ceil(page_size.max(1))),
        total_items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_pages_rounds_up_and_never_drops_below_one() {
        assert_eq!(build_pagination(1, 30, 0).total_pages, 1);
        assert_eq!(build_pagination(1, 30, 1).total_pages, 1);
        assert_eq!(build_pagination(1, 30, 30).total_pages, 1);
        assert_eq!(build_pagination(1, 30, 31).total_pages, 2);
        assert_eq!(build_pagination(1, 0, 5).total_pages, 5);
        assert_eq!(
            build_pagination(7, 20, u32::MAX).total_pages,
            u32::MAX / 20 + 1
        );
        let page = build_pagination(7, 20, 93);
        assert_eq!((page.page, page.page_size, page.total_items), (7, 20, 93));
    }
}
