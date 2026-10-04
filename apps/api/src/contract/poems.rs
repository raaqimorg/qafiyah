use serde::Serialize;
use utoipa::ToSchema;
use utoipa::openapi::Schema;

use crate::contract::taxonomy::PoemCountStats;
use crate::contract::{EraRef, MeterRef, PoemTypeRef, PoetRef, RhymeRef, ThemeRef};
use crate::domain::poems::{FacetCounts, Poem, PoemLink, PoemSummary, RecensionLink};

fn verse_list() -> utoipa::openapi::schema::Array {
    use utoipa::openapi::RefOr;
    use utoipa::openapi::schema::ArrayBuilder;

    ArrayBuilder::new()
        .description(Some(
            "The text in order as pairs of half-lines (hemistichs), with diacritics as transmitted. A classical verse is one pair. Lines with no half-line break, as in free verse, are paired two at a time, and the last pair ends with an empty string when the parts are odd in number.",
        ))
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
    /// The poem's title, without diacritics. Most poems are titled by their opening half-line.
    #[schema(example = "أمن أم أوفى دمنة لم تكلم")]
    pub title: String,
    /// The poem's four-letter, case-sensitive slug, for `GET /poems/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "gnNg")]
    pub slug: String,
    /// The poet.
    pub poet: PoetRef,
    /// The poem's meter.
    pub meter: MeterRef,
    /// The poem's era, which is its poet's. Included in `relatedPoems`, left out in `GET /poems` lists.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub era: Option<EraRef>,
}

#[derive(Serialize, ToSchema)]
pub struct PoemNavRef {
    /// The poem's title, without diacritics. Most poems are titled by their opening half-line.
    #[schema(example = "صرمت جديد حبالها أسماء")]
    pub title: String,
    /// The poem's four-letter, case-sensitive slug, for `GET /poems/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "wAJE")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemRecensionRef {
    /// The poem's title, without diacritics. Most poems are titled by their opening half-line.
    #[schema(example = "أمن أم أوفى دمنة لم تكلم")]
    pub title: String,
    /// The poem's four-letter, case-sensitive slug, for `GET /poems/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "wOvI")]
    pub slug: String,
    /// Number of verses as stored in this reading, counted like `verseCount` on a poem.
    #[schema(example = 68)]
    pub verse_count: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemDetail {
    /// The poem's title, without diacritics. Most poems are titled by their opening half-line.
    #[schema(example = "أمن أم أوفى دمنة لم تكلم")]
    pub title: String,
    /// The poem's four-letter, case-sensitive slug, for `GET /poems/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "gnNg")]
    pub slug: String,
    #[schema(schema_with = verse_list)]
    pub verses: Vec<[String; 2]>,
    /// Number of verses as stored, or lines for free verse. It can differ from the length of `verses`, which pairs the text by half-lines.
    #[schema(example = 70)]
    pub verse_count: i32,
    /// The first three half-lines joined by ` * `.
    pub sample: String,
    /// Every word of the text in order, separated by commas.
    pub keywords: String,
    /// The poet.
    pub poet: PoetRef,
    /// The poem's era, which is its poet's.
    pub era: EraRef,
    /// The poem's meter.
    pub meter: MeterRef,
    /// The poem's theme.
    pub theme: ThemeRef,
    /// The poem's rhyme letter.
    pub rhyme: RhymeRef,
    /// The poem's verse form.
    pub poem_type: PoemTypeRef,
    /// The poet's previous poem in the order of `GET /poems?poet=`, primary readings only. Left out for the poet's first poem.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub prev: Option<PoemNavRef>,
    /// The poet's next poem in the order of `GET /poems?poet=`, primary readings only. Left out for the poet's last poem.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub next: Option<PoemNavRef>,
    /// Present when this poem is an alternate reading: the primary reading it belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub recension_of: Option<PoemNavRef>,
    /// The poem's other readings, the primary first. For a primary these are its alternate readings; for an alternate reading, the primary and its siblings. Empty when the poem has one reading.
    pub recensions: Vec<PoemRecensionRef>,
    /// Up to 10 related poems, precomputed. Only primary amudi poems of a known meter, by a named poet of the jahili through mamluki eras, are suggested.
    pub related_poems: Vec<PoemListItem>,
}

#[derive(Serialize, ToSchema)]
pub struct Total {
    /// Number of poems, primary readings only.
    #[schema(example = 342432)]
    pub total: i32,
}

#[derive(Serialize, ToSchema)]
pub struct PoemFacets {
    /// The poet's meters with poem counts under the rhyme and theme filters.
    pub meters: Vec<PoemCountStats>,
    /// The poet's rhymes with poem counts under the meter and theme filters.
    pub rhymes: Vec<PoemCountStats>,
    /// The poet's themes with poem counts under the meter and rhyme filters.
    pub themes: Vec<PoemCountStats>,
}

impl From<PoemSummary> for PoemListItem {
    fn from(poem: PoemSummary) -> Self {
        PoemListItem {
            title: poem.title,
            slug: poem.slug,
            poet: poem.poet.into(),
            meter: poem.meter.into(),
            era: poem.era.map(Into::into),
        }
    }
}

impl From<PoemLink> for PoemNavRef {
    fn from(poem: PoemLink) -> Self {
        PoemNavRef {
            title: poem.title,
            slug: poem.slug,
        }
    }
}

impl From<RecensionLink> for PoemRecensionRef {
    fn from(poem: RecensionLink) -> Self {
        PoemRecensionRef {
            title: poem.title,
            slug: poem.slug,
            verse_count: poem.verse_count,
        }
    }
}

impl From<Poem> for PoemDetail {
    fn from(poem: Poem) -> Self {
        PoemDetail {
            title: poem.title,
            slug: poem.slug,
            verses: poem.verses,
            verse_count: poem.verse_count,
            sample: poem.sample,
            keywords: poem.keywords,
            poet: poem.poet.into(),
            era: poem.era.into(),
            meter: poem.meter.into(),
            theme: poem.theme.into(),
            rhyme: poem.rhyme.into(),
            poem_type: poem.poem_type.into(),
            prev: poem.prev.map(Into::into),
            next: poem.next.map(Into::into),
            recension_of: poem.recension_of.map(Into::into),
            recensions: poem.recensions.into_iter().map(Into::into).collect(),
            related_poems: poem.related.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<FacetCounts> for PoemFacets {
    fn from(counts: FacetCounts) -> Self {
        PoemFacets {
            meters: counts.meters.into_iter().map(Into::into).collect(),
            rhymes: counts.rhymes.into_iter().map(Into::into).collect(),
            themes: counts.themes.into_iter().map(Into::into).collect(),
        }
    }
}
