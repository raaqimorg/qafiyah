use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::domain::EraRef;
use crate::error::{AppError, Resource};

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

#[derive(Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PoetSlugEntry {
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "yoFB")]
    pub slug: String,
    pub has_avatar: bool,
}

#[derive(sqlx::FromRow)]
struct PoetStatsRow {
    name: String,
    slug: String,
    nickname: Option<String>,
    bio: Option<String>,
    era_name: String,
    era_slug: String,
    poems_count: i32,
    has_avatar: bool,
}

impl From<PoetStatsRow> for PoetStats {
    fn from(row: PoetStatsRow) -> Self {
        PoetStats {
            name: row.name,
            slug: row.slug,
            nickname: row.nickname,
            bio: row.bio,
            era: EraRef {
                name: row.era_name,
                slug: row.era_slug,
            },
            poems_count: row.poems_count,
            has_avatar: row.has_avatar,
        }
    }
}

pub async fn get(pg: &PgPool, slug: &str) -> Result<PoetStats, AppError> {
    sqlx::query_as::<_, PoetStatsRow>(
        r#"
      SELECT p.name, p.slug, p.nickname, p.bio, p.has_avatar,
             e.name AS era_name, e.slug AS era_slug,
             COALESCE(ps.poems_count, 0)::int AS poems_count
      FROM public.poets p
      JOIN public.eras e ON e.id = p.era_id
      LEFT JOIN public.poet_stats ps ON ps.slug = p.slug
      WHERE p.slug = $1
      LIMIT 1
    "#,
    )
    .bind(slug)
    .fetch_optional(pg)
    .await?
    .map(PoetStats::from)
    .ok_or(AppError::NotFound(Resource::Poet))
}

pub async fn alias_target(pg: &PgPool, slug: &str) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT p.slug FROM public.poet_aliases a \
         JOIN public.poets p ON p.id = a.poet_id WHERE a.slug = $1",
    )
    .bind(slug)
    .fetch_optional(pg)
    .await?)
}

pub async fn count(pg: &PgPool) -> Result<i32, AppError> {
    let total: Option<i32> = sqlx::query_scalar("SELECT COUNT(*)::int AS total FROM public.poets")
        .fetch_one(pg)
        .await?;
    Ok(total.unwrap_or(0))
}

pub async fn list_slugs(
    pg: &PgPool,
    page: u32,
    page_size: u32,
) -> Result<Vec<PoetSlugEntry>, AppError> {
    Ok(sqlx::query_as::<_, PoetSlugEntry>(
        "SELECT slug, has_avatar FROM public.poets ORDER BY slug LIMIT $1 OFFSET $2",
    )
    .bind(i64::from(page_size))
    .bind(i64::from(page.saturating_sub(1)).saturating_mul(i64::from(page_size)))
    .fetch_all(pg)
    .await?)
}
