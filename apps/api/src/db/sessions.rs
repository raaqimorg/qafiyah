use async_trait::async_trait;
use diesel::dsl::{IntervalDsl, now};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::accounts::sessions::SessionRepository;
use crate::accounts::users::Profile;
use crate::db::accounts_schema::{sessions, users};
use crate::db::users::ProfileRow;
use crate::db::{PgPool, date_add};
use crate::domain::StoreError;

pub struct PgSessions {
    pool: PgPool,
}

impl PgSessions {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for PgSessions {
    async fn create(&self, user_id: i64, id_hash: &[u8], ttl_days: i64) -> Result<(), StoreError> {
        let mut conn = self.pool.get().await?;
        diesel::delete(sessions::table.filter(sessions::expires_at.le(now)))
            .execute(&mut conn)
            .await?;
        diesel::insert_into(sessions::table)
            .values((
                sessions::id.eq(id_hash),
                sessions::user_id.eq(user_id),
                sessions::expires_at.eq(date_add(now, ttl_days.days())),
            ))
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    async fn profile_for(&self, id_hash: &[u8]) -> Result<Option<Profile>, StoreError> {
        let mut conn = self.pool.get().await?;
        Ok(sessions::table
            .inner_join(users::table)
            .filter(sessions::id.eq(id_hash))
            .filter(sessions::expires_at.gt(now))
            .select(ProfileRow::as_select())
            .first::<ProfileRow>(&mut conn)
            .await
            .optional()?
            .map(Profile::from))
    }

    async fn delete(&self, id_hash: &[u8]) -> Result<(), StoreError> {
        let mut conn = self.pool.get().await?;
        diesel::delete(sessions::table.filter(sessions::id.eq(id_hash)))
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    async fn delete_all_for(&self, user_id: i64) -> Result<(), StoreError> {
        let mut conn = self.pool.get().await?;
        diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
            .execute(&mut conn)
            .await?;
        Ok(())
    }
}
