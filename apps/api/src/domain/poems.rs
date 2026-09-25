use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sqlx::{AssertSqlSafe, PgPool};
use utoipa::ToSchema;
use utoipa::openapi::Schema;

use crate::constants::{MAX_TWEET_LENGTH, RANDOM_POEM_MAX_ATTEMPTS};
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

#[derive(sqlx::FromRow)]
struct PoemDetailRow {
    title: String,
    content: Option<String>,
    verse_count: i32,
    poet_name: String,
    poet_slug: String,
    poet_has_avatar: bool,
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
    let total: Option<i32> = sqlx::query_scalar("SELECT COUNT(*)::int AS total FROM poems")
        .fetch_one(pg)
        .await?;
    Ok(total.unwrap_or(0))
}

pub async fn list_slugs(pg: &PgPool, page: u32, page_size: u32) -> Result<Vec<String>, AppError> {
    Ok(
        sqlx::query_scalar("SELECT slug FROM poems ORDER BY slug LIMIT $1 OFFSET $2")
            .bind(i64::from(page_size))
            .bind(i64::from(page.saturating_sub(1)).saturating_mul(i64::from(page_size)))
            .fetch_all(pg)
            .await?,
    )
}

struct Clauses<'a> {
    filter_joins: String,
    where_clause: String,
    bound: Vec<&'a Vec<String>>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Table {
    Poets,
    Eras,
}

impl Table {
    fn join(self) -> &'static str {
        match self {
            Table::Poets => "JOIN public.poets pt ON p.poet_id = pt.id",
            Table::Eras => "JOIN public.eras e ON pt.era_id = e.id",
        }
    }
}

fn join_sql(tables: &BTreeSet<Table>) -> String {
    tables
        .iter()
        .map(|table| table.join())
        .collect::<Vec<&str>>()
        .join(" ")
}

enum FacetKind {
    Fk {
        column: &'static str,
        table: &'static str,
    },
    Slug {
        column: &'static str,
        reached_through: &'static [Table],
    },
}

fn clauses(facets: &Facets) -> Clauses<'_> {
    let mut filtered: BTreeSet<Table> = BTreeSet::new();
    let mut conditions: Vec<String> = Vec::new();
    let mut bound: Vec<&Vec<String>> = Vec::new();
    for (kind, values) in [
        (
            FacetKind::Fk {
                column: "p.poet_id",
                table: "public.poets",
            },
            &facets.poet,
        ),
        (
            FacetKind::Slug {
                column: "e.slug",
                reached_through: &[Table::Poets, Table::Eras],
            },
            &facets.era,
        ),
        (
            FacetKind::Fk {
                column: "p.meter_id",
                table: "public.meters",
            },
            &facets.meter,
        ),
        (
            FacetKind::Fk {
                column: "p.theme_id",
                table: "public.themes",
            },
            &facets.theme,
        ),
        (
            FacetKind::Fk {
                column: "p.rhyme_id",
                table: "public.rhymes",
            },
            &facets.rhyme,
        ),
        (
            FacetKind::Fk {
                column: "p.collection_id",
                table: "public.collections",
            },
            &facets.collection,
        ),
    ] {
        if values.is_empty() {
            continue;
        }
        bound.push(values);
        let n = bound.len();
        match kind {
            FacetKind::Fk { column, table } if values.len() == 1 => {
                conditions.push(format!(
                    "{column} = (SELECT id FROM {table} WHERE slug = (${n})[1])"
                ));
            }
            FacetKind::Fk { column, table } => {
                conditions.push(format!(
                    "{column} IN (SELECT id FROM {table} WHERE slug = ANY(${n}))"
                ));
            }
            FacetKind::Slug {
                column,
                reached_through,
            } => {
                filtered.extend(reached_through);
                conditions.push(format!("{column} = ANY(${n})"));
            }
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    Clauses {
        filter_joins: join_sql(&filtered),
        where_clause,
        bound,
    }
}

fn list_sql(filter_joins: &str, where_clause: &str, bind_count: usize) -> (String, String) {
    let rows_sql = format!(
        "SELECT p.title AS title, p.slug AS slug, pt.name AS poet_name, pt.slug AS poet_slug, \
         pt.has_avatar AS poet_has_avatar, \
         m.name AS meter_name, m.slug AS meter_slug \
         FROM (SELECT p.id FROM public.poems p {filter_joins} {where_clause} \
         ORDER BY p.id LIMIT ${} OFFSET ${}) page \
         JOIN public.poems p ON p.id = page.id \
         JOIN public.poets pt ON p.poet_id = pt.id \
         JOIN public.meters m ON p.meter_id = m.id \
         ORDER BY p.id",
        bind_count.saturating_add(1),
        bind_count.saturating_add(2)
    );
    let count_sql =
        format!("SELECT COUNT(*)::int AS total FROM public.poems p {filter_joins} {where_clause}");
    (rows_sql, count_sql)
}

pub async fn list(
    pg: &PgPool,
    facets: &Facets,
    page: u32,
    page_size: u32,
) -> Result<(Vec<PoemListItem>, i32), AppError> {
    let Clauses {
        filter_joins,
        where_clause,
        bound,
    } = clauses(facets);
    let (rows_sql, count_sql) = list_sql(&filter_joins, &where_clause, bound.len());

    let mut rows_query = sqlx::query_as::<_, PoemListRow>(AssertSqlSafe(rows_sql));
    let mut count_query = sqlx::query_scalar::<_, Option<i32>>(AssertSqlSafe(count_sql));
    for values in &bound {
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

pub async fn get(pg: &PgPool, slug: &str) -> Result<PoemDetail, AppError> {
    let row = sqlx::query_as::<_, PoemDetailRow>(
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
          WHERE pp.poet_id = p.poet_id AND pp.id < p.id
          ORDER BY pp.id DESC LIMIT 1
        ) AS prev_poem,
        (
          SELECT jsonb_build_object('title', np.title, 'slug', np.slug)
          FROM public.poems np
          WHERE np.poet_id = p.poet_id AND np.id > p.id
          ORDER BY np.id ASC LIMIT 1
        ) AS next_poem,
        pt.name        AS poet_name,
        pt.slug        AS poet_slug,
        pt.has_avatar  AS poet_has_avatar,
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
      WHERE p.slug = $1
      GROUP BY
        p.id, p.slug, p.title, p.verse_count,
        pt.name, pt.slug, pt.has_avatar, m.name, m.slug,
        th.name, th.slug, e.name, e.slug, r.name, r.slug, ty.name, ty.slug
    "#,
    )
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
        related_poems: related.into_iter().map(PoemListItem::from).collect(),
    })
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
    fn the_filter_joins_only_what_a_filter_reads() {
        let unfiltered = Facets::default();
        let none = clauses(&unfiltered);
        assert_eq!(none.filter_joins, "");

        let era_only = Facets {
            era: vec!["abbasi".into()],
            ..Facets::default()
        };
        let by_era = clauses(&era_only);
        assert!(by_era.filter_joins.contains("public.eras"));
        assert!(by_era.filter_joins.contains("public.poets"));
        assert!(!by_era.filter_joins.contains("public.meters"));
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
            "WHERE e.slug = ANY($1) AND p.rhyme_id = (SELECT id FROM public.rhymes WHERE slug = ($2)[1])"
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
            "WHERE p.meter_id = (SELECT id FROM public.meters WHERE slug = ($1)[1])"
        );
        assert!(single.filter_joins.is_empty());
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
            "WHERE p.meter_id IN (SELECT id FROM public.meters WHERE slug = ANY($1))"
        );
        assert!(multi.filter_joins.is_empty());
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
    fn every_facet_together_binds_in_declaration_order_and_joins_each_table_once() {
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
            "WHERE p.poet_id = (SELECT id FROM public.poets WHERE slug = ($1)[1]) \
             AND e.slug = ANY($2) \
             AND p.meter_id = (SELECT id FROM public.meters WHERE slug = ($3)[1]) \
             AND p.theme_id = (SELECT id FROM public.themes WHERE slug = ($4)[1]) \
             AND p.rhyme_id = (SELECT id FROM public.rhymes WHERE slug = ($5)[1]) \
             AND p.collection_id = (SELECT id FROM public.collections WHERE slug = ($6)[1])"
        );
        assert_eq!(
            all.filter_joins,
            "JOIN public.poets pt ON p.poet_id = pt.id JOIN public.eras e ON pt.era_id = e.id"
        );
    }

    #[test]
    fn the_list_sql_numbers_limit_and_offset_after_the_facet_binds() {
        let unfiltered = Facets::default();
        let none = clauses(&unfiltered);
        let (rows, count) = list_sql(&none.filter_joins, &none.where_clause, none.bound.len());
        assert!(rows.starts_with("SELECT p.title AS title, p.slug AS slug, pt.name AS poet_name"));
        assert!(
            rows.contains("ORDER BY p.id LIMIT $1 OFFSET $2) page"),
            "{rows}"
        );
        assert!(rows.ends_with("ORDER BY p.id"), "{rows}");
        assert!(rows.contains("JOIN public.poets pt") && rows.contains("JOIN public.meters m"));
        assert!(!rows.contains("WHERE"));
        assert!(count.starts_with("SELECT COUNT(*)::int AS total FROM public.poems p"));
        assert!(!count.contains("JOIN"));

        let era_only = Facets {
            era: vec!["abbasi".into()],
            ..Facets::default()
        };
        let by_era = clauses(&era_only);
        let (rows, count) = list_sql(
            &by_era.filter_joins,
            &by_era.where_clause,
            by_era.bound.len(),
        );
        assert!(
            rows.contains("WHERE e.slug = ANY($1) ORDER BY p.id LIMIT $2 OFFSET $3) page"),
            "{rows}"
        );
        assert_eq!(rows.matches("JOIN public.eras").count(), 1, "{rows}");
        assert!(count.ends_with("JOIN public.poets pt ON p.poet_id = pt.id JOIN public.eras e ON pt.era_id = e.id WHERE e.slug = ANY($1)"), "{count}");
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
