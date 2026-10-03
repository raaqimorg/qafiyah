use async_trait::async_trait;
use chrono::DateTime;
use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;

use crate::accounts::usage::UsageRepository;
use crate::constants::{SECONDS_PER_HOUR, USAGE_FLUSH_BATCH_ROWS};
use crate::db::PgPool;
use crate::db::accounts_schema::{api_keys, usage_hourly};
use crate::error::StoreError;

pub struct PgUsage {
    pool: PgPool,
}

impl PgUsage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UsageRepository for PgUsage {
    async fn add(&self, counts: &[(i64, i64, u32)]) -> Result<(), StoreError> {
        let mut conn = self.pool.get().await?;
        for chunk in counts.chunks(USAGE_FLUSH_BATCH_ROWS) {
            let rows = chunk
                .iter()
                .map(|&(key_id, hour, count)| {
                    (
                        usage_hourly::api_key_id.eq(key_id),
                        usage_hourly::hour.eq(DateTime::from_timestamp(
                            hour.saturating_mul(SECONDS_PER_HOUR),
                            0,
                        )
                        .unwrap_or_default()),
                        usage_hourly::requests.eq(count.cast_signed()),
                    )
                })
                .collect::<Vec<_>>();
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "this addition builds SQL that Postgres evaluates"
            )]
            let accumulated = usage_hourly::requests + excluded(usage_hourly::requests);
            diesel::insert_into(usage_hourly::table)
                .values(rows)
                .on_conflict((usage_hourly::api_key_id, usage_hourly::hour))
                .do_update()
                .set(usage_hourly::requests.eq(accumulated))
                .execute(&mut conn)
                .await?;

            let key_ids: Vec<i64> = chunk.iter().map(|&(key_id, _, _)| key_id).collect();
            diesel::update(api_keys::table.filter(api_keys::id.eq_any(key_ids)))
                .set(api_keys::last_used_at.eq(now))
                .execute(&mut conn)
                .await?;
        }
        Ok(())
    }
}
