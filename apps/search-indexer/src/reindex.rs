use async_trait::async_trait;
use qafiyah_elasticsearch::{Schema, folding_rules};
use serde_json::Value;

use crate::docs::{Document, PoemSource, PoetSource, to_poem_doc, to_poet_doc};
use crate::error::IndexerError;

const PROGRESS_EVERY: usize = 10_000;

#[async_trait]
pub(crate) trait CorpusSource: Send + Sync {
    async fn count(&self, is_poems: bool) -> Result<i64, IndexerError>;
    async fn poems_after(&self, after_id: i32, limit: i64)
    -> Result<Vec<PoemSource>, IndexerError>;
    async fn poets_after(&self, after_id: i32, limit: i64)
    -> Result<Vec<PoetSource>, IndexerError>;
}

#[async_trait]
pub(crate) trait IndexStore: Send + Sync {
    async fn list_indices_for_alias(&self, prefix: &str) -> Result<Vec<String>, IndexerError>;
    async fn create_index(&self, index: &str, body: &Value) -> Result<(), IndexerError>;
    async fn put_refresh_interval(&self, index: &str, value: &str) -> Result<(), IndexerError>;
    async fn refresh(&self, index: &str) -> Result<(), IndexerError>;
    async fn force_merge(&self, index: &str) -> Result<(), IndexerError>;
    async fn bulk(&self, index: &str, docs: &[Document]) -> Result<(), IndexerError>;
    async fn swap_alias(
        &self,
        alias: &str,
        prefix: &str,
        to_index: &str,
    ) -> Result<(), IndexerError>;
    async fn delete_index_quietly(&self, index: &str);
    async fn alias_count(&self, alias: &str) -> Result<Option<u64>, IndexerError>;
    async fn ensure_read_only_user(
        &self,
        role: &str,
        username: &str,
        password: &str,
        index_patterns: &[String],
    ) -> Result<(), IndexerError>;
}

pub(crate) struct Progress<'a> {
    pub index: &'a str,
    pub indexed: usize,
    pub total: i64,
}

pub(crate) struct Target<'a> {
    pub alias: &'a str,
    pub prefix: &'a str,
    pub body: &'a Value,
    pub is_poems: bool,
}

pub(crate) struct Ctx<'a> {
    pub index: &'a dyn IndexStore,
    pub corpus: &'a dyn CorpusSource,
    pub batch_size: usize,
    pub rules: &'a [(String, String)],
    pub progress: &'a (dyn Fn(Progress<'_>) + Sync),
}

pub(crate) struct Plan<'a> {
    pub schema: &'a Schema,
    pub reader_password: &'a str,
    pub force: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Skipped,
    Rebuilt {
        poems: (String, usize),
        poets: (String, usize),
    },
}

pub(crate) fn batch_size(configured: usize) -> Result<usize, IndexerError> {
    if configured == 0 {
        return Err(IndexerError::Config(
            "bulkBatchSize must be at least 1".to_string(),
        ));
    }
    Ok(configured)
}

pub(crate) fn next_index_name(prefix: &str, existing: &[String]) -> String {
    let next = existing
        .iter()
        .filter(|name| name.starts_with(prefix))
        .filter_map(|name| {
            name.get(prefix.len()..)
                .and_then(|suffix| suffix.parse::<u32>().ok())
        })
        .max()
        .map_or(1, |v| v.saturating_add(1));
    format!("{prefix}{next}")
}

fn reached_progress_mark(before: usize, after: usize) -> bool {
    after / PROGRESS_EVERY > before / PROGRESS_EVERY
}

pub(crate) async fn reindex(
    ctx: &Ctx<'_>,
    target_def: &Target<'_>,
) -> Result<(String, usize), IndexerError> {
    let es = ctx.index;
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

async fn populate(ctx: &Ctx<'_>, target: &str, is_poems: bool) -> Result<usize, IndexerError> {
    let (es, corpus, batch_size, rules) = (ctx.index, ctx.corpus, ctx.batch_size, ctx.rules);
    let limit = i64::try_from(batch_size)
        .map_err(|_| IndexerError::Config("bulkBatchSize exceeds i64".to_string()))?;
    let expected = corpus.count(is_poems).await?;
    let index = if is_poems { "poems" } else { "poets" };
    es.put_refresh_interval(target, "-1").await?;
    let mut cursor: i32 = 0;
    let mut total = 0usize;
    loop {
        let batch: Vec<Document> = if is_poems {
            let rows = corpus.poems_after(cursor, limit).await?;
            let Some(last) = rows.last() else {
                break;
            };
            cursor = last.id;
            rows.into_iter()
                .map(|row| Document::Poem(to_poem_doc(row)))
                .collect()
        } else {
            let rows = corpus.poets_after(cursor, limit).await?;
            let Some(last) = rows.last() else {
                break;
            };
            cursor = last.id;
            rows.into_iter()
                .map(|row| Document::Poet(to_poet_doc(row, rules)))
                .collect()
        };
        let before = total;
        total = total
            .checked_add(batch.len())
            .ok_or(IndexerError::CountOverflow)?;
        es.bulk(target, &batch).await?;
        if reached_progress_mark(before, total) {
            (ctx.progress)(Progress {
                index,
                indexed: total,
                total: expected,
            });
        }
    }
    es.put_refresh_interval(target, "1s").await?;
    es.force_merge(target).await?;
    es.refresh(target).await?;
    Ok(total)
}

pub(crate) async fn bootstrap<Connect, Connecting>(
    index: &dyn IndexStore,
    plan: &Plan<'_>,
    connect: Connect,
    progress: &(dyn Fn(Progress<'_>) + Sync),
) -> Result<Outcome, IndexerError>
where
    Connect: FnOnce() -> Connecting,
    Connecting: Future<Output = Result<Box<dyn CorpusSource>, IndexerError>>,
{
    let id = &plan.schema.identity;
    index
        .ensure_read_only_user(
            &id.api_role,
            &id.api_username,
            plan.reader_password,
            &[
                id.poems_alias.clone(),
                id.poets_alias.clone(),
                format!("{}*", id.poems_prefix),
                format!("{}*", id.poets_prefix),
            ],
        )
        .await?;

    let poems_ready = index.alias_count(&id.poems_alias).await?.unwrap_or(0) > 0;
    let poets_ready = index.alias_count(&id.poets_alias).await?.unwrap_or(0) > 0;
    if !plan.force && poems_ready && poets_ready {
        return Ok(Outcome::Skipped);
    }

    let rules = folding_rules(&plan.schema.poems);
    let batch_size = batch_size(id.bulk_batch_size)?;
    let corpus = connect().await?;
    let ctx = Ctx {
        index,
        corpus: corpus.as_ref(),
        batch_size,
        rules: &rules,
        progress,
    };
    let poems = reindex(
        &ctx,
        &Target {
            alias: &id.poems_alias,
            prefix: &id.poems_prefix,
            body: &plan.schema.poems,
            is_poems: true,
        },
    )
    .await?;
    let poets = reindex(
        &ctx,
        &Target {
            alias: &id.poets_alias,
            prefix: &id.poets_prefix,
            body: &plan.schema.poets,
            is_poems: false,
        },
    )
    .await?;
    Ok(Outcome::Rebuilt { poems, poets })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use serde_json::json;

    use super::*;

    #[test]
    fn the_next_index_name_follows_the_audit_table() {
        let owned = |names: &[&str]| names.iter().map(|n| (*n).to_string()).collect::<Vec<_>>();
        assert_eq!(next_index_name("poems_v", &[]), "poems_v1");
        assert_eq!(
            next_index_name("poems_v", &owned(&["poems_v1", "poems_v3"])),
            "poems_v4"
        );
        assert_eq!(
            next_index_name("poems_v", &owned(&["poems_v2_old", "poems_v", "poems_v1"])),
            "poems_v2"
        );
        assert_eq!(
            next_index_name("poems_v", &owned(&["poems_v007"])),
            "poems_v8"
        );
        assert_eq!(
            next_index_name("poems_v", &owned(&["poems_v+5"])),
            "poems_v6"
        );
        assert_eq!(
            next_index_name("poems_v", &owned(&["poets_v9"])),
            "poems_v1"
        );
    }

    #[test]
    fn progress_is_reported_each_time_the_count_passes_a_multiple_of_ten_thousand() {
        assert!(reached_progress_mark(9_000, 10_000));
        assert!(reached_progress_mark(19_500, 20_500));
        assert!(!reached_progress_mark(10_000, 11_000));
        assert!(!reached_progress_mark(0, 1_000));
        assert!(!reached_progress_mark(348_000, 348_691));
    }

    #[test]
    fn a_zero_batch_size_is_refused() {
        assert_eq!(
            batch_size(0),
            Err(IndexerError::Config(
                "bulkBatchSize must be at least 1".to_string()
            ))
        );
        assert_eq!(batch_size(1000), Ok(1000));
    }

    struct Corpus {
        poet_ids: Vec<i32>,
        poem_ids: Vec<i32>,
    }

    impl Corpus {
        fn of(poet_ids: impl IntoIterator<Item = i32>) -> Self {
            Self {
                poet_ids: poet_ids.into_iter().collect(),
                poem_ids: Vec::new(),
            }
        }

        fn with_poems(self, poem_ids: impl IntoIterator<Item = i32>) -> Self {
            Self {
                poem_ids: poem_ids.into_iter().collect(),
                ..self
            }
        }
    }

    fn poem(id: i32) -> PoemSource {
        PoemSource {
            id,
            slug: format!("poem-{id}"),
            title: format!("Poem {id}"),
            content: "first*second".into(),
            poet_name: "Poet".into(),
            poet_slug: "poet".into(),
            poet_has_avatar: false,
            poet_is_anonymous: false,
            era_name: "Era".into(),
            era_slug: "era".into(),
            meter_name: "Meter".into(),
            meter_slug: "meter".into(),
            theme_slug: "theme".into(),
            rhyme_slug: "rhyme".into(),
            poem_type_slug: "amudi".into(),
            collection_slug: String::new(),
            primary_id: id,
            is_primary: true,
        }
    }

    #[async_trait]
    impl CorpusSource for Corpus {
        async fn count(&self, is_poems: bool) -> Result<i64, IndexerError> {
            let ids = if is_poems {
                &self.poem_ids
            } else {
                &self.poet_ids
            };
            Ok(i64::try_from(ids.len()).expect("a small corpus"))
        }
        async fn poems_after(
            &self,
            after_id: i32,
            limit: i64,
        ) -> Result<Vec<PoemSource>, IndexerError> {
            Ok(self
                .poem_ids
                .iter()
                .filter(|id| **id > after_id)
                .take(usize::try_from(limit).expect("a small limit"))
                .map(|id| poem(*id))
                .collect())
        }
        async fn poets_after(
            &self,
            after_id: i32,
            limit: i64,
        ) -> Result<Vec<PoetSource>, IndexerError> {
            Ok(self
                .poet_ids
                .iter()
                .filter(|id| **id > after_id)
                .take(usize::try_from(limit).expect("a small limit"))
                .map(|id| PoetSource {
                    id: *id,
                    slug: format!("poet-{id}"),
                    name: format!("Poet {id}"),
                    nickname: String::new(),
                    era_name: "Era".into(),
                    era_slug: "era".into(),
                    poems_count: 1,
                })
                .collect())
        }
    }

    #[derive(Default)]
    struct Stored {
        slugs: Vec<String>,
        refresh: String,
        merged: bool,
    }

    #[derive(Default)]
    struct Cluster {
        indices: Mutex<BTreeMap<String, Stored>>,
        aliases: Mutex<BTreeMap<String, String>>,
        readers: Mutex<Vec<(String, Vec<String>)>>,
        rejects_writes: bool,
    }

    impl Cluster {
        fn serving(alias: &str, index: &str, slugs: &[&str]) -> Self {
            let cluster = Cluster::default();
            cluster.indices.lock().expect("indices").insert(
                index.to_string(),
                Stored {
                    slugs: slugs.iter().map(|slug| (*slug).to_string()).collect(),
                    refresh: "1s".into(),
                    merged: true,
                },
            );
            cluster
                .aliases
                .lock()
                .expect("aliases")
                .insert(alias.to_string(), index.to_string());
            cluster
        }

        fn alias(&self, alias: &str) -> Option<String> {
            self.aliases.lock().expect("aliases").get(alias).cloned()
        }

        fn index_names(&self) -> Vec<String> {
            self.indices
                .lock()
                .expect("indices")
                .keys()
                .cloned()
                .collect()
        }

        fn with_index<T>(&self, index: &str, read: impl FnOnce(&Stored) -> T) -> T {
            read(
                self.indices
                    .lock()
                    .expect("indices")
                    .get(index)
                    .expect("the index"),
            )
        }
    }

    #[async_trait]
    impl IndexStore for Cluster {
        async fn list_indices_for_alias(&self, prefix: &str) -> Result<Vec<String>, IndexerError> {
            Ok(self
                .index_names()
                .into_iter()
                .filter(|name| name.starts_with(prefix))
                .collect())
        }
        async fn create_index(&self, index: &str, _: &Value) -> Result<(), IndexerError> {
            self.indices
                .lock()
                .expect("indices")
                .entry(index.to_string())
                .or_insert_with(|| Stored {
                    refresh: "1s".into(),
                    ..Stored::default()
                });
            Ok(())
        }
        async fn put_refresh_interval(&self, index: &str, value: &str) -> Result<(), IndexerError> {
            if let Some(stored) = self.indices.lock().expect("indices").get_mut(index) {
                stored.refresh = value.to_string();
            }
            Ok(())
        }
        async fn refresh(&self, _: &str) -> Result<(), IndexerError> {
            Ok(())
        }
        async fn force_merge(&self, index: &str) -> Result<(), IndexerError> {
            if let Some(stored) = self.indices.lock().expect("indices").get_mut(index) {
                stored.merged = true;
            }
            Ok(())
        }
        async fn bulk(&self, index: &str, docs: &[Document]) -> Result<(), IndexerError> {
            if self.rejects_writes {
                return Err(IndexerError::Elasticsearch("bulk errors: rejected".into()));
            }
            if let Some(stored) = self.indices.lock().expect("indices").get_mut(index) {
                stored
                    .slugs
                    .extend(docs.iter().map(|doc| doc.slug().to_string()));
            }
            Ok(())
        }
        async fn swap_alias(
            &self,
            alias: &str,
            _: &str,
            to_index: &str,
        ) -> Result<(), IndexerError> {
            self.aliases
                .lock()
                .expect("aliases")
                .insert(alias.to_string(), to_index.to_string());
            Ok(())
        }
        async fn delete_index_quietly(&self, index: &str) {
            self.indices.lock().expect("indices").remove(index);
            self.aliases
                .lock()
                .expect("aliases")
                .retain(|_, aliased| aliased != index);
        }
        async fn alias_count(&self, alias: &str) -> Result<Option<u64>, IndexerError> {
            Ok(self.alias(alias).map(|index| {
                self.with_index(&index, |stored| {
                    u64::try_from(stored.slugs.len()).expect("a small index")
                })
            }))
        }
        async fn ensure_read_only_user(
            &self,
            _: &str,
            username: &str,
            _: &str,
            index_patterns: &[String],
        ) -> Result<(), IndexerError> {
            self.readers
                .lock()
                .expect("readers")
                .push((username.to_string(), index_patterns.to_vec()));
            Ok(())
        }
    }

    async fn reindex_poets(
        cluster: &Cluster,
        corpus: &Corpus,
    ) -> Result<(String, usize), IndexerError> {
        let body = json!({});
        reindex(
            &Ctx {
                index: cluster,
                corpus,
                batch_size: 2,
                rules: &[],
                progress: &|_| {},
            },
            &Target {
                alias: "poets",
                prefix: "poets_v",
                body: &body,
                is_poems: false,
            },
        )
        .await
    }

    #[tokio::test]
    async fn a_reindex_moves_the_alias_to_a_new_merged_index_holding_every_poet_once() {
        let cluster = Cluster::serving("poets", "poets_v1", &["stale"]);
        let outcome = reindex_poets(&cluster, &Corpus::of([3, 5, 8, 13, 21])).await;
        assert_eq!(outcome, Ok(("poets_v2".to_string(), 5)));
        assert_eq!(cluster.alias("poets").as_deref(), Some("poets_v2"));
        assert_eq!(
            cluster.index_names(),
            ["poets_v2"],
            "the old index is dropped"
        );
        cluster.with_index("poets_v2", |stored| {
            assert_eq!(
                stored.slugs,
                ["poet-3", "poet-5", "poet-8", "poet-13", "poet-21"]
            );
            assert_eq!(stored.refresh, "1s", "refresh is turned back on");
            assert!(stored.merged);
        });
    }

    #[tokio::test]
    async fn a_failed_write_keeps_searches_on_the_old_index_and_leaves_no_half_built_one() {
        let mut cluster = Cluster::serving("poets", "poets_v1", &["kept"]);
        cluster.rejects_writes = true;
        let outcome = reindex_poets(&cluster, &Corpus::of([1, 2, 3])).await;
        assert_eq!(
            outcome,
            Err(IndexerError::Elasticsearch(
                "bulk errors: rejected".to_string()
            ))
        );
        assert_eq!(cluster.alias("poets").as_deref(), Some("poets_v1"));
        assert_eq!(cluster.index_names(), ["poets_v1"]);
        cluster.with_index("poets_v1", |stored| assert_eq!(stored.slugs, ["kept"]));
    }

    #[tokio::test]
    async fn an_empty_corpus_still_ends_on_a_live_empty_index() {
        let cluster = Cluster::default();
        let outcome = reindex_poets(&cluster, &Corpus::of([])).await;
        assert_eq!(outcome, Ok(("poets_v1".to_string(), 0)));
        assert_eq!(cluster.alias("poets").as_deref(), Some("poets_v1"));
        cluster.with_index("poets_v1", |stored| assert!(stored.slugs.is_empty()));
    }

    #[tokio::test]
    async fn progress_is_reported_each_time_another_ten_thousand_are_written() {
        let cluster = Cluster::default();
        let reports = Mutex::new(Vec::new());
        let record = |progress: Progress<'_>| {
            reports.lock().expect("reports").push((
                progress.index.to_string(),
                progress.indexed,
                progress.total,
            ));
        };
        let body = json!({});
        reindex(
            &Ctx {
                index: &cluster,
                corpus: &Corpus::of(1..=25_000),
                batch_size: 5_000,
                rules: &[],
                progress: &record,
            },
            &Target {
                alias: "poets",
                prefix: "poets_v",
                body: &body,
                is_poems: false,
            },
        )
        .await
        .expect("a reindex");
        assert_eq!(
            *reports.lock().expect("reports"),
            [
                ("poets".to_string(), 10_000, 25_000),
                ("poets".to_string(), 20_000, 25_000)
            ]
        );
    }

    async fn boot(cluster: &Cluster, force: bool) -> (Result<Outcome, IndexerError>, bool) {
        let schema = qafiyah_elasticsearch::load();
        let connected = AtomicBool::new(false);
        let outcome = bootstrap(
            cluster,
            &Plan {
                schema: &schema,
                reader_password: "secret",
                force,
            },
            || async {
                connected.store(true, Ordering::SeqCst);
                let corpus: Box<dyn CorpusSource> =
                    Box::new(Corpus::of([1, 2]).with_poems([10, 20, 30]));
                Ok(corpus)
            },
            &|_| {},
        )
        .await;
        (outcome, connected.load(Ordering::SeqCst))
    }

    fn serving_both() -> Cluster {
        let identity = qafiyah_elasticsearch::load().identity;
        let cluster = Cluster::serving(&identity.poems_alias, "poems_v1", &["a poem"]);
        let poets = Cluster::serving(&identity.poets_alias, "poets_v1", &["a poet"]);
        cluster
            .indices
            .lock()
            .expect("indices")
            .append(&mut poets.indices.lock().expect("indices"));
        cluster
            .aliases
            .lock()
            .expect("aliases")
            .append(&mut poets.aliases.lock().expect("aliases"));
        cluster
    }

    #[tokio::test]
    async fn a_store_whose_aliases_hold_documents_is_left_alone_and_postgres_is_never_asked() {
        let cluster = serving_both();
        let (outcome, connected) = boot(&cluster, false).await;
        assert_eq!(outcome, Ok(Outcome::Skipped));
        assert!(!connected);
        assert_eq!(cluster.index_names(), ["poems_v1", "poets_v1"]);
    }

    #[tokio::test]
    async fn a_forced_run_rebuilds_both_indices_even_when_they_hold_documents() {
        let cluster = serving_both();
        let (outcome, connected) = boot(&cluster, true).await;
        assert!(connected);
        assert_eq!(
            outcome,
            Ok(Outcome::Rebuilt {
                poems: ("poems_v2".to_string(), 3),
                poets: ("poets_v2".to_string(), 2),
            })
        );
        cluster.with_index("poems_v2", |stored| {
            assert_eq!(stored.slugs, ["poem-10", "poem-20", "poem-30"]);
        });
        cluster.with_index("poets_v2", |stored| {
            assert_eq!(stored.slugs, ["poet-1", "poet-2"]);
        });
    }

    #[tokio::test]
    async fn an_alias_that_is_missing_or_empty_rebuilds_both_indices() {
        let identity = qafiyah_elasticsearch::load().identity;
        let cluster = Cluster::serving(&identity.poems_alias, "poems_v1", &["a poem"]);
        let (outcome, connected) = boot(&cluster, false).await;
        assert!(connected);
        assert!(matches!(outcome, Ok(Outcome::Rebuilt { .. })));
        assert_eq!(
            cluster.alias(&identity.poets_alias).as_deref(),
            Some("poets_v1")
        );
    }

    #[tokio::test]
    async fn the_api_reader_may_read_only_the_aliases_and_their_versioned_indices() {
        let identity = qafiyah_elasticsearch::load().identity;
        let cluster = serving_both();
        let (outcome, _) = boot(&cluster, false).await;
        assert_eq!(outcome, Ok(Outcome::Skipped));
        assert_eq!(
            *cluster.readers.lock().expect("readers"),
            [(
                identity.api_username.clone(),
                vec![
                    identity.poems_alias.clone(),
                    identity.poets_alias.clone(),
                    format!("{}*", identity.poems_prefix),
                    format!("{}*", identity.poets_prefix),
                ]
            )]
        );
    }
}
