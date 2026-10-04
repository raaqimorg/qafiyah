pub mod poems;
pub mod poets;
pub mod search;
pub mod taxonomy;

use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::{PoetBrief, Term};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoetRef {
    /// The poet's name in Arabic.
    #[schema(example = "زهير بن أبي سلمى")]
    pub name: String,
    /// The poet's four-letter, case-sensitive slug, for `GET /poets/{slug}`.
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "PAKT")]
    pub slug: String,
    /// Whether the poet has an avatar image, served at `https://cdn.qafiyah.com/poets/{slug}/avatar.webp`.
    #[schema(example = true)]
    pub has_avatar: bool,
    /// Whether the poet stands for an unknown author, such as غير معروف or مجهول (عباسي), rather than a named person.
    #[schema(example = false)]
    pub is_anonymous: bool,
}

#[derive(Serialize, ToSchema)]
pub struct MeterRef {
    /// The meter's name in Arabic.
    #[schema(example = "الطويل")]
    pub name: String,
    /// The meter's slug, for `GET /meters/{slug}`.
    #[schema(pattern = "^[a-z][a-z-]*$", example = "altawil")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct EraRef {
    /// The era's name in Arabic.
    #[schema(example = "جاهلي")]
    pub name: String,
    /// The era's slug, for `GET /eras/{slug}`.
    #[schema(pattern = "^[a-z][a-z-]*$", example = "jahili")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct ThemeRef {
    /// The theme's name in Arabic.
    #[schema(example = "الحكمة")]
    pub name: String,
    /// The theme's slug, for `GET /themes/{slug}`.
    #[schema(pattern = "^[a-z][a-z-]*$", example = "alhikma")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct RhymeRef {
    /// The rhyme letter's name in Arabic.
    #[schema(example = "ميم")]
    pub name: String,
    /// The rhyme's slug, for `GET /rhymes/{slug}`.
    #[schema(pattern = "^[a-z][a-z-]*$", example = "meem")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct PoemTypeRef {
    /// The verse form's name in Arabic.
    #[schema(example = "عمودي")]
    pub name: String,
    /// The verse form's slug, for `GET /poem-types/{slug}`.
    #[schema(pattern = "^[a-z][a-z-]*$", example = "amudi")]
    pub slug: String,
}

impl From<PoetBrief> for PoetRef {
    fn from(poet: PoetBrief) -> Self {
        PoetRef {
            name: poet.name,
            slug: poet.slug,
            has_avatar: poet.has_avatar,
            is_anonymous: poet.is_anonymous,
        }
    }
}

impl From<Term> for MeterRef {
    fn from(term: Term) -> Self {
        MeterRef {
            name: term.name,
            slug: term.slug,
        }
    }
}

impl From<Term> for EraRef {
    fn from(term: Term) -> Self {
        EraRef {
            name: term.name,
            slug: term.slug,
        }
    }
}

impl From<Term> for ThemeRef {
    fn from(term: Term) -> Self {
        ThemeRef {
            name: term.name,
            slug: term.slug,
        }
    }
}

impl From<Term> for RhymeRef {
    fn from(term: Term) -> Self {
        RhymeRef {
            name: term.name,
            slug: term.slug,
        }
    }
}

impl From<Term> for PoemTypeRef {
    fn from(term: Term) -> Self {
        PoemTypeRef {
            name: term.name,
            slug: term.slug,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EraRef, PoetRef};

    #[test]
    fn serializes_the_four_fields_the_contract_names() {
        let json = serde_json::to_value(PoetRef {
            name: "المتنبي".into(),
            slug: "yoFB".into(),
            has_avatar: true,
            is_anonymous: false,
        })
        .expect("serializable");
        assert_eq!(json["name"], "المتنبي");
        assert_eq!(json["slug"], "yoFB");
        assert_eq!(json["hasAvatar"], true);
        assert_eq!(json["isAnonymous"], false);
        assert_eq!(json.as_object().expect("object").len(), 4);
    }

    #[test]
    fn every_reference_shares_the_same_wire_shape() {
        let era = serde_json::to_value(EraRef {
            name: "عباسي".into(),
            slug: "abbasi".into(),
        })
        .expect("serializable");
        assert_eq!(
            era.as_object().expect("object").keys().collect::<Vec<_>>(),
            ["name", "slug"]
        );
    }
}
