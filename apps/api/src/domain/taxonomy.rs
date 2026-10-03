use async_trait::async_trait;

use crate::error::StoreError;

pub struct TermStats {
    pub name: String,
    pub slug: String,
    pub poems_count: i32,
    pub poets_count: i32,
}

pub struct TermCount {
    pub name: String,
    pub slug: String,
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
    async fn list_counted(&self, kind: Counted) -> Result<Vec<TermStats>, StoreError>;
    async fn get_counted(&self, kind: Counted, slug: &str)
    -> Result<Option<TermStats>, StoreError>;
    async fn list_by_poem_count(&self, kind: PoemCounted) -> Result<Vec<TermCount>, StoreError>;
    async fn get_by_poem_count(
        &self,
        kind: PoemCounted,
        slug: &str,
    ) -> Result<Option<TermCount>, StoreError>;
}
