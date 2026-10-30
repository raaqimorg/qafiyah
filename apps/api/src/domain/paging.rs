#[derive(Debug, Clone, PartialEq)]
pub enum Cursor<K> {
    After(K),
    Before(K),
}

impl<K> Cursor<K> {
    pub fn key(&self) -> &K {
        match self {
            Self::After(key) | Self::Before(key) => key,
        }
    }

    pub fn is_before(&self) -> bool {
        matches!(self, Self::Before(_))
    }
}

#[derive(Debug, PartialEq)]
pub struct Slice<T, K> {
    pub items: Vec<T>,
    pub next: Option<Cursor<K>>,
    pub prev: Option<Cursor<K>>,
}

impl<T, K> Slice<T, K> {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            next: None,
            prev: None,
        }
    }

    pub fn map<U>(self, convert: impl FnMut(T) -> U) -> Slice<U, K> {
        Slice {
            items: self.items.into_iter().map(convert).collect(),
            next: self.next,
            prev: self.prev,
        }
    }
}

pub fn fetch_limit(page_size: u32) -> i64 {
    i64::from(page_size).saturating_add(1)
}

pub fn slice<T, K>(
    mut rows: Vec<T>,
    cursor: Option<&Cursor<K>>,
    page_size: u32,
    key: impl Fn(&T) -> K,
) -> Slice<T, K> {
    let page_size = usize::try_from(page_size).unwrap_or(usize::MAX);
    let more = rows.len() > page_size;
    rows.truncate(page_size);
    let backward = cursor.is_some_and(Cursor::is_before);
    if backward {
        rows.reverse();
    }
    let first = rows.first().map(&key);
    let last = rows.last().map(&key);
    let (next, prev) = match cursor {
        None => (last.filter(|_| more).map(Cursor::After), None),
        Some(Cursor::After(_)) => (
            last.filter(|_| more).map(Cursor::After),
            first.map(Cursor::Before),
        ),
        Some(Cursor::Before(_)) => (
            last.map(Cursor::After),
            first.filter(|_| more).map(Cursor::Before),
        ),
    };
    Slice {
        items: rows,
        next,
        prev,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(rows: &[i32], cursor: Option<Cursor<i32>>) -> Slice<i32, i32> {
        slice(rows.to_vec(), cursor.as_ref(), 3, |row| *row)
    }

    #[test]
    fn the_first_page_points_forward_only_when_a_row_is_left_over() {
        assert_eq!(
            page(&[1, 2, 3, 4], None),
            Slice {
                items: vec![1, 2, 3],
                next: Some(Cursor::After(3)),
                prev: None
            }
        );
        assert_eq!(
            page(&[1, 2, 3], None),
            Slice {
                items: vec![1, 2, 3],
                next: None,
                prev: None
            }
        );
    }

    #[test]
    fn a_page_after_a_cursor_points_back_to_its_first_row() {
        assert_eq!(
            page(&[4, 5, 6, 7], Some(Cursor::After(3))),
            Slice {
                items: vec![4, 5, 6],
                next: Some(Cursor::After(6)),
                prev: Some(Cursor::Before(4))
            }
        );
        assert_eq!(
            page(&[7], Some(Cursor::After(6))),
            Slice {
                items: vec![7],
                next: None,
                prev: Some(Cursor::Before(7))
            }
        );
    }

    #[test]
    fn a_page_before_a_cursor_arrives_nearest_first_and_reads_in_list_order() {
        assert_eq!(
            page(&[6, 5, 4, 3], Some(Cursor::Before(7))),
            Slice {
                items: vec![4, 5, 6],
                next: Some(Cursor::After(6)),
                prev: Some(Cursor::Before(4))
            }
        );
        assert_eq!(
            page(&[3, 2, 1], Some(Cursor::Before(4))),
            Slice {
                items: vec![1, 2, 3],
                next: Some(Cursor::After(3)),
                prev: None
            }
        );
    }

    #[test]
    fn an_empty_page_points_nowhere() {
        for cursor in [None, Some(Cursor::After(9)), Some(Cursor::Before(1))] {
            assert_eq!(
                page(&[], cursor),
                Slice {
                    items: vec![],
                    next: None,
                    prev: None
                }
            );
        }
    }

    #[test]
    fn walking_forward_then_back_visits_the_same_pages() {
        let all: Vec<i32> = (1..=10).collect();
        let fetch = |cursor: Option<&Cursor<i32>>| {
            let rows: Vec<i32> = match cursor {
                None => all.iter().copied().take(4).collect(),
                Some(Cursor::After(k)) => all.iter().copied().filter(|r| r > k).take(4).collect(),
                Some(Cursor::Before(k)) => all
                    .iter()
                    .rev()
                    .copied()
                    .filter(|r| r < k)
                    .take(4)
                    .collect(),
            };
            slice(rows, cursor, 3, |row| *row)
        };
        let mut forward = vec![fetch(None)];
        while let Some(next) = forward.last().and_then(|page| page.next.clone()) {
            forward.push(fetch(Some(&next)));
        }
        let pages: Vec<Vec<i32>> = forward.iter().map(|page| page.items.clone()).collect();
        assert_eq!(
            pages,
            [vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9], vec![10]]
        );
        let mut back = vec![forward.last().expect("a last page").items.clone()];
        let mut cursor = forward.last().and_then(|page| page.prev.clone());
        while let Some(prev) = cursor {
            let page = fetch(Some(&prev));
            back.push(page.items.clone());
            cursor = page.prev;
        }
        back.reverse();
        assert_eq!(back, pages);
    }
}
