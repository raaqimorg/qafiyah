use async_trait::async_trait;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

use crate::accounts::users::{Identity, Profile, UpsertError, UserRepository};
use crate::db::accounts_schema::{identities, users};
use crate::db::{PgPool, coalesce, lower};
use crate::domain::StoreError;

pub struct PgUsers {
    pool: PgPool,
}

impl PgUsers {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
pub struct ProfileRow {
    id: i64,
    email: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

impl From<ProfileRow> for Profile {
    fn from(row: ProfileRow) -> Self {
        Profile {
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            avatar_url: row.avatar_url,
        }
    }
}

impl From<diesel::result::Error> for UpsertError {
    fn from(error: diesel::result::Error) -> Self {
        UpsertError::Store(error.into())
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
            .returning(ProfileRow::as_returning())
            .get_result::<ProfileRow>(&mut *conn)
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
            .returning(ProfileRow::as_returning())
            .get_result::<ProfileRow>(&mut *conn)
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
    Ok(profile.into())
}

#[async_trait]
impl UserRepository for PgUsers {
    async fn upsert(&self, identity: &Identity, email: &str) -> Result<Profile, UpsertError> {
        let mut pooled = self.pool.get().await.map_err(StoreError::from)?;
        let conn: &mut AsyncPgConnection = &mut pooled;
        conn.transaction::<_, UpsertError, _>(async |conn| upsert_with(conn, identity, email).await)
            .await
    }

    async fn find_or_create(&self, email: &str) -> Result<Profile, StoreError> {
        let mut conn = self.pool.get().await?;
        Ok(diesel::insert_into(users::table)
            .values(users::email.eq(email))
            .on_conflict(users::email)
            .do_update()
            .set(users::email.eq(excluded(users::email)))
            .returning(ProfileRow::as_returning())
            .get_result::<ProfileRow>(&mut conn)
            .await?
            .into())
    }
}
