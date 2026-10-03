use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngExt;
use sha2::{Digest, Sha256};

use crate::accounts::users::Profile;
use crate::constants::{SESSION_ID_BYTES, SESSION_TTL_DAYS};
use crate::error::StoreError;

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

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, user_id: i64, id_hash: &[u8], ttl_days: i64) -> Result<(), StoreError>;
    async fn profile_for(&self, id_hash: &[u8]) -> Result<Option<Profile>, StoreError>;
    async fn delete(&self, id_hash: &[u8]) -> Result<(), StoreError>;
    async fn delete_all_for(&self, user_id: i64) -> Result<(), StoreError>;
}

pub async fn create(sessions: &dyn SessionRepository, user_id: i64) -> Result<Vec<u8>, StoreError> {
    let id = generate_id();
    sessions
        .create(user_id, &hash_id(&id), SESSION_TTL_DAYS)
        .await?;
    Ok(id)
}

pub async fn resolve(
    sessions: &dyn SessionRepository,
    id: &[u8],
) -> Result<Option<Profile>, StoreError> {
    sessions.profile_for(&hash_id(id)).await
}

pub async fn delete(sessions: &dyn SessionRepository, id: &[u8]) -> Result<(), StoreError> {
    sessions.delete(&hash_id(id)).await
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
