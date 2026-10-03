use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use crate::accounts::keys::{Caller, KeyRepository};
use crate::constants::{
    ACCOUNTS_LOOKUP_TIMEOUT_MILLIS, API_KEY_CACHE_MAX_ENTRIES, API_KEY_CACHE_MISS_TTL_SECONDS,
    API_KEY_CACHE_TTL_SECONDS,
};
use crate::error::StoreError;

struct Entry {
    record: Option<Caller>,
    stored_at: i64,
    ttl: i64,
}

impl Entry {
    fn expired(&self, now: i64) -> bool {
        now.saturating_sub(self.stored_at) >= self.ttl
    }
}

#[derive(Default)]
pub struct KeyCache {
    entries: Mutex<HashMap<String, Entry>>,
}

impl KeyCache {
    fn entries(&self) -> MutexGuard<'_, HashMap<String, Entry>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn get(&self, raw: &str, now: i64) -> Option<Option<Caller>> {
        let entries = self.entries();
        let entry = entries.get(raw)?;
        if entry.expired(now) {
            return None;
        }
        Some(entry.record)
    }

    pub fn store(&self, raw: &str, record: Option<Caller>, now: i64) {
        self.insert(raw, record, now, API_KEY_CACHE_TTL_SECONDS);
    }

    fn store_failure(&self, raw: &str, now: i64) {
        self.insert(raw, None, now, API_KEY_CACHE_MISS_TTL_SECONDS);
    }

    fn insert(&self, raw: &str, record: Option<Caller>, now: i64, ttl: i64) {
        let mut entries = self.entries();
        if entries.len() >= API_KEY_CACHE_MAX_ENTRIES {
            entries.clear();
        }
        entries.insert(
            raw.to_string(),
            Entry {
                record,
                stored_at: now,
                ttl,
            },
        );
    }

    pub fn len(&self) -> usize {
        self.entries().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub async fn resolve(
        &self,
        keys: &dyn KeyRepository,
        raw: &str,
        now: i64,
    ) -> Result<Option<Caller>, StoreError> {
        if let Some(cached) = self.get(raw, now) {
            return Ok(cached);
        }
        let lookup = tokio::time::timeout(
            Duration::from_millis(ACCOUNTS_LOOKUP_TIMEOUT_MILLIS),
            crate::accounts::keys::lookup(keys, raw),
        )
        .await;
        let record = match lookup {
            Ok(Ok(record)) => record,
            Ok(Err(error)) => {
                self.store_failure(raw, now);
                return Err(error);
            }
            Err(_) => {
                self.store_failure(raw, now);
                return Err(StoreError::Database("api key lookup timed out".to_string()));
            }
        };
        self.store(raw, record, now);
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::keys::{KeyError, KeyRecord, PlanView, RawKey};

    fn caller(key_id: i64) -> Caller {
        Caller {
            key_id,
            user_id: 42,
            requests: 500,
            burst: 10,
            ip_ceiling: Some(1_500),
        }
    }

    #[test]
    fn a_cached_caller_carries_its_user_and_every_ceiling() {
        let cache = KeyCache::default();
        cache.store("qaf_a", Some(caller(1)), 0);
        let cached = cache.get("qaf_a", 0).flatten().expect("a hit");
        assert_eq!(cached.key_id, 1);
        assert_eq!(cached.user_id, 42);
        assert_eq!(cached.requests, 500);
        assert_eq!(cached.burst, 10);
        assert_eq!(cached.ip_ceiling, Some(1_500));
    }

    #[test]
    fn a_stored_hit_is_returned_within_the_ttl() {
        let cache = KeyCache::default();
        cache.store("qaf_a", Some(caller(1)), 0);
        assert_eq!(
            cache.get("qaf_a", 59).map(|r| r.map(|k| k.key_id)),
            Some(Some(1))
        );
    }

    #[test]
    fn an_entry_expires_once_the_ttl_has_passed() {
        let cache = KeyCache::default();
        cache.store("qaf_a", Some(caller(1)), 0);
        assert!(cache.get("qaf_a", 60).is_none());
    }

    #[test]
    fn a_miss_is_cached_so_invalid_keys_do_not_hit_the_database() {
        let cache = KeyCache::default();
        cache.store("qaf_bad", None, 0);
        assert_eq!(cache.get("qaf_bad", 10), Some(None));
    }

    #[test]
    fn an_unknown_key_is_not_a_cached_miss() {
        let cache = KeyCache::default();
        assert!(cache.get("qaf_never_seen", 0).is_none());
    }

    struct Failing {
        hangs: bool,
    }

    #[async_trait::async_trait]
    impl KeyRepository for Failing {
        async fn caller(&self, _: &[u8; 32]) -> Result<Option<Caller>, StoreError> {
            if self.hangs {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            Err(StoreError::Database("unreachable".into()))
        }
        async fn active_for(&self, _: i64) -> Result<Vec<KeyRecord>, StoreError> {
            Err(StoreError::Database("unreachable".into()))
        }
        async fn plan_for(&self, _: i64) -> Result<Option<PlanView>, StoreError> {
            Err(StoreError::Database("unreachable".into()))
        }
        async fn create_below(
            &self,
            _: i64,
            _: &RawKey,
            _: Option<&str>,
            _: i64,
        ) -> Result<(), KeyError> {
            Err(StoreError::Database("unreachable".into()).into())
        }
        async fn revoke(&self, _: i64, _: i64) -> Result<bool, StoreError> {
            Err(StoreError::Database("unreachable".into()))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_lookup_that_hangs_gives_up_and_is_cached_as_a_short_lived_miss() {
        let cache = KeyCache::default();
        let outcome = cache
            .resolve(&Failing { hangs: true }, "qaf_hanging", 0)
            .await;
        assert!(outcome.is_err_and(|error| error.to_string().contains("timed out")));
        assert_eq!(cache.get("qaf_hanging", 1), Some(None));
    }

    #[tokio::test]
    async fn a_lookup_failure_is_cached_as_a_short_lived_miss() {
        let cache = KeyCache::default();
        assert!(
            cache
                .resolve(&Failing { hangs: false }, "qaf_failing", 0)
                .await
                .is_err()
        );
        assert_eq!(cache.get("qaf_failing", 1), Some(None));
        assert!(cache.get("qaf_failing", 6).is_none());
    }

    #[test]
    fn the_cache_is_cleared_when_it_grows_past_its_ceiling() {
        let cache = KeyCache::default();
        for i in 0..API_KEY_CACHE_MAX_ENTRIES {
            cache.store(
                &format!("qaf_{i}"),
                Some(caller(i64::try_from(i).expect("small index"))),
                0,
            );
        }
        assert_eq!(cache.len(), API_KEY_CACHE_MAX_ENTRIES);
        cache.store("qaf_one_too_many", Some(caller(-1)), 0);
        assert_eq!(cache.len(), 1);
    }
}
