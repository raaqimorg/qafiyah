pub mod poems;
pub mod poets;
pub mod search;
pub mod taxonomy;

pub struct Term {
    pub name: String,
    pub slug: String,
}

pub struct PoetBrief {
    pub name: String,
    pub slug: String,
    pub has_avatar: bool,
    pub is_anonymous: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(String),
    #[error("search error: {0}")]
    Search(String),
}
