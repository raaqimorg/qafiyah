use async_trait::async_trait;

use crate::domain::StoreError;
use crate::domain::Term;
use crate::domain::paging::{Cursor, Slice};

pub struct PoetProfile {
    pub name: String,
    pub slug: String,
    pub nickname: Option<String>,
    pub bio: Option<String>,
    pub era: Term,
    pub poems_count: i32,
    pub has_avatar: bool,
}

pub struct PoetSlug {
    pub slug: String,
    pub has_avatar: bool,
}

#[async_trait]
pub trait PoetRepository: Send + Sync {
    async fn get(&self, slug: &str) -> Result<Option<PoetProfile>, StoreError>;
    async fn alias_target(&self, slug: &str) -> Result<Option<String>, StoreError>;
    async fn count_with_poems(&self) -> Result<i32, StoreError>;
    async fn list_slugs(
        &self,
        cursor: Option<&Cursor<i32>>,
        page_size: u32,
    ) -> Result<Slice<PoetSlug, i32>, StoreError>;
}
