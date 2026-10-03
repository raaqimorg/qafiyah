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
    #[serde(rename = "type")]
    #[schema(value_type = PoemKind)]
    pub kind: &'static str,
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
    pub snippet: String,
    pub poet: PoetRef,
    pub meter: MeterRef,
    pub era: EraRef,
    #[serde(serialize_with = "js::serialize_number")]
    pub relevance: f64,
}

#[derive(Serialize, ToSchema)]
pub struct PoetResult {
    #[serde(rename = "type")]
    #[schema(value_type = PoetKind)]
    pub kind: &'static str,
    pub name: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "yoFB")]
    pub slug: String,
    pub era: EraRef,
    #[serde(serialize_with = "js::serialize_number")]
    pub relevance: f64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoetListItem {
    pub name: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "yoFB")]
    pub slug: String,
    #[schema(example = 42)]
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
