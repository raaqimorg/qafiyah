use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::db::corpus::{
    eras, feeling_branch_verses, feeling_stats, feelings, poem_verses, poems, poets, verses,
};
use crate::db::{PgPool, int, present};
use crate::domain::feelings::{Feeling, FeelingRepository, FeelingVerse};
use crate::domain::paging::{self, Cursor, Slice};
use crate::domain::poems::PoemLink;
use crate::domain::{PoetBrief, StoreError, Term};

fn rank_window(cursor: Option<&Cursor<i32>>, page_size: u32) -> (i32, i32) {
    let span = i32::try_from(paging::fetch_limit(page_size)).unwrap_or(i32::MAX);
    match cursor {
        Some(Cursor::Before(rank)) => (rank.saturating_sub(span).max(1), rank.saturating_sub(1)),
        Some(Cursor::After(rank)) => (rank.saturating_add(1), rank.saturating_add(span)),
        None => (1, span),
    }
}

pub struct PgFeelings {
    pool: PgPool,
}

impl PgFeelings {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type FeelingRow = (i32, Option<i32>, String, String, Option<String>, i32);
type VerseRow = (
    String,
    i32,
    String,
    String,
    String,
    String,
    String,
    String,
    bool,
    bool,
    String,
    String,
);

fn verse(
    (
        content,
        position,
        feeling_name,
        feeling_slug,
        title,
        poem_slug,
        poet_name,
        poet_slug,
        has_avatar,
        is_anonymous,
        era_name,
        era_slug,
    ): VerseRow,
) -> FeelingVerse {
    FeelingVerse {
        parts: content.split('*').map(str::to_string).collect(),
        position,
        feeling: Term {
            name: feeling_name,
            slug: feeling_slug,
        },
        poem: PoemLink {
            title,
            slug: poem_slug,
        },
        poet: PoetBrief {
            name: poet_name,
            slug: poet_slug,
            has_avatar,
            is_anonymous,
        },
        era: Term {
            name: era_name,
            slug: era_slug,
        },
    }
}

#[async_trait]
impl FeelingRepository for PgFeelings {
    async fn all(&self) -> Result<Vec<Feeling>, StoreError> {
        let mut conn = self.pool.get().await?;
        Ok(feelings::table
            .select((
                feelings::id,
                feelings::parent_id,
                feelings::slug,
                feelings::name,
                feelings::wheel_term,
                feelings::sort_order,
            ))
            .order(feelings::id.asc())
            .load::<FeelingRow>(&mut conn)
            .await?
            .into_iter()
            .map(
                |(id, parent_id, slug, name, wheel_term, sort_order)| Feeling {
                    id,
                    parent_id,
                    slug,
                    name,
                    wheel_term,
                    sort_order,
                },
            )
            .collect())
    }

    async fn branch_counts(&self) -> Result<Vec<(i32, i32)>, StoreError> {
        let mut conn = self.pool.get().await?;
        feeling_stats::table
            .select((feeling_stats::id, feeling_stats::verses_count))
            .load::<(i32, Option<i64>)>(&mut conn)
            .await?
            .into_iter()
            .map(|(id, count)| Ok((id, int(present(count)?)?)))
            .collect()
    }

    async fn verses(
        &self,
        feeling_id: i32,
        cursor: Option<&Cursor<i32>>,
        page_size: u32,
    ) -> Result<Slice<FeelingVerse, i32>, StoreError> {
        let (first, last) = rank_window(cursor, page_size);
        let mut conn = self.pool.get().await?;
        let query = feeling_branch_verses::table
            .inner_join(
                feelings::table.on(feelings::id.eq(feeling_branch_verses::verse_feeling_id)),
            )
            .inner_join(
                poem_verses::table.on(poem_verses::poem_id
                    .eq(feeling_branch_verses::poem_id)
                    .and(poem_verses::position.eq(feeling_branch_verses::position))),
            )
            .inner_join(verses::table.on(verses::id.eq(poem_verses::verse_id)))
            .inner_join(poems::table.on(poems::id.eq(feeling_branch_verses::poem_id)))
            .inner_join(poets::table.on(poets::id.eq(poems::poet_id)))
            .inner_join(eras::table.on(eras::id.eq(poems::era_id)))
            .filter(feeling_branch_verses::feeling_id.eq(feeling_id))
            .select((
                feeling_branch_verses::rank,
                (
                    verses::content,
                    feeling_branch_verses::position,
                    feelings::name,
                    feelings::slug,
                    poems::title,
                    poems::slug,
                    poets::name,
                    poets::slug,
                    poets::has_avatar,
                    poets::is_anonymous,
                    eras::name,
                    eras::slug,
                ),
            ))
            .filter(feeling_branch_verses::rank.between(first, last))
            .into_boxed();
        let query = if cursor.is_some_and(Cursor::is_before) {
            query.order(feeling_branch_verses::rank.desc())
        } else {
            query.order(feeling_branch_verses::rank.asc())
        };
        let rows = query.load::<(i32, VerseRow)>(&mut conn).await?;
        Ok(paging::slice(rows, cursor, page_size, |(rank, _)| *rank).map(|(_, row)| verse(row)))
    }
}
