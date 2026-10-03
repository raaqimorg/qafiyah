use serde::Serialize;
use utoipa::ToSchema;

use crate::contract::EraRef;
use crate::domain::poets::{PoetProfile, PoetSlug};

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

impl From<PoetProfile> for PoetStats {
    fn from(poet: PoetProfile) -> Self {
        PoetStats {
            name: poet.name,
            slug: poet.slug,
            nickname: poet.nickname,
            bio: poet.bio,
            era: poet.era.into(),
            poems_count: poet.poems_count,
            has_avatar: poet.has_avatar,
        }
    }
}

impl From<PoetSlug> for PoetSlugEntry {
    fn from(poet: PoetSlug) -> Self {
        PoetSlugEntry {
            slug: poet.slug,
            has_avatar: poet.has_avatar,
        }
    }
}
