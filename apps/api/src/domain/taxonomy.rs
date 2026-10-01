use std::cmp::Reverse;

use serde::Serialize;
use sqlx::{AssertSqlSafe, PgPool};
use utoipa::ToSchema;

use crate::error::{AppError, Resource};

#[derive(Serialize, sqlx::FromRow, ToSchema)]
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

#[derive(Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemCountStats {
    pub name: String,
    #[schema(pattern = "^[a-z][a-z-]*$", example = "alnasib")]
    pub slug: String,
    #[schema(example = 47457)]
    pub poems_count: i32,
}

#[derive(Clone, Copy)]
pub enum Counted {
    Meters,
    Rhymes,
    Eras,
    PoemTypes,
}

impl Counted {
    fn view(self) -> &'static str {
        match self {
            Counted::Meters => "meter_stats",
            Counted::Rhymes => "rhyme_stats",
            Counted::Eras => "era_stats",
            Counted::PoemTypes => "poem_type_stats",
        }
    }

    fn order_by(self) -> &'static str {
        match self {
            Counted::Meters => "name ASC",
            Counted::Rhymes => "id ASC",
            Counted::Eras => "sort_order ASC",
            Counted::PoemTypes => "poems_count DESC, id ASC",
        }
    }

    fn resource(self) -> Resource {
        match self {
            Counted::Meters => Resource::Meter,
            Counted::Rhymes => Resource::Rhyme,
            Counted::Eras => Resource::Era,
            Counted::PoemTypes => Resource::PoemType,
        }
    }

    pub fn log_field(self) -> &'static str {
        match self {
            Counted::Meters => "meter",
            Counted::Rhymes => "rhyme",
            Counted::Eras => "era",
            Counted::PoemTypes => "poem_type",
        }
    }
}

#[derive(Clone, Copy)]
pub enum PoemCounted {
    Themes,
    Collections,
}

impl PoemCounted {
    fn view(self) -> &'static str {
        match self {
            PoemCounted::Themes => "theme_stats",
            PoemCounted::Collections => "collection_stats",
        }
    }

    fn resource(self) -> Resource {
        match self {
            PoemCounted::Themes => Resource::Theme,
            PoemCounted::Collections => Resource::Collection,
        }
    }

    pub fn log_field(self) -> &'static str {
        match self {
            PoemCounted::Themes => "theme",
            PoemCounted::Collections => "collection",
        }
    }
}

const COUNTED_COLUMNS: &str =
    "name, slug, poems_count::int AS poems_count, poets_count::int AS poets_count";
const POEM_COUNT_COLUMNS: &str = "name, slug, poems_count::int AS poems_count";

pub async fn list_counted(pg: &PgPool, kind: Counted) -> Result<Vec<CountedStats>, AppError> {
    let sql = format!(
        "SELECT {COUNTED_COLUMNS} FROM {} ORDER BY {}",
        kind.view(),
        kind.order_by()
    );
    Ok(sqlx::query_as::<_, CountedStats>(AssertSqlSafe(sql))
        .fetch_all(pg)
        .await?)
}

pub async fn get_counted(pg: &PgPool, kind: Counted, slug: &str) -> Result<CountedStats, AppError> {
    let sql = format!(
        "SELECT {COUNTED_COLUMNS} FROM {} WHERE slug = $1 LIMIT 1",
        kind.view()
    );
    sqlx::query_as::<_, CountedStats>(AssertSqlSafe(sql))
        .bind(slug)
        .fetch_optional(pg)
        .await?
        .ok_or(AppError::NotFound(kind.resource()))
}

pub async fn list_by_poem_count(
    pg: &PgPool,
    kind: PoemCounted,
) -> Result<Vec<PoemCountStats>, AppError> {
    let sql = format!("SELECT {POEM_COUNT_COLUMNS} FROM {}", kind.view());
    let mut rows = sqlx::query_as::<_, PoemCountStats>(AssertSqlSafe(sql))
        .fetch_all(pg)
        .await?;
    rows.sort_by_key(|row| Reverse(row.poems_count));
    Ok(rows)
}

pub async fn get_by_poem_count(
    pg: &PgPool,
    kind: PoemCounted,
    slug: &str,
) -> Result<PoemCountStats, AppError> {
    let sql = format!(
        "SELECT {POEM_COUNT_COLUMNS} FROM {} WHERE slug = $1 LIMIT 1",
        kind.view()
    );
    sqlx::query_as::<_, PoemCountStats>(AssertSqlSafe(sql))
        .bind(slug)
        .fetch_optional(pg)
        .await?
        .ok_or(AppError::NotFound(kind.resource()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorting_by_poem_count_keeps_ties_in_row_order() {
        let mut rows = [
            PoemCountStats {
                name: "first".into(),
                slug: "a".into(),
                poems_count: 5,
            },
            PoemCountStats {
                name: "second".into(),
                slug: "b".into(),
                poems_count: 9,
            },
            PoemCountStats {
                name: "third".into(),
                slug: "c".into(),
                poems_count: 5,
            },
        ];
        rows.sort_by_key(|row| Reverse(row.poems_count));
        assert_eq!(
            rows.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>(),
            ["b", "a", "c"]
        );
    }
}
