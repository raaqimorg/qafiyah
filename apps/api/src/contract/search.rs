use serde::Serialize;
use utoipa::ToSchema;

use crate::contract::{EraRef, MeterRef, PoetRef};
use crate::domain::search::{PoemHit, PoetHit, PoetListing};
use crate::js;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PoemKind {
    Poem,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PoetKind {
    Poet,
}

#[derive(Serialize, ToSchema)]
pub struct PoemResult {
    /// Always `poem`.
    #[serde(rename = "type")]
    #[schema(value_type = PoemKind)]
    pub kind: &'static str,
    /// The poem's title, without diacritics. Most poems are titled by their opening half-line.
    #[schema(example = "أمن أم أوفى دمنة لم تكلم")]
    pub title: String,
    /// The poem's four-letter, case-sensitive slug, for `GET /poems/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "gnNg")]
    pub slug: String,
    /// The best-matching verse, its two half-lines joined by `*`, with matched words wrapped in `<mark>`. The opening verse when no verse text matched, as on a title match or when browsing.
    #[schema(example = "<mark>أَََمِنْ أُمِّ أَوْفَى</mark> دِمْنَةٌ لمَ ْتَكَلَّمِ*بحُِوْمَانَةِ الدَّرَّاجِ فَالْمُتَثَلَّمِ")]
    pub snippet: String,
    /// The poet.
    pub poet: PoetRef,
    /// The poem's meter.
    pub meter: MeterRef,
    /// The poem's era, which is its poet's.
    pub era: EraRef,
    /// The raw search score, higher first, comparable only within the poems section of one response. When browsing (no `q`) it only puts the classical eras first: 1.1 for a poem of the jahili through mamluki eras and 1 for any other, or 0 under an era filter.
    #[serde(serialize_with = "js::serialize_number")]
    #[schema(example = 119903610)]
    pub relevance: f64,
}

#[derive(Serialize, ToSchema)]
pub struct PoetResult {
    /// Always `poet`.
    #[serde(rename = "type")]
    #[schema(value_type = PoetKind)]
    pub kind: &'static str,
    /// The poet's name in Arabic.
    #[schema(example = "زهير بن أبي سلمى")]
    pub name: String,
    /// The poet's four-letter, case-sensitive slug, for `GET /poets/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "PAKT")]
    pub slug: String,
    /// The poet's era.
    pub era: EraRef,
    /// The raw search score, higher first, comparable only within the poets section of one response. 0 when browsing (no `q`).
    #[serde(serialize_with = "js::serialize_number")]
    #[schema(example = 341.06555)]
    pub relevance: f64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoetListItem {
    /// The poet's name in Arabic.
    #[schema(example = "زهير بن أبي سلمى")]
    pub name: String,
    /// The poet's four-letter, case-sensitive slug, for `GET /poets/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "PAKT")]
    pub slug: String,
    /// Number of the poet's poems, primary readings only.
    #[schema(example = 104)]
    pub poems_count: i64,
}

impl From<PoemHit> for PoemResult {
    fn from(hit: PoemHit) -> Self {
        PoemResult {
            kind: "poem",
            title: hit.title,
            slug: hit.slug,
            snippet: hit.snippet,
            poet: hit.poet.into(),
            meter: hit.meter.into(),
            era: hit.era.into(),
            relevance: hit.relevance,
        }
    }
}

impl From<PoetHit> for PoetResult {
    fn from(hit: PoetHit) -> Self {
        PoetResult {
            kind: "poet",
            name: hit.name,
            slug: hit.slug,
            era: hit.era.into(),
            relevance: hit.relevance,
        }
    }
}

impl From<PoetListing> for PoetListItem {
    fn from(poet: PoetListing) -> Self {
        PoetListItem {
            name: poet.name,
            slug: poet.slug,
            poems_count: poet.poems_count,
        }
    }
}
