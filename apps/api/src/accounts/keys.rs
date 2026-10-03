use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use rand::RngExt;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::constants::{
    API_KEY_BODY_LENGTH, API_KEY_DISPLAY_PREFIX_LENGTH, API_KEY_PREFIX, MAX_ACTIVE_KEYS_PER_USER,
};
use crate::error::StoreError;

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const UNBIASED_CEILING: u8 = 248;

pub struct RawKey {
    pub value: String,
    pub prefix: String,
    pub hash: [u8; 32],
}

pub fn generate() -> RawKey {
    let mut rng = rand::rng();
    let target_len = API_KEY_PREFIX.len().saturating_add(API_KEY_BODY_LENGTH);
    let mut value = String::with_capacity(target_len);
    value.push_str(API_KEY_PREFIX);
    while value.len() < target_len {
        let byte: u8 = rng.random();
        if byte >= UNBIASED_CEILING {
            continue;
        }
        let Some(&ch) = ALPHABET.get(usize::from(byte).rem_euclid(ALPHABET.len())) else {
            continue;
        };
        value.push(char::from(ch));
    }
    let hash = hash(&value);
    let prefix = prefix_of(&value);
    RawKey {
        value,
        prefix,
        hash,
    }
}

pub fn hash(raw: &str) -> [u8; 32] {
    Sha256::digest(raw.as_bytes()).into()
}

pub fn prefix_of(raw: &str) -> String {
    raw.chars().take(API_KEY_DISPLAY_PREFIX_LENGTH).collect()
}

pub fn is_well_formed(raw: &str) -> bool {
    let Some(body) = raw.strip_prefix(API_KEY_PREFIX) else {
        return false;
    };
    body.len() == API_KEY_BODY_LENGTH && body.bytes().all(|b| b.is_ascii_alphanumeric())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caller {
    pub key_id: i64,
    pub user_id: i64,
    pub requests: u32,
    pub burst: u32,
    pub ip_ceiling: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("at most {} active keys per user", MAX_ACTIVE_KEYS_PER_USER)]
    TooMany,
    #[error("no user with this id")]
    NoSuchUser,
    #[error(transparent)]
    Store(#[from] StoreError),
}

pub struct KeyRecord {
    pub id: i64,
    pub prefix: String,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub requests_this_hour: i64,
}

#[derive(Debug, Serialize)]
pub struct KeySummary {
    pub id: i64,
    pub prefix: String,
    pub label: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub requests_this_hour: i64,
}

#[derive(Debug, Serialize)]
pub struct PlanView {
    pub plan: String,
    pub requests: i32,
    pub burst: i32,
    pub used: i64,
}

#[async_trait]
pub trait KeyRepository: Send + Sync {
    async fn caller(&self, key_hash: &[u8; 32]) -> Result<Option<Caller>, StoreError>;
    async fn active_for(&self, user_id: i64) -> Result<Vec<KeyRecord>, StoreError>;
    async fn plan_for(&self, user_id: i64) -> Result<Option<PlanView>, StoreError>;
    async fn create_below(
        &self,
        user_id: i64,
        key: &RawKey,
        label: Option<&str>,
        cap: i64,
    ) -> Result<(), KeyError>;
    async fn revoke(&self, user_id: i64, key_id: i64) -> Result<bool, StoreError>;
}

fn instant(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn summary(record: KeyRecord) -> KeySummary {
    KeySummary {
        id: record.id,
        prefix: record.prefix,
        label: record.label,
        created_at: instant(record.created_at),
        last_used_at: record.last_used_at.map(instant),
        requests_this_hour: record.requests_this_hour,
    }
}

pub async fn lookup(keys: &dyn KeyRepository, raw: &str) -> Result<Option<Caller>, StoreError> {
    keys.caller(&hash(raw)).await
}

pub async fn list_for(
    keys: &dyn KeyRepository,
    user_id: i64,
) -> Result<Vec<KeySummary>, StoreError> {
    Ok(keys
        .active_for(user_id)
        .await?
        .into_iter()
        .map(summary)
        .collect())
}

pub async fn create_for(
    keys: &dyn KeyRepository,
    user_id: i64,
    label: Option<&str>,
) -> Result<RawKey, KeyError> {
    let key = generate();
    keys.create_below(user_id, &key, label, MAX_ACTIVE_KEYS_PER_USER)
        .await?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn a_generated_key_has_the_documented_shape() {
        let key = generate();
        assert!(key.value.starts_with(API_KEY_PREFIX));
        assert_eq!(key.value.len(), API_KEY_PREFIX.len() + API_KEY_BODY_LENGTH);
        assert!(
            key.value
                .chars()
                .skip(API_KEY_PREFIX.len())
                .all(|c| c.is_ascii_alphanumeric())
        );
    }

    #[test]
    fn the_prefix_is_the_leading_slice_of_the_raw_key() {
        let key = generate();
        assert_eq!(
            key.prefix,
            key.value
                .get(..API_KEY_DISPLAY_PREFIX_LENGTH)
                .expect("a long key")
        );
        assert_eq!(prefix_of(&key.value), key.prefix);
    }

    #[test]
    fn the_stored_hash_is_reproducible_from_the_raw_key() {
        let key = generate();
        assert_eq!(hash(&key.value), key.hash);
    }

    #[test]
    fn two_generated_keys_never_collide() {
        let mut seen = HashSet::new();
        for _ in 0..1_000 {
            assert!(seen.insert(generate().value));
        }
    }

    #[test]
    fn hashing_is_stable_across_calls_and_differs_per_input() {
        assert_eq!(hash("qaf_aaaa"), hash("qaf_aaaa"));
        assert_ne!(hash("qaf_aaaa"), hash("qaf_aaab"));
    }

    #[test]
    fn hashing_matches_the_standard_sha256_vector_so_stored_keys_survive_upgrades() {
        let hex: String = hash("abc").iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn the_active_key_ceiling_is_two() {
        assert_eq!(MAX_ACTIVE_KEYS_PER_USER, 2);
    }

    #[test]
    fn a_key_error_reports_which_limit_was_hit() {
        assert!(format!("{}", KeyError::TooMany).contains('2'));
    }

    #[test]
    fn a_short_key_yields_a_prefix_no_longer_than_itself() {
        assert_eq!(prefix_of("qaf_ab"), "qaf_ab");
    }

    #[test]
    fn a_key_is_well_formed_only_with_the_prefix_and_thirty_two_alphanumerics() {
        assert!(is_well_formed("qaf_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef"));
        assert!(is_well_formed("qaf_00000000000000000000000000000000"));
        for raw in [
            "",
            "qaf_",
            "qaf_short",
            "qaf_ABCDEFGHIJKLMNOPQRSTUVWXYZabcde",
            "qaf_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefg",
            "qaf_ABCDEFGHIJKLMNOPQRSTUVWXYZabcde-",
            "xaf_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef",
            "QAF_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef",
        ] {
            assert!(!is_well_formed(raw), "should reject {raw:?}");
        }
    }
}
