use std::cmp::Reverse;

use async_trait::async_trait;

use crate::constants::{MAX_TWEET_LENGTH, RANDOM_POEM_MAX_ATTEMPTS};
use crate::domain::StoreError;
use crate::domain::taxonomy::TermCount;
use crate::domain::{PoetBrief, Term};
use crate::js;

pub struct PoemSummary {
    pub title: String,
    pub slug: String,
    pub poet: PoetBrief,
    pub meter: Term,
    pub era: Option<Term>,
}

pub struct PoemLink {
    pub title: String,
    pub slug: String,
}

pub struct RecensionLink {
    pub title: String,
    pub slug: String,
    pub verse_count: i32,
}

pub struct Poem {
    pub title: String,
    pub slug: String,
    pub verses: Vec<Vec<String>>,
    pub verse_count: i32,
    pub sample: String,
    pub keywords: String,
    pub poet: PoetBrief,
    pub era: Term,
    pub meter: Term,
    pub theme: Term,
    pub rhyme: Term,
    pub poem_type: Term,
    pub prev: Option<PoemLink>,
    pub next: Option<PoemLink>,
    pub recension_of: Option<PoemLink>,
    pub recensions: Vec<RecensionLink>,
    pub related: Vec<PoemSummary>,
}

#[derive(Default)]
pub struct Facets {
    pub poet: Vec<String>,
    pub era: Vec<String>,
    pub theme: Vec<String>,
    pub meter: Vec<String>,
    pub rhyme: Vec<String>,
    pub collection: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    Poet,
    Era,
    Meter,
    Theme,
    Rhyme,
    Collection,
}

impl Filter {
    pub const ALL: [Filter; 6] = [
        Filter::Poet,
        Filter::Era,
        Filter::Meter,
        Filter::Theme,
        Filter::Rhyme,
        Filter::Collection,
    ];
}

impl Facets {
    pub fn values(&self, filter: Filter) -> &Vec<String> {
        match filter {
            Filter::Poet => &self.poet,
            Filter::Era => &self.era,
            Filter::Meter => &self.meter,
            Filter::Theme => &self.theme,
            Filter::Rhyme => &self.rhyme,
            Filter::Collection => &self.collection,
        }
    }
}

pub struct FacetCounts {
    pub meters: Vec<TermCount>,
    pub rhymes: Vec<TermCount>,
    pub themes: Vec<TermCount>,
}

pub struct Recension {
    pub id: i32,
    pub title: String,
    pub slug: String,
    pub verse_count: i32,
}

pub struct PoemRecord {
    pub title: String,
    pub verse_count: i32,
    pub recension_of_id: Option<i32>,
    pub poet: PoetBrief,
    pub era: Term,
    pub meter: Term,
    pub theme: Term,
    pub rhyme: Term,
    pub poem_type: Term,
    pub lines: Vec<String>,
    pub prev: Option<PoemLink>,
    pub next: Option<PoemLink>,
    pub family: Vec<Recension>,
    pub related: Vec<PoemSummary>,
}

pub struct RandomPoem {
    pub poet_name: String,
    pub content: String,
    pub slug: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PoemError {
    #[error("the poet filter names no shown poet")]
    PoetNotShown,
    #[error("the poem has no verses")]
    MissingVerses,
    #[error("no random poem could be excerpted")]
    NoRandomPoem,
    #[error(transparent)]
    Store(#[from] StoreError),
}

#[async_trait]
pub trait PoemRepository: Send + Sync {
    async fn count(&self) -> Result<i32, StoreError>;
    async fn list_slugs(&self, page: u32, page_size: u32) -> Result<Vec<String>, StoreError>;
    async fn list(
        &self,
        facets: &Facets,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<PoemSummary>, i32), StoreError>;
    async fn facet_counts(&self, facets: &Facets) -> Result<Option<FacetCounts>, StoreError>;
    async fn find(&self, slug: &str) -> Result<Option<PoemRecord>, StoreError>;
    async fn alias_target(&self, slug: &str) -> Result<Option<String>, StoreError>;
    async fn random(&self) -> Result<Option<RandomPoem>, StoreError>;
}

pub struct ParsedContent {
    pub verses: Vec<Vec<String>>,
    pub sample: String,
    pub keywords: String,
}

pub fn parse_poem_rows(rows: &[String]) -> ParsedContent {
    let verses: Vec<Vec<String>> = rows
        .iter()
        .map(|row| row.split('*').map(str::to_string).collect())
        .collect();
    let parts: Vec<&str> = verses.iter().flatten().map(String::as_str).collect();
    ParsedContent {
        sample: parts
            .iter()
            .take(3)
            .copied()
            .collect::<Vec<&str>>()
            .join(" * "),
        keywords: parts.join(" ").replace(' ', ","),
        verses,
    }
}

fn detail(slug: &str, record: PoemRecord) -> Result<Poem, PoemError> {
    let recension_of = record.recension_of_id.and_then(|primary| {
        record
            .family
            .iter()
            .find(|relative| relative.id == primary)
            .map(|relative| PoemLink {
                title: relative.title.clone(),
                slug: relative.slug.clone(),
            })
    });
    let recensions = record
        .family
        .into_iter()
        .map(|relative| RecensionLink {
            title: relative.title,
            slug: relative.slug,
            verse_count: relative.verse_count,
        })
        .collect();
    if record.lines.is_empty() {
        return Err(PoemError::MissingVerses);
    }
    let parsed = parse_poem_rows(&record.lines);

    Ok(Poem {
        title: record.title,
        slug: slug.to_string(),
        verses: parsed.verses,
        verse_count: record.verse_count,
        sample: parsed.sample,
        keywords: parsed.keywords,
        poet: record.poet,
        era: record.era,
        meter: record.meter,
        theme: record.theme,
        rhyme: record.rhyme,
        poem_type: record.poem_type,
        prev: record.prev,
        next: record.next,
        recension_of,
        recensions,
        related: record.related,
    })
}

pub async fn get(poems: &dyn PoemRepository, slug: &str) -> Result<Option<Poem>, PoemError> {
    match poems.find(slug).await? {
        Some(record) => detail(slug, record).map(Some),
        None => Ok(None),
    }
}

fn shown_terms(terms: Vec<TermCount>, selected: &[String]) -> Vec<TermCount> {
    let mut shown: Vec<TermCount> = terms
        .into_iter()
        .filter(|term| term.poems_count > 0 || selected.contains(&term.slug))
        .collect();
    shown.sort_by_key(|term| Reverse(term.poems_count));
    shown
}

pub async fn facets(poems: &dyn PoemRepository, facets: &Facets) -> Result<FacetCounts, PoemError> {
    let counts = poems
        .facet_counts(facets)
        .await?
        .ok_or(PoemError::PoetNotShown)?;
    Ok(FacetCounts {
        meters: shown_terms(counts.meters, &facets.meter),
        rhymes: shown_terms(counts.rhymes, &facets.rhyme),
        themes: shown_terms(counts.themes, &facets.theme),
    })
}

pub enum RandomPoemOption {
    Slug,
    Lines,
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "excerpt math runs on a poem's small hemistich count"
)]
#[expect(
    clippy::as_conversions,
    reason = "the float casts stay within the verse count, bounded by a short poem"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the floored roll is at most the verse count"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "the verse count and roll are non-negative"
)]
fn excerpt_start(line_count: usize, roll: f64) -> usize {
    let max_start = line_count.saturating_sub(2);
    let verse_count = max_start / 2 + 1;
    (((roll * verse_count as f64).floor() as usize) * 2).min(max_start)
}

fn build_excerpt(poem: &RandomPoem, roll: f64) -> Option<String> {
    let lines: Vec<&str> = poem.content.split('*').collect();
    if lines.len() < 2 {
        return None;
    }
    let start = excerpt_start(lines.len(), roll);
    let first = lines.get(start).copied().unwrap_or_default();
    let second = lines
        .get(start.saturating_add(1))
        .copied()
        .unwrap_or_default();
    let excerpt = js::trim(&format!("{first}\n{second}\n\n{}", poem.poet_name)).to_string();
    (excerpt.encode_utf16().count() <= MAX_TWEET_LENGTH).then_some(excerpt)
}

async fn fetch_random_poem(poems: &dyn PoemRepository) -> Result<RandomPoem, PoemError> {
    poems.random().await?.ok_or(PoemError::NoRandomPoem)
}

pub async fn random(
    poems: &dyn PoemRepository,
    option: &RandomPoemOption,
    roll: f64,
) -> Result<String, PoemError> {
    match option {
        RandomPoemOption::Slug => Ok(fetch_random_poem(poems).await?.slug),
        RandomPoemOption::Lines => {
            for _ in 0..RANDOM_POEM_MAX_ATTEMPTS {
                if let Some(excerpt) = build_excerpt(&fetch_random_poem(poems).await?, roll) {
                    return Ok(excerpt);
                }
            }
            Err(PoemError::NoRandomPoem)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(items: &[&str]) -> Vec<String> {
        items.iter().map(|row| (*row).to_string()).collect()
    }

    #[test]
    fn each_stored_row_is_one_entry_holding_its_parts() {
        let parsed = parse_poem_rows(&rows(&["a*b", "c", "d*e"]));
        assert_eq!(
            parsed.verses,
            vec![vec!["a", "b"], vec!["c"], vec!["d", "e"]]
        );
    }

    #[test]
    fn a_lone_line_never_shifts_the_verses_after_it() {
        let parsed = parse_poem_rows(&rows(&["a1*a2", "c", "d1*d2", "e1*e2"]));
        assert_eq!(parsed.verses[2], vec!["d1", "d2"]);
        assert_eq!(parsed.verses[3], vec!["e1", "e2"]);
    }

    #[test]
    fn a_row_with_extra_breaks_keeps_every_part() {
        let parsed = parse_poem_rows(&rows(&["a*b*c"]));
        assert_eq!(parsed.verses, vec![vec!["a", "b", "c"]]);
    }

    #[test]
    fn an_empty_row_is_one_empty_line() {
        let parsed = parse_poem_rows(&rows(&["a*b", "", "c*d"]));
        assert_eq!(parsed.verses[1], vec![""]);
    }

    #[test]
    fn the_sample_is_the_first_three_parts_and_keywords_are_every_word() {
        let parsed = parse_poem_rows(&rows(&["one two*three", "four", "five*six"]));
        assert_eq!(parsed.sample, "one two * three * four");
        assert_eq!(parsed.keywords, "one,two,three,four,five,six");
    }

    #[test]
    fn turns_every_space_into_a_comma() {
        let parsed = parse_poem_rows(&rows(&["one two*three  four"]));
        assert_eq!(parsed.keywords, "one,two,three,,four");
    }

    #[test]
    fn starts_an_excerpt_on_a_verse_boundary() {
        for line_count in [2usize, 3, 4, 5, 6, 7, 20] {
            let max_start = line_count - 2;
            for step in 0..100 {
                let start = excerpt_start(line_count, f64::from(step) / 100.0);
                assert_eq!(start % 2, 0, "{line_count} hemistichs at step {step}");
                assert!(start <= max_start, "{line_count} hemistichs at step {step}");
            }
        }
    }

    #[test]
    fn reaches_every_verse_including_the_last() {
        assert_eq!(excerpt_start(6, 0.0), 0);
        assert_eq!(excerpt_start(6, 0.5), 2);
        assert_eq!(excerpt_start(6, 0.99), 4);
        assert_eq!(excerpt_start(4, 0.99), 2);
        assert_eq!(excerpt_start(2, 0.99), 0);
    }

    fn random_poem(content: &str) -> RandomPoem {
        RandomPoem {
            poet_name: "poet".into(),
            content: content.into(),
            slug: "slug".into(),
        }
    }

    #[test]
    fn rejects_a_fragment_with_fewer_than_two_hemistichs() {
        assert!(build_excerpt(&random_poem("one lone hemistich"), 0.0).is_none());
        assert!(build_excerpt(&random_poem(""), 0.0).is_none());
    }

    #[test]
    fn rejects_an_excerpt_longer_than_a_tweet() {
        let long_line = "a".repeat(MAX_TWEET_LENGTH);
        let poem = random_poem(&format!("{long_line}*{long_line}"));
        assert!(build_excerpt(&poem, 0.0).is_none());
    }

    #[test]
    fn builds_the_first_and_second_hemistich_with_the_poet_name() {
        let poem = random_poem("first*second*third*fourth");
        let excerpt = build_excerpt(&poem, 0.0).expect("well-formed poem builds an excerpt");
        assert_eq!(excerpt, "first\nsecond\n\npoet");
    }

    #[test]
    fn row_parsing_never_panics_and_keeps_every_part_in_its_row() {
        let mut rng = crate::test_support::Rng::new(3);
        let alphabet = ["*", "ا", "ب", " ", "**", "\n", "\u{00a0}", "😀"];
        for _ in 0..3_000 {
            let row_count = rng.below(6);
            let stored: Vec<String> = (0..row_count)
                .map(|_| (0..rng.below(12)).map(|_| rng.pick(&alphabet)).collect())
                .collect();
            let parsed = parse_poem_rows(&stored);
            assert_eq!(parsed.verses.len(), stored.len());
            for (entry, row) in parsed.verses.iter().zip(&stored) {
                assert_eq!(entry.join("*"), *row);
            }
            let parts: Vec<&str> = stored.iter().flat_map(|row| row.split('*')).collect();
            assert_eq!(
                parsed.sample,
                parts
                    .iter()
                    .take(3)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" * ")
            );
            assert!(!parsed.keywords.contains(' '));
        }
    }

    #[test]
    fn consecutive_delimiters_produce_empty_parts_rather_than_being_collapsed() {
        let parsed = parse_poem_rows(&rows(&["a**b"]));
        assert_eq!(parsed.verses, vec![vec!["a", "", "b"]]);
        assert_eq!(parsed.sample, "a *  * b");
    }

    #[test]
    fn an_excerpt_at_exactly_the_tweet_cap_is_accepted_and_one_over_is_refused() {
        let hemistich = "ا".repeat(138);
        let at_cap = RandomPoem {
            poet_name: "x".into(),
            content: format!("{hemistich}*{hemistich}"),
            slug: "TnKK".into(),
        };
        let excerpt = build_excerpt(&at_cap, 0.0).expect("exactly 280 units");
        assert_eq!(excerpt.encode_utf16().count(), MAX_TWEET_LENGTH);
        let over = RandomPoem {
            poet_name: "xy".into(),
            content: format!("{hemistich}*{hemistich}"),
            slug: "TnKK".into(),
        };
        assert!(build_excerpt(&over, 0.0).is_none());
    }
}
