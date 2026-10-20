use serde::Serialize;
use utoipa::ToSchema;

use crate::contract::poems::PoemNavRef;
use crate::contract::{EraRef, PoetRef};
use crate::domain::Term;
use crate::domain::feelings::{FeelingNode, FeelingVerse};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeelingItem {
    /// The feeling's name in Arabic, a verbal noun with its article.
    #[schema(example = "الحزن")]
    pub name: String,
    /// The feeling's slug, for `GET /feelings/{slug}`.
    #[schema(pattern = "^[a-z][a-z-]*$", example = "alhuzn")]
    pub slug: String,
    /// The word on the Feelings Wheel that this feeling translates. Absent for a feeling that the wheel does not have.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Sad")]
    pub wheel_term: Option<String>,
    /// The slug of the feeling above this one. Absent for the seven core feelings.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(pattern = "^[a-z][a-z-]*$", example = "alhuzn")]
    pub parent_slug: Option<String>,
    /// Number of verses that express this feeling or a feeling below it, each verse counted once. At least 1, since a feeling with no verses is left out.
    #[schema(example = 1200)]
    pub verse_count: i32,
}

#[derive(Serialize, ToSchema)]
pub struct FeelingRef {
    /// The feeling's name in Arabic.
    #[schema(example = "الوحشة")]
    pub name: String,
    /// The feeling's slug, for `GET /feelings/{slug}`.
    #[schema(pattern = "^[a-z][a-z-]*$", example = "alwahsha")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct FeelingVerseItem {
    /// The verse's parts in reading order, usually its two hemistichs.
    pub parts: Vec<String>,
    /// The verse's position in its poem, from 1.
    #[schema(example = 3)]
    pub position: i32,
    /// The verse's strongest feeling within the requested feeling's branch. A verse can express up to three feelings.
    pub feeling: FeelingRef,
    /// The poem that holds the verse.
    pub poem: PoemNavRef,
    /// The poem's poet.
    pub poet: PoetRef,
    /// The poem's era.
    pub era: EraRef,
}

impl From<FeelingNode> for FeelingItem {
    fn from(node: FeelingNode) -> Self {
        FeelingItem {
            name: node.name,
            slug: node.slug,
            wheel_term: node.wheel_term,
            parent_slug: node.parent_slug,
            verse_count: node.verse_count,
        }
    }
}

impl From<Term> for FeelingRef {
    fn from(term: Term) -> Self {
        FeelingRef {
            name: term.name,
            slug: term.slug,
        }
    }
}

impl From<FeelingVerse> for FeelingVerseItem {
    fn from(verse: FeelingVerse) -> Self {
        FeelingVerseItem {
            parts: verse.parts,
            position: verse.position,
            feeling: FeelingRef::from(verse.feeling),
            poem: PoemNavRef::from(verse.poem),
            poet: PoetRef::from(verse.poet),
            era: EraRef::from(verse.era),
        }
    }
}
