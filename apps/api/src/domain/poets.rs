use async_trait::async_trait;
use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::EraRef;
use crate::error::StoreError;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoetStats {
    pub name: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "yoFB")]
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub bio: Option<String>,
    pub era: EraRef,
    #[schema(example = 2967)]
    pub poems_count: i32,
    pub has_avatar: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoetSlugEntry {
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "yoFB")]
    pub slug: String,
    pub has_avatar: bool,
}

#[async_trait]
pub trait PoetRepository: Send + Sync {
    async fn get(&self, slug: &str) -> Result<Option<PoetStats>, StoreError>;
    async fn alias_target(&self, slug: &str) -> Result<Option<String>, StoreError>;
    async fn count_with_poems(&self) -> Result<i32, StoreError>;
    async fn list_slugs(&self, page: u32, page_size: u32)
    -> Result<Vec<PoetSlugEntry>, StoreError>;
}
