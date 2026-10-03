use std::cmp::Reverse;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa::openapi::Schema;

use crate::constants::{MAX_TWEET_LENGTH, RANDOM_POEM_MAX_ATTEMPTS};
use crate::domain::taxonomy::PoemCountStats;
use crate::domain::{EraRef, MeterRef, PoemTypeRef, PoetRef, RhymeRef, ThemeRef};
use crate::error::{AppError, Resource, RouteProblem, StoreError};
use crate::js;

fn verse_list() -> utoipa::openapi::schema::Array {
    use utoipa::openapi::RefOr;
    use utoipa::openapi::schema::ArrayBuilder;

    ArrayBuilder::new()
        .items(RefOr::T(Schema::Array(hemistich_pair())))
        .build()
}

fn hemistich_pair() -> utoipa::openapi::schema::Array {
    use utoipa::openapi::schema::{ArrayBuilder, ArrayItems, ObjectBuilder, Type};

    let hemistich = || Schema::Object(ObjectBuilder::new().schema_type(Type::String).build());
    ArrayBuilder::new()
        .prefix_items([hemistich(), hemistich()])
        .items(ArrayItems::False)
        .min_items(Some(2))
        .max_items(Some(2))
        .build()
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemListItem {
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
    pub poet: PoetRef,
    pub meter: MeterRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub era: Option<EraRef>,
}

#[derive(Serialize, ToSchema)]
pub struct PoemNavRef {
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemRecensionRef {
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
    pub verse_count: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemDetail {
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
    #[schema(schema_with = verse_list)]
    pub verses: Vec<[String; 2]>,
    #[schema(example = 10)]
    pub verse_count: i32,
    pub sample: String,
    pub keywords: String,
    pub poet: PoetRef,
    pub era: EraRef,
    pub meter: MeterRef,
    pub theme: ThemeRef,
    pub rhyme: RhymeRef,
    pub poem_type: PoemTypeRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub prev: Option<PoemNavRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub next: Option<PoemNavRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub recension_of: Option<PoemNavRef>,
    pub recensions: Vec<PoemRecensionRef>,
    pub related_poems: Vec<PoemListItem>,
}

#[derive(Serialize, ToSchema)]
pub struct Total {
    #[schema(example = 394174)]
    pub total: i32,
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

#[derive(Serialize, ToSchema)]
pub struct PoemFacets {
    pub meters: Vec<PoemCountStats>,
    pub rhymes: Vec<PoemCountStats>,
    pub themes: Vec<PoemCountStats>,
}

pub struct FacetCounts {
    pub meters: Vec<PoemCountStats>,
    pub rhymes: Vec<PoemCountStats>,
    pub themes: Vec<PoemCountStats>,
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
    pub poet: PoetRef,
    pub era: EraRef,
    pub meter: MeterRef,
    pub theme: ThemeRef,
    pub rhyme: RhymeRef,
    pub poem_type: PoemTypeRef,
    pub lines: Vec<String>,
    pub prev: Option<PoemNavRef>,
    pub next: Option<PoemNavRef>,
    pub family: Vec<Recension>,
    pub related: Vec<PoemListItem>,
}

#[derive(Deserialize)]
pub struct RandomPoem {
    pub poet_name: String,
    pub content: String,
    pub slug: String,
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
    ) -> Result<(Vec<PoemListItem>, i32), StoreError>;
    async fn facet_counts(&self, facets: &Facets) -> Result<Option<FacetCounts>, StoreError>;
    async fn find(&self, slug: &str) -> Result<Option<PoemRecord>, StoreError>;
    async fn alias_target(&self, slug: &str) -> Result<Option<String>, StoreError>;
    async fn random(&self) -> Result<Option<RandomPoem>, StoreError>;
}

pub struct ParsedContent {
    pub verses: Vec<[String; 2]>,
    pub sample: String,
    pub keywords: String,
}

pub fn parse_poem_content(content: &str) -> ParsedContent {
    let lines: Vec<&str> = content.split('*').collect();
    ParsedContent {
        verses: lines
            .chunks(2)
            .map(|pair| {
                let first = pair.first().copied().unwrap_or_default();
                let second = pair.get(1).copied().unwrap_or_default();
                [first.to_string(), second.to_string()]
            })
            .collect(),
        sample: lines
            .iter()
            .take(3)
            .copied()
            .collect::<Vec<&str>>()
            .join(" * "),
        keywords: lines.join(" ").replace(' ', ","),
    }
}

fn detail(slug: &str, record: PoemRecord) -> Result<PoemDetail, AppError> {
    let recension_of = record.recension_of_id.and_then(|primary| {
        record
            .family
            .iter()
            .find(|relative| relative.id == primary)
            .map(|relative| PoemNavRef {
                title: relative.title.clone(),
                slug: relative.slug.clone(),
            })
    });
    let recensions = record
        .family
        .into_iter()
        .map(|relative| PoemRecensionRef {
            title: relative.title,
            slug: relative.slug,
            verse_count: relative.verse_count,
        })
        .collect();
    if record.lines.is_empty() {
        return Err(AppError::PoemParse);
    }
    let parsed = parse_poem_content(&record.lines.join("*"));

    Ok(PoemDetail {
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
        related_poems: record.related,
    })
}

pub async fn get(poems: &dyn PoemRepository, slug: &str) -> Result<Option<PoemDetail>, AppError> {
    match poems.find(slug).await? {
        Some(record) => detail(slug, record).map(Some),
        None => Ok(None),
    }
}

fn shown_terms(terms: Vec<PoemCountStats>, selected: &[String]) -> Vec<PoemCountStats> {
    let mut shown: Vec<PoemCountStats> = terms
        .into_iter()
        .filter(|term| term.poems_count > 0 || selected.contains(&term.slug))
        .collect();
    shown.sort_by_key(|term| Reverse(term.poems_count));
    shown
}

pub async fn facets(poems: &dyn PoemRepository, facets: &Facets) -> Result<PoemFacets, AppError> {
    let counts = poems
        .facet_counts(facets)
        .await?
        .ok_or(AppError::NotFound(Resource::Poet))?;
    Ok(PoemFacets {
        meters: shown_terms(counts.meters, &facets.meter),
        rhymes: shown_terms(counts.rhymes, &facets.rhyme),
        themes: shown_terms(counts.themes, &facets.theme),
    })
}

pub enum RandomPoemOption {
    Slug,
    Lines,
}

impl RandomPoemOption {
    pub fn parse(raw: Option<&str>) -> Result<Self, AppError> {
        match raw {
            None | Some("slug") => Ok(RandomPoemOption::Slug),
            Some("lines") => Ok(RandomPoemOption::Lines),
            Some(_) => Err(RouteProblem::bad_request(
                "Invalid ?option value (expected 'slug' or 'lines')",
            )
            .into()),
        }
    }
}

fn random_poem_failed() -> AppError {
    RouteProblem::internal("Failed to fetch random poem").into()
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

fn build_excerpt(poem: &RandomPoem, roll: f64) -> Result<String, AppError> {
    let lines: Vec<&str> = poem.content.split('*').collect();
    if lines.len() < 2 {
        return Err(random_poem_failed());
    }
    let start = excerpt_start(lines.len(), roll);
    let first = lines.get(start).copied().unwrap_or_default();
    let second = lines
        .get(start.saturating_add(1))
        .copied()
        .unwrap_or_default();
    let excerpt = js::trim(&format!("{first}\n{second}\n\n{}", poem.poet_name)).to_string();
    if excerpt.encode_utf16().count() > MAX_TWEET_LENGTH {
        return Err(random_poem_failed());
    }
    Ok(excerpt)
}

async fn fetch_random_poem(poems: &dyn PoemRepository) -> Result<RandomPoem, AppError> {
    poems.random().await?.ok_or_else(random_poem_failed)
}

pub async fn random(
    poems: &dyn PoemRepository,
    option: &RandomPoemOption,
    roll: f64,
) -> Result<String, AppError> {
    match option {
        RandomPoemOption::Slug => Ok(fetch_random_poem(poems).await?.slug),
        RandomPoemOption::Lines => {
            let mut last_err = None;
            for _ in 0..RANDOM_POEM_MAX_ATTEMPTS {
                let poem = fetch_random_poem(poems).await?;
                match build_excerpt(&poem, roll) {
                    Ok(excerpt) => return Ok(excerpt),
                    Err(err) => last_err = Some(err),
                }
            }
            Err(last_err.unwrap_or_else(random_poem_failed))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_hemistichs_and_leaves_an_odd_one_half_empty() {
        let parsed = parse_poem_content("a*b*c");
        assert_eq!(parsed.verses, [["a", "b"], ["c", ""]]);
    }

    #[test]
    fn samples_the_first_three_hemistichs() {
        let parsed = parse_poem_content("a*b*c*d");
        assert_eq!(parsed.sample, "a * b * c");
    }

    #[test]
    fn turns_every_space_into_a_comma() {
        let parsed = parse_poem_content("one two*three  four");
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

    #[test]
    fn empty_content_is_one_empty_verse() {
        let parsed = parse_poem_content("");
        assert_eq!(parsed.verses, [["", ""]]);
        assert_eq!(parsed.sample, "");
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
        assert!(build_excerpt(&random_poem("one lone hemistich"), 0.0).is_err());
        assert!(build_excerpt(&random_poem(""), 0.0).is_err());
    }

    #[test]
    fn rejects_an_excerpt_longer_than_a_tweet() {
        let long_line = "a".repeat(MAX_TWEET_LENGTH);
        let poem = random_poem(&format!("{long_line}*{long_line}"));
        assert!(build_excerpt(&poem, 0.0).is_err());
    }

    #[test]
    fn builds_the_first_and_second_hemistich_with_the_poet_name() {
        let poem = random_poem("first*second*third*fourth");
        let excerpt = build_excerpt(&poem, 0.0).expect("well-formed poem builds an excerpt");
        assert_eq!(excerpt, "first\nsecond\n\npoet");
    }

    #[test]
    fn content_parsing_never_panics_and_keeps_every_hemistich() {
        let mut rng = crate::test_support::Rng::new(3);
        let alphabet = ["*", "ا", "ب", " ", "**", "\n", "\u{00a0}", "😀"];
        for _ in 0..3_000 {
            let len = rng.below(30);
            let content: String = (0..len).map(|_| rng.pick(&alphabet)).collect();
            let parsed = parse_poem_content(&content);
            let hemistichs: Vec<&str> = content.split('*').collect();
            assert_eq!(parsed.verses.len(), hemistichs.len().div_ceil(2));
            assert_eq!(
                parsed.sample,
                hemistichs
                    .iter()
                    .take(3)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" * ")
            );
            assert!(!parsed.keywords.contains(' '));
            for (index, hemistich) in hemistichs.iter().enumerate() {
                assert_eq!(parsed.verses[index / 2][index % 2], *hemistich);
            }
        }
    }

    #[test]
    fn consecutive_delimiters_produce_empty_hemistichs_rather_than_being_collapsed() {
        let parsed = parse_poem_content("a**b");
        assert_eq!(parsed.verses, [["a", ""], ["b", ""]]);
        assert_eq!(parsed.sample, "a *  * b");
    }

    #[test]
    fn the_random_option_accepts_only_the_two_documented_values() {
        assert!(matches!(
            RandomPoemOption::parse(None),
            Ok(RandomPoemOption::Slug)
        ));
        assert!(matches!(
            RandomPoemOption::parse(Some("slug")),
            Ok(RandomPoemOption::Slug)
        ));
        assert!(matches!(
            RandomPoemOption::parse(Some("lines")),
            Ok(RandomPoemOption::Lines)
        ));
        for raw in ["", "Lines", "verses", "slug "] {
            assert!(RandomPoemOption::parse(Some(raw)).is_err(), "{raw}");
        }
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
        assert!(build_excerpt(&over, 0.0).is_err());
    }
}
