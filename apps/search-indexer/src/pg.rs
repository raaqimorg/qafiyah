use std::time::Duration;

use tokio_postgres::{Client, NoTls};

use crate::docs::{PoemSource, PoetSource};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const QUERY_TIMEOUT: Duration = Duration::from_secs(60);

const POEM_SELECT: &str = "
  SELECT
    p.id AS id, p.slug AS slug, p.title AS title,
    COALESCE((
      SELECT string_agg(v.content, '*' ORDER BY pv.position)
      FROM public.poem_verses pv
      JOIN  public.verses     v ON v.id = pv.verse_id
      WHERE pv.poem_id = p.id
    ), '') AS content,
    pt.name AS poet_name, pt.slug AS poet_slug, pt.has_avatar AS poet_has_avatar,
    pt.is_anonymous AS poet_is_anonymous,
    e.name AS era_name, e.slug AS era_slug,
    m.name AS meter_name, m.slug AS meter_slug,
    t.slug AS theme_slug,
    r.slug AS rhyme_slug,
    ty.slug AS poem_type_slug,
    COALESCE(c.slug, '') AS collection_slug,
    COALESCE(p.recension_of_id, p.id) AS primary_id, p.recension_of_id IS NULL AS is_primary
  FROM public.poems p
  JOIN public.poets pt ON p.poet_id = pt.id
  JOIN public.eras e ON pt.era_id = e.id
  JOIN public.meters m ON p.meter_id = m.id
  JOIN public.themes t ON p.theme_id = t.id
  JOIN public.rhymes r ON p.rhyme_id = r.id
  JOIN public.poem_types ty ON p.poem_type_id = ty.id
  LEFT JOIN public.collections c ON p.collection_id = c.id
  WHERE p.id > $1 AND NOT p.is_hidden ORDER BY p.id ASC LIMIT $2
";

const POET_SELECT: &str = "
  SELECT pt.id AS id, pt.slug AS slug, pt.name AS name, COALESCE(pt.nickname, '') AS nickname,
         e.name AS era_name, e.slug AS era_slug,
         COALESCE(ps.poems_count, 0)::int AS poems_count
  FROM public.poets pt
  JOIN public.eras e ON pt.era_id = e.id
  LEFT JOIN public.poet_stats ps ON ps.slug = pt.slug
  WHERE pt.id > $1 AND NOT pt.is_hidden ORDER BY pt.id ASC LIMIT $2
";

const POEM_COUNT: &str = "SELECT count(*) FROM public.poems WHERE NOT is_hidden";
const POET_COUNT: &str = "SELECT count(*) FROM public.poets WHERE NOT is_hidden";

#[expect(
    clippy::print_stderr,
    reason = "a dropped postgres connection reports its failure to stderr"
)]
pub(crate) async fn connect(url: &str) -> Result<Client, String> {
    let mut config = url
        .parse::<tokio_postgres::Config>()
        .map_err(|e| format!("postgres connect: {e}"))?;
    config.connect_timeout(CONNECT_TIMEOUT);
    let (client, connection) = config
        .connect(NoTls)
        .await
        .map_err(|e| format!("postgres connect: {e}"))?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("{}", crate::log::line("pg_connection", &e.to_string()));
        }
    });
    Ok(client)
}

async fn query_poem_batch(
    client: &Client,
    after_id: i32,
    limit: i64,
) -> Result<Vec<tokio_postgres::Row>, String> {
    tokio::time::timeout(
        QUERY_TIMEOUT,
        client.query(POEM_SELECT, &[&after_id, &limit]),
    )
    .await
    .map_err(|_| "streamPoemBatch: query timed out".to_string())?
    .map_err(|e| format!("streamPoemBatch: {e}"))
}

async fn query_poet_batch(
    client: &Client,
    after_id: i32,
    limit: i64,
) -> Result<Vec<tokio_postgres::Row>, String> {
    tokio::time::timeout(
        QUERY_TIMEOUT,
        client.query(POET_SELECT, &[&after_id, &limit]),
    )
    .await
    .map_err(|_| "streamPoetBatch: query timed out".to_string())?
    .map_err(|e| format!("streamPoetBatch: {e}"))
}

pub(crate) async fn stream_poem_batch(
    client: &Client,
    after_id: i32,
    limit: i64,
) -> Result<Vec<PoemSource>, String> {
    let rows = query_poem_batch(client, after_id, limit).await?;
    Ok(rows
        .iter()
        .map(|r| PoemSource {
            id: r.get("id"),
            slug: r.get("slug"),
            title: r.get("title"),
            content: r.get("content"),
            poet_name: r.get("poet_name"),
            poet_slug: r.get("poet_slug"),
            poet_has_avatar: r.get("poet_has_avatar"),
            poet_is_anonymous: r.get("poet_is_anonymous"),
            era_name: r.get("era_name"),
            era_slug: r.get("era_slug"),
            meter_name: r.get("meter_name"),
            meter_slug: r.get("meter_slug"),
            theme_slug: r.get("theme_slug"),
            rhyme_slug: r.get("rhyme_slug"),
            poem_type_slug: r.get("poem_type_slug"),
            collection_slug: r.get("collection_slug"),
            primary_id: r.get("primary_id"),
            is_primary: r.get("is_primary"),
        })
        .collect())
}

pub(crate) async fn count_rows(client: &Client, is_poems: bool) -> Result<i64, String> {
    let sql = if is_poems { POEM_COUNT } else { POET_COUNT };
    let row = tokio::time::timeout(QUERY_TIMEOUT, client.query_one(sql, &[]))
        .await
        .map_err(|_| "countRows: query timed out".to_string())?
        .map_err(|e| format!("countRows: {e}"))?;
    Ok(row.get(0))
}

pub(crate) async fn stream_poet_batch(
    client: &Client,
    after_id: i32,
    limit: i64,
) -> Result<Vec<PoetSource>, String> {
    let rows = query_poet_batch(client, after_id, limit).await?;
    Ok(rows
        .iter()
        .map(|r| PoetSource {
            id: r.get("id"),
            slug: r.get("slug"),
            name: r.get("name"),
            nickname: r.get("nickname"),
            era_name: r.get("era_name"),
            era_slug: r.get("era_slug"),
            poems_count: r.get("poems_count"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poem_with_no_verses_selects_an_empty_content_string_rather_than_null() {
        assert!(
            POEM_SELECT.contains("COALESCE((") && POEM_SELECT.contains("), '') AS content"),
            "{POEM_SELECT}"
        );
        assert!(POEM_SELECT.contains("string_agg(v.content, '*' ORDER BY pv.position)"));
    }

    #[test]
    fn every_shown_reading_is_selected_with_the_id_of_its_primary() {
        assert!(
            !POEM_SELECT.contains("recension_of_id IS NULL AND"),
            "{POEM_SELECT}"
        );
        assert!(POEM_SELECT.contains("COALESCE(p.recension_of_id, p.id) AS primary_id"));
        assert!(POEM_SELECT.contains("p.recension_of_id IS NULL AS is_primary"));
        assert!(POEM_SELECT.contains("NOT p.is_hidden"));
    }

    #[test]
    fn the_progress_totals_count_the_same_rows_the_selects_read() {
        assert!(POEM_COUNT.contains("FROM public.poems") && POEM_COUNT.contains("NOT is_hidden"));
        assert!(POET_COUNT.contains("FROM public.poets") && POET_COUNT.contains("NOT is_hidden"));
    }

    #[test]
    fn both_selects_page_by_id_with_a_bound_cursor_and_limit() {
        for select in [POEM_SELECT, POET_SELECT] {
            assert!(select.contains("WHERE p.id > $1") || select.contains("WHERE pt.id > $1"));
            assert!(select.contains("ORDER BY"));
            assert!(select.trim_end().ends_with("LIMIT $2"), "{select}");
        }
    }
}
