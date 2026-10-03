use async_trait::async_trait;
use unicode_normalization::UnicodeNormalization;

use crate::constants::{ES_MAX_RESULT_WINDOW, SEARCH_POEMS_PER_PAGE, SEARCH_POETS_PER_PAGE};
use crate::domain::StoreError;
use crate::domain::{PoetBrief, Term};
use crate::js;

pub struct PoemHit {
    pub title: String,
    pub slug: String,
    pub snippet: String,
    pub poet: PoetBrief,
    pub meter: Term,
    pub era: Term,
    pub relevance: f64,
}

pub struct PoetHit {
    pub name: String,
    pub slug: String,
    pub era: Term,
    pub relevance: f64,
}

pub struct PoetListing {
    pub name: String,
    pub slug: String,
    pub poems_count: i64,
}

pub struct Page<T> {
    pub hits: Vec<T>,
    pub total: u32,
}

pub struct PoemSearchParams {
    pub q: String,
    pub page: u32,
    pub page_size: u32,
    pub poet_slugs: Vec<String>,
    pub era_slugs: Vec<String>,
    pub meter_slugs: Vec<String>,
    pub theme_slugs: Vec<String>,
    pub rhyme_slugs: Vec<String>,
    pub poem_type_slugs: Vec<String>,
    pub collection_slugs: Vec<String>,
    pub exact: bool,
}

impl Default for PoemSearchParams {
    fn default() -> Self {
        Self {
            q: String::new(),
            page: 1,
            page_size: SEARCH_POEMS_PER_PAGE,
            poet_slugs: Vec::new(),
            era_slugs: Vec::new(),
            meter_slugs: Vec::new(),
            theme_slugs: Vec::new(),
            rhyme_slugs: Vec::new(),
            poem_type_slugs: Vec::new(),
            collection_slugs: Vec::new(),
            exact: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PoetSort {
    Id,
    PoemsCount,
}

pub struct PoetSearchParams {
    pub q: String,
    pub page: u32,
    pub era_slugs: Vec<String>,
    pub page_size: u32,
    pub sort: PoetSort,
    pub exact: bool,
    pub window: u32,
}

impl Default for PoetSearchParams {
    fn default() -> Self {
        Self {
            q: String::new(),
            page: 1,
            era_slugs: Vec::new(),
            page_size: SEARCH_POETS_PER_PAGE,
            sort: PoetSort::Id,
            exact: false,
            window: ES_MAX_RESULT_WINDOW,
        }
    }
}

#[async_trait]
pub trait SearchIndex: Send + Sync {
    async fn search_poems(&self, params: &PoemSearchParams) -> Result<Page<PoemHit>, StoreError>;
    async fn search_poets(&self, params: &PoetSearchParams) -> Result<Page<PoetHit>, StoreError>;
    async fn list_poets(&self, params: &PoetSearchParams) -> Result<Page<PoetListing>, StoreError>;
}

const MARK_OPEN: &str = "<mark>";
const MARK_CLOSE: &str = "</mark>";

fn longest_mark_span(hemistich: Option<&&str>) -> usize {
    let Some(text) = hemistich else {
        return 0;
    };
    let mut longest = 0;
    let mut rest = *text;
    while let Some(open_at) = rest.find(MARK_OPEN) {
        let Some(after_open) = rest.get(open_at.saturating_add(MARK_OPEN.len())..) else {
            break;
        };
        let Some(close_at) = after_open.find(MARK_CLOSE) else {
            break;
        };
        if let Some(marked) = after_open.get(..close_at) {
            longest = longest.max(marked.encode_utf16().count());
        }
        let Some(remainder) = after_open.get(close_at.saturating_add(MARK_CLOSE.len())..) else {
            break;
        };
        rest = remainder;
    }
    longest
}

fn leading_verse(content: &str) -> String {
    content.split('*').take(2).collect::<Vec<&str>>().join("*")
}

fn ends_inside_mark(text: &str) -> bool {
    match (text.rfind(MARK_OPEN), text.rfind(MARK_CLOSE)) {
        (Some(open_at), Some(close_at)) => open_at > close_at,
        (Some(_), None) => true,
        _ => false,
    }
}

fn balanced_hemistichs(highlighted: &str) -> Vec<String> {
    let mut open = false;
    highlighted
        .split('*')
        .map(|hemistich| {
            let mut balanced = if open {
                format!("{MARK_OPEN}{hemistich}")
            } else {
                hemistich.to_string()
            };
            open = ends_inside_mark(&balanced);
            if open {
                balanced.push_str(MARK_CLOSE);
            }
            balanced
        })
        .collect()
}

pub fn poem_snippet(highlight: Option<&str>, content: &str) -> String {
    if let Some(highlighted) = highlight {
        let balanced = balanced_hemistichs(highlighted);
        let hemistichs: Vec<&str> = balanced.iter().map(String::as_str).collect();
        let mut best_start = None;
        let mut best_span = 0;
        let mut index = 0;
        while index < hemistichs.len() {
            let span = longest_mark_span(hemistichs.get(index))
                .max(longest_mark_span(hemistichs.get(index.saturating_add(1))));
            if span > best_span {
                best_span = span;
                best_start = Some(index);
            }
            index = index.saturating_add(2);
        }
        if let Some(start) = best_start {
            let end = start.saturating_add(2).min(hemistichs.len());
            if let Some(verse) = hemistichs.get(start..end) {
                return verse.join("*");
            }
        }
    }
    leading_verse(content)
}

pub fn normalize_query(raw: &str) -> String {
    js::collapse_whitespace(&raw.nfkc().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_verse_with_the_widest_highlight() {
        let highlighted = "one*two <mark>ab</mark>*three*four <mark>abcd</mark>";
        assert_eq!(
            poem_snippet(Some(highlighted), "ignored"),
            "three*four <mark>abcd</mark>"
        );
    }

    #[test]
    fn falls_back_to_the_opening_verse_when_nothing_is_highlighted() {
        assert_eq!(poem_snippet(None, "a*b*c*d"), "a*b");
        assert_eq!(poem_snippet(Some("a*b*c*d"), "x*y*z"), "x*y");
    }

    #[test]
    fn measures_a_span_in_utf16_code_units() {
        assert_eq!(longest_mark_span(Some(&"<mark>ab</mark>")), 2);
        assert_eq!(longest_mark_span(Some(&"<mark>عربي</mark>")), 4);
        assert_eq!(longest_mark_span(Some(&"<mark>\u{1F4DC}</mark>")), 2);
        assert_eq!(longest_mark_span(Some(&"no marks here")), 0);
        assert_eq!(longest_mark_span(Some(&"<mark>unclosed")), 0);
        assert_eq!(longest_mark_span(None), 0);
    }

    #[test]
    fn balances_a_mark_that_spans_the_two_halves_of_a_verse() {
        let highlighted =
            "طلمباتُ الطريق الزراعي*ما تزال في مكانها*«<mark>يا ليلُ،*الصَبُّ</mark> متى غدُه؟";
        assert_eq!(
            poem_snippet(Some(highlighted), "ignored*fallback"),
            "«<mark>يا ليلُ،</mark>*<mark>الصَبُّ</mark> متى غدُه؟"
        );
    }

    #[test]
    fn picks_the_verse_holding_the_longer_part_of_a_mark_that_spans_two_verses() {
        let highlighted = "a*b <mark>cd*efg</mark>*h";
        assert_eq!(
            poem_snippet(Some(highlighted), "ignored*fallback"),
            "<mark>efg</mark>*h"
        );
    }

    #[test]
    fn keeps_the_first_of_two_equal_spans() {
        let highlighted = "<mark>ab</mark>*x*<mark>cd</mark>*y";
        assert_eq!(
            poem_snippet(Some(highlighted), "ignored"),
            "<mark>ab</mark>*x"
        );
    }

    #[test]
    fn a_query_pasted_in_presentation_forms_becomes_plain_letters() {
        let pasted = "\u{FECB}\u{FEE8}\u{FE98}\u{FEAE}\u{FE93} \u{FE91}\u{FEE6} \u{FEB7}\u{FEAA}\u{FE8D}\u{FEA9}";
        assert_eq!(normalize_query(pasted), "عنترة بن شداد");
    }

    #[test]
    fn a_decomposed_hamza_is_composed() {
        assert_eq!(normalize_query("امرو\u{0654} القيس"), "امرؤ القيس");
    }

    #[test]
    fn plain_vocalized_and_digit_queries_pass_through_unchanged() {
        for q in ["قفا نبك من ذكرى", "قِفَا نَبْكِ", "١٩٤٨"] {
            assert_eq!(normalize_query(q), q);
        }
    }
}
