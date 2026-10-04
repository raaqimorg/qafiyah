use serde::Serialize;
use utoipa::ToSchema;

use crate::contract::EraRef;
use crate::domain::poets::{PoetProfile, PoetSlug};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoetStats {
    /// The poet's name in Arabic.
    #[schema(example = "زهير بن أبي سلمى")]
    pub name: String,
    /// The poet's four-letter, case-sensitive slug, for `GET /poets/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "PAKT")]
    pub slug: String,
    /// Another name the poet is known by, such as the kunya أبو الطيب for المتنبي. Left out when there is none.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub nickname: Option<String>,
    /// A short biography in Arabic. Left out when there is none.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub bio: Option<String>,
    /// The poet's era.
    pub era: EraRef,
    /// Number of the poet's poems, primary readings only.
    #[schema(example = 104)]
    pub poems_count: i32,
    /// Whether the poet has an avatar image, served at `https://cdn.qafiyah.com/poets/{slug}/avatar.webp`.
    #[schema(example = true)]
    pub has_avatar: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoetSlugEntry {
    /// The poet's four-letter, case-sensitive slug, for `GET /poets/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "PAKT")]
    pub slug: String,
    /// Whether the poet has an avatar image, served at `https://cdn.qafiyah.com/poets/{slug}/avatar.webp`.
    #[schema(example = true)]
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
