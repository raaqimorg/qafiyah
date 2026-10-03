use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::StoreError;

#[derive(Debug, Deserialize)]
pub struct Identity {
    pub provider: String,
    pub provider_uid: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Profile {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

#[derive(Debug, thiserror::Error)]
pub enum UpsertError {
    #[error("this email already belongs to a different account")]
    EmailTaken,
    #[error(transparent)]
    Store(#[from] StoreError),
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn upsert(&self, identity: &Identity, email: &str) -> Result<Profile, UpsertError>;
    async fn find_or_create(&self, email: &str) -> Result<Profile, StoreError>;
}

pub async fn for_email(users: &dyn UserRepository, raw: &str) -> Result<Profile, StoreError> {
    users.find_or_create(&normalize_email(raw)).await
}

pub async fn upsert(
    users: &dyn UserRepository,
    identity: &Identity,
) -> Result<Profile, UpsertError> {
    users
        .upsert(identity, &normalize_email(&identity.email))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(provider: &str, uid: &str, email: &str) -> Identity {
        Identity {
            provider: provider.to_string(),
            provider_uid: uid.to_string(),
            email: email.to_string(),
            display_name: Some("Test User".to_string()),
            avatar_url: None,
        }
    }

    #[test]
    fn an_identity_carries_the_provider_and_its_stable_uid() {
        let id = identity("google", "12345", "a@example.test");
        assert_eq!(id.provider, "google");
        assert_eq!(id.provider_uid, "12345");
    }

    #[test]
    fn emails_are_compared_case_insensitively() {
        assert_eq!(normalize_email("A@Example.TEST"), "a@example.test");
        assert_eq!(normalize_email("  a@example.test "), "a@example.test");
    }
}
