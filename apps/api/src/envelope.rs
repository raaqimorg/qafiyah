use serde::Serialize;
use utoipa::ToSchema;

use crate::constants::ES_MAX_RESULT_WINDOW;
use crate::cursor::{self, Key};
use crate::domain::paging::Slice;

#[derive(Serialize, ToSchema)]
#[schema(
    description = "One page of a longer list under `data`. Pass `next` or `prev` as the `cursor` parameter to get the next or previous page."
)]
pub struct PageEnvelope<T> {
    /// The items of this page.
    pub data: Vec<T>,
    /// Number of items in the whole list. In a search section it stops at 10,000.
    #[schema(example = 104)]
    pub total: u32,
    /// The cursor of the next page, or null on the last page.
    #[schema(required = true, example = "eyJhZnRlciI6MTI2NH0")]
    pub next: Option<String>,
    /// The cursor of the previous page, or null on the first page.
    #[schema(required = true, example = "eyJiZWZvcmUiOjEyMzV9")]
    pub prev: Option<String>,
}

impl<T> PageEnvelope<T> {
    pub fn new<U, K: Key>(page: Slice<U, K>, total: u32, item: impl FnMut(U) -> T) -> Self {
        Self {
            data: page.items.into_iter().map(item).collect(),
            total,
            next: page.next.map(cursor::encode),
            prev: page.prev.map(cursor::encode),
        }
    }

    pub fn at_offset(data: Vec<T>, total: u32, offset: u32, page_size: u32) -> Self {
        let following = offset.saturating_add(page_size);
        Self {
            data,
            total,
            next: (following < total
                && following.saturating_add(page_size) <= ES_MAX_RESULT_WINDOW)
                .then(|| cursor::encode_offset(following)),
            prev: (offset > 0).then(|| cursor::encode_offset(offset.saturating_sub(page_size))),
        }
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "Every item of a short list under `data`.")]
pub struct ListEnvelope<T> {
    /// The items.
    pub data: Vec<T>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "The requested item under `data`.")]
pub struct ItemEnvelope<T> {
    /// The requested item.
    pub data: T,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offsets(page: &PageEnvelope<u8>) -> (Option<u32>, Option<u32>) {
        let read = |raw: &Option<String>| raw.as_deref().and_then(cursor::decode_offset);
        (read(&page.next), read(&page.prev))
    }

    #[test]
    fn a_search_page_points_to_its_neighbours_inside_the_result_window() {
        assert_eq!(
            offsets(&PageEnvelope::at_offset(vec![], 45, 0, 20)),
            (Some(20), None)
        );
        assert_eq!(
            offsets(&PageEnvelope::at_offset(vec![], 45, 20, 20)),
            (Some(40), Some(0))
        );
        assert_eq!(
            offsets(&PageEnvelope::at_offset(vec![], 45, 40, 20)),
            (None, Some(20))
        );
        assert_eq!(
            offsets(&PageEnvelope::at_offset(vec![], 40, 20, 20)),
            (None, Some(0))
        );
        assert_eq!(
            offsets(&PageEnvelope::at_offset(vec![], 10_000, 9960, 20)),
            (Some(9980), Some(9940))
        );
        assert_eq!(
            offsets(&PageEnvelope::at_offset(vec![], 10_000, 9980, 20)),
            (None, Some(9960))
        );
        assert_eq!(
            offsets(&PageEnvelope::at_offset(vec![], 0, 0, 20)),
            (None, None)
        );
    }
}
