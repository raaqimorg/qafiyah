use chrono::{DateTime, SecondsFormat, Utc};
use diesel::dsl::{now, sum};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use rand::RngExt;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::db::accounts_schema::{api_keys, plans, usage_hourly, users};
use crate::db::{PgPool, date_trunc};
use crate::error::AppError;

use crate::constants::{
    API_KEY_BODY_LENGTH, API_KEY_DISPLAY_PREFIX_LENGTH, API_KEY_PREFIX, MAX_ACTIVE_KEYS_PER_USER,
};

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

pub async fn lookup(accounts: &PgPool, raw: &str) -> Result<Option<Caller>, AppError> {
    let mut conn = accounts.get().await?;
    let row = api_keys::table
        .inner_join(users::table.inner_join(plans::table))
        .filter(api_keys::key_hash.eq(hash(raw).as_slice()))
        .filter(api_keys::revoked_at.is_null())
        .select((
            api_keys::id,
            users::id,
            plans::requests,
            plans::burst,
            plans::ip_ceiling,
        ))
        .first::<(i64, i64, i32, i32, Option<i32>)>(&mut conn)
        .await
        .optional()?;
    Ok(
        row.map(|(key_id, user_id, requests, burst, ip_ceiling)| Caller {
            key_id,
            user_id,
            requests: requests.max(0).cast_unsigned(),
            burst: burst.max(0).cast_unsigned(),
            ip_ceiling: ip_ceiling.map(|ceiling| ceiling.max(0).cast_unsigned()),
        }),
    )
}

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("at most {} active keys per user", MAX_ACTIVE_KEYS_PER_USER)]
    TooMany,
    #[error("no user with this id")]
    NoSuchUser,
    #[error(transparent)]
    Database(#[from] AppError),
}

impl From<diesel::result::Error> for KeyError {
    fn from(error: diesel::result::Error) -> Self {
        KeyError::Database(error.into())
    }
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

fn instant(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

type KeyRow = (
    i64,
    String,
    Option<String>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<i32>,
);

pub async fn list_for(accounts: &PgPool, user_id: i64) -> Result<Vec<KeySummary>, AppError> {
    let mut conn = accounts.get().await?;
    Ok(api_keys::table
        .left_join(
            usage_hourly::table.on(usage_hourly::api_key_id
                .eq(api_keys::id)
                .and(usage_hourly::hour.eq(date_trunc("hour", now)))),
        )
        .filter(api_keys::user_id.eq(user_id))
        .filter(api_keys::revoked_at.is_null())
        .order(api_keys::created_at.asc())
        .select((
            api_keys::id,
            api_keys::prefix,
            api_keys::label,
            api_keys::created_at,
            api_keys::last_used_at,
            usage_hourly::requests.nullable(),
        ))
        .load::<KeyRow>(&mut conn)
        .await?
        .into_iter()
        .map(
            |(id, prefix, label, created_at, last_used_at, requests)| KeySummary {
                id,
                prefix,
                label,
                created_at: instant(created_at),
                last_used_at: last_used_at.map(instant),
                requests_this_hour: i64::from(requests.unwrap_or(0)),
            },
        )
        .collect())
}

#[derive(Debug, Serialize)]
pub struct PlanView {
    pub plan: String,
    pub requests: i32,
    pub burst: i32,
    pub used: i64,
}

async fn plan_row(
    mut conn: &AsyncPgConnection,
    user_id: i64,
) -> QueryResult<Option<(String, i32, i32)>> {
    users::table
        .inner_join(plans::table)
        .filter(users::id.eq(user_id))
        .select((plans::slug, plans::requests, plans::burst))
        .first(&mut conn)
        .await
        .optional()
}

async fn used_this_hour(mut conn: &AsyncPgConnection, user_id: i64) -> QueryResult<Option<i64>> {
    usage_hourly::table
        .inner_join(api_keys::table)
        .filter(api_keys::user_id.eq(user_id))
        .filter(usage_hourly::hour.eq(date_trunc("hour", now)))
        .select(sum(usage_hourly::requests))
        .first(&mut conn)
        .await
}

pub async fn plan_for(accounts: &PgPool, user_id: i64) -> Result<Option<PlanView>, AppError> {
    let pooled = accounts.get().await?;
    let conn: &AsyncPgConnection = &pooled;
    let (plan, used) = tokio::try_join!(plan_row(conn, user_id), used_this_hour(conn, user_id))?;
    Ok(plan.map(|(plan, requests, burst)| PlanView {
        plan,
        requests,
        burst,
        used: used.unwrap_or(0),
    }))
}

async fn create_with(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    label: Option<&str>,
) -> Result<RawKey, KeyError> {
    let user = users::table
        .find(user_id)
        .select(users::id)
        .for_update()
        .first::<i64>(&mut *conn)
        .await
        .optional()?;
    if user.is_none() {
        return Err(KeyError::NoSuchUser);
    }
    let active: i64 = api_keys::table
        .filter(api_keys::user_id.eq(user_id))
        .filter(api_keys::revoked_at.is_null())
        .count()
        .get_result(&mut *conn)
        .await?;
    if active >= MAX_ACTIVE_KEYS_PER_USER {
        return Err(KeyError::TooMany);
    }
    let key = generate();
    diesel::insert_into(api_keys::table)
        .values((
            api_keys::user_id.eq(user_id),
            api_keys::key_hash.eq(key.hash.as_slice()),
            api_keys::prefix.eq(&key.prefix),
            api_keys::label.eq(label),
        ))
        .execute(&mut *conn)
        .await?;
    Ok(key)
}

pub async fn create_for(
    accounts: &PgPool,
    user_id: i64,
    label: Option<&str>,
) -> Result<RawKey, KeyError> {
    let mut pooled = accounts.get().await.map_err(AppError::from)?;
    let conn: &mut AsyncPgConnection = &mut pooled;
    conn.transaction::<_, KeyError, _>(async |conn| create_with(conn, user_id, label).await)
        .await
}

pub async fn revoke(accounts: &PgPool, user_id: i64, key_id: i64) -> Result<u64, AppError> {
    let mut conn = accounts.get().await?;
    let done = diesel::update(
        api_keys::table
            .filter(api_keys::id.eq(key_id))
            .filter(api_keys::user_id.eq(user_id))
            .filter(api_keys::revoked_at.is_null()),
    )
    .set(api_keys::revoked_at.eq(now))
    .execute(&mut conn)
    .await?;
    Ok(u64::try_from(done).unwrap_or(u64::MAX))
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
