//! Background task health — tracks last-success timestamps for spawned tasks.
//! Updated by each background task on successful tick, read by /health.
//! Detects silently crashed tasks: if last_success > 2× expected_interval, task is stale.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Shared background task health state.
/// All fields are atomic — safe for concurrent reads/writes from spawned tasks.
#[derive(Default)]
pub struct TaskHealth {
    coord_expiry: AtomicU64,
    usage_flush: AtomicU64,
    ccm_aggregate: AtomicU64,
    summarizer: AtomicU64,
    rate_eviction: AtomicU64,
    health_loop: AtomicU64,
    remediation: AtomicU64,
    introspection: AtomicU64,
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
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
    }

    pub fn touch_coord_expiry(&self)  { self.coord_expiry.store(Self::now_secs(), Ordering::Relaxed); }
    pub fn touch_usage_flush(&self)   { self.usage_flush.store(Self::now_secs(), Ordering::Relaxed); }
    pub fn touch_ccm_aggregate(&self) { self.ccm_aggregate.store(Self::now_secs(), Ordering::Relaxed); }
    pub fn touch_summarizer(&self)    { self.summarizer.store(Self::now_secs(), Ordering::Relaxed); }
    pub fn touch_rate_eviction(&self) { self.rate_eviction.store(Self::now_secs(), Ordering::Relaxed); }
    pub fn touch_health_loop(&self)  { self.health_loop.store(Self::now_secs(), Ordering::Relaxed); }
    pub fn touch_remediation(&self)  { self.remediation.store(Self::now_secs(), Ordering::Relaxed); }
    pub fn touch_introspection(&self) { self.introspection.store(Self::now_secs(), Ordering::Relaxed); }

    /// Snapshot of all task health — for /health endpoint.
    /// Returns (task_name, last_success_secs_ago, status) tuples.
    pub fn snapshot(&self) -> Vec<TaskSnapshot> {
        let now = Self::now_secs();
        vec![
            TaskSnapshot::new("coord_expiry", self.coord_expiry.load(Ordering::Relaxed), now, 120),
            TaskSnapshot::new("usage_flush", self.usage_flush.load(Ordering::Relaxed), now, 120),
            TaskSnapshot::new("ccm_aggregate", self.ccm_aggregate.load(Ordering::Relaxed), now, 600),
            TaskSnapshot::new("summarizer", self.summarizer.load(Ordering::Relaxed), now, 1200),
            TaskSnapshot::new("rate_eviction", self.rate_eviction.load(Ordering::Relaxed), now, 120),
            TaskSnapshot::new("health_loop", self.health_loop.load(Ordering::Relaxed), now, 60),
            TaskSnapshot::new("remediation", self.remediation.load(Ordering::Relaxed), now, 60),
            TaskSnapshot::new("introspection", self.introspection.load(Ordering::Relaxed), now, 600),
        ]
    }
}

#[derive(serde::Serialize)]
pub struct TaskSnapshot {
    pub name: &'static str,
    /// Seconds since last successful tick (0 = never ran)
    pub last_success_ago: u64,
    /// "ok" | "stale" | "never"
    pub status: &'static str,
}

impl TaskSnapshot {
    fn new(name: &'static str, last: u64, now: u64, expected_interval: u64) -> Self {
        if last == 0 {
            Self { name, last_success_ago: 0, status: "never" }
        } else {
            let ago = now.saturating_sub(last);
            let status = if ago <= expected_interval * 2 { "ok" } else { "stale" };
            Self { name, last_success_ago: ago, status }
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
}
