//! Background task health — tracks last-success timestamps AND honest status for spawned tasks.
//! Updated by each background task on every tick, read by /health.
//! Detects silently crashed tasks: if last_success > 2× expected_interval, task is stale.
//!
//! HONESTY RULE: touch() means "I ran." detail() explains WHAT happened.
//! "ok" + "llm_unavailable" is honest. "ok" with no detail when LLM is down is a lie.

use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Shared background task health state.
/// Timestamps are atomic. Details use RwLock (written every tick, read by /health).
#[derive(Debug)]
pub struct TaskHealth {
    coord_expiry: AtomicU64,
    usage_flush: AtomicU64,
    ccm_aggregate: AtomicU64,
    summarizer: AtomicU64,
    rate_eviction: AtomicU64,
    health_loop: AtomicU64,
    remediation: AtomicU64,
    introspection: AtomicU64,
    backfill: AtomicU64,
    event_consumer: AtomicU64,
    // Honest details — explain WHAT happened, not just WHEN
    summarizer_detail: RwLock<&'static str>,
    ccm_aggregate_detail: RwLock<&'static str>,
    backfill_detail: RwLock<&'static str>,
}

impl Default for TaskHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskHealth {
    pub fn new() -> Self {
        Self {
            coord_expiry: AtomicU64::new(0),
            usage_flush: AtomicU64::new(0),
            ccm_aggregate: AtomicU64::new(0),
            summarizer: AtomicU64::new(0),
            rate_eviction: AtomicU64::new(0),
            health_loop: AtomicU64::new(0),
            remediation: AtomicU64::new(0),
            introspection: AtomicU64::new(0),
            backfill: AtomicU64::new(0),
            event_consumer: AtomicU64::new(0),
            summarizer_detail: RwLock::new("waiting"),
            ccm_aggregate_detail: RwLock::new("waiting"),
            backfill_detail: RwLock::new("scheduled"),
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn touch_coord_expiry(&self) {
        self.coord_expiry.store(Self::now_secs(), Ordering::Relaxed);
    }
    pub fn touch_usage_flush(&self) {
        self.usage_flush.store(Self::now_secs(), Ordering::Relaxed);
    }
    pub fn touch_rate_eviction(&self) {
        self.rate_eviction
            .store(Self::now_secs(), Ordering::Relaxed);
    }
    pub fn touch_health_loop(&self) {
        self.health_loop.store(Self::now_secs(), Ordering::Relaxed);
    }
    pub fn touch_remediation(&self) {
        self.remediation.store(Self::now_secs(), Ordering::Relaxed);
    }
    pub fn touch_introspection(&self) {
        self.introspection
            .store(Self::now_secs(), Ordering::Relaxed);
    }
    pub fn touch_event_consumer(&self) {
        self.event_consumer
            .store(Self::now_secs(), Ordering::Relaxed);
    }

    /// Summarizer: HONEST touch with detail about what actually happened.
    pub fn touch_summarizer(&self, detail: &'static str) {
        self.summarizer.store(Self::now_secs(), Ordering::Relaxed);
        if let Ok(mut d) = self.summarizer_detail.write() {
            *d = detail;
        }
    }

    /// CCM aggregate: HONEST touch with detail.
    pub fn touch_ccm_aggregate(&self, detail: &'static str) {
        self.ccm_aggregate
            .store(Self::now_secs(), Ordering::Relaxed);
        if let Ok(mut d) = self.ccm_aggregate_detail.write() {
            *d = detail;
        }
    }

    /// Backfill: track the one-shot task.
    pub fn touch_backfill(&self, detail: &'static str) {
        self.backfill.store(Self::now_secs(), Ordering::Relaxed);
        if let Ok(mut d) = self.backfill_detail.write() {
            *d = detail;
        }
    }

    /// Snapshot of all task health — for /health endpoint.
    pub fn snapshot(&self) -> Vec<TaskSnapshot> {
        let now = Self::now_secs();
        let sum_detail = self
            .summarizer_detail
            .read()
            .map(|d| *d)
            .unwrap_or("unknown");
        let ccm_detail = self
            .ccm_aggregate_detail
            .read()
            .map(|d| *d)
            .unwrap_or("unknown");
        let bf_detail = self.backfill_detail.read().map(|d| *d).unwrap_or("unknown");
        vec![
            TaskSnapshot::new(
                "coord_expiry",
                self.coord_expiry.load(Ordering::Relaxed),
                now,
                120,
                None,
            ),
            TaskSnapshot::new(
                "usage_flush",
                self.usage_flush.load(Ordering::Relaxed),
                now,
                120,
                None,
            ),
            TaskSnapshot::new(
                "ccm_aggregate",
                self.ccm_aggregate.load(Ordering::Relaxed),
                now,
                600,
                Some(ccm_detail),
            ),
            TaskSnapshot::new(
                "summarizer",
                self.summarizer.load(Ordering::Relaxed),
                now,
                1200,
                Some(sum_detail),
            ),
            TaskSnapshot::new(
                "rate_eviction",
                self.rate_eviction.load(Ordering::Relaxed),
                now,
                120,
                None,
            ),
            TaskSnapshot::new(
                "health_loop",
                self.health_loop.load(Ordering::Relaxed),
                now,
                60,
                None,
            ),
            TaskSnapshot::new(
                "remediation",
                self.remediation.load(Ordering::Relaxed),
                now,
                60,
                None,
            ),
            TaskSnapshot::new(
                "introspection",
                self.introspection.load(Ordering::Relaxed),
                now,
                600,
                None,
            ),
            TaskSnapshot::new(
                "backfill",
                self.backfill.load(Ordering::Relaxed),
                now,
                600,
                Some(bf_detail),
            ),
            TaskSnapshot::new(
                "event_consumer",
                self.event_consumer.load(Ordering::Relaxed),
                now,
                300,
                None,
            ),
        ]
    }
}

#[derive(Debug, serde::Serialize)]
pub struct TaskSnapshot {
    pub name: &'static str,
    /// Seconds since last successful tick (0 = never ran)
    pub last_success_ago: u64,
    /// "ok" | "stale" | "never"
    pub status: &'static str,
    /// Honest detail: what actually happened. None = simple task, no detail needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<&'static str>,
}

impl TaskSnapshot {
    fn new(
        name: &'static str,
        last: u64,
        now: u64,
        expected_interval: u64,
        detail: Option<&'static str>,
    ) -> Self {
        if last == 0 {
            Self {
                name,
                last_success_ago: 0,
                status: "never",
                detail,
            }
        } else {
            let ago = now.saturating_sub(last);
            let status = if ago <= expected_interval * 2 {
                "ok"
            } else {
                "stale"
            };
            Self {
                name,
                last_success_ago: ago,
                status,
                detail,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_and_snapshot() {
        let th = TaskHealth::new();
        assert_eq!(th.snapshot()[0].status, "never");
        th.touch_coord_expiry();
        let snap = th.snapshot();
        assert_eq!(snap[0].status, "ok");
        assert!(snap[0].last_success_ago <= 1);
    }

    #[test]
    fn summarizer_detail_tracks_llm_state() {
        let th = TaskHealth::new();
        th.touch_summarizer("llm_unavailable");
        let snap = th.snapshot();
        let sum = snap
            .iter()
            .find(|s| s.name == "summarizer")
            .expect("summarizer");
        assert_eq!(sum.status, "ok"); // task is alive
        assert_eq!(sum.detail, Some("llm_unavailable")); // but honest about what happened
    }

    #[test]
    fn ccm_aggregate_detail_tracks_idle() {
        let th = TaskHealth::new();
        th.touch_ccm_aggregate("idle:0");
        let snap = th.snapshot();
        let ccm = snap
            .iter()
            .find(|s| s.name == "ccm_aggregate")
            .expect("ccm");
        assert_eq!(ccm.detail, Some("idle:0"));
    }

    #[test]
    fn backfill_visible_in_snapshot() {
        let th = TaskHealth::new();
        assert!(th.snapshot().iter().any(|s| s.name == "backfill"));
        th.touch_backfill("done:87");
        let snap = th.snapshot();
        let bf = snap
            .iter()
            .find(|s| s.name == "backfill")
            .expect("backfill");
        assert_eq!(bf.status, "ok");
        assert_eq!(bf.detail, Some("done:87"));
    }
}
