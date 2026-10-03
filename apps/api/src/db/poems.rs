use std::collections::HashMap;

use async_trait::async_trait;
use diesel::dsl::{count_star, exists};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Json, Nullable};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::future::try_join_all;

use crate::db::corpus::{
    collection_stats, collections, era_stats, eras, meter_stats, meters, poem_aliases,
    poem_relations, poem_types, poem_verses, poems, poet_stats, poets, rhyme_stats, rhymes,
    theme_stats, themes, verses,
};
use crate::db::{PgPool, Uncached, int};
use crate::domain::poems::{
    FacetCounts, Facets, Filter, PoemListItem, PoemNavRef, PoemRecord, PoemRepository, RandomPoem,
    Recension,
};
use crate::domain::taxonomy::PoemCountStats;
use crate::domain::{EraRef, MeterRef, PoemTypeRef, PoetRef, RhymeRef, ThemeRef};
use crate::error::StoreError;

pub struct PgPoems {
    pool: PgPool,
}

impl PgPoems {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type PoemListRow = (String, String, String, String, bool, bool, String, String);

fn list_item(row: PoemListRow) -> PoemListItem {
    let (title, slug, poet_name, poet_slug, has_avatar, is_anonymous, meter_name, meter_slug) = row;
    PoemListItem {
        title,
        slug,
        poet: PoetRef {
            name: poet_name,
            slug: poet_slug,
            has_avatar,
            is_anonymous,
        },
        meter: MeterRef {
            name: meter_name,
            slug: meter_slug,
        },
        era: None,
    }
}

type RelatedRow = (
    String,
    String,
    String,
    String,
    bool,
    bool,
    String,
    String,
    String,
    String,
);

fn related_item(row: RelatedRow) -> PoemListItem {
    let (
        title,
        slug,
        poet_name,
        poet_slug,
        has_avatar,
        is_anonymous,
        meter_name,
        meter_slug,
        era_name,
        era_slug,
    ) = row;
    PoemListItem {
        title,
        slug,
        poet: PoetRef {
            name: poet_name,
            slug: poet_slug,
            has_avatar,
            is_anonymous,
        },
        meter: MeterRef {
            name: meter_name,
            slug: meter_slug,
        },
        era: Some(EraRef {
            name: era_name,
            slug: era_slug,
        }),
    }
}

type DetailRow = (
    i32,
    i32,
    Option<i32>,
    String,
    i32,
    String,
    String,
    bool,
    bool,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

fn total_of<T: Into<i64>>(counts: Vec<Option<T>>) -> Result<i32, StoreError> {
    let total = counts
        .into_iter()
        .flatten()
        .map(Into::into)
        .try_fold(0_i64, i64::checked_add)
        .ok_or_else(|| StoreError::Database("poem total overflowed".to_string()))?;
    int(total)
}

struct FilterIds {
    filter: Filter,
    ids: Vec<i32>,
}

async fn ids_for(
    mut conn: &AsyncPgConnection,
    filter: Filter,
    slugs: &[String],
) -> QueryResult<FilterIds> {
    let ids = match filter {
        Filter::Poet => {
            poets::table
                .filter(poets::slug.eq_any(slugs))
                .select(poets::id)
                .load::<i32>(&mut conn)
                .await?
        }
        Filter::Era => {
            eras::table
                .filter(eras::slug.eq_any(slugs))
                .select(eras::id)
                .load::<i32>(&mut conn)
                .await?
        }
        Filter::Meter => {
            meters::table
                .filter(meters::slug.eq_any(slugs))
                .select(meters::id)
                .load::<i32>(&mut conn)
                .await?
        }
        Filter::Theme => {
            themes::table
                .filter(themes::slug.eq_any(slugs))
                .select(themes::id)
                .load::<i32>(&mut conn)
                .await?
        }
        Filter::Rhyme => {
            rhymes::table
                .filter(rhymes::slug.eq_any(slugs))
                .select(rhymes::id)
                .load::<i32>(&mut conn)
                .await?
        }
        Filter::Collection => {
            collections::table
                .filter(collections::slug.eq_any(slugs))
                .select(collections::id)
                .load::<i32>(&mut conn)
                .await?
        }
    };
    Ok(FilterIds { filter, ids })
}

async fn resolve_ids(conn: &AsyncPgConnection, facets: &Facets) -> QueryResult<Vec<FilterIds>> {
    try_join_all(
        Filter::ALL
            .into_iter()
            .filter(|filter| !facets.values(*filter).is_empty())
            .map(|filter| ids_for(conn, filter, facets.values(filter))),
    )
    .await
}

type Condition = Box<dyn BoxableExpression<poems::table, Pg, SqlType = Bool>>;

fn shown() -> Condition {
    Box::new(
        poems::recension_of_id
            .is_null()
            .and(poems::is_hidden.eq(false)),
    )
}

fn matching(filter: &FilterIds) -> Condition {
    match (filter.filter, filter.ids.as_slice()) {
        (Filter::Poet, [id]) => Box::new(poems::poet_id.eq(*id)),
        (Filter::Poet, ids) => Box::new(poems::poet_id.eq_any(ids.to_vec())),
        (Filter::Era, [id]) => Box::new(poems::era_id.eq(*id)),
        (Filter::Era, ids) => Box::new(poems::era_id.eq_any(ids.to_vec())),
        (Filter::Meter, [id]) => Box::new(poems::meter_id.eq(*id)),
        (Filter::Meter, ids) => Box::new(poems::meter_id.eq_any(ids.to_vec())),
        (Filter::Theme, [id]) => Box::new(poems::theme_id.eq(*id)),
        (Filter::Theme, ids) => Box::new(poems::theme_id.eq_any(ids.to_vec())),
        (Filter::Rhyme, [id]) => Box::new(poems::rhyme_id.eq(*id)),
        (Filter::Rhyme, ids) => Box::new(poems::rhyme_id.eq_any(ids.to_vec())),
        (Filter::Collection, [id]) => Box::new(poems::collection_id.assume_not_null().eq(*id)),
        (Filter::Collection, ids) => {
            Box::new(poems::collection_id.assume_not_null().eq_any(ids.to_vec()))
        }
    }
}

fn page_ids(
    filters: &[FilterIds],
    page: u32,
    page_size: u32,
) -> poems::BoxedQuery<'static, Pg, Integer> {
    let mut query = poems::table.select(poems::id).filter(shown()).into_boxed();
    for filter in filters {
        query = query.filter(matching(filter));
    }
    query
        .order(poems::id.asc())
        .limit(i64::from(page_size))
        .offset(i64::from(page.saturating_sub(1)).saturating_mul(i64::from(page_size)))
}

async fn page_rows(
    mut conn: &AsyncPgConnection,
    filters: &[FilterIds],
    page: u32,
    page_size: u32,
) -> Result<Vec<PoemListRow>, StoreError> {
    let rows = poems::table
        .inner_join(poets::table.on(poets::id.eq(poems::poet_id)))
        .inner_join(meters::table.on(meters::id.eq(poems::meter_id)))
        .filter(poems::id.eq_any(page_ids(filters, page, page_size)))
        .order(poems::id.asc())
        .select((
            poems::title,
            poems::slug,
            poets::name,
            poets::slug,
            poets::has_avatar,
            poets::is_anonymous,
            meters::name,
            meters::slug,
        ));
    Ok(Uncached(rows).load::<PoemListRow>(&mut conn).await?)
}

async fn stats_total(mut conn: &AsyncPgConnection, filter: &FilterIds) -> Result<i32, StoreError> {
    let ids = &filter.ids;
    match filter.filter {
        Filter::Poet => total_of(
            poet_stats::table
                .filter(poet_stats::id.eq_any(ids))
                .select(poet_stats::poems_count)
                .load::<Option<i64>>(&mut conn)
                .await?,
        ),
        Filter::Era => total_of(
            era_stats::table
                .filter(era_stats::id.eq_any(ids))
                .select(era_stats::poems_count)
                .load::<Option<i64>>(&mut conn)
                .await?,
        ),
        Filter::Meter => total_of(
            meter_stats::table
                .filter(meter_stats::id.eq_any(ids))
                .select(meter_stats::poems_count)
                .load::<Option<i64>>(&mut conn)
                .await?,
        ),
        Filter::Theme => total_of(
            theme_stats::table
                .filter(theme_stats::id.eq_any(ids))
                .select(theme_stats::poems_count)
                .load::<Option<i64>>(&mut conn)
                .await?,
        ),
        Filter::Rhyme => total_of(
            rhyme_stats::table
                .filter(rhyme_stats::id.eq_any(ids))
                .select(rhyme_stats::poems_count)
                .load::<Option<i32>>(&mut conn)
                .await?,
        ),
        Filter::Collection => total_of(
            collection_stats::table
                .filter(collection_stats::id.eq_any(ids))
                .select(collection_stats::poems_count)
                .load::<Option<i64>>(&mut conn)
                .await?,
        ),
    }
}

async fn total(mut conn: &AsyncPgConnection, filters: &[FilterIds]) -> Result<i32, StoreError> {
    match filters {
        [] => total_of(
            meter_stats::table
                .select(meter_stats::poems_count)
                .load::<Option<i64>>(&mut conn)
                .await?,
        ),
        [only] => stats_total(conn, only).await,
        several => {
            let mut query = poems::table
                .select(count_star())
                .filter(shown())
                .into_boxed();
            for filter in several {
                query = query.filter(matching(filter));
            }
            int(Uncached(query).get_result::<i64>(&mut conn).await?)
        }
    }
}

async fn grouped_counts(
    mut conn: &AsyncPgConnection,
    counted: Filter,
    filters: &[FilterIds],
) -> QueryResult<Vec<(i32, i64)>> {
    let others = filters.iter().filter(|filter| filter.filter != counted);
    match counted {
        Filter::Meter => {
            let mut query = poems::table
                .group_by(poems::meter_id)
                .select((poems::meter_id, count_star()))
                .filter(shown())
                .into_boxed();
            for filter in others {
                query = query.filter(matching(filter));
            }
            Uncached(query).load(&mut conn).await
        }
        Filter::Rhyme => {
            let mut query = poems::table
                .group_by(poems::rhyme_id)
                .select((poems::rhyme_id, count_star()))
                .filter(shown())
                .into_boxed();
            for filter in others {
                query = query.filter(matching(filter));
            }
            Uncached(query).load(&mut conn).await
        }
        Filter::Theme => {
            let mut query = poems::table
                .group_by(poems::theme_id)
                .select((poems::theme_id, count_star()))
                .filter(shown())
                .into_boxed();
            for filter in others {
                query = query.filter(matching(filter));
            }
            Uncached(query).load(&mut conn).await
        }
        Filter::Poet | Filter::Era | Filter::Collection => Ok(Vec::new()),
    }
}

async fn named_terms(
    mut conn: &AsyncPgConnection,
    counted: Filter,
) -> QueryResult<Vec<(i32, String, String)>> {
    match counted {
        Filter::Meter => {
            meters::table
                .order(meters::name.asc())
                .select((meters::id, meters::name, meters::slug))
                .load(&mut conn)
                .await
        }
        Filter::Rhyme => {
            rhymes::table
                .order(rhymes::name.asc())
                .select((rhymes::id, rhymes::name, rhymes::slug))
                .load(&mut conn)
                .await
        }
        Filter::Theme => {
            themes::table
                .order(themes::name.asc())
                .select((themes::id, themes::name, themes::slug))
                .load(&mut conn)
                .await
        }
        Filter::Poet | Filter::Era | Filter::Collection => Ok(Vec::new()),
    }
}

async fn term_counts(
    conn: &AsyncPgConnection,
    counted: Filter,
    filters: &[FilterIds],
) -> Result<Vec<PoemCountStats>, StoreError> {
    let (counts, terms) = tokio::try_join!(
        grouped_counts(conn, counted, filters),
        named_terms(conn, counted)
    )?;
    let count_of: HashMap<i32, i64> = counts.into_iter().collect();
    terms
        .into_iter()
        .map(|(id, name, slug)| {
            Ok(PoemCountStats {
                name,
                slug,
                poems_count: int(count_of.get(&id).copied().unwrap_or(0))?,
            })
        })
        .collect()
}

async fn poet_is_shown(mut conn: &AsyncPgConnection, slugs: &[String]) -> QueryResult<bool> {
    diesel::select(exists(
        poets::table
            .filter(poets::slug.eq_any(slugs))
            .filter(poets::is_hidden.eq(false)),
    ))
    .get_result::<bool>(&mut conn)
    .await
}

fn neighbour(
    poet_id: i32,
    id: i32,
    before: bool,
) -> poems::BoxedQuery<'static, Pg, (Integer, diesel::sql_types::Text, diesel::sql_types::Text)> {
    let query = poems::table
        .select((poems::id, poems::title, poems::slug))
        .filter(poems::poet_id.eq(poet_id))
        .filter(shown())
        .into_boxed();
    if before {
        query
            .filter(poems::id.lt(id))
            .order(poems::id.desc())
            .limit(1)
    } else {
        query
            .filter(poems::id.gt(id))
            .order(poems::id.asc())
            .limit(1)
    }
}

async fn neighbours(
    mut conn: &AsyncPgConnection,
    poet_id: i32,
    id: i32,
) -> QueryResult<(Option<PoemNavRef>, Option<PoemNavRef>)> {
    let rows = neighbour(poet_id, id, true)
        .union_all(neighbour(poet_id, id, false))
        .load::<(i32, String, String)>(&mut conn)
        .await?;
    let pick = |before: bool| {
        rows.iter()
            .find(|(other, _, _)| (*other < id) == before)
            .map(|(_, title, slug)| PoemNavRef {
                title: title.clone(),
                slug: slug.clone(),
            })
    };
    Ok((pick(true), pick(false)))
}

async fn verses_of(mut conn: &AsyncPgConnection, id: i32) -> QueryResult<Vec<String>> {
    poem_verses::table
        .inner_join(verses::table)
        .filter(poem_verses::poem_id.eq(id))
        .order(poem_verses::position.asc())
        .select(verses::content)
        .load(&mut conn)
        .await
}

async fn recensions_of(
    mut conn: &AsyncPgConnection,
    id: i32,
    root: i32,
) -> QueryResult<Vec<(i32, String, String, i32)>> {
    poems::table
        .filter(poems::id.ne(id))
        .filter(poems::id.eq(root).or(poems::recension_of_id.eq(root)))
        .order((poems::recension_of_id.is_not_null().asc(), poems::id.asc()))
        .select((poems::id, poems::title, poems::slug, poems::verse_count))
        .load(&mut conn)
        .await
}

async fn related_of(mut conn: &AsyncPgConnection, id: i32) -> QueryResult<Vec<PoemListItem>> {
    Ok(poem_relations::table
        .inner_join(poems::table.on(poems::id.eq(poem_relations::related_id)))
        .inner_join(poets::table.on(poets::id.eq(poems::poet_id)))
        .inner_join(eras::table.on(eras::id.eq(poets::era_id)))
        .inner_join(meters::table.on(meters::id.eq(poems::meter_id)))
        .filter(poem_relations::poem_id.eq(id))
        .order(poem_relations::rank.asc())
        .select((
            poems::title,
            poems::slug,
            poets::name,
            poets::slug,
            poets::has_avatar,
            poets::is_anonymous,
            meters::name,
            meters::slug,
            eras::name,
            eras::slug,
        ))
        .load::<RelatedRow>(&mut conn)
        .await?
        .into_iter()
        .map(related_item)
        .collect())
}

#[diesel::declare_sql_function]
extern "SQL" {
    fn random_poem_json() -> Nullable<Json>;
}

#[async_trait]
impl PoemRepository for PgPoems {
    async fn count(&self) -> Result<i32, StoreError> {
        let mut conn = self.pool.get().await?;
        total_of(
            meter_stats::table
                .select(meter_stats::poems_count)
                .load::<Option<i64>>(&mut conn)
                .await?,
        )
    }

    async fn list_slugs(&self, page: u32, page_size: u32) -> Result<Vec<String>, StoreError> {
        let mut conn = self.pool.get().await?;
        Ok(poems::table
            .filter(poems::recension_of_id.is_null())
            .filter(poems::is_hidden.eq(false))
            .order(poems::slug.asc())
            .select(poems::slug)
            .limit(i64::from(page_size))
            .offset(i64::from(page.saturating_sub(1)).saturating_mul(i64::from(page_size)))
            .load::<String>(&mut conn)
            .await?)
    }

    async fn list(
        &self,
        facets: &Facets,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<PoemListItem>, i32), StoreError> {
        let pooled = self.pool.get().await?;
        let conn: &AsyncPgConnection = &pooled;
        let filters = resolve_ids(conn, facets).await?;
        if filters.iter().any(|filter| filter.ids.is_empty()) {
            return Ok((Vec::new(), 0));
        }
        let (rows, total) = tokio::try_join!(
            page_rows(conn, &filters, page, page_size),
            total(conn, &filters)
        )?;
        Ok((rows.into_iter().map(list_item).collect(), total))
    }

    async fn facet_counts(&self, facets: &Facets) -> Result<Option<FacetCounts>, StoreError> {
        let pooled = self.pool.get().await?;
        let conn: &AsyncPgConnection = &pooled;
        let (is_shown, filters) =
            tokio::try_join!(poet_is_shown(conn, &facets.poet), resolve_ids(conn, facets))?;
        if !is_shown {
            return Ok(None);
        }
        let (meters, rhymes, themes) = tokio::try_join!(
            term_counts(conn, Filter::Meter, &filters),
            term_counts(conn, Filter::Rhyme, &filters),
            term_counts(conn, Filter::Theme, &filters),
        )?;
        Ok(Some(FacetCounts {
            meters,
            rhymes,
            themes,
        }))
    }

    async fn find(&self, slug: &str) -> Result<Option<PoemRecord>, StoreError> {
        let pooled = self.pool.get().await?;
        let conn: &AsyncPgConnection = &pooled;
        let mut main = conn;
        let Some(row) = poems::table
            .inner_join(poets::table.on(poets::id.eq(poems::poet_id)))
            .inner_join(eras::table.on(eras::id.eq(poets::era_id)))
            .inner_join(meters::table.on(meters::id.eq(poems::meter_id)))
            .inner_join(themes::table.on(themes::id.eq(poems::theme_id)))
            .inner_join(rhymes::table.on(rhymes::id.eq(poems::rhyme_id)))
            .inner_join(poem_types::table.on(poem_types::id.eq(poems::poem_type_id)))
            .filter(poems::slug.eq(slug))
            .filter(poems::is_hidden.eq(false))
            .select((
                poems::id,
                poems::poet_id,
                poems::recension_of_id,
                poems::title,
                poems::verse_count,
                poets::name,
                poets::slug,
                poets::has_avatar,
                poets::is_anonymous,
                meters::name,
                meters::slug,
                themes::name,
                themes::slug,
                eras::name,
                eras::slug,
                rhymes::name,
                rhymes::slug,
                poem_types::name,
                poem_types::slug,
            ))
            .first::<DetailRow>(&mut main)
            .await
            .optional()?
        else {
            return Ok(None);
        };
        let (
            id,
            poet_id,
            recension_of_id,
            title,
            verse_count,
            poet_name,
            poet_slug,
            poet_has_avatar,
            poet_is_anonymous,
            meter_name,
            meter_slug,
            theme_name,
            theme_slug,
            era_name,
            era_slug,
            rhyme_name,
            rhyme_slug,
            poem_type_name,
            poem_type_slug,
        ) = row;

        let (lines, (prev, next), family, related) = tokio::try_join!(
            verses_of(conn, id),
            neighbours(conn, poet_id, id),
            recensions_of(conn, id, recension_of_id.unwrap_or(id)),
            related_of(conn, id),
        )?;
        Ok(Some(PoemRecord {
            title,
            verse_count,
            recension_of_id,
            poet: PoetRef {
                name: poet_name,
                slug: poet_slug,
                has_avatar: poet_has_avatar,
                is_anonymous: poet_is_anonymous,
            },
            era: EraRef {
                name: era_name,
                slug: era_slug,
            },
            meter: MeterRef {
                name: meter_name,
                slug: meter_slug,
            },
            theme: ThemeRef {
                name: theme_name,
                slug: theme_slug,
            },
            rhyme: RhymeRef {
                name: rhyme_name,
                slug: rhyme_slug,
            },
            poem_type: PoemTypeRef {
                name: poem_type_name,
                slug: poem_type_slug,
            },
            lines,
            prev,
            next,
            family: family
                .into_iter()
                .map(|(id, title, slug, verse_count)| Recension {
                    id,
                    title,
                    slug,
                    verse_count,
                })
                .collect(),
            related,
        }))
    }

    async fn alias_target(&self, slug: &str) -> Result<Option<String>, StoreError> {
        let mut conn = self.pool.get().await?;
        Ok(poem_aliases::table
            .inner_join(poems::table)
            .filter(poem_aliases::slug.eq(slug))
            .filter(poems::is_hidden.eq(false))
            .select(poems::slug)
            .first::<String>(&mut conn)
            .await
            .optional()?)
    }

    async fn random(&self) -> Result<Option<RandomPoem>, StoreError> {
        let mut conn = self.pool.get().await?;
        diesel::select(random_poem_json())
            .get_result::<Option<serde_json::Value>>(&mut conn)
            .await?
            .map(|payload| {
                serde_json::from_value(payload)
                    .map_err(|error| StoreError::Database(error.to_string()))
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(filter: Filter, ids: &[i32]) -> FilterIds {
        FilterIds {
            filter,
            ids: ids.to_vec(),
        }
    }

    fn page_sql(filters: &[FilterIds], page: u32) -> String {
        diesel::debug_query::<Pg, _>(&page_ids(filters, page, 30)).to_string()
    }

    #[test]
    fn one_id_is_matched_as_a_scalar_so_the_planner_walks_the_filter_index() {
        let sql = page_sql(&[ids(Filter::Meter, &[5])], 1);
        assert!(sql.contains(r#""poems"."meter_id" = $2"#), "{sql}");
        assert!(!sql.contains("ANY"), "{sql}");
    }

    #[test]
    fn several_ids_are_matched_as_a_set() {
        let sql = page_sql(&[ids(Filter::Meter, &[5, 9])], 1);
        assert!(sql.contains(r#""poems"."meter_id" = ANY($2)"#), "{sql}");
    }
}
