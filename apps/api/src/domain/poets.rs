use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;
use utoipa::ToSchema;

use crate::db::corpus::{eras, poet_aliases, poet_stats, poets};
use crate::db::{PgPool, int};
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

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoetSlugEntry {
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "yoFB")]
    pub slug: String,
    pub has_avatar: bool,
}

type PoetStatsRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    bool,
    String,
    String,
    Option<i64>,
);

pub async fn get(pg: &PgPool, slug: &str) -> Result<PoetStats, AppError> {
    let mut conn = pg.get().await?;
    let row = poets::table
        .inner_join(eras::table)
        .left_join(poet_stats::table.on(poet_stats::slug.eq(poets::slug.nullable())))
        .filter(poets::slug.eq(slug))
        .filter(poets::is_hidden.eq(false))
        .select((
            poets::name,
            poets::slug,
            poets::nickname,
            poets::bio,
            poets::has_avatar,
            eras::name,
            eras::slug,
            poet_stats::poems_count.nullable(),
        ))
        .first::<PoetStatsRow>(&mut conn)
        .await
        .optional()?
        .ok_or(AppError::NotFound(Resource::Poet))?;
    let (name, slug, nickname, bio, has_avatar, era_name, era_slug, poems_count) = row;
    Ok(PoetStats {
        name,
        slug,
        nickname,
        bio,
        era: EraRef {
            name: era_name,
            slug: era_slug,
        },
        poems_count: int(poems_count.unwrap_or(0))?,
        has_avatar,
    })
}

pub async fn alias_target(pg: &PgPool, slug: &str) -> Result<Option<String>, AppError> {
    let mut conn = pg.get().await?;
    Ok(poet_aliases::table
        .inner_join(poets::table)
        .filter(poet_aliases::slug.eq(slug))
        .filter(poets::is_hidden.eq(false))
        .select(poets::slug)
        .first::<String>(&mut conn)
        .await
        .optional()?)
}

pub async fn count_with_poems(pg: &PgPool) -> Result<i32, AppError> {
    let mut conn = pg.get().await?;
    let total: i64 = poets::table
        .inner_join(poet_stats::table.on(poet_stats::id.eq(poets::id)))
        .filter(poet_stats::poems_count.gt(0))
        .count()
        .get_result(&mut conn)
        .await?;
    int(total)
}

pub async fn list_slugs(
    pg: &PgPool,
    page: u32,
    page_size: u32,
) -> Result<Vec<PoetSlugEntry>, AppError> {
    let mut conn = pg.get().await?;
    Ok(poets::table
        .inner_join(poet_stats::table.on(poet_stats::id.eq(poets::id)))
        .filter(poet_stats::poems_count.gt(0))
        .order(poets::slug.asc())
        .select((poets::slug, poets::has_avatar))
        .limit(i64::from(page_size))
        .offset(i64::from(page.saturating_sub(1)).saturating_mul(i64::from(page_size)))
        .load::<(String, bool)>(&mut conn)
        .await?
        .into_iter()
        .map(|(slug, has_avatar)| PoetSlugEntry { slug, has_avatar })
        .collect())
}
