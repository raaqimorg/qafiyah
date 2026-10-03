use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::dsl::{now, sum};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

use crate::accounts::keys::{Caller, KeyError, KeyRecord, KeyRepository, PlanView, RawKey};
use crate::db::accounts_schema::{api_keys, plans, usage_hourly, users};
use crate::db::{PgPool, date_trunc};
use crate::error::StoreError;

pub struct PgKeys {
    pool: PgPool,
}

impl PgKeys {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<diesel::result::Error> for KeyError {
    fn from(error: diesel::result::Error) -> Self {
        KeyError::Store(error.into())
    }
}

type KeyRow = (
    i64,
    String,
    Option<String>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<i32>,
);

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

async fn create_with(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    key: &RawKey,
    label: Option<&str>,
    cap: i64,
) -> Result<(), KeyError> {
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
    if active >= cap {
        return Err(KeyError::TooMany);
    }
    diesel::insert_into(api_keys::table)
        .values((
            api_keys::user_id.eq(user_id),
            api_keys::key_hash.eq(key.hash.as_slice()),
            api_keys::prefix.eq(&key.prefix),
            api_keys::label.eq(label),
        ))
        .execute(&mut *conn)
        .await?;
    Ok(())
}

#[async_trait]
impl KeyRepository for PgKeys {
    async fn caller(&self, key_hash: &[u8; 32]) -> Result<Option<Caller>, StoreError> {
        let mut conn = self.pool.get().await?;
        let row = api_keys::table
            .inner_join(users::table.inner_join(plans::table))
            .filter(api_keys::key_hash.eq(key_hash.as_slice()))
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

    async fn active_for(&self, user_id: i64) -> Result<Vec<KeyRecord>, StoreError> {
        let mut conn = self.pool.get().await?;
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
                |(id, prefix, label, created_at, last_used_at, requests)| KeyRecord {
                    id,
                    prefix,
                    label,
                    created_at,
                    last_used_at,
                    requests_this_hour: i64::from(requests.unwrap_or(0)),
                },
            )
            .collect())
    }

    async fn plan_for(&self, user_id: i64) -> Result<Option<PlanView>, StoreError> {
        let pooled = self.pool.get().await?;
        let conn: &AsyncPgConnection = &pooled;
        let (plan, used) =
            tokio::try_join!(plan_row(conn, user_id), used_this_hour(conn, user_id))?;
        Ok(plan.map(|(plan, requests, burst)| PlanView {
            plan,
            requests,
            burst,
            used: used.unwrap_or(0),
        }))
    }

    async fn create_below(
        &self,
        user_id: i64,
        key: &RawKey,
        label: Option<&str>,
        cap: i64,
    ) -> Result<(), KeyError> {
        let mut pooled = self.pool.get().await.map_err(StoreError::from)?;
        let conn: &mut AsyncPgConnection = &mut pooled;
        conn.transaction::<_, KeyError, _>(async |conn| {
            create_with(conn, user_id, key, label, cap).await
        })
        .await
    }

    async fn revoke(&self, user_id: i64, key_id: i64) -> Result<bool, StoreError> {
        let mut conn = self.pool.get().await?;
        let revoked = diesel::update(
            api_keys::table
                .filter(api_keys::id.eq(key_id))
                .filter(api_keys::user_id.eq(user_id))
                .filter(api_keys::revoked_at.is_null()),
        )
        .set(api_keys::revoked_at.eq(now))
        .execute(&mut conn)
        .await?;
        Ok(revoked > 0)
    }
}
