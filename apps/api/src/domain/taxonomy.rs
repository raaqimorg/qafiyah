use async_trait::async_trait;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::{Resource, StoreError};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountedStats {
    pub name: String,
    #[schema(pattern = "^[a-z][a-z-]*$", example = "altawil")]
    pub slug: String,
    #[schema(example = 44474)]
    pub poems_count: i32,
    #[schema(example = 3637)]
    pub poets_count: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemCountStats {
    pub name: String,
    #[schema(pattern = "^[a-z][a-z-]*$", example = "alnasib")]
    pub slug: String,
    #[schema(example = 47457)]
    pub poems_count: i32,
}

#[derive(Clone, Copy)]
pub enum Counted {
    Meters,
    Rhymes,
    Eras,
    PoemTypes,
}

impl Counted {
    pub fn resource(self) -> Resource {
        match self {
            Counted::Meters => Resource::Meter,
            Counted::Rhymes => Resource::Rhyme,
            Counted::Eras => Resource::Era,
            Counted::PoemTypes => Resource::PoemType,
        }
    }

    pub fn log_field(self) -> &'static str {
        match self {
            Counted::Meters => "meter",
            Counted::Rhymes => "rhyme",
            Counted::Eras => "era",
            Counted::PoemTypes => "poem_type",
        }
    }
}

#[derive(Clone, Copy)]
pub enum PoemCounted {
    Themes,
    Collections,
}

impl PoemCounted {
    pub fn resource(self) -> Resource {
        match self {
            PoemCounted::Themes => Resource::Theme,
            PoemCounted::Collections => Resource::Collection,
        }
    }

    pub fn log_field(self) -> &'static str {
        match self {
            PoemCounted::Themes => "theme",
            PoemCounted::Collections => "collection",
        }
    }
}

#[async_trait]
pub trait TaxonomyRepository: Send + Sync {
    async fn list_counted(&self, kind: Counted) -> Result<Vec<CountedStats>, StoreError>;
    async fn get_counted(
        &self,
        kind: Counted,
        slug: &str,
    ) -> Result<Option<CountedStats>, StoreError>;
    async fn list_by_poem_count(
        &self,
        kind: PoemCounted,
    ) -> Result<Vec<PoemCountStats>, StoreError>;
    async fn get_by_poem_count(
        &self,
        kind: PoemCounted,
        slug: &str,
    ) -> Result<Option<PoemCountStats>, StoreError>;
}
