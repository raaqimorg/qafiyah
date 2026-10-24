use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::db::corpus::{eras, poet_aliases, poet_stats, poets};
use crate::db::{PgPool, int};
use crate::domain::StoreError;
use crate::domain::Term;
use crate::domain::paging::{self, Cursor, Slice};
use crate::domain::poets::{PoetProfile, PoetRepository, PoetSlug};

pub struct PgPoets {
    pool: PgPool,
}

impl PgPoets {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
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

#[async_trait]
impl PoetRepository for PgPoets {
    async fn get(&self, slug: &str) -> Result<Option<PoetProfile>, StoreError> {
        let mut conn = self.pool.get().await?;
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
            .optional()?;
        let Some((name, slug, nickname, bio, has_avatar, era_name, era_slug, poems_count)) = row
        else {
            return Ok(None);
        };
        Ok(Some(PoetProfile {
            name,
            slug,
            nickname,
            bio,
            era: Term {
                name: era_name,
                slug: era_slug,
            },
            poems_count: int(poems_count.unwrap_or(0))?,
            has_avatar,
        }))
    }

    async fn alias_target(&self, slug: &str) -> Result<Option<String>, StoreError> {
        let mut conn = self.pool.get().await?;
        Ok(poet_aliases::table
            .inner_join(poets::table)
            .filter(poet_aliases::slug.eq(slug))
            .filter(poets::is_hidden.eq(false))
            .select(poets::slug)
            .first::<String>(&mut conn)
            .await
            .optional()?)
    }

    async fn count_with_poems(&self) -> Result<i32, StoreError> {
        let mut conn = self.pool.get().await?;
        let total: i64 = poets::table
            .inner_join(poet_stats::table.on(poet_stats::id.eq(poets::id)))
            .filter(poet_stats::poems_count.gt(0))
            .count()
            .get_result(&mut conn)
            .await?;
        int(total)
    }

    async fn list_slugs(
        &self,
        cursor: Option<&Cursor<i32>>,
        page_size: u32,
    ) -> Result<Slice<PoetSlug, i32>, StoreError> {
        let mut conn = self.pool.get().await?;
        let query = poets::table
            .inner_join(poet_stats::table.on(poet_stats::id.eq(poets::id)))
            .filter(poet_stats::poems_count.gt(0))
            .select((poets::id, poets::slug, poets::has_avatar))
            .limit(paging::fetch_limit(page_size))
            .into_boxed();
        let query = match cursor {
            None => query.order(poets::id.asc()),
            Some(Cursor::After(id)) => query.filter(poets::id.gt(*id)).order(poets::id.asc()),
            Some(Cursor::Before(id)) => query.filter(poets::id.lt(*id)).order(poets::id.desc()),
        };
        let rows = query.load::<(i32, String, bool)>(&mut conn).await?;
        Ok(paging::slice(rows, cursor, page_size, |(id, _, _)| *id)
            .map(|(_, slug, has_avatar)| PoetSlug { slug, has_avatar }))
    }
}
