use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;

use crate::constants::USAGE_FLUSH_SECONDS;
use crate::domain::StoreError;

#[async_trait]
pub trait UsageRepository: Send + Sync {
    async fn add(&self, counts: &[(i64, i64, u32)]) -> Result<(), StoreError>;
}

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

    pub async fn flush(&self, usage: &dyn UsageRepository) -> Result<u64, StoreError> {
        let batch = self.drain();
        if batch.is_empty() {
            return Ok(0);
        }
        usage.add(&batch).await?;
        Ok(u64::try_from(batch.len()).unwrap_or(u64::MAX))
    }
}

pub fn flusher(recorder: Arc<UsageRecorder>, usage: Arc<dyn UsageRepository>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(USAGE_FLUSH_SECONDS));
        loop {
            ticker.tick().await;
            if let Err(e) = recorder.flush(usage.as_ref()).await {
                tracing::error!(stage = "usage_flush", error = %e);
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
