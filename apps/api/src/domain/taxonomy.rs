use async_trait::async_trait;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::StoreError;

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

#[derive(Clone, Copy)]
pub enum PoemCounted {
    Themes,
    Collections,
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
