use std::cmp::Reverse;
use std::collections::HashMap;

use diesel::dsl::{count_star, exists};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Json, Nullable};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::future::try_join_all;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa::openapi::Schema;

use crate::constants::{MAX_TWEET_LENGTH, RANDOM_POEM_MAX_ATTEMPTS};
use crate::db::corpus::{
    collection_stats, collections, era_stats, eras, meter_stats, meters, poem_aliases,
    poem_relations, poem_types, poem_verses, poems, poet_stats, poets, rhyme_stats, rhymes,
    theme_stats, themes, verses,
};
use crate::db::{PgPool, Uncached, int};
use crate::domain::taxonomy::PoemCountStats;
use crate::domain::{EraRef, MeterRef, PoemTypeRef, PoetRef, RhymeRef, ThemeRef};
use crate::error::{AppError, Resource, RouteProblem};
use crate::js;

fn verse_list() -> utoipa::openapi::schema::Array {
    use utoipa::openapi::RefOr;
    use utoipa::openapi::schema::ArrayBuilder;

    ArrayBuilder::new()
        .items(RefOr::T(Schema::Array(hemistich_pair())))
        .build()
}

fn hemistich_pair() -> utoipa::openapi::schema::Array {
    use utoipa::openapi::schema::{ArrayBuilder, ArrayItems, ObjectBuilder, Type};

    let hemistich = || Schema::Object(ObjectBuilder::new().schema_type(Type::String).build());
    ArrayBuilder::new()
        .prefix_items([hemistich(), hemistich()])
        .items(ArrayItems::False)
        .min_items(Some(2))
        .max_items(Some(2))
        .build()
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemListItem {
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
    pub poet: PoetRef,
    pub meter: MeterRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub era: Option<EraRef>,
}

#[derive(Serialize, ToSchema)]
pub struct PoemNavRef {
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemRecensionRef {
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
    pub verse_count: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoemDetail {
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
    #[schema(schema_with = verse_list)]
    pub verses: Vec<[String; 2]>,
    #[schema(example = 10)]
    pub verse_count: i32,
    pub sample: String,
    pub keywords: String,
    pub poet: PoetRef,
    pub era: EraRef,
    pub meter: MeterRef,
    pub theme: ThemeRef,
    pub rhyme: RhymeRef,
    pub poem_type: PoemTypeRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub prev: Option<PoemNavRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub next: Option<PoemNavRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub recension_of: Option<PoemNavRef>,
    pub recensions: Vec<PoemRecensionRef>,
    pub related_poems: Vec<PoemListItem>,
}

#[derive(Serialize, ToSchema)]
pub struct Total {
    #[schema(example = 394174)]
    pub total: i32,
}

#[derive(Default)]
pub struct Facets {
    pub poet: Vec<String>,
    pub era: Vec<String>,
    pub theme: Vec<String>,
    pub meter: Vec<String>,
    pub rhyme: Vec<String>,
    pub collection: Vec<String>,
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

pub struct ParsedContent {
    pub verses: Vec<[String; 2]>,
    pub sample: String,
    pub keywords: String,
}

pub fn parse_poem_content(content: &str) -> ParsedContent {
    let lines: Vec<&str> = content.split('*').collect();
    ParsedContent {
        verses: lines
            .chunks(2)
            .map(|pair| {
                let first = pair.first().copied().unwrap_or_default();
                let second = pair.get(1).copied().unwrap_or_default();
                [first.to_string(), second.to_string()]
            })
            .collect(),
        sample: lines
            .iter()
            .take(3)
            .copied()
            .collect::<Vec<&str>>()
            .join(" * "),
        keywords: lines.join(" ").replace(' ', ","),
    }
}

fn total_of<T: Into<i64>>(counts: Vec<Option<T>>) -> Result<i32, AppError> {
    let total = counts
        .into_iter()
        .flatten()
        .map(Into::into)
        .try_fold(0_i64, i64::checked_add)
        .ok_or_else(|| AppError::Database("poem total overflowed".to_string()))?;
    int(total)
}

pub async fn count(pg: &PgPool) -> Result<i32, AppError> {
    let mut conn = pg.get().await?;
    total_of(
        meter_stats::table
            .select(meter_stats::poems_count)
            .load::<Option<i64>>(&mut conn)
            .await?,
    )
}

pub async fn list_slugs(pg: &PgPool, page: u32, page_size: u32) -> Result<Vec<String>, AppError> {
    let mut conn = pg.get().await?;
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Filter {
    Poet,
    Era,
    Meter,
    Theme,
    Rhyme,
    Collection,
}

impl Filter {
    const ALL: [Filter; 6] = [
        Filter::Poet,
        Filter::Era,
        Filter::Meter,
        Filter::Theme,
        Filter::Rhyme,
        Filter::Collection,
    ];
}

impl Facets {
    fn values(&self, filter: Filter) -> &Vec<String> {
        match filter {
            Filter::Poet => &self.poet,
            Filter::Era => &self.era,
            Filter::Meter => &self.meter,
            Filter::Theme => &self.theme,
            Filter::Rhyme => &self.rhyme,
            Filter::Collection => &self.collection,
        }
    }
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
) -> Result<Vec<PoemListRow>, AppError> {
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

async fn stats_total(mut conn: &AsyncPgConnection, filter: &FilterIds) -> Result<i32, AppError> {
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

async fn total(mut conn: &AsyncPgConnection, filters: &[FilterIds]) -> Result<i32, AppError> {
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

pub async fn list(
    pg: &PgPool,
    facets: &Facets,
    page: u32,
    page_size: u32,
) -> Result<(Vec<PoemListItem>, i32), AppError> {
    let pooled = pg.get().await?;
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

#[derive(Serialize, ToSchema)]
pub struct PoemFacets {
    pub meters: Vec<PoemCountStats>,
    pub rhymes: Vec<PoemCountStats>,
    pub themes: Vec<PoemCountStats>,
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

async fn count_facet(
    conn: &AsyncPgConnection,
    counted: Filter,
    filters: &[FilterIds],
    selected: &[String],
) -> Result<Vec<PoemCountStats>, AppError> {
    let (counts, terms) = tokio::try_join!(
        grouped_counts(conn, counted, filters),
        named_terms(conn, counted)
    )?;
    let count_of: HashMap<i32, i64> = counts.into_iter().collect();
    let mut stats = terms
        .into_iter()
        .filter_map(|(id, name, slug)| {
            let poems_count = count_of.get(&id).copied().unwrap_or(0);
            (poems_count > 0 || selected.contains(&slug)).then(|| {
                Ok(PoemCountStats {
                    name,
                    slug,
                    poems_count: int(poems_count)?,
                })
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    stats.sort_by_key(|stat| Reverse(stat.poems_count));
    Ok(stats)
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

pub async fn facets(pg: &PgPool, facets: &Facets) -> Result<PoemFacets, AppError> {
    let pooled = pg.get().await?;
    let conn: &AsyncPgConnection = &pooled;
    let (is_shown, filters) =
        tokio::try_join!(poet_is_shown(conn, &facets.poet), resolve_ids(conn, facets))?;
    if !is_shown {
        return Err(AppError::NotFound(Resource::Poet));
    }
    let (meters, rhymes, themes) = tokio::try_join!(
        count_facet(conn, Filter::Meter, &filters, &facets.meter),
        count_facet(conn, Filter::Rhyme, &filters, &facets.rhyme),
        count_facet(conn, Filter::Theme, &filters, &facets.theme),
    )?;
    Ok(PoemFacets {
        meters,
        rhymes,
        themes,
    })
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

pub async fn get(pg: &PgPool, slug: &str) -> Result<PoemDetail, AppError> {
    let pooled = pg.get().await?;
    let conn: &AsyncPgConnection = &pooled;
    let mut main = conn;
    let row = poems::table
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
        .ok_or(AppError::NotFound(Resource::Poem))?;
    let (
        id,
        poet_id,
        primary,
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

    let (lines, (prev, next), family, related_poems) = tokio::try_join!(
        verses_of(conn, id),
        neighbours(conn, poet_id, id),
        recensions_of(conn, id, primary.unwrap_or(id)),
        related_of(conn, id),
    )?;
    let recension_of = primary.and_then(|primary| {
        family
            .iter()
            .find(|(other, _, _, _)| *other == primary)
            .map(|(_, title, slug, _)| PoemNavRef {
                title: title.clone(),
                slug: slug.clone(),
            })
    });
    let recensions = family
        .into_iter()
        .map(|(_, title, slug, verse_count)| PoemRecensionRef {
            title,
            slug,
            verse_count,
        })
        .collect();
    if lines.is_empty() {
        return Err(AppError::PoemParse);
    }
    let parsed = parse_poem_content(&lines.join("*"));

    Ok(PoemDetail {
        title,
        slug: slug.to_string(),
        verses: parsed.verses,
        verse_count,
        sample: parsed.sample,
        keywords: parsed.keywords,
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
        prev,
        next,
        recension_of,
        recensions,
        related_poems,
    })
}

pub async fn alias_target(pg: &PgPool, slug: &str) -> Result<Option<String>, AppError> {
    let mut conn = pg.get().await?;
    Ok(poem_aliases::table
        .inner_join(poems::table)
        .filter(poem_aliases::slug.eq(slug))
        .filter(poems::is_hidden.eq(false))
        .select(poems::slug)
        .first::<String>(&mut conn)
        .await
        .optional()?)
}

pub enum RandomPoemOption {
    Slug,
    Lines,
}

impl RandomPoemOption {
    pub fn parse(raw: Option<&str>) -> Result<Self, AppError> {
        match raw {
            None | Some("slug") => Ok(RandomPoemOption::Slug),
            Some("lines") => Ok(RandomPoemOption::Lines),
            Some(_) => Err(RouteProblem::bad_request(
                "Invalid ?option value (expected 'slug' or 'lines')",
            )
            .into()),
        }
    }
}

#[derive(Deserialize)]
struct RandomPoem {
    poet_name: String,
    content: String,
    slug: String,
}

fn random_poem_failed() -> AppError {
    RouteProblem::internal("Failed to fetch random poem").into()
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "excerpt math runs on a poem's small hemistich count"
)]
#[expect(
    clippy::as_conversions,
    reason = "the float casts stay within the verse count, bounded by a short poem"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the floored roll is at most the verse count"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "the verse count and roll are non-negative"
)]
fn excerpt_start(line_count: usize, roll: f64) -> usize {
    let max_start = line_count.saturating_sub(2);
    let verse_count = max_start / 2 + 1;
    (((roll * verse_count as f64).floor() as usize) * 2).min(max_start)
}

fn build_excerpt(poem: &RandomPoem, roll: f64) -> Result<String, AppError> {
    let lines: Vec<&str> = poem.content.split('*').collect();
    if lines.len() < 2 {
        return Err(random_poem_failed());
    }
    let start = excerpt_start(lines.len(), roll);
    let first = lines.get(start).copied().unwrap_or_default();
    let second = lines
        .get(start.saturating_add(1))
        .copied()
        .unwrap_or_default();
    let excerpt = js::trim(&format!("{first}\n{second}\n\n{}", poem.poet_name)).to_string();
    if excerpt.encode_utf16().count() > MAX_TWEET_LENGTH {
        return Err(random_poem_failed());
    }
    Ok(excerpt)
}

#[diesel::declare_sql_function]
extern "SQL" {
    fn random_poem_json() -> Nullable<Json>;
}

async fn fetch_random_poem(pg: &PgPool) -> Result<RandomPoem, AppError> {
    let mut conn = pg.get().await?;
    let payload = diesel::select(random_poem_json())
        .get_result::<Option<serde_json::Value>>(&mut conn)
        .await?
        .ok_or_else(random_poem_failed)?;
    serde_json::from_value(payload).map_err(|error| AppError::Database(error.to_string()))
}

pub async fn random(pg: &PgPool, option: &RandomPoemOption, roll: f64) -> Result<String, AppError> {
    match option {
        RandomPoemOption::Slug => Ok(fetch_random_poem(pg).await?.slug),
        RandomPoemOption::Lines => {
            let mut last_err = None;
            for _ in 0..RANDOM_POEM_MAX_ATTEMPTS {
                let poem = fetch_random_poem(pg).await?;
                match build_excerpt(&poem, roll) {
                    Ok(excerpt) => return Ok(excerpt),
                    Err(err) => last_err = Some(err),
                }
            }
            Err(last_err.unwrap_or_else(random_poem_failed))
        }
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

    #[test]
    fn pairs_hemistichs_and_leaves_an_odd_one_half_empty() {
        let parsed = parse_poem_content("a*b*c");
        assert_eq!(parsed.verses, [["a", "b"], ["c", ""]]);
    }

    #[test]
    fn samples_the_first_three_hemistichs() {
        let parsed = parse_poem_content("a*b*c*d");
        assert_eq!(parsed.sample, "a * b * c");
    }

    #[test]
    fn turns_every_space_into_a_comma() {
        let parsed = parse_poem_content("one two*three  four");
        assert_eq!(parsed.keywords, "one,two,three,,four");
    }

    #[test]
    fn starts_an_excerpt_on_a_verse_boundary() {
        for line_count in [2usize, 3, 4, 5, 6, 7, 20] {
            let max_start = line_count - 2;
            for step in 0..100 {
                let start = excerpt_start(line_count, f64::from(step) / 100.0);
                assert_eq!(start % 2, 0, "{line_count} hemistichs at step {step}");
                assert!(start <= max_start, "{line_count} hemistichs at step {step}");
            }
        }
    }

    #[test]
    fn reaches_every_verse_including_the_last() {
        assert_eq!(excerpt_start(6, 0.0), 0);
        assert_eq!(excerpt_start(6, 0.5), 2);
        assert_eq!(excerpt_start(6, 0.99), 4);
        assert_eq!(excerpt_start(4, 0.99), 2);
        assert_eq!(excerpt_start(2, 0.99), 0);
    }

    #[test]
    fn empty_content_is_one_empty_verse() {
        let parsed = parse_poem_content("");
        assert_eq!(parsed.verses, [["", ""]]);
        assert_eq!(parsed.sample, "");
    }

    fn random_poem(content: &str) -> RandomPoem {
        RandomPoem {
            poet_name: "poet".into(),
            content: content.into(),
            slug: "slug".into(),
        }
    }

    #[test]
    fn rejects_a_fragment_with_fewer_than_two_hemistichs() {
        assert!(build_excerpt(&random_poem("one lone hemistich"), 0.0).is_err());
        assert!(build_excerpt(&random_poem(""), 0.0).is_err());
    }

    #[test]
    fn rejects_an_excerpt_longer_than_a_tweet() {
        let long_line = "a".repeat(MAX_TWEET_LENGTH);
        let poem = random_poem(&format!("{long_line}*{long_line}"));
        assert!(build_excerpt(&poem, 0.0).is_err());
    }

    #[test]
    fn builds_the_first_and_second_hemistich_with_the_poet_name() {
        let poem = random_poem("first*second*third*fourth");
        let excerpt = build_excerpt(&poem, 0.0).expect("well-formed poem builds an excerpt");
        assert_eq!(excerpt, "first\nsecond\n\npoet");
    }

    #[test]
    fn content_parsing_never_panics_and_keeps_every_hemistich() {
        let mut rng = crate::test_support::Rng::new(3);
        let alphabet = ["*", "ا", "ب", " ", "**", "\n", "\u{00a0}", "😀"];
        for _ in 0..3_000 {
            let len = rng.below(30);
            let content: String = (0..len).map(|_| rng.pick(&alphabet)).collect();
            let parsed = parse_poem_content(&content);
            let hemistichs: Vec<&str> = content.split('*').collect();
            assert_eq!(parsed.verses.len(), hemistichs.len().div_ceil(2));
            assert_eq!(
                parsed.sample,
                hemistichs
                    .iter()
                    .take(3)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" * ")
            );
            assert!(!parsed.keywords.contains(' '));
            for (index, hemistich) in hemistichs.iter().enumerate() {
                assert_eq!(parsed.verses[index / 2][index % 2], *hemistich);
            }
        }
    }

    #[test]
    fn consecutive_delimiters_produce_empty_hemistichs_rather_than_being_collapsed() {
        let parsed = parse_poem_content("a**b");
        assert_eq!(parsed.verses, [["a", ""], ["b", ""]]);
        assert_eq!(parsed.sample, "a *  * b");
    }

    #[test]
    fn the_random_option_accepts_only_the_two_documented_values() {
        assert!(matches!(
            RandomPoemOption::parse(None),
            Ok(RandomPoemOption::Slug)
        ));
        assert!(matches!(
            RandomPoemOption::parse(Some("slug")),
            Ok(RandomPoemOption::Slug)
        ));
        assert!(matches!(
            RandomPoemOption::parse(Some("lines")),
            Ok(RandomPoemOption::Lines)
        ));
        for raw in ["", "Lines", "verses", "slug "] {
            assert!(RandomPoemOption::parse(Some(raw)).is_err(), "{raw}");
        }
    }

    #[test]
    fn an_excerpt_at_exactly_the_tweet_cap_is_accepted_and_one_over_is_refused() {
        let hemistich = "ا".repeat(138);
        let at_cap = RandomPoem {
            poet_name: "x".into(),
            content: format!("{hemistich}*{hemistich}"),
            slug: "TnKK".into(),
        };
        let excerpt = build_excerpt(&at_cap, 0.0).expect("exactly 280 units");
        assert_eq!(excerpt.encode_utf16().count(), MAX_TWEET_LENGTH);
        let over = RandomPoem {
            poet_name: "xy".into(),
            content: format!("{hemistich}*{hemistich}"),
            slug: "TnKK".into(),
        };
        assert!(build_excerpt(&over, 0.0).is_err());
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

    #[test]
    fn every_filter_narrows_the_one_page_query() {
        let filters: Vec<FilterIds> = Filter::ALL
            .into_iter()
            .map(|filter| ids(filter, &[1, 2]))
            .collect();
        let sql = page_sql(&filters, 1);
        for column in [
            "poet_id",
            "era_id",
            "meter_id",
            "theme_id",
            "rhyme_id",
            "collection_id",
        ] {
            assert!(
                sql.contains(&format!(r#""poems"."{column}" = ANY("#)),
                "{column}: {sql}"
            );
        }
    }

    #[test]
    fn every_page_hides_recensions_and_hidden_poems_even_without_a_filter() {
        let sql = page_sql(&[], 1);
        assert!(
            sql.contains(r#""poems"."recension_of_id" IS NULL"#),
            "{sql}"
        );
        assert!(sql.contains(r#""poems"."is_hidden" = $1"#), "{sql}");
        assert!(sql.contains("binds: [false,"), "{sql}");
    }

    #[test]
    fn a_page_is_picked_in_id_order_and_offset_by_whole_pages() {
        let sql = page_sql(&[], 3);
        assert!(
            sql.contains(r#"ORDER BY "poems"."id" ASC LIMIT $2 OFFSET $3"#),
            "{sql}"
        );
        assert!(sql.ends_with("binds: [false, 30, 60]"), "{sql}");
    }

    #[test]
    fn the_previous_poem_is_the_poets_nearest_shown_primary_before_it() {
        let sql = diesel::debug_query::<Pg, _>(&neighbour(7, 100, true)).to_string();
        assert!(sql.contains(r#""poems"."poet_id" = $1"#), "{sql}");
        assert!(
            sql.contains(r#""poems"."recension_of_id" IS NULL"#),
            "{sql}"
        );
        assert!(sql.contains(r#""poems"."id" < $3"#), "{sql}");
        assert!(sql.contains(r#"ORDER BY "poems"."id" DESC LIMIT"#), "{sql}");
    }

    #[test]
    fn the_previous_and_next_poems_are_read_in_one_union_of_two_bounded_lookups() {
        let union = neighbour(7, 100, true).union_all(neighbour(7, 100, false));
        let sql = diesel::debug_query::<Pg, _>(&union).to_string();
        assert!(sql.starts_with("(SELECT"), "{sql}");
        assert!(sql.contains(") UNION ALL (SELECT"), "{sql}");
        assert_eq!(sql.matches("LIMIT").count(), 2, "{sql}");
    }

    #[test]
    fn the_next_poem_is_the_poets_nearest_shown_primary_after_it() {
        let sql = diesel::debug_query::<Pg, _>(&neighbour(7, 100, false)).to_string();
        assert!(sql.contains(r#""poems"."id" > $3"#), "{sql}");
        assert!(sql.contains(r#"ORDER BY "poems"."id" ASC LIMIT"#), "{sql}");
    }
}
