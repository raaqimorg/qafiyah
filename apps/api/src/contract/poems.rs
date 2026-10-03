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

#[derive(Serialize, ToSchema)]
pub struct PoemFacets {
    pub meters: Vec<PoemCountStats>,
    pub rhymes: Vec<PoemCountStats>,
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
