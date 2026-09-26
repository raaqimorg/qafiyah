use serde::{Deserialize, Serialize};
use sqlx::{AssertSqlSafe, PgPool};
use utoipa::ToSchema;
use utoipa::openapi::Schema;

use crate::constants::{MAX_TWEET_LENGTH, RANDOM_POEM_MAX_ATTEMPTS};
use crate::domain::{CollectionRef, EraRef, MeterRef, PoemTypeRef, PoetRef, RhymeRef, ThemeRef};
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
    pub collection: Option<CollectionRef>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavScope {
    Poet,
    Theme,
    Meter,
    Rhyme,
    Collection,
}

impl NavScope {
    pub fn from_param(value: &str) -> Option<Self> {
        match value {
            "theme" => Some(Self::Theme),
            "meter" => Some(Self::Meter),
            "rhyme" => Some(Self::Rhyme),
            "collection" => Some(Self::Collection),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Poet => "poet",
            Self::Theme => "theme",
            Self::Meter => "meter",
            Self::Rhyme => "rhyme",
            Self::Collection => "collection",
        }
    }

    fn column(self) -> &'static str {
        match self {
            Self::Poet => "poet_id",
            Self::Theme => "theme_id",
            Self::Meter => "meter_id",
            Self::Rhyme => "rhyme_id",
            Self::Collection => "collection_id",
        }
    }
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
    collection_name: Option<String>,
    collection_slug: Option<String>,
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
        "SELECT COUNT(*)::int AS total FROM poems WHERE recension_of_id IS NULL",
    )
    .fetch_one(pg)
    .await?;
    Ok(total.unwrap_or(0))
}

pub async fn list_slugs(pg: &PgPool, page: u32, page_size: u32) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT slug FROM poems WHERE recension_of_id IS NULL ORDER BY slug LIMIT $1 OFFSET $2",
    )
    .bind(i64::from(page_size))
    .bind(i64::from(page.saturating_sub(1)).saturating_mul(i64::from(page_size)))
    .fetch_all(pg)
    .await?)
}

struct Clauses<'a> {
    where_clause: String,
    counted_by: Option<&'static str>,
    bound: Vec<&'a Vec<String>>,
}

fn clauses(facets: &Facets) -> Clauses<'_> {
    let mut conditions: Vec<String> = vec!["p.recension_of_id IS NULL".to_string()];
    let mut bound: Vec<&Vec<String>> = Vec::new();
    let mut stats: Vec<&'static str> = Vec::new();
    for (column, table, stats_table, values) in [
        (
            "p.poet_id",
            "public.poets",
            "public.poet_stats",
            &facets.poet,
        ),
        ("p.era_id", "public.eras", "public.era_stats", &facets.era),
        (
            "p.meter_id",
            "public.meters",
            "public.meter_stats",
            &facets.meter,
        ),
        (
            "p.theme_id",
            "public.themes",
            "public.theme_stats",
            &facets.theme,
        ),
        (
            "p.rhyme_id",
            "public.rhymes",
            "public.rhyme_stats",
            &facets.rhyme,
        ),
        (
            "p.collection_id",
            "public.collections",
            "public.collection_stats",
            &facets.collection,
        ),
    ] {
        if values.is_empty() {
            continue;
        }
        bound.push(values);
        stats.push(stats_table);
        let n = bound.len();
        if values.len() == 1 {
            conditions.push(format!(
                "{column} = (SELECT id FROM {table} WHERE slug = (${n})[1])"
            ));
        } else {
            conditions.push(format!(
                "{column} IN (SELECT id FROM {table} WHERE slug = ANY(${n}))"
            ));
        }
    }

    let where_clause = format!("WHERE {}", conditions.join(" AND "));

    let counted_by = match (bound.as_slice(), stats.as_slice()) {
        ([values], [stats_table]) if values.len() == 1 => Some(*stats_table),
        _ => None,
    };

    Clauses {
        where_clause,
        counted_by,
        bound,
    }
}

fn list_sql(clauses: &Clauses<'_>) -> (String, String) {
    let Clauses {
        where_clause,
        counted_by,
        bound,
    } = clauses;
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
            "SELECT (SELECT poems_count::int FROM {stats_table} WHERE slug = ($1)[1]) AS total"
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
    let built = clauses(facets);
    let (rows_sql, count_sql) = list_sql(&built);

    let mut rows_query = sqlx::query_as::<_, PoemListRow>(AssertSqlSafe(rows_sql));
    let mut count_query = sqlx::query_scalar::<_, Option<i32>>(AssertSqlSafe(count_sql));
    for values in &built.bound {
        rows_query = rows_query.bind(*values);
        count_query = count_query.bind(*values);
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

fn detail_sql(scope: NavScope) -> String {
    let column = scope.column();
    format!(
        r#"
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
          WHERE pp.{column} = p.{column} AND pp.id < p.id AND pp.recension_of_id IS NULL
          ORDER BY pp.id DESC LIMIT 1
        ) AS prev_poem,
        (
          SELECT jsonb_build_object('title', np.title, 'slug', np.slug)
          FROM public.poems np
          WHERE np.{column} = p.{column} AND np.id > p.id AND np.recension_of_id IS NULL
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
        c.name   AS collection_name,
        c.slug   AS collection_slug,
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
      LEFT JOIN public.collections     c   ON c.id = p.collection_id
      LEFT JOIN public.poem_relations  pr  ON pr.poem_id = p.id
      LEFT JOIN public.poems           rp  ON rp.id = pr.related_id
      LEFT JOIN public.poets           rpt ON rpt.id = rp.poet_id
      LEFT JOIN public.eras            re  ON re.id = rpt.era_id
      LEFT JOIN public.meters          rm  ON rm.id = rp.meter_id
      WHERE p.slug = $1
      GROUP BY
        p.id, p.slug, p.title, p.verse_count,
        pt.name, pt.slug, pt.has_avatar, pt.is_anonymous, m.name, m.slug,
        th.name, th.slug, e.name, e.slug, r.name, r.slug, ty.name, ty.slug,
        c.name, c.slug
    "#
    )
}

pub async fn get(pg: &PgPool, slug: &str, scope: NavScope) -> Result<PoemDetail, AppError> {
    let row = sqlx::query_as::<_, PoemDetailRow>(AssertSqlSafe(detail_sql(scope)))
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
        collection: row
            .collection_name
            .zip(row.collection_slug)
            .map(|(name, slug)| CollectionRef { name, slug }),
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
         JOIN public.poems p ON p.id = a.poem_id WHERE a.slug = $1",
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

    #[test]
    fn the_era_filter_reads_the_poems_own_era_without_joining_poets() {
        let facets = Facets {
            era: vec!["abbasi".into(), "umawi".into()],
            ..Facets::default()
        };
        let (rows, count) = list_sql(&clauses(&facets));
        assert_eq!(
            count,
            "SELECT COUNT(*)::int AS total FROM public.poems p \
             WHERE p.recension_of_id IS NULL AND p.era_id IN (SELECT id FROM public.eras WHERE slug = ANY($1))"
        );
        assert_eq!(rows.matches("JOIN public.poets").count(), 1, "{rows}");
        assert!(!rows.contains("public.eras e"), "{rows}");
    }

    #[test]
    fn conditions_stay_in_bind_order() {
        let facets = Facets {
            era: vec!["abbasi".into()],
            rhyme: vec!["meem".into()],
            ..Facets::default()
        };
        let both = clauses(&facets);
        assert_eq!(
            both.where_clause,
            "WHERE p.recension_of_id IS NULL AND p.era_id = (SELECT id FROM public.eras WHERE slug = ($1)[1]) \
             AND p.rhyme_id = (SELECT id FROM public.rhymes WHERE slug = ($2)[1])"
        );
        assert_eq!(both.bound.len(), 2);
    }

    #[test]
    fn a_single_facet_value_resolves_to_a_scalar_fk_lookup() {
        let facets = Facets {
            meter: vec!["altawil".into()],
            ..Facets::default()
        };
        let single = clauses(&facets);
        assert_eq!(
            single.where_clause,
            "WHERE p.recension_of_id IS NULL AND p.meter_id = (SELECT id FROM public.meters WHERE slug = ($1)[1])"
        );
    }

    #[test]
    fn multiple_facet_values_fall_back_to_a_set_lookup() {
        let facets = Facets {
            meter: vec!["altawil".into(), "alkamil".into()],
            ..Facets::default()
        };
        let multi = clauses(&facets);
        assert_eq!(
            multi.where_clause,
            "WHERE p.recension_of_id IS NULL AND p.meter_id IN (SELECT id FROM public.meters WHERE slug = ANY($1))"
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
    fn every_facet_together_binds_in_declaration_order() {
        let facets = Facets {
            poet: vec!["yoFB".into()],
            era: vec!["abbasi".into(), "jahili".into()],
            theme: vec!["alnasib".into()],
            meter: vec!["altawil".into()],
            rhyme: vec!["meem".into()],
            collection: vec!["almuallaqat".into()],
        };
        let all = clauses(&facets);
        assert_eq!(all.bound.len(), 6);
        assert_eq!(
            all.where_clause,
            "WHERE p.recension_of_id IS NULL AND p.poet_id = (SELECT id FROM public.poets WHERE slug = ($1)[1]) \
             AND p.era_id IN (SELECT id FROM public.eras WHERE slug = ANY($2)) \
             AND p.meter_id = (SELECT id FROM public.meters WHERE slug = ($3)[1]) \
             AND p.theme_id = (SELECT id FROM public.themes WHERE slug = ($4)[1]) \
             AND p.rhyme_id = (SELECT id FROM public.rhymes WHERE slug = ($5)[1]) \
             AND p.collection_id = (SELECT id FROM public.collections WHERE slug = ($6)[1])"
        );
    }

    #[test]
    fn the_detail_neighbors_step_through_the_requested_scope_only() {
        for (scope, column) in [
            (NavScope::Poet, "poet_id"),
            (NavScope::Theme, "theme_id"),
            (NavScope::Meter, "meter_id"),
            (NavScope::Rhyme, "rhyme_id"),
            (NavScope::Collection, "collection_id"),
        ] {
            let sql = detail_sql(scope);
            assert!(sql.contains(&format!("WHERE pp.{column} = p.{column} AND pp.id < p.id")));
            assert!(sql.contains(&format!("WHERE np.{column} = p.{column} AND np.id > p.id")));
        }
    }

    #[test]
    fn the_by_param_names_every_scope_but_the_default_poet() {
        assert_eq!(NavScope::from_param("theme"), Some(NavScope::Theme));
        assert_eq!(NavScope::from_param("meter"), Some(NavScope::Meter));
        assert_eq!(NavScope::from_param("rhyme"), Some(NavScope::Rhyme));
        assert_eq!(
            NavScope::from_param("collection"),
            Some(NavScope::Collection)
        );
        assert_eq!(NavScope::from_param("poet"), None);
        assert_eq!(NavScope::from_param("era"), None);
        assert_eq!(NavScope::from_param("Theme"), None);
    }

    #[test]
    fn the_list_sql_numbers_limit_and_offset_after_the_facet_binds() {
        let unfiltered = Facets::default();
        let none = clauses(&unfiltered);
        let (rows, count) = list_sql(&none);
        assert!(rows.starts_with("SELECT p.title AS title, p.slug AS slug, pt.name AS poet_name"));
        assert!(
            rows.contains("ORDER BY p.id LIMIT $1 OFFSET $2) page"),
            "{rows}"
        );
        assert!(rows.ends_with("ORDER BY p.id"), "{rows}");
        assert!(rows.contains("JOIN public.poets pt") && rows.contains("JOIN public.meters m"));
        assert!(rows.contains("WHERE p.recension_of_id IS NULL ORDER BY"));
        assert!(count.starts_with("SELECT COUNT(*)::int AS total FROM public.poems p"));
        assert!(!count.contains("JOIN"));

        let two_eras = Facets {
            era: vec!["abbasi".into(), "umawi".into()],
            ..Facets::default()
        };
        let by_era = clauses(&two_eras);
        let (rows, count) = list_sql(&by_era);
        assert!(
            rows.contains("ANY($1)) ORDER BY p.id LIMIT $2 OFFSET $3) page"),
            "{rows}"
        );
        assert!(
            count.ends_with("WHERE p.recension_of_id IS NULL AND p.era_id IN (SELECT id FROM public.eras WHERE slug = ANY($1))"),
            "{count}"
        );
    }

    #[test]
    fn every_list_query_hides_recensions_even_without_a_facet() {
        let (rows, count) = list_sql(&clauses(&Facets::default()));
        assert!(
            rows.contains("WHERE p.recension_of_id IS NULL ORDER BY p.id"),
            "{rows}"
        );
        assert_eq!(
            count,
            "SELECT COUNT(*)::int AS total FROM public.poems p WHERE p.recension_of_id IS NULL"
        );
        let two = Facets {
            theme: vec!["alnasib".into()],
            meter: vec!["altawil".into()],
            ..Facets::default()
        };
        let (rows, count) = list_sql(&clauses(&two));
        assert!(
            rows.contains("WHERE p.recension_of_id IS NULL AND p.meter_id"),
            "{rows}"
        );
        assert!(
            count.contains("WHERE p.recension_of_id IS NULL AND p.meter_id"),
            "{count}"
        );
    }

    #[test]
    fn a_single_term_is_counted_from_its_stats_table() {
        let one = |facets: Facets| list_sql(&clauses(&facets)).1;
        let term = || vec!["x".to_string()];
        for (count, stats_table) in [
            (
                one(Facets {
                    poet: term(),
                    ..Facets::default()
                }),
                "poet_stats",
            ),
            (
                one(Facets {
                    era: term(),
                    ..Facets::default()
                }),
                "era_stats",
            ),
            (
                one(Facets {
                    meter: term(),
                    ..Facets::default()
                }),
                "meter_stats",
            ),
            (
                one(Facets {
                    theme: term(),
                    ..Facets::default()
                }),
                "theme_stats",
            ),
            (
                one(Facets {
                    rhyme: term(),
                    ..Facets::default()
                }),
                "rhyme_stats",
            ),
            (
                one(Facets {
                    collection: term(),
                    ..Facets::default()
                }),
                "collection_stats",
            ),
        ] {
            assert_eq!(
                count,
                format!(
                    "SELECT (SELECT poems_count::int FROM public.{stats_table} WHERE slug = ($1)[1]) AS total"
                )
            );
        }
    }

    #[test]
    fn anything_wider_than_one_term_counts_the_poems_themselves() {
        for wider in [
            Facets::default(),
            Facets {
                theme: vec!["alnasib".into(), "almadih".into()],
                ..Facets::default()
            },
            Facets {
                theme: vec!["alnasib".into()],
                meter: vec!["altawil".into()],
                ..Facets::default()
            },
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
}
