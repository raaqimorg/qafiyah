use serde::{Deserialize, Serialize};
use sqlx::{AssertSqlSafe, PgPool, Row};
use utoipa::ToSchema;
use utoipa::openapi::Schema;

use crate::constants::{MAX_TWEET_LENGTH, RANDOM_POEM_MAX_ATTEMPTS};
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

#[derive(sqlx::FromRow)]
struct PoemListRow {
    title: String,
    slug: String,
    poet_name: String,
    poet_slug: String,
    poet_has_avatar: bool,
    poet_is_anonymous: bool,
    meter_name: String,
    meter_slug: String,
}

impl From<PoemListRow> for PoemListItem {
    fn from(row: PoemListRow) -> Self {
        PoemListItem {
            title: row.title,
            slug: row.slug,
            poet: PoetRef {
                name: row.poet_name,
                slug: row.poet_slug,
                has_avatar: row.poet_has_avatar,
                is_anonymous: row.poet_is_anonymous,
            },
            meter: MeterRef {
                name: row.meter_name,
                slug: row.meter_slug,
            },
            era: None,
        }
    }
}

#[derive(Deserialize)]
struct RelatedRow {
    title: String,
    slug: String,
    poet_name: String,
    poet_slug: String,
    poet_has_avatar: bool,
    poet_is_anonymous: bool,
    meter_name: String,
    meter_slug: String,
    era_name: String,
    era_slug: String,
}

impl From<RelatedRow> for PoemListItem {
    fn from(row: RelatedRow) -> Self {
        PoemListItem {
            title: row.title,
            slug: row.slug,
            poet: PoetRef {
                name: row.poet_name,
                slug: row.poet_slug,
                has_avatar: row.poet_has_avatar,
                is_anonymous: row.poet_is_anonymous,
            },
            meter: MeterRef {
                name: row.meter_name,
                slug: row.meter_slug,
            },
            era: Some(EraRef {
                name: row.era_name,
                slug: row.era_slug,
            }),
        }
    }
}

#[derive(Deserialize)]
struct PoemNavRow {
    title: String,
    slug: String,
}

#[derive(Deserialize)]
struct RecensionRow {
    title: String,
    slug: String,
    verse_count: i32,
}

#[derive(sqlx::FromRow)]
struct PoemDetailRow {
    title: String,
    content: Option<String>,
    verse_count: i32,
    poet_name: String,
    poet_slug: String,
    poet_has_avatar: bool,
    poet_is_anonymous: bool,
    meter_name: String,
    meter_slug: String,
    theme_name: String,
    theme_slug: String,
    era_name: String,
    era_slug: String,
    rhyme_name: String,
    rhyme_slug: String,
    poem_type_name: String,
    poem_type_slug: String,
    prev_poem: Option<sqlx::types::Json<PoemNavRow>>,
    next_poem: Option<sqlx::types::Json<PoemNavRow>>,
    recension_of: Option<sqlx::types::Json<PoemNavRow>>,
    recensions: sqlx::types::Json<Vec<RecensionRow>>,
    related_poems: sqlx::types::Json<serde_json::Value>,
}

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

pub async fn count(pg: &PgPool) -> Result<i32, AppError> {
    let total: Option<i32> = sqlx::query_scalar(
        "SELECT COUNT(*)::int AS total FROM poems WHERE recension_of_id IS NULL AND NOT is_hidden",
    )
    .fetch_one(pg)
    .await?;
    Ok(total.unwrap_or(0))
}

pub async fn list_slugs(pg: &PgPool, page: u32, page_size: u32) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT slug FROM poems WHERE recension_of_id IS NULL AND NOT is_hidden \
         ORDER BY slug LIMIT $1 OFFSET $2",
    )
    .bind(i64::from(page_size))
    .bind(i64::from(page.saturating_sub(1)).saturating_mul(i64::from(page_size)))
    .fetch_all(pg)
    .await?)
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

    fn column(self) -> &'static str {
        match self {
            Filter::Poet => "p.poet_id",
            Filter::Era => "p.era_id",
            Filter::Meter => "p.meter_id",
            Filter::Theme => "p.theme_id",
            Filter::Rhyme => "p.rhyme_id",
            Filter::Collection => "p.collection_id",
        }
    }

    fn table(self) -> &'static str {
        match self {
            Filter::Poet => "public.poets",
            Filter::Era => "public.eras",
            Filter::Meter => "public.meters",
            Filter::Theme => "public.themes",
            Filter::Rhyme => "public.rhymes",
            Filter::Collection => "public.collections",
        }
    }

    fn stats_table(self) -> &'static str {
        match self {
            Filter::Poet => "public.poet_stats",
            Filter::Era => "public.era_stats",
            Filter::Meter => "public.meter_stats",
            Filter::Theme => "public.theme_stats",
            Filter::Rhyme => "public.rhyme_stats",
            Filter::Collection => "public.collection_stats",
        }
    }
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

fn resolve_sql(filters: &[Filter]) -> String {
    let lookups: Vec<String> = filters
        .iter()
        .enumerate()
        .map(|(index, filter)| {
            format!(
                "ARRAY(SELECT id FROM {} WHERE slug = ANY(${}))",
                filter.table(),
                index.saturating_add(1)
            )
        })
        .collect();
    format!("SELECT {}", lookups.join(", "))
}

async fn resolve_ids(pg: &PgPool, facets: &Facets) -> Result<Vec<FilterIds>, AppError> {
    let requested: Vec<Filter> = Filter::ALL
        .into_iter()
        .filter(|filter| !facets.values(*filter).is_empty())
        .collect();
    if requested.is_empty() {
        return Ok(Vec::new());
    }
    let mut query = sqlx::query(AssertSqlSafe(resolve_sql(&requested)));
    for filter in &requested {
        query = query.bind(facets.values(*filter));
    }
    let row = query.fetch_one(pg).await?;
    requested
        .into_iter()
        .enumerate()
        .map(|(index, filter)| {
            Ok(FilterIds {
                filter,
                ids: row.try_get(index)?,
            })
        })
        .collect()
}

struct Clauses<'a> {
    conditions: Vec<String>,
    counted_by: Option<&'static str>,
    bound: Vec<&'a Vec<i32>>,
}

impl Clauses<'_> {
    fn where_clause(&self) -> String {
        format!("WHERE {}", self.conditions.join(" AND "))
    }
}

fn clauses(filters: &[FilterIds]) -> Clauses<'_> {
    clauses_except(filters, None)
}

fn clauses_except(filters: &[FilterIds], except: Option<Filter>) -> Clauses<'_> {
    let mut conditions: Vec<String> = vec![
        "p.recension_of_id IS NULL".to_string(),
        "NOT p.is_hidden".to_string(),
    ];
    let mut bound: Vec<&Vec<i32>> = Vec::new();
    let mut stats: Vec<&'static str> = Vec::new();
    for FilterIds { filter, ids } in filters {
        if except == Some(*filter) {
            continue;
        }
        bound.push(ids);
        stats.push(filter.stats_table());
        conditions.push(format!("{} = ANY(${})", filter.column(), bound.len()));
    }

    let counted_by = match (bound.as_slice(), stats.as_slice()) {
        ([ids], [stats_table]) if ids.len() == 1 => Some(*stats_table),
        _ => None,
    };

    Clauses {
        conditions,
        counted_by,
        bound,
    }
}

fn list_sql(clauses: &Clauses<'_>) -> (String, String) {
    let Clauses {
        counted_by, bound, ..
    } = clauses;
    let where_clause = clauses.where_clause();
    let rows_sql = format!(
        "SELECT p.title AS title, p.slug AS slug, pt.name AS poet_name, pt.slug AS poet_slug, \
         pt.has_avatar AS poet_has_avatar, pt.is_anonymous AS poet_is_anonymous, \
         m.name AS meter_name, m.slug AS meter_slug \
         FROM (SELECT p.id FROM public.poems p {where_clause} \
         ORDER BY p.id LIMIT ${} OFFSET ${}) page \
         JOIN public.poems p ON p.id = page.id \
         JOIN public.poets pt ON p.poet_id = pt.id \
         JOIN public.meters m ON p.meter_id = m.id \
         ORDER BY p.id",
        bound.len().saturating_add(1),
        bound.len().saturating_add(2)
    );
    let count_sql = match counted_by {
        Some(stats_table) => format!(
            "SELECT (SELECT poems_count::int FROM {stats_table} WHERE id = ($1)[1]) AS total"
        ),
        None => format!("SELECT COUNT(*)::int AS total FROM public.poems p {where_clause}"),
    };
    (rows_sql, count_sql)
}

pub async fn list(
    pg: &PgPool,
    facets: &Facets,
    page: u32,
    page_size: u32,
) -> Result<(Vec<PoemListItem>, i32), AppError> {
    let filters = resolve_ids(pg, facets).await?;
    if filters.iter().any(|filter| filter.ids.is_empty()) {
        return Ok((Vec::new(), 0));
    }
    let built = clauses(&filters);
    let (rows_sql, count_sql) = list_sql(&built);

    // Unnamed statements are planned for their ids; a cached generic plan walks every poem.
    let mut rows_query =
        sqlx::query_as::<_, PoemListRow>(AssertSqlSafe(rows_sql)).persistent(false);
    let mut count_query =
        sqlx::query_scalar::<_, Option<i32>>(AssertSqlSafe(count_sql)).persistent(false);
    for ids in &built.bound {
        rows_query = rows_query.bind(*ids);
        count_query = count_query.bind(*ids);
    }
    rows_query = rows_query
        .bind(i64::from(page_size))
        .bind(i64::from(page.saturating_sub(1)).saturating_mul(i64::from(page_size)));

    let (rows, total) = tokio::try_join!(rows_query.fetch_all(pg), count_query.fetch_one(pg))?;
    Ok((
        rows.into_iter().map(PoemListItem::from).collect(),
        total.unwrap_or(0),
    ))
}

#[derive(Serialize, ToSchema)]
pub struct PoemFacets {
    pub meters: Vec<PoemCountStats>,
    pub rhymes: Vec<PoemCountStats>,
    pub themes: Vec<PoemCountStats>,
}

struct FacetQuery<'a> {
    sql: String,
    bound: Vec<&'a Vec<i32>>,
    selected: &'a Vec<String>,
}

fn facet_sql<'a>(
    counted: Filter,
    filters: &'a [FilterIds],
    selected: &'a Vec<String>,
) -> FacetQuery<'a> {
    let Clauses {
        conditions, bound, ..
    } = clauses_except(filters, Some(counted));
    let sql = format!(
        "SELECT t.name, t.slug, COUNT(p.id)::int AS poems_count FROM {} t \
         LEFT JOIN public.poems p ON {} = t.id AND {} \
         GROUP BY t.id, t.name, t.slug \
         HAVING COUNT(p.id) > 0 OR t.slug = ANY(${}) \
         ORDER BY poems_count DESC, t.name",
        counted.table(),
        counted.column(),
        conditions.join(" AND "),
        bound.len().saturating_add(1)
    );
    FacetQuery {
        sql,
        bound,
        selected,
    }
}

async fn count_facet(
    pg: &PgPool,
    counted: Filter,
    filters: &[FilterIds],
    facets: &Facets,
) -> Result<Vec<PoemCountStats>, AppError> {
    let built = facet_sql(counted, filters, facets.values(counted));
    let mut query = sqlx::query_as::<_, PoemCountStats>(AssertSqlSafe(built.sql)).persistent(false);
    for ids in built.bound {
        query = query.bind(ids);
    }
    Ok(query.bind(built.selected).fetch_all(pg).await?)
}

async fn poet_is_shown(pg: &PgPool, poets: &[String]) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM public.poets WHERE slug = ANY($1) AND NOT is_hidden)",
    )
    .bind(poets)
    .fetch_one(pg)
    .await?)
}

pub async fn facets(pg: &PgPool, facets: &Facets) -> Result<PoemFacets, AppError> {
    let (is_shown, filters) =
        tokio::try_join!(poet_is_shown(pg, &facets.poet), resolve_ids(pg, facets))?;
    if !is_shown {
        return Err(AppError::NotFound(Resource::Poet));
    }
    let (meters, rhymes, themes) = tokio::try_join!(
        count_facet(pg, Filter::Meter, &filters, facets),
        count_facet(pg, Filter::Rhyme, &filters, facets),
        count_facet(pg, Filter::Theme, &filters, facets),
    )?;
    Ok(PoemFacets {
        meters,
        rhymes,
        themes,
    })
}

const DETAIL_SQL: &str = r#"
      SELECT
        p.slug,
        p.title,
        (
          SELECT string_agg(v.content, '*' ORDER BY pv.position)
          FROM public.poem_verses pv
          JOIN  public.verses     v ON v.id = pv.verse_id
          WHERE pv.poem_id = p.id
        ) AS content,
        p.verse_count,
        (
          SELECT jsonb_build_object('title', pp.title, 'slug', pp.slug)
          FROM public.poems pp
          WHERE pp.poet_id = p.poet_id AND pp.id < p.id AND pp.recension_of_id IS NULL AND NOT pp.is_hidden
          ORDER BY pp.id DESC LIMIT 1
        ) AS prev_poem,
        (
          SELECT jsonb_build_object('title', np.title, 'slug', np.slug)
          FROM public.poems np
          WHERE np.poet_id = p.poet_id AND np.id > p.id AND np.recension_of_id IS NULL AND NOT np.is_hidden
          ORDER BY np.id ASC LIMIT 1
        ) AS next_poem,
        (
          SELECT jsonb_build_object('title', rp.title, 'slug', rp.slug)
          FROM public.poems rp
          WHERE rp.id = p.recension_of_id
        ) AS recension_of,
        COALESCE((
          SELECT jsonb_agg(
                   jsonb_build_object('title', rc.title, 'slug', rc.slug, 'verse_count', rc.verse_count)
                   ORDER BY rc.recension_of_id IS NOT NULL, rc.id)
          FROM public.poems rc
          WHERE rc.id <> p.id
            AND (rc.id = COALESCE(p.recension_of_id, p.id)
                 OR rc.recension_of_id = COALESCE(p.recension_of_id, p.id))
        ), '[]'::jsonb) AS recensions,
        pt.name        AS poet_name,
        pt.slug        AS poet_slug,
        pt.has_avatar  AS poet_has_avatar,
        pt.is_anonymous AS poet_is_anonymous,
        m.name   AS meter_name,
        m.slug   AS meter_slug,
        th.name  AS theme_name,
        th.slug  AS theme_slug,
        e.name   AS era_name,
        e.slug   AS era_slug,
        r.name   AS rhyme_name,
        r.slug   AS rhyme_slug,
        ty.name  AS poem_type_name,
        ty.slug  AS poem_type_slug,
        COALESCE(
          jsonb_agg(
            jsonb_build_object(
              'title',      rp.title,
              'slug',       rp.slug,
              'poet_name',       rpt.name,
              'poet_slug',       rpt.slug,
              'poet_has_avatar', rpt.has_avatar,
              'poet_is_anonymous', rpt.is_anonymous,
              'meter_name', rm.name,
              'meter_slug', rm.slug,
              'era_name',   re.name,
              'era_slug',   re.slug
            ) ORDER BY pr.rank
          ) FILTER (WHERE pr.related_id IS NOT NULL),
          '[]'::jsonb
        ) AS related_poems
      FROM public.poems p
      JOIN  public.poets  pt  ON pt.id = p.poet_id
      JOIN  public.eras   e   ON e.id  = pt.era_id
      JOIN  public.meters m   ON m.id  = p.meter_id
      JOIN  public.themes th  ON th.id = p.theme_id
      JOIN  public.rhymes r   ON r.id  = p.rhyme_id
      JOIN  public.poem_types ty ON ty.id = p.poem_type_id
      LEFT JOIN public.poem_relations  pr  ON pr.poem_id = p.id
      LEFT JOIN public.poems           rp  ON rp.id = pr.related_id
      LEFT JOIN public.poets           rpt ON rpt.id = rp.poet_id
      LEFT JOIN public.eras            re  ON re.id = rpt.era_id
      LEFT JOIN public.meters          rm  ON rm.id = rp.meter_id
      WHERE p.slug = $1 AND NOT p.is_hidden
      GROUP BY
        p.id, p.slug, p.title, p.verse_count,
        pt.name, pt.slug, pt.has_avatar, pt.is_anonymous, m.name, m.slug,
        th.name, th.slug, e.name, e.slug, r.name, r.slug, ty.name, ty.slug
"#;

pub async fn get(pg: &PgPool, slug: &str) -> Result<PoemDetail, AppError> {
    let row = sqlx::query_as::<_, PoemDetailRow>(DETAIL_SQL)
        .bind(slug)
        .fetch_optional(pg)
        .await?
        .ok_or(AppError::NotFound(Resource::Poem))?;

    let content = row.content.ok_or(AppError::PoemParse)?;
    let related: Vec<RelatedRow> =
        serde_json::from_value(row.related_poems.0).map_err(|_| AppError::PoemParse)?;
    let parsed = parse_poem_content(&content);

    Ok(PoemDetail {
        title: row.title,
        slug: slug.to_string(),
        verses: parsed.verses,
        verse_count: row.verse_count,
        sample: parsed.sample,
        keywords: parsed.keywords,
        poet: PoetRef {
            name: row.poet_name,
            slug: row.poet_slug,
            has_avatar: row.poet_has_avatar,
            is_anonymous: row.poet_is_anonymous,
        },
        era: EraRef {
            name: row.era_name,
            slug: row.era_slug,
        },
        meter: MeterRef {
            name: row.meter_name,
            slug: row.meter_slug,
        },
        theme: ThemeRef {
            name: row.theme_name,
            slug: row.theme_slug,
        },
        rhyme: RhymeRef {
            name: row.rhyme_name,
            slug: row.rhyme_slug,
        },
        poem_type: PoemTypeRef {
            name: row.poem_type_name,
            slug: row.poem_type_slug,
        },
        prev: row.prev_poem.map(|j| PoemNavRef {
            title: j.0.title,
            slug: j.0.slug,
        }),
        next: row.next_poem.map(|j| PoemNavRef {
            title: j.0.title,
            slug: j.0.slug,
        }),
        recension_of: row.recension_of.map(|j| PoemNavRef {
            title: j.0.title,
            slug: j.0.slug,
        }),
        recensions: row
            .recensions
            .0
            .into_iter()
            .map(|r| PoemRecensionRef {
                title: r.title,
                slug: r.slug,
                verse_count: r.verse_count,
            })
            .collect(),
        related_poems: related.into_iter().map(PoemListItem::from).collect(),
    })
}

pub async fn alias_target(pg: &PgPool, slug: &str) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT p.slug FROM public.poem_aliases a \
         JOIN public.poems p ON p.id = a.poem_id WHERE a.slug = $1 AND NOT p.is_hidden",
    )
    .bind(slug)
    .fetch_optional(pg)
    .await?)
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

async fn fetch_random_poem(pg: &PgPool) -> Result<RandomPoem, AppError> {
    let payload: Option<sqlx::types::Json<RandomPoem>> =
        sqlx::query_scalar("SELECT random_poem_json()")
            .fetch_optional(pg)
            .await?
            .flatten();
    Ok(payload.ok_or_else(random_poem_failed)?.0)
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
    fn the_era_filter_reads_the_poems_own_era_without_joining_poets() {
        let (rows, count) = list_sql(&clauses(&[ids(Filter::Era, &[3, 4])]));
        assert_eq!(
            count,
            "SELECT COUNT(*)::int AS total FROM public.poems p \
             WHERE p.recension_of_id IS NULL AND NOT p.is_hidden AND p.era_id = ANY($1)"
        );
        assert_eq!(rows.matches("JOIN public.poets").count(), 1, "{rows}");
        assert!(!rows.contains("public.eras e"), "{rows}");
    }
    #[test]
    fn conditions_stay_in_bind_order() {
        let filters = [ids(Filter::Era, &[3]), ids(Filter::Rhyme, &[7])];
        let both = clauses(&filters);
        assert_eq!(
            both.where_clause(),
            "WHERE p.recension_of_id IS NULL AND NOT p.is_hidden AND p.era_id = ANY($1) AND p.rhyme_id = ANY($2)"
        );
        assert_eq!(both.bound.len(), 2);
    }

    #[test]
    fn a_filter_matches_its_resolved_ids_whether_one_or_many() {
        for resolved in [&[5][..], &[5, 9][..]] {
            let filters = [ids(Filter::Meter, resolved)];
            assert_eq!(
                clauses(&filters).where_clause(),
                "WHERE p.recension_of_id IS NULL AND NOT p.is_hidden AND p.meter_id = ANY($1)"
            );
        }
    }

    #[test]
    fn the_slug_lookup_reads_each_requested_filter_table_in_bind_order() {
        assert_eq!(
            resolve_sql(&[Filter::Poet, Filter::Meter]),
            "SELECT ARRAY(SELECT id FROM public.poets WHERE slug = ANY($1)), \
             ARRAY(SELECT id FROM public.meters WHERE slug = ANY($2))"
        );
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
    fn every_filter_together_binds_one_id_array_each_in_order() {
        let filters: Vec<FilterIds> = Filter::ALL
            .into_iter()
            .map(|filter| ids(filter, &[1, 2]))
            .collect();
        let all = clauses(&filters);
        assert_eq!(all.bound.len(), 6);
        assert_eq!(
            all.where_clause(),
            "WHERE p.recension_of_id IS NULL AND NOT p.is_hidden AND p.poet_id = ANY($1) AND p.era_id = ANY($2) \
             AND p.meter_id = ANY($3) AND p.theme_id = ANY($4) \
             AND p.rhyme_id = ANY($5) AND p.collection_id = ANY($6)"
        );
    }
    #[test]
    fn the_list_sql_numbers_limit_and_offset_after_the_facet_binds() {
        let (rows, count) = list_sql(&clauses(&[]));
        assert!(rows.starts_with("SELECT p.title AS title, p.slug AS slug, pt.name AS poet_name"));
        assert!(
            rows.contains("ORDER BY p.id LIMIT $1 OFFSET $2) page"),
            "{rows}"
        );
        assert!(rows.ends_with("ORDER BY p.id"), "{rows}");
        assert!(rows.contains("JOIN public.poets pt") && rows.contains("JOIN public.meters m"));
        assert!(rows.contains("WHERE p.recension_of_id IS NULL AND NOT p.is_hidden ORDER BY"));
        assert!(count.starts_with("SELECT COUNT(*)::int AS total FROM public.poems p"));
        assert!(!count.contains("JOIN"));

        let (rows, count) = list_sql(&clauses(&[ids(Filter::Era, &[3, 4])]));
        assert!(
            rows.contains("ANY($1) ORDER BY p.id LIMIT $2 OFFSET $3) page"),
            "{rows}"
        );
        assert!(
            count.ends_with(
                "WHERE p.recension_of_id IS NULL AND NOT p.is_hidden AND p.era_id = ANY($1)"
            ),
            "{count}"
        );
    }
    #[test]
    fn every_list_query_hides_recensions_and_hidden_poets_even_without_a_facet() {
        let (rows, count) = list_sql(&clauses(&[]));
        assert!(
            rows.contains("WHERE p.recension_of_id IS NULL AND NOT p.is_hidden ORDER BY p.id"),
            "{rows}"
        );
        assert_eq!(
            count,
            "SELECT COUNT(*)::int AS total FROM public.poems p WHERE p.recension_of_id IS NULL AND NOT p.is_hidden"
        );
        let two = [ids(Filter::Meter, &[5]), ids(Filter::Theme, &[2])];
        let (rows, count) = list_sql(&clauses(&two));
        assert!(
            rows.contains("WHERE p.recension_of_id IS NULL AND NOT p.is_hidden AND p.meter_id"),
            "{rows}"
        );
        assert!(
            count.contains("WHERE p.recension_of_id IS NULL AND NOT p.is_hidden AND p.meter_id"),
            "{count}"
        );
    }
    #[test]
    fn the_detail_query_finds_prev_and_next_through_the_shown_primaries_index() {
        for alias in ["pp", "np"] {
            let predicate = format!("{alias}.recension_of_id IS NULL AND NOT {alias}.is_hidden");
            assert!(DETAIL_SQL.contains(&predicate), "missing: {predicate}");
        }
    }

    #[test]
    fn the_detail_query_never_returns_a_hidden_poem() {
        assert!(DETAIL_SQL.contains("WHERE p.slug = $1 AND NOT p.is_hidden"));
    }

    #[test]
    fn a_single_term_is_counted_from_its_stats_table() {
        for (filter, stats_table) in [
            (Filter::Poet, "poet_stats"),
            (Filter::Era, "era_stats"),
            (Filter::Meter, "meter_stats"),
            (Filter::Theme, "theme_stats"),
            (Filter::Rhyme, "rhyme_stats"),
            (Filter::Collection, "collection_stats"),
        ] {
            let (_, count) = list_sql(&clauses(&[ids(filter, &[7])]));
            assert_eq!(
                count,
                format!(
                    "SELECT (SELECT poems_count::int FROM public.{stats_table} WHERE id = ($1)[1]) AS total"
                )
            );
        }
    }
    #[test]
    fn anything_wider_than_one_term_counts_the_poems_themselves() {
        for wider in [
            vec![],
            vec![ids(Filter::Theme, &[1, 2])],
            vec![ids(Filter::Theme, &[1]), ids(Filter::Meter, &[2])],
        ] {
            let (_, count) = list_sql(&clauses(&wider));
            assert!(
                count.starts_with("SELECT COUNT(*)::int AS total FROM public.poems p"),
                "{count}"
            );
        }
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

    #[test]
    fn a_facet_is_counted_under_the_poet_and_the_other_selections_but_not_its_own() {
        let filters = [
            ids(Filter::Poet, &[11]),
            ids(Filter::Meter, &[5]),
            ids(Filter::Rhyme, &[7, 8]),
        ];
        let selected = vec!["altawil".to_string()];
        let meters = facet_sql(Filter::Meter, &filters, &selected);
        assert_eq!(
            meters.sql,
            "SELECT t.name, t.slug, COUNT(p.id)::int AS poems_count FROM public.meters t \
             LEFT JOIN public.poems p ON p.meter_id = t.id AND p.recension_of_id IS NULL \
             AND NOT p.is_hidden AND p.poet_id = ANY($1) AND p.rhyme_id = ANY($2) \
             GROUP BY t.id, t.name, t.slug \
             HAVING COUNT(p.id) > 0 OR t.slug = ANY($3) \
             ORDER BY poems_count DESC, t.name"
        );
        assert_eq!(meters.bound, [&filters[0].ids, &filters[2].ids]);
        assert_eq!(meters.selected, &selected);
    }
}
