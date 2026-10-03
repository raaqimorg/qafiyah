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
