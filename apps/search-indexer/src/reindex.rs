use async_trait::async_trait;
use serde_json::{Value, json};

use crate::docs::{PoemSource, PoetSource, to_poem_doc, to_poet_doc};
use crate::log;

const PROGRESS_EVERY: usize = 10_000;

#[async_trait]
pub(crate) trait CorpusSource: Send + Sync {
    async fn count(&self, is_poems: bool) -> Result<i64, String>;
    async fn poems_after(&self, after_id: i32, limit: i64) -> Result<Vec<PoemSource>, String>;
    async fn poets_after(&self, after_id: i32, limit: i64) -> Result<Vec<PoetSource>, String>;
}

#[async_trait]
pub(crate) trait IndexStore: Send + Sync {
    async fn list_indices_for_alias(&self, prefix: &str) -> Result<Vec<String>, String>;
    async fn create_index(&self, index: &str, body: &Value) -> Result<(), String>;
    async fn put_refresh_interval(&self, index: &str, value: &str) -> Result<(), String>;
    async fn refresh(&self, index: &str) -> Result<(), String>;
    async fn force_merge(&self, index: &str) -> Result<(), String>;
    async fn bulk(&self, index: &str, docs: &[(String, String)]) -> Result<(), String>;
    async fn swap_alias(&self, alias: &str, prefix: &str, to_index: &str) -> Result<(), String>;
    async fn delete_index_quietly(&self, index: &str);
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
) -> Result<(String, usize), String> {
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

#[expect(
    clippy::print_stdout,
    reason = "this batch job's output is its structured log"
)]
async fn populate(ctx: &Ctx<'_>, target: &str, is_poems: bool) -> Result<usize, String> {
    let (es, corpus, batch_size, rules) = (ctx.index, ctx.corpus, ctx.batch_size, ctx.rules);
    let limit = i64::try_from(batch_size).map_err(|_| "bulkBatchSize exceeds i64".to_string())?;
    let expected = corpus.count(is_poems).await?;
    let index = if is_poems { "poems" } else { "poets" };
    es.put_refresh_interval(target, "-1").await?;
    let mut cursor: i32 = 0;
    let mut total = 0usize;
    loop {
        let batch: Vec<(String, String)> = if is_poems {
            let rows = corpus.poems_after(cursor, limit).await?;
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
            let rows = corpus.poets_after(cursor, limit).await?;
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
    es.force_merge(target).await?;
    es.refresh(target).await?;
    Ok(total)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

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

    struct Corpus {
        ids: Vec<i32>,
        cursors: Mutex<Vec<i32>>,
    }

    impl Corpus {
        fn of(ids: &[i32]) -> Self {
            Self {
                ids: ids.to_vec(),
                cursors: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl CorpusSource for Corpus {
        async fn count(&self, _: bool) -> Result<i64, String> {
            Ok(i64::try_from(self.ids.len()).expect("a small corpus"))
        }
        async fn poems_after(&self, _: i32, _: i64) -> Result<Vec<PoemSource>, String> {
            Ok(Vec::new())
        }
        async fn poets_after(&self, after_id: i32, limit: i64) -> Result<Vec<PoetSource>, String> {
            self.cursors.lock().expect("the cursors").push(after_id);
            Ok(self
                .ids
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

    struct Index {
        existing: Vec<String>,
        rejects_writes: bool,
        calls: Mutex<Vec<String>>,
    }

    impl Index {
        fn holding(existing: &[&str]) -> Self {
            Self {
                existing: existing.iter().map(|name| (*name).to_string()).collect(),
                rejects_writes: false,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn record(&self, call: String) {
            self.calls.lock().expect("the calls").push(call);
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("the calls").clone()
        }
    }

    #[async_trait]
    impl IndexStore for Index {
        async fn list_indices_for_alias(&self, _: &str) -> Result<Vec<String>, String> {
            Ok(self.existing.clone())
        }
        async fn create_index(&self, index: &str, _: &Value) -> Result<(), String> {
            self.record(format!("create {index}"));
            Ok(())
        }
        async fn put_refresh_interval(&self, index: &str, value: &str) -> Result<(), String> {
            self.record(format!("refresh_interval {index} {value}"));
            Ok(())
        }
        async fn refresh(&self, index: &str) -> Result<(), String> {
            self.record(format!("refresh {index}"));
            Ok(())
        }
        async fn force_merge(&self, index: &str) -> Result<(), String> {
            self.record(format!("force_merge {index}"));
            Ok(())
        }
        async fn bulk(&self, index: &str, docs: &[(String, String)]) -> Result<(), String> {
            if self.rejects_writes {
                return Err("bulk errors: rejected".into());
            }
            self.record(format!("bulk {index} {}", docs.len()));
            Ok(())
        }
        async fn swap_alias(&self, alias: &str, _: &str, to_index: &str) -> Result<(), String> {
            self.record(format!("swap {alias} {to_index}"));
            Ok(())
        }
        async fn delete_index_quietly(&self, index: &str) {
            self.record(format!("delete {index}"));
        }
    }

    async fn reindex_poets(index: &Index, corpus: &Corpus) -> Result<(String, usize), String> {
        let body = json!({});
        reindex(
            &Ctx {
                index,
                corpus,
                batch_size: 2,
                rules: &[],
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
    async fn a_reindex_writes_every_batch_then_moves_the_alias_and_drops_the_older_indices() {
        let index = Index::holding(&["poets_v1", "poets_v2"]);
        let corpus = Corpus::of(&[3, 5, 8, 13, 21]);
        let outcome = reindex_poets(&index, &corpus).await;
        assert_eq!(outcome, Ok(("poets_v3".to_string(), 5)));
        assert_eq!(
            index.calls(),
            [
                "create poets_v3",
                "refresh_interval poets_v3 -1",
                "bulk poets_v3 2",
                "bulk poets_v3 2",
                "bulk poets_v3 1",
                "refresh_interval poets_v3 1s",
                "force_merge poets_v3",
                "refresh poets_v3",
                "swap poets poets_v3",
                "delete poets_v1",
                "delete poets_v2",
            ]
        );
        assert_eq!(*corpus.cursors.lock().expect("the cursors"), [0, 5, 13, 21]);
    }

    #[tokio::test]
    async fn a_failed_write_removes_its_own_index_and_never_moves_the_alias() {
        let mut index = Index::holding(&["poets_v1"]);
        index.rejects_writes = true;
        let outcome = reindex_poets(&index, &Corpus::of(&[1, 2, 3])).await;
        assert_eq!(outcome, Err("bulk errors: rejected".to_string()));
        assert_eq!(
            index.calls(),
            [
                "create poets_v2",
                "refresh_interval poets_v2 -1",
                "delete poets_v2"
            ]
        );
    }

    #[tokio::test]
    async fn an_empty_corpus_still_ends_on_a_live_empty_index() {
        let index = Index::holding(&[]);
        let outcome = reindex_poets(&index, &Corpus::of(&[])).await;
        assert_eq!(outcome, Ok(("poets_v1".to_string(), 0)));
        assert!(index.calls().contains(&"swap poets poets_v1".to_string()));
        assert!(!index.calls().iter().any(|call| call.starts_with("bulk")));
    }
}
