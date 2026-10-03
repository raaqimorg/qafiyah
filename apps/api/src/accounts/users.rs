use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::db::accounts_schema::{identities, users};
use crate::db::{PgPool, coalesce, lower};
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct Identity {
    pub provider: String,
    pub provider_uid: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Queryable, Selectable)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
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
    Database(#[from] AppError),
}

impl From<diesel::result::Error> for UpsertError {
    fn from(error: diesel::result::Error) -> Self {
        UpsertError::Database(error.into())
    }
}

async fn upsert_with(
    conn: &mut AsyncPgConnection,
    identity: &Identity,
    email: &str,
) -> Result<Profile, UpsertError> {
    let linked = identities::table
        .filter(identities::provider.eq(&identity.provider))
        .filter(identities::provider_uid.eq(&identity.provider_uid))
        .select(identities::user_id)
        .first::<i64>(&mut *conn)
        .await
        .optional()?;

    let profile = if let Some(user_id) = linked {
        let taken = users::table
            .filter(lower(users::email).eq(email))
            .filter(users::id.ne(user_id))
            .select(users::id)
            .first::<i64>(&mut *conn)
            .await
            .optional()?;
        if taken.is_some() {
            return Err(UpsertError::EmailTaken);
        }
        diesel::update(users::table.find(user_id))
            .set((
                users::email.eq(email),
                users::display_name.eq(coalesce(
                    identity.display_name.as_deref(),
                    users::display_name,
                )),
                users::avatar_url.eq(coalesce(identity.avatar_url.as_deref(), users::avatar_url)),
            ))
            .returning(Profile::as_returning())
            .get_result::<Profile>(&mut *conn)
            .await?
    } else {
        diesel::insert_into(users::table)
            .values((
                users::email.eq(email),
                users::display_name.eq(identity.display_name.as_deref()),
                users::avatar_url.eq(identity.avatar_url.as_deref()),
            ))
            .on_conflict(users::email)
            .do_update()
            .set((
                users::display_name
                    .eq(coalesce(excluded(users::display_name), users::display_name)),
                users::avatar_url.eq(coalesce(excluded(users::avatar_url), users::avatar_url)),
            ))
            .returning(Profile::as_returning())
            .get_result::<Profile>(&mut *conn)
            .await?
    };

    diesel::insert_into(identities::table)
        .values((
            identities::provider.eq(&identity.provider),
            identities::provider_uid.eq(&identity.provider_uid),
            identities::user_id.eq(profile.id),
        ))
        .on_conflict((identities::provider, identities::provider_uid))
        .do_update()
        .set(identities::user_id.eq(excluded(identities::user_id)))
        .execute(&mut *conn)
        .await?;
    Ok(profile)
}

pub async fn upsert(accounts: &PgPool, identity: &Identity) -> Result<Profile, UpsertError> {
    let email = normalize_email(&identity.email);
    let mut pooled = accounts.get().await.map_err(AppError::from)?;
    let conn: &mut AsyncPgConnection = &mut pooled;
    conn.transaction::<_, UpsertError, _>(async |conn| upsert_with(conn, identity, &email).await)
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
