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
    pub name: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "yoFB")]
    pub slug: String,
    pub has_avatar: bool,
    pub is_anonymous: bool,
}

#[derive(Serialize, ToSchema)]
pub struct MeterRef {
    pub name: String,
    #[schema(pattern = "^[a-z][a-z-]*$", example = "altawil")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct EraRef {
    pub name: String,
    #[schema(pattern = "^[a-z][a-z-]*$", example = "abbasi")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct ThemeRef {
    pub name: String,
    #[schema(pattern = "^[a-z][a-z-]*$", example = "alnasib")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct RhymeRef {
    pub name: String,
    #[schema(pattern = "^[a-z][a-z-]*$", example = "meem")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct PoemTypeRef {
    pub name: String,
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
