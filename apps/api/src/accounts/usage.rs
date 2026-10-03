use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::DateTime;
use diesel::dsl::now;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;

use crate::constants::{SECONDS_PER_HOUR, USAGE_FLUSH_BATCH_ROWS, USAGE_FLUSH_SECONDS};
use crate::db::PgPool;
use crate::db::accounts_schema::{api_keys, usage_hourly};
use crate::error::AppError;

#[derive(Default)]
pub struct UsageRecorder {
    pending: Mutex<HashMap<(i64, i64), u32>>,
}

impl UsageRecorder {
    fn pending(&self) -> MutexGuard<'_, HashMap<(i64, i64), u32>> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn record(&self, key_id: i64, hour: i64) {
        let mut pending = self.pending();
        let count = pending.entry((key_id, hour)).or_insert(0);
        *count = count.saturating_add(1);
    }

    pub fn drain(&self) -> Vec<(i64, i64, u32)> {
        let mut pending = self.pending();
        pending
            .drain()
            .map(|((key_id, hour), count)| (key_id, hour, count))
            .collect()
    }

    pub async fn flush(&self, accounts: &PgPool) -> Result<u64, AppError> {
        let batch = self.drain();
        if batch.is_empty() {
            return Ok(0);
        }
        let mut conn = accounts.get().await?;
        for chunk in batch.chunks(USAGE_FLUSH_BATCH_ROWS) {
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

        Ok(u64::try_from(batch.len()).unwrap_or(u64::MAX))
    }
}

#[expect(
    clippy::print_stderr,
    reason = "a failed usage flush reports to stderr and keeps the recorder draining"
)]
pub fn flusher(recorder: Arc<UsageRecorder>, accounts: PgPool) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(USAGE_FLUSH_SECONDS));
        loop {
            ticker.tick().await;
            if let Err(e) = recorder.flush(&accounts).await {
                eprintln!(
                    "{}",
                    crate::log::stage_event("usage_flush", Some(("error", e.to_string().into())))
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_requests_accumulate_into_one_pending_row() {
        let recorder = UsageRecorder::default();
        for _ in 0..5 {
            recorder.record(1, 100);
        }
        let drained = recorder.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0], (1, 100, 5));
    }

    #[test]
    fn different_keys_and_hours_are_separate_rows() {
        let recorder = UsageRecorder::default();
        recorder.record(1, 100);
        recorder.record(2, 100);
        recorder.record(1, 101);
        let mut drained = recorder.drain();
        drained.sort_unstable();
        assert_eq!(drained, vec![(1, 100, 1), (1, 101, 1), (2, 100, 1)]);
    }

    #[test]
    fn draining_empties_the_recorder() {
        let recorder = UsageRecorder::default();
        recorder.record(1, 100);
        assert_eq!(recorder.drain().len(), 1);
        assert!(recorder.drain().is_empty());
    }

    #[test]
    fn counts_recorded_after_a_drain_start_from_zero_again() {
        let recorder = UsageRecorder::default();
        recorder.record(1, 100);
        recorder.drain();
        recorder.record(1, 100);
        assert_eq!(recorder.drain(), vec![(1, 100, 1)]);
    }
}
