mod arabic;
mod docs;
mod error;
mod es;
mod log;
mod pg;
mod reindex;

use serde_json::json;

use crate::error::IndexerError;
use crate::es::Es;
use crate::reindex::{Ctx, Target, reindex};
use qafiyah_elasticsearch::{Schema, load as load_schema};

struct Env {
    database_url: String,
    elasticsearch_url: String,
    es_reader_password: String,
    force: bool,
}

fn parse_env(lookup: impl Fn(&str) -> Option<String>) -> Result<Env, IndexerError> {
    let need =
        |key: &str| lookup(key).ok_or_else(|| IndexerError::Config(format!("{key} is required")));
    Ok(Env {
        database_url: need("DATABASE_URL")?,
        elasticsearch_url: need("ELASTICSEARCH_URL")?,
        es_reader_password: need("ES_READER_PASSWORD")?,
        force: lookup("SEARCH_INDEXER_FORCE").as_deref() == Some("true"),
    })
}

fn read_env() -> Result<Env, IndexerError> {
    parse_env(|key| std::env::var(key).ok())
}

fn batch_size(configured: usize) -> Result<usize, IndexerError> {
    if configured == 0 {
        return Err(IndexerError::Config(
            "bulkBatchSize must be at least 1".to_string(),
        ));
    }
    Ok(configured)
}

#[expect(
    clippy::print_stdout,
    reason = "this batch job's output is its structured log"
)]
async fn bootstrap(env: &Env, schema: &Schema) -> Result<(), IndexerError> {
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
        return Ok(());
    }

    let rules = qafiyah_elasticsearch::folding_rules(&schema.poems);
    let corpus = pg::connect(&env.database_url).await?;

    let ctx = Ctx {
        index: &es,
        corpus: &corpus,
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
    Ok(())
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
            eprintln!("{}", log::line("env", &e.to_string()));
            std::process::exit(1);
        }
    };

    if let Err(e) = bootstrap(&env, &schema).await {
        eprintln!("{}", log::line("boot", &e.to_string()));
        std::process::exit(1);
    }

    println!("{}", log::event("done", json!({})));
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::reindex::{CorpusSource, IndexStore};

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
                Some(IndexerError::Config(format!("{missing} is required")))
            );
        }
    }

    #[test]
    fn a_zero_batch_size_is_refused_before_any_query_runs() {
        assert_eq!(
            batch_size(0),
            Err(IndexerError::Config(
                "bulkBatchSize must be at least 1".to_string()
            ))
        );
        assert_eq!(batch_size(1000), Ok(1000));
    }

    async fn searchable_segments(admin_url: &str, alias: &str) -> u64 {
        let report: Value = qafiyah_elasticsearch::Endpoint::new(admin_url)
            .expect("an Elasticsearch endpoint")
            .request(reqwest::Method::GET, &format!("/{alias}/_segments"))
            .send()
            .await
            .expect("the segments report")
            .json()
            .await
            .expect("a segments report");
        report["indices"]
            .as_object()
            .into_iter()
            .flat_map(|indices| indices.values())
            .filter_map(|index| index["shards"].as_object())
            .flat_map(|shards| shards.values())
            .filter_map(Value::as_array)
            .flatten()
            .filter_map(|copy| copy["num_search_segments"].as_u64())
            .sum()
    }

    #[expect(clippy::print_stderr, reason = "the skip notice goes to stderr")]
    fn database_and_admin_urls() -> Option<(String, String)> {
        let (Ok(database_url), Ok(admin_url)) = (
            std::env::var("QAFIYAH_TEST_DATABASE_URL"),
            std::env::var("QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL"),
        ) else {
            eprintln!(
                "skipping: QAFIYAH_TEST_DATABASE_URL and QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL are not set"
            );
            return None;
        };
        Some((database_url, admin_url))
    }

    fn scratch_alias(test: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!("test-guard-poets-{test}-{}-{nanos}", std::process::id())
    }

    async fn aliased_indices(admin_url: &str, alias: &str) -> Vec<String> {
        let report: Value = qafiyah_elasticsearch::Endpoint::new(admin_url)
            .expect("an Elasticsearch endpoint")
            .request(reqwest::Method::GET, &format!("/_alias/{alias}"))
            .send()
            .await
            .expect("the alias report")
            .json()
            .await
            .expect("an alias report");
        report
            .as_object()
            .map(|indices| indices.keys().cloned().collect())
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn a_reindex_puts_the_alias_on_one_merged_segment_even_when_writing_split_the_index() {
        let Some((database_url, admin_url)) = database_and_admin_urls() else {
            return;
        };
        let schema = load_schema();
        let mut definition = schema.poets.clone();
        definition["settings"]["index.translog.flush_threshold_size"] = json!("1kb");
        let es = Es::new(&admin_url).expect("an Elasticsearch endpoint");
        let corpus = pg::connect(&database_url)
            .await
            .expect("the corpus database");
        let poets = corpus.count(false).await.expect("a poet count");
        let rules = qafiyah_elasticsearch::folding_rules(&schema.poems);
        let ctx = Ctx {
            index: &es,
            corpus: &corpus,
            batch_size: usize::try_from(poets)
                .expect("a count")
                .div_ceil(20)
                .max(10),
            rules: &rules,
        };
        let alias = scratch_alias("merged");
        let prefix = format!("{alias}_v");

        let outcome = reindex(
            &ctx,
            &Target {
                alias: &alias,
                prefix: &prefix,
                body: &definition,
                is_poems: false,
            },
        )
        .await;
        let segments = searchable_segments(&admin_url, &alias).await;
        let searchable = es.alias_count(&alias).await;
        if let Ok((index, _)) = &outcome {
            es.delete_index_quietly(index).await;
        }

        let (_, indexed) = outcome.expect("a reindex");
        assert_eq!(i64::try_from(indexed), Ok(poets));
        assert_eq!(searchable, Ok(Some(u64::try_from(poets).expect("a count"))));
        assert_eq!(segments, 1, "the alias must point at one merged segment");
    }

    #[tokio::test]
    async fn a_reindex_that_fails_leaves_searches_on_the_index_they_were_reading_and_removes_its_own()
     {
        let Some((database_url, admin_url)) = database_and_admin_urls() else {
            return;
        };
        let schema = load_schema();
        let es = Es::new(&admin_url).expect("an Elasticsearch endpoint");
        let corpus = pg::connect(&database_url)
            .await
            .expect("the corpus database");
        let rules = qafiyah_elasticsearch::folding_rules(&schema.poems);
        let ctx = Ctx {
            index: &es,
            corpus: &corpus,
            batch_size: 1000,
            rules: &rules,
        };
        let alias = scratch_alias("rejected");
        let prefix = format!("{alias}_v");
        let mut rejecting = schema.poets.clone();
        rejecting["mappings"]["properties"]
            .as_object_mut()
            .expect("the poet fields")
            .remove("nameSort");

        let served = reindex(
            &ctx,
            &Target {
                alias: &alias,
                prefix: &prefix,
                body: &schema.poets,
                is_poems: false,
            },
        )
        .await;
        let failed = reindex(
            &ctx,
            &Target {
                alias: &alias,
                prefix: &prefix,
                body: &rejecting,
                is_poems: false,
            },
        )
        .await;
        let aliased = aliased_indices(&admin_url, &alias).await;
        let left = es.list_indices_for_alias(&prefix).await;
        for index in left.iter().flatten() {
            es.delete_index_quietly(index).await;
        }

        let (served, _) = served.expect("the first reindex");
        assert!(
            failed.as_ref().is_err_and(|error| error
                .to_string()
                .contains("dynamic introduction of [nameSort]")),
            "{failed:?}"
        );
        assert_eq!(aliased, std::slice::from_ref(&served));
        assert_eq!(left, Ok(vec![served]));
    }
}
