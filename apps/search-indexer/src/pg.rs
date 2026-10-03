use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use diesel::dsl::not;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use qafiyah_corpus::schema::{
    collections, eras, meters, poem_types, poem_verses, poems, poet_stats, poets, rhymes, themes,
    verses,
};

use crate::docs::{PoemSource, PoetSource};
use crate::reindex::CorpusSource;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const QUERY_TIMEOUT: Duration = Duration::from_secs(60);
const VERSE_SEPARATOR: &str = "*";

#[derive(Queryable, Selectable)]
#[diesel(table_name = poems, check_for_backend(Pg))]
struct PoemRow {
    id: i32,
    slug: String,
    title: String,
    #[diesel(select_expression = poets::name)]
    poet_name: String,
    #[diesel(select_expression = poets::slug)]
    poet_slug: String,
    #[diesel(select_expression = poets::has_avatar)]
    poet_has_avatar: bool,
    #[diesel(select_expression = poets::is_anonymous)]
    poet_is_anonymous: bool,
    #[diesel(select_expression = eras::name)]
    era_name: String,
    #[diesel(select_expression = eras::slug)]
    era_slug: String,
    #[diesel(select_expression = meters::name)]
    meter_name: String,
    #[diesel(select_expression = meters::slug)]
    meter_slug: String,
    #[diesel(select_expression = themes::slug)]
    theme_slug: String,
    #[diesel(select_expression = rhymes::slug)]
    rhyme_slug: String,
    #[diesel(select_expression = poem_types::slug)]
    poem_type_slug: String,
    #[diesel(select_expression = collections::slug.nullable())]
    collection_slug: Option<String>,
    recension_of_id: Option<i32>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = poets, check_for_backend(Pg))]
struct PoetRow {
    id: i32,
    slug: String,
    name: String,
    nickname: Option<String>,
    #[diesel(select_expression = eras::name)]
    era_name: String,
    #[diesel(select_expression = eras::slug)]
    era_slug: String,
    #[diesel(select_expression = poet_stats::poems_count.nullable())]
    poems_count: Option<i64>,
}

pub(crate) struct PgCorpus {
    conn: AsyncPgConnection,
}

pub(crate) async fn connect(url: &str) -> Result<PgCorpus, String> {
    let conn = tokio::time::timeout(CONNECT_TIMEOUT, AsyncPgConnection::establish(url))
        .await
        .map_err(|_| "postgres connect: timed out".to_string())?
        .map_err(|e| format!("postgres connect: {e}"))?;
    Ok(PgCorpus { conn })
}

async fn within<T>(stage: &str, query: impl Future<Output = QueryResult<T>>) -> Result<T, String> {
    tokio::time::timeout(QUERY_TIMEOUT, query)
        .await
        .map_err(|_| format!("{stage}: query timed out"))?
        .map_err(|e| format!("{stage}: {e}"))
}

fn poem_sources(rows: Vec<PoemRow>, verses: Vec<(i32, String)>) -> Vec<PoemSource> {
    let mut contents: HashMap<i32, Vec<String>> = HashMap::new();
    for (poem_id, content) in verses {
        contents.entry(poem_id).or_default().push(content);
    }
    rows.into_iter()
        .map(|row| PoemSource {
            content: contents
                .remove(&row.id)
                .map(|lines| lines.join(VERSE_SEPARATOR))
                .unwrap_or_default(),
            primary_id: row.recension_of_id.unwrap_or(row.id),
            is_primary: row.recension_of_id.is_none(),
            id: row.id,
            slug: row.slug,
            title: row.title,
            poet_name: row.poet_name,
            poet_slug: row.poet_slug,
            poet_has_avatar: row.poet_has_avatar,
            poet_is_anonymous: row.poet_is_anonymous,
            era_name: row.era_name,
            era_slug: row.era_slug,
            meter_name: row.meter_name,
            meter_slug: row.meter_slug,
            theme_slug: row.theme_slug,
            rhyme_slug: row.rhyme_slug,
            poem_type_slug: row.poem_type_slug,
            collection_slug: row.collection_slug.unwrap_or_default(),
        })
        .collect()
}

async fn stream_poem_batch(
    mut conn: &AsyncPgConnection,
    after_id: i32,
    limit: i64,
) -> Result<Vec<PoemSource>, String> {
    let rows: Vec<PoemRow> = within(
        "streamPoemBatch",
        poems::table
            .inner_join(poets::table.on(poets::id.eq(poems::poet_id)))
            .inner_join(eras::table.on(eras::id.eq(poets::era_id)))
            .inner_join(meters::table.on(meters::id.eq(poems::meter_id)))
            .inner_join(themes::table.on(themes::id.eq(poems::theme_id)))
            .inner_join(rhymes::table.on(rhymes::id.eq(poems::rhyme_id)))
            .inner_join(poem_types::table.on(poem_types::id.eq(poems::poem_type_id)))
            .left_join(collections::table.on(poems::collection_id.eq(collections::id.nullable())))
            .filter(poems::id.gt(after_id))
            .filter(not(poems::is_hidden))
            .order(poems::id.asc())
            .limit(limit)
            .select(PoemRow::as_select())
            .load(&mut conn),
    )
    .await?;
    let ids: Vec<i32> = rows.iter().map(|row| row.id).collect();
    let verses: Vec<(i32, String)> = within(
        "streamPoemBatch",
        poem_verses::table
            .inner_join(verses::table)
            .filter(poem_verses::poem_id.eq_any(ids))
            .order((poem_verses::poem_id.asc(), poem_verses::position.asc()))
            .select((poem_verses::poem_id, verses::content))
            .load(&mut conn),
    )
    .await?;
    Ok(poem_sources(rows, verses))
}

async fn count_rows(mut conn: &AsyncPgConnection, is_poems: bool) -> Result<i64, String> {
    if is_poems {
        within(
            "countRows",
            poems::table
                .filter(not(poems::is_hidden))
                .count()
                .get_result(&mut conn),
        )
        .await
    } else {
        within(
            "countRows",
            poets::table
                .filter(not(poets::is_hidden))
                .count()
                .get_result(&mut conn),
        )
        .await
    }
}

async fn stream_poet_batch(
    mut conn: &AsyncPgConnection,
    after_id: i32,
    limit: i64,
) -> Result<Vec<PoetSource>, String> {
    let rows: Vec<PoetRow> = within(
        "streamPoetBatch",
        poets::table
            .inner_join(eras::table.on(eras::id.eq(poets::era_id)))
            .left_join(poet_stats::table.on(poet_stats::slug.eq(poets::slug.nullable())))
            .filter(poets::id.gt(after_id))
            .filter(not(poets::is_hidden))
            .order(poets::id.asc())
            .limit(limit)
            .select(PoetRow::as_select())
            .load(&mut conn),
    )
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(PoetSource {
                poems_count: i32::try_from(row.poems_count.unwrap_or(0)).map_err(|_| {
                    format!("streamPoetBatch: poet {} poems_count out of range", row.id)
                })?,
                id: row.id,
                slug: row.slug,
                name: row.name,
                nickname: row.nickname.unwrap_or_default(),
                era_name: row.era_name,
                era_slug: row.era_slug,
            })
        })
        .collect()
}

#[async_trait]
impl CorpusSource for PgCorpus {
    async fn count(&self, is_poems: bool) -> Result<i64, String> {
        count_rows(&self.conn, is_poems).await
    }

    async fn poems_after(&self, after_id: i32, limit: i64) -> Result<Vec<PoemSource>, String> {
        stream_poem_batch(&self.conn, after_id, limit).await
    }

    async fn poets_after(&self, after_id: i32, limit: i64) -> Result<Vec<PoetSource>, String> {
        stream_poet_batch(&self.conn, after_id, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i32, recension_of_id: Option<i32>) -> PoemRow {
        PoemRow {
            id,
            slug: format!("poem-{id}"),
            title: String::new(),
            poet_name: String::new(),
            poet_slug: String::new(),
            poet_has_avatar: false,
            poet_is_anonymous: false,
            era_name: String::new(),
            era_slug: String::new(),
            meter_name: String::new(),
            meter_slug: String::new(),
            theme_slug: String::new(),
            rhyme_slug: String::new(),
            poem_type_slug: String::new(),
            collection_slug: None,
            recension_of_id,
        }
    }

    #[test]
    fn a_poem_with_no_verses_gets_an_empty_content_string() {
        let sources = poem_sources(vec![row(1, None)], vec![(2, "other".into())]);
        assert_eq!(sources[0].content, "");
    }

    #[test]
    fn verses_join_in_the_order_they_are_read_and_keep_empty_ones() {
        let verses = vec![(1, "a".into()), (1, String::new()), (1, "b".into())];
        assert_eq!(poem_sources(vec![row(1, None)], verses)[0].content, "a**b");
    }

    #[test]
    fn a_recension_carries_the_id_of_its_primary() {
        let sources = poem_sources(vec![row(1, None), row(2, Some(1))], Vec::new());
        assert_eq!((sources[0].primary_id, sources[0].is_primary), (1, true));
        assert_eq!((sources[1].primary_id, sources[1].is_primary), (1, false));
    }

    #[test]
    fn a_poem_outside_any_collection_gets_an_empty_collection_slug() {
        assert_eq!(
            poem_sources(vec![row(1, None)], Vec::new())[0].collection_slug,
            ""
        );
    }

    #[expect(clippy::print_stderr, reason = "the skip notice goes to stderr")]
    async fn database() -> Option<PgCorpus> {
        let Ok(url) = std::env::var("QAFIYAH_TEST_DATABASE_URL") else {
            eprintln!("skipping: QAFIYAH_TEST_DATABASE_URL is not set");
            return None;
        };
        Some(connect(&url).await.expect("the corpus database"))
    }

    #[tokio::test]
    async fn streamed_poets_add_up_to_the_poet_count_in_id_order() {
        let Some(corpus) = database().await else {
            return;
        };
        let mut cursor = 0;
        let mut streamed = 0_i64;
        loop {
            let batch = corpus.poets_after(cursor, 1000).await.expect("a batch");
            let Some(last) = batch.last() else { break };
            assert!(batch.iter().all(|poet| poet.id > cursor));
            assert!(batch.windows(2).all(|pair| pair[0].id < pair[1].id));
            cursor = last.id;
            streamed += i64::try_from(batch.len()).expect("a small batch");
        }
        assert_eq!(streamed, corpus.count(false).await.expect("a count"));
    }

    #[tokio::test]
    async fn a_hidden_poem_is_skipped_and_a_recension_streams_with_its_primary() {
        let Some(corpus) = database().await else {
            return;
        };
        let mut conn = &corpus.conn;
        let hidden: Option<i32> = poems::table
            .filter(poems::is_hidden)
            .select(poems::id)
            .first(&mut conn)
            .await
            .optional()
            .expect("a hidden poem lookup");
        if let Some(hidden) = hidden {
            let batch = corpus.poems_after(hidden - 1, 1).await.expect("a batch");
            assert!(batch.iter().all(|poem| poem.id != hidden));
        }
        let recension: Option<(i32, Option<i32>)> = poems::table
            .filter(poems::recension_of_id.is_not_null())
            .filter(not(poems::is_hidden))
            .select((poems::id, poems::recension_of_id))
            .first(&mut conn)
            .await
            .optional()
            .expect("a recension lookup");
        if let Some((id, Some(primary))) = recension {
            let batch = corpus.poems_after(id - 1, 1).await.expect("a batch");
            assert_eq!(batch[0].id, id);
            assert_eq!((batch[0].primary_id, batch[0].is_primary), (primary, false));
            assert!(!batch[0].content.is_empty());
        }
    }
}
