mod arabic;
mod docs;
mod es;
mod log;
mod pg;

use serde_json::{Value, json};

use crate::docs::{to_poem_doc, to_poet_doc};
use crate::es::{Es, next_index_name};
use qafiyah_elasticsearch::{Schema, load as load_schema};

struct Env {
    database_url: String,
    elasticsearch_url: String,
    es_reader_password: String,
    force: bool,
}

fn parse_env(lookup: impl Fn(&str) -> Option<String>) -> Result<Env, String> {
    let need = |key: &str| lookup(key).ok_or_else(|| format!("{key} is required"));
    Ok(Env {
        database_url: need("DATABASE_URL")?,
        elasticsearch_url: need("ELASTICSEARCH_URL")?,
        es_reader_password: need("ES_READER_PASSWORD")?,
        force: lookup("SEARCH_INDEXER_FORCE").as_deref() == Some("true"),
    })
}

fn read_env() -> Result<Env, String> {
    parse_env(|key| std::env::var(key).ok())
}

fn batch_size(configured: usize) -> Result<usize, String> {
    if configured == 0 {
        return Err("bulkBatchSize must be at least 1".to_string());
    }
    Ok(configured)
}

const PROGRESS_EVERY: usize = 10_000;

fn reached_progress_mark(before: usize, after: usize) -> bool {
    after / PROGRESS_EVERY > before / PROGRESS_EVERY
}

struct Target<'a> {
    alias: &'a str,
    prefix: &'a str,
    body: &'a Value,
    is_poems: bool,
}

struct Ctx<'a> {
    es: &'a Es,
    pgc: &'a tokio_postgres::Client,
    batch_size: usize,
    rules: &'a [(String, String)],
}

async fn reindex(ctx: &Ctx<'_>, target_def: &Target<'_>) -> Result<(String, usize), String> {
    let es = ctx.es;
    let existing = es.list_indices_for_alias(target_def.prefix).await?;
    let target = next_index_name(target_def.prefix, &existing);
    es.create_index(&target, target_def.body).await?;

    let populated = populate(ctx, &target, target_def.is_poems).await;
    let count = match populated {
        Ok(count) => count,
        Err(e) => {
            es.delete_index_quietly(&target).await;
            return Err(e);
        }
    };

    es.swap_alias(target_def.alias, target_def.prefix, &target)
        .await?;
    for name in existing.iter().filter(|n| *n != &target) {
        es.delete_index_quietly(name).await;
    }
    Ok((target, count))
}

#[expect(
    clippy::print_stdout,
    reason = "this batch job's output is its structured log"
)]
async fn populate(ctx: &Ctx<'_>, target: &str, is_poems: bool) -> Result<usize, String> {
    let (es, pgc, batch_size, rules) = (ctx.es, ctx.pgc, ctx.batch_size, ctx.rules);
    let limit = i64::try_from(batch_size).map_err(|_| "bulkBatchSize exceeds i64".to_string())?;
    let expected = pg::count_rows(pgc, is_poems).await?;
    let index = if is_poems { "poems" } else { "poets" };
    es.put_refresh_interval(target, "-1").await?;
    let mut cursor: i32 = 0;
    let mut total = 0usize;
    loop {
        let batch: Vec<(String, String)> = if is_poems {
            let rows = pg::stream_poem_batch(pgc, cursor, limit).await?;
            let Some(last) = rows.last() else {
                break;
            };
            cursor = last.id;
            rows.into_iter()
                .map(to_poem_doc)
                .map(|d| {
                    (
                        d.slug.clone(),
                        serde_json::to_string(&d).unwrap_or_default(),
                    )
                })
                .collect()
        } else {
            let rows = pg::stream_poet_batch(pgc, cursor, limit).await?;
            let Some(last) = rows.last() else {
                break;
            };
            cursor = last.id;
            rows.into_iter()
                .map(|r| to_poet_doc(r, rules))
                .map(|d| {
                    (
                        d.slug.clone(),
                        serde_json::to_string(&d).unwrap_or_default(),
                    )
                })
                .collect()
        };
        let before = total;
        total = total
            .checked_add(batch.len())
            .ok_or_else(|| "indexed count overflow".to_string())?;
        es.bulk(target, &batch).await?;
        if reached_progress_mark(before, total) {
            println!(
                "{}",
                log::event(
                    "progress",
                    json!({ "index": index, "indexed": total, "total": expected })
                )
            );
        }
    }
    es.put_refresh_interval(target, "1s").await?;
    es.refresh(target).await?;
    Ok(total)
}

#[expect(
    clippy::print_stdout,
    reason = "this batch job's output is its structured log"
)]
async fn bootstrap(env: &Env, schema: &Schema) -> Result<Value, String> {
    let es = Es::new(&env.elasticsearch_url)?;
    let id = &schema.identity;

    es.ensure_read_only_user(
        &id.api_role,
        &id.api_username,
        &env.es_reader_password,
        &[
            id.poems_alias.clone(),
            id.poets_alias.clone(),
            format!("{}*", id.poems_prefix),
            format!("{}*", id.poets_prefix),
        ],
    )
    .await?;

    let poems_ready = es.alias_count(&id.poems_alias).await?.unwrap_or(0) > 0;
    let poets_ready = es.alias_count(&id.poets_alias).await?.unwrap_or(0) > 0;
    if !env.force && poems_ready && poets_ready {
        return Ok(json!({ "lastReindexAt": Value::Null, "lastError": Value::Null }));
    }

    let rules = qafiyah_elasticsearch::folding_rules(&schema.poems);
    let pgc = pg::connect(&env.database_url).await?;

    let ctx = Ctx {
        es: &es,
        pgc: &pgc,
        batch_size: batch_size(id.bulk_batch_size)?,
        rules: &rules,
    };
    let (poems_index, poems_count) = reindex(
        &ctx,
        &Target {
            alias: &id.poems_alias,
            prefix: &id.poems_prefix,
            body: &schema.poems,
            is_poems: true,
        },
    )
    .await?;
    let (poets_index, poets_count) = reindex(
        &ctx,
        &Target {
            alias: &id.poets_alias,
            prefix: &id.poets_prefix,
            body: &schema.poets,
            is_poems: false,
        },
    )
    .await?;

    println!(
        "{}",
        log::event(
            "reindex",
            json!({
                "poems": { "index": poems_index, "count": poems_count },
                "poets": { "index": poets_index, "count": poets_count }
            })
        )
    );
    Ok(json!({ "lastReindexAt": now_iso(), "lastError": Value::Null }))
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(i64::try_from(days).unwrap_or_default());
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "civil-from-days math runs on a days-since-epoch value bounded far below overflow"
)]
#[expect(
    clippy::as_conversions,
    reason = "each cast narrows a component the algorithm already bounds"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "day and month are bounded to 31 and 12"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "the day-of-era is non-negative by construction"
)]
#[expect(
    clippy::cast_possible_wrap,
    reason = "the year-of-era is bounded to a few hundred"
)]
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[expect(
    clippy::print_stdout,
    reason = "this batch job's output is its structured log"
)]
#[expect(
    clippy::print_stderr,
    reason = "fatal errors go to stderr before a non-zero exit"
)]
#[tokio::main]
async fn main() {
    let schema = load_schema();
    let env = match read_env() {
        Ok(env) => env,
        Err(e) => {
            eprintln!("{}", log::line("env", &e));
            std::process::exit(1);
        }
    };

    let state = match bootstrap(&env, &schema).await {
        Ok(state) => state,
        Err(e) => {
            eprintln!("{}", log::line("boot", &e));
            std::process::exit(1);
        }
    };

    println!("{}", log::event("done", state));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_reported_each_time_the_count_passes_a_multiple_of_ten_thousand() {
        assert!(reached_progress_mark(9_000, 10_000));
        assert!(reached_progress_mark(19_500, 20_500));
        assert!(!reached_progress_mark(10_000, 11_000));
        assert!(!reached_progress_mark(0, 1_000));
        assert!(!reached_progress_mark(348_000, 348_691));
    }

    fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn the_environment_needs_three_values_and_reads_force_only_when_exactly_true() {
        let env = parse_env(vars(&[
            ("DATABASE_URL", "p"),
            ("ELASTICSEARCH_URL", "e"),
            ("ES_READER_PASSWORD", "x"),
            ("SEARCH_INDEXER_FORCE", "True"),
        ]))
        .expect("an env");
        assert!(!env.force, "only the exact string true is true");
        let forced = parse_env(vars(&[
            ("DATABASE_URL", "p"),
            ("ELASTICSEARCH_URL", "e"),
            ("ES_READER_PASSWORD", "x"),
            ("SEARCH_INDEXER_FORCE", "true"),
        ]))
        .expect("an env");
        assert!(forced.force);
        for missing in ["DATABASE_URL", "ELASTICSEARCH_URL", "ES_READER_PASSWORD"] {
            let pairs: Vec<(&str, &str)> = [
                ("DATABASE_URL", "p"),
                ("ELASTICSEARCH_URL", "e"),
                ("ES_READER_PASSWORD", "x"),
            ]
            .into_iter()
            .filter(|(n, _)| *n != missing)
            .collect();
            assert_eq!(
                parse_env(vars(&pairs)).err(),
                Some(format!("{missing} is required"))
            );
        }
    }

    #[test]
    fn a_zero_batch_size_is_refused_before_any_query_runs() {
        assert_eq!(
            batch_size(0),
            Err("bulkBatchSize must be at least 1".to_string())
        );
        assert_eq!(batch_size(1000), Ok(1000));
    }

    #[test]
    fn civil_dates_round_trip_known_days() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(19_675), (2023, 11, 14));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }
}
