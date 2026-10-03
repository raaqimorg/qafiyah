use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::taxonomy::{TermCount, TermStats};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountedStats {
    pub name: String,
    #[schema(pattern = "^[a-z][a-z-]*$", example = "altawil")]
    pub slug: String,
    #[schema(example = 44474)]
    pub poems_count: i32,
    #[schema(example = 3637)]
    pub poets_count: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemCountStats {
    pub name: String,
    #[schema(pattern = "^[a-z][a-z-]*$", example = "alnasib")]
    pub slug: String,
    #[schema(example = 47457)]
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
