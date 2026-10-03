use std::cmp::Reverse;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;
use utoipa::ToSchema;

use crate::db::corpus::{
    collection_stats, era_stats, meter_stats, poem_type_stats, rhyme_stats, theme_stats,
};
use crate::db::{PgPool, int, present};
use crate::error::{AppError, Resource};

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

#[derive(Clone, Copy)]
pub enum Counted {
    Meters,
    Rhymes,
    Eras,
    PoemTypes,
}

impl Counted {
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

type CountedRow<C> = (Option<String>, Option<String>, Option<C>, Option<C>);
type PoemCountRow = (Option<String>, Option<String>, Option<i64>);

fn counted<C: TryInto<i32>>(
    (name, slug, poems, poets): CountedRow<C>,
) -> Result<CountedStats, AppError> {
    Ok(CountedStats {
        name: present(name)?,
        slug: present(slug)?,
        poems_count: int(present(poems)?)?,
        poets_count: int(present(poets)?)?,
    })
}

fn poem_counted((name, slug, poems): PoemCountRow) -> Result<PoemCountStats, AppError> {
    Ok(PoemCountStats {
        name: present(name)?,
        slug: present(slug)?,
        poems_count: int(present(poems)?)?,
    })
}

pub async fn list_counted(pg: &PgPool, kind: Counted) -> Result<Vec<CountedStats>, AppError> {
    let mut conn = pg.get().await?;
    match kind {
        Counted::Meters => meter_stats::table
            .select((
                meter_stats::name,
                meter_stats::slug,
                meter_stats::poems_count,
                meter_stats::poets_count,
            ))
            .order(meter_stats::name.asc())
            .load::<CountedRow<i64>>(&mut conn)
            .await?
            .into_iter()
            .map(counted)
            .collect(),
        Counted::Rhymes => rhyme_stats::table
            .select((
                rhyme_stats::name,
                rhyme_stats::slug,
                rhyme_stats::poems_count,
                rhyme_stats::poets_count,
            ))
            .order(rhyme_stats::id.asc())
            .load::<CountedRow<i32>>(&mut conn)
            .await?
            .into_iter()
            .map(counted)
            .collect(),
        Counted::Eras => era_stats::table
            .select((
                era_stats::name,
                era_stats::slug,
                era_stats::poems_count,
                era_stats::poets_count,
            ))
            .order(era_stats::sort_order.asc())
            .load::<CountedRow<i64>>(&mut conn)
            .await?
            .into_iter()
            .map(counted)
            .collect(),
        Counted::PoemTypes => poem_type_stats::table
            .select((
                poem_type_stats::name,
                poem_type_stats::slug,
                poem_type_stats::poems_count,
                poem_type_stats::poets_count,
            ))
            .order((
                poem_type_stats::poems_count.desc(),
                poem_type_stats::id.asc(),
            ))
            .load::<CountedRow<i64>>(&mut conn)
            .await?
            .into_iter()
            .map(counted)
            .collect(),
    }
}

pub async fn get_counted(pg: &PgPool, kind: Counted, slug: &str) -> Result<CountedStats, AppError> {
    let mut conn = pg.get().await?;
    let row = match kind {
        Counted::Meters => meter_stats::table
            .select((
                meter_stats::name,
                meter_stats::slug,
                meter_stats::poems_count,
                meter_stats::poets_count,
            ))
            .filter(meter_stats::slug.eq(slug))
            .first::<CountedRow<i64>>(&mut conn)
            .await
            .optional()?
            .map(counted),
        Counted::Rhymes => rhyme_stats::table
            .select((
                rhyme_stats::name,
                rhyme_stats::slug,
                rhyme_stats::poems_count,
                rhyme_stats::poets_count,
            ))
            .filter(rhyme_stats::slug.eq(slug))
            .first::<CountedRow<i32>>(&mut conn)
            .await
            .optional()?
            .map(counted),
        Counted::Eras => era_stats::table
            .select((
                era_stats::name,
                era_stats::slug,
                era_stats::poems_count,
                era_stats::poets_count,
            ))
            .filter(era_stats::slug.eq(slug))
            .first::<CountedRow<i64>>(&mut conn)
            .await
            .optional()?
            .map(counted),
        Counted::PoemTypes => poem_type_stats::table
            .select((
                poem_type_stats::name,
                poem_type_stats::slug,
                poem_type_stats::poems_count,
                poem_type_stats::poets_count,
            ))
            .filter(poem_type_stats::slug.eq(slug))
            .first::<CountedRow<i64>>(&mut conn)
            .await
            .optional()?
            .map(counted),
    };
    row.transpose()?.ok_or(AppError::NotFound(kind.resource()))
}

pub async fn list_by_poem_count(
    pg: &PgPool,
    kind: PoemCounted,
) -> Result<Vec<PoemCountStats>, AppError> {
    let mut conn = pg.get().await?;
    let rows = match kind {
        PoemCounted::Themes => {
            theme_stats::table
                .select((
                    theme_stats::name,
                    theme_stats::slug,
                    theme_stats::poems_count,
                ))
                .load::<PoemCountRow>(&mut conn)
                .await?
        }
        PoemCounted::Collections => {
            collection_stats::table
                .select((
                    collection_stats::name,
                    collection_stats::slug,
                    collection_stats::poems_count,
                ))
                .load::<PoemCountRow>(&mut conn)
                .await?
        }
    };
    let mut stats = rows
        .into_iter()
        .map(poem_counted)
        .collect::<Result<Vec<_>, _>>()?;
    stats.sort_by_key(|row| Reverse(row.poems_count));
    Ok(stats)
}

pub async fn get_by_poem_count(
    pg: &PgPool,
    kind: PoemCounted,
    slug: &str,
) -> Result<PoemCountStats, AppError> {
    let mut conn = pg.get().await?;
    let row = match kind {
        PoemCounted::Themes => theme_stats::table
            .select((
                theme_stats::name,
                theme_stats::slug,
                theme_stats::poems_count,
            ))
            .filter(theme_stats::slug.eq(slug))
            .first::<PoemCountRow>(&mut conn)
            .await
            .optional()?,
        PoemCounted::Collections => collection_stats::table
            .select((
                collection_stats::name,
                collection_stats::slug,
                collection_stats::poems_count,
            ))
            .filter(collection_stats::slug.eq(slug))
            .first::<PoemCountRow>(&mut conn)
            .await
            .optional()?,
    };
    row.map(poem_counted)
        .transpose()?
        .ok_or(AppError::NotFound(kind.resource()))
}
