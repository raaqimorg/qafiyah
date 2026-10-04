use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::taxonomy::{TermCount, TermStats};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountedStats {
    /// The term's name in Arabic.
    #[schema(example = "الطويل")]
    pub name: String,
    /// The term's slug.
    #[schema(pattern = "^[a-z][a-z-]*$", example = "altawil")]
    pub slug: String,
    /// Number of poems with this term, primary readings only.
    #[schema(example = 40697)]
    pub poems_count: i32,
    /// Number of poets with at least one such poem.
    #[schema(example = 3697)]
    pub poets_count: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemCountStats {
    /// The term's name in Arabic.
    #[schema(example = "الحكمة")]
    pub name: String,
    /// The term's slug.
    #[schema(pattern = "^[a-z][a-z-]*$", example = "alhikma")]
    pub slug: String,
    /// Number of poems with this term, primary readings only.
    #[schema(example = 22521)]
    pub poems_count: i32,
}

impl From<TermStats> for CountedStats {
    fn from(term: TermStats) -> Self {
        CountedStats {
            name: term.name,
            slug: term.slug,
            poems_count: term.poems_count,
            poets_count: term.poets_count,
        }
    }
}

impl From<TermCount> for PoemCountStats {
    fn from(term: TermCount) -> Self {
        PoemCountStats {
            name: term.name,
            slug: term.slug,
            poems_count: term.poems_count,
        }
    }
}
