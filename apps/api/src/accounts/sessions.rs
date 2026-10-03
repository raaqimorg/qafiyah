use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use diesel::dsl::{IntervalDsl, now};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rand::RngExt;
use sha2::{Digest, Sha256};

use crate::accounts::users::Profile;
use crate::constants::{SESSION_ID_BYTES, SESSION_TTL_DAYS};
use crate::db::accounts_schema::{sessions, users};
use crate::db::{PgPool, date_add};
use crate::error::AppError;

pub fn generate_id() -> Vec<u8> {
    let mut rng = rand::rng();
    (0..SESSION_ID_BYTES).map(|_| rng.random::<u8>()).collect()
}

fn hash_id(id: &[u8]) -> Vec<u8> {
    Sha256::digest(id).to_vec()
}

pub fn encode_id(id: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(id)
}

pub fn decode_id(encoded: &str) -> Option<Vec<u8>> {
    let decoded = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    (decoded.len() == SESSION_ID_BYTES).then_some(decoded)
}

pub async fn create(accounts: &PgPool, user_id: i64) -> Result<Vec<u8>, AppError> {
    let mut conn = accounts.get().await?;
    diesel::delete(sessions::table.filter(sessions::expires_at.le(now)))
        .execute(&mut conn)
        .await?;
    let id = generate_id();
    diesel::insert_into(sessions::table)
        .values((
            sessions::id.eq(hash_id(&id)),
            sessions::user_id.eq(user_id),
            sessions::expires_at.eq(date_add(now, SESSION_TTL_DAYS.days())),
        ))
        .execute(&mut conn)
        .await?;
    Ok(id)
}

pub async fn resolve(accounts: &PgPool, id: &[u8]) -> Result<Option<Profile>, AppError> {
    let mut conn = accounts.get().await?;
    Ok(sessions::table
        .inner_join(users::table)
        .filter(sessions::id.eq(hash_id(id)))
        .filter(sessions::expires_at.gt(now))
        .select(Profile::as_select())
        .first(&mut conn)
        .await
        .optional()?)
}

pub async fn delete(accounts: &PgPool, id: &[u8]) -> Result<(), AppError> {
    let mut conn = accounts.get().await?;
    diesel::delete(sessions::table.filter(sessions::id.eq(hash_id(id))))
        .execute(&mut conn)
        .await?;
    Ok(())
}

pub async fn delete_all_for(accounts: &PgPool, user_id: i64) -> Result<(), AppError> {
    let mut conn = accounts.get().await?;
    diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
        .execute(&mut conn)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn a_generated_id_is_the_documented_length() {
        assert_eq!(generate_id().len(), SESSION_ID_BYTES);
    }

    #[test]
    fn two_generated_ids_never_collide() {
        let mut seen = HashSet::new();
        for _ in 0..1_000 {
            assert!(seen.insert(generate_id()));
        }
    }

    #[test]
    fn encoding_round_trips() {
        let id = generate_id();
        let encoded = encode_id(&id);
        assert_eq!(decode_id(&encoded), Some(id));
    }

    #[test]
    fn a_known_id_keeps_its_cookie_value_and_stored_hash_so_live_sessions_survive_upgrades() {
        let id = vec![0xfb_u8; SESSION_ID_BYTES];
        let cookie = "-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_s";
        assert_eq!(encode_id(&id), cookie);
        assert_eq!(decode_id(cookie), Some(id.clone()));
        let stored: String = hash_id(&id).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            stored,
            "456a04986c2572de19b058ef2ef20b0077017bcdb15819af052eb9d5d9b8e504"
        );
    }

    #[test]
    fn a_malformed_cookie_value_decodes_to_nothing() {
        assert_eq!(decode_id("not base64url!!"), None);
        assert_eq!(decode_id(""), None);
        assert_eq!(decode_id("c2hvcnQ"), None);
    }
}
