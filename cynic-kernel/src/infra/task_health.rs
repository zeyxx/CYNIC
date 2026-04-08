//! Background task health — tracks last-success timestamps AND honest status for spawned tasks.
//! Updated by each background task on every tick, read by /health.
//! Detects silently crashed tasks: if last_success > 2× expected_interval, task is stale.
//!
//! HONESTY RULE: touch() means "I ran." detail() explains WHAT happened.
//! "ok" + "llm_unavailable" is honest. "ok" with no detail when LLM is down is a lie.
//!
//! ONE-SHOT RULE: tasks that run once at boot (e.g. backfill) must not age into "stale".
//! Their terminality is encoded in `detail` — see `is_oneshot_terminal_detail`.

use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A one-shot task is healthy forever once it reaches a "good" terminal detail.
///
/// Only successful terminations count. `timeout` is intentionally excluded: it
/// signals an incomplete run, so the aging logic still applies and `/health`
/// will report the task as stale if it never recovers — which is the honest
/// answer. Every addition to this list is an implicit contract with the tasks
/// that touch_*(detail) — keep it minimal.
fn is_oneshot_terminal_detail(detail: &str) -> bool {
    matches!(detail, "done" | "clean:0_orphans")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCriticality {
    ReadinessCritical,
    Housekeeping,
    DiagnosticOnly,
    StartupOneShot,
}

#[derive(Debug, Clone, Copy)]
struct TaskContract {
    name: &'static str,
    expected_interval: u64,
    criticality: TaskCriticality,
    consumer: &'static str,
    failure_effect: &'static str,
}

const COORD_EXPIRY: TaskContract = TaskContract {
    name: "coord_expiry",
    expected_interval: 120,
    criticality: TaskCriticality::Housekeeping,
    consumer: "coord claims",
    failure_effect: "stale claims linger longer than intended",
};
const USAGE_FLUSH: TaskContract = TaskContract {
    name: "usage_flush",
    expected_interval: 120,
    criticality: TaskCriticality::Housekeeping,
    consumer: "usage accounting",
    failure_effect: "cost and token telemetry lag behind runtime",
};
const SUMMARIZER: TaskContract = TaskContract {
    name: "summarizer",
    expected_interval: 1200,
    criticality: TaskCriticality::DiagnosticOnly,
    consumer: "session summaries",
    failure_effect: "session memory compacts more slowly",
};
const RATE_EVICTION: TaskContract = TaskContract {
    name: "rate_eviction",
    expected_interval: 120,
    criticality: TaskCriticality::Housekeeping,
    consumer: "per-ip limiter cleanup",
    failure_effect: "stale limiter buckets accumulate",
};
const HEALTH_LOOP: TaskContract = TaskContract {
    name: "health_loop",
    expected_interval: 60,
    criticality: TaskCriticality::ReadinessCritical,
    consumer: "dog circuit state",
    failure_effect: "serving health drifts from actual dog reachability",
};
const REMEDIATION: TaskContract = TaskContract {
    name: "remediation",
    expected_interval: 60,
    criticality: TaskCriticality::Housekeeping,
    consumer: "backend recovery",
    failure_effect: "auto-recovery lags while serving can continue degraded",
};
const INTROSPECTION: TaskContract = TaskContract {
    name: "introspection",
    expected_interval: 600,
    criticality: TaskCriticality::DiagnosticOnly,
    consumer: "ops alerts",
    failure_effect: "anomalies stop being clustered into actionable alerts",
};
const BACKFILL: TaskContract = TaskContract {
    name: "backfill",
    expected_interval: 1200,
    criticality: TaskCriticality::StartupOneShot,
    consumer: "crystal embeddings",
    failure_effect: "old crystals remain semantically incomplete",
};
const EVENT_CONSUMER: TaskContract = TaskContract {
    name: "event_consumer",
    expected_interval: 300,
    criticality: TaskCriticality::DiagnosticOnly,
    consumer: "event lag visibility",
    failure_effect: "internal events stop proving they are consumed",
};
const PROBE_SCHEDULER: TaskContract = TaskContract {
    name: "probe_scheduler",
    expected_interval: 20,
    criticality: TaskCriticality::ReadinessCritical,
    consumer: "environment snapshot + fleet gate",
    failure_effect: "readiness and backend quality drift from live environment",
};
const DOG_TTL: TaskContract = TaskContract {
    name: "dog_ttl",
    expected_interval: 60,
    criticality: TaskCriticality::ReadinessCritical,
    consumer: "dynamic dog roster",
    failure_effect: "expired dogs can stay routable and health can overclaim",
};

fn task_contract(name: &str) -> TaskContract {
    match name {
        "coord_expiry" => COORD_EXPIRY,
        "usage_flush" => USAGE_FLUSH,
        "summarizer" => SUMMARIZER,
        "rate_eviction" => RATE_EVICTION,
        "health_loop" => HEALTH_LOOP,
        "remediation" => REMEDIATION,
        "introspection" => INTROSPECTION,
        "backfill" => BACKFILL,
        "event_consumer" => EVENT_CONSUMER,
        "probe_scheduler" => PROBE_SCHEDULER,
        "dog_ttl" => DOG_TTL,
        other => panic!("task contract missing for '{other}'"),
    }
}

fn is_readiness_critical_task(name: &str) -> bool {
    matches!(
        task_contract(name).criticality,
        TaskCriticality::ReadinessCritical
    )
}

/// Shared background task health state.
/// Timestamps are atomic. Details use RwLock (written every tick, read by /health).
#[derive(Debug)]
pub struct TaskHealth {
    coord_expiry: AtomicU64,
    usage_flush: AtomicU64,
    summarizer: AtomicU64,
    rate_eviction: AtomicU64,
    health_loop: AtomicU64,
    remediation: AtomicU64,
    introspection: AtomicU64,
    backfill: AtomicU64,
    event_consumer: AtomicU64,
    probe_scheduler: AtomicU64,
    dog_ttl: AtomicU64,
    // Honest details — explain WHAT happened, not just WHEN
    summarizer_detail: RwLock<&'static str>,
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
            summarizer: AtomicU64::new(0),
            rate_eviction: AtomicU64::new(0),
            health_loop: AtomicU64::new(0),
            remediation: AtomicU64::new(0),
            introspection: AtomicU64::new(0),
            backfill: AtomicU64::new(0),
            event_consumer: AtomicU64::new(0),
            probe_scheduler: AtomicU64::new(0),
            dog_ttl: AtomicU64::new(0),
            summarizer_detail: RwLock::new("waiting"),
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
    pub fn touch_probe_scheduler(&self) {
        self.probe_scheduler
            .store(Self::now_secs(), Ordering::Relaxed);
    }
    pub fn touch_dog_ttl(&self) {
        self.dog_ttl.store(Self::now_secs(), Ordering::Relaxed);
    }

    /// Summarizer: HONEST touch with detail about what actually happened.
    pub fn touch_summarizer(&self, detail: &'static str) {
        self.summarizer.store(Self::now_secs(), Ordering::Relaxed);
        if let Ok(mut d) = self.summarizer_detail.write() {
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

    /// Returns true if any critical task is stale (last success > 2× interval).
    /// Used by system_health_status to degrade from "sovereign".
    pub fn has_stale(&self) -> bool {
        !self.readiness_stale_tasks().is_empty()
    }

    /// Explicit list of readiness-critical tasks that are currently stale.
    pub fn readiness_stale_tasks(&self) -> Vec<&'static str> {
        self.snapshot()
            .into_iter()
            .filter(|t| is_readiness_critical_task(t.name) && t.status == "stale")
            .map(|t| t.name)
            .collect()
    }

    /// Snapshot of all task health — for /health endpoint.
    pub fn snapshot(&self) -> Vec<TaskSnapshot> {
        let now = Self::now_secs();
        let sum_detail = self
            .summarizer_detail
            .read()
            .map(|d| *d)
            .unwrap_or("unknown");
        let bf_detail = self.backfill_detail.read().map(|d| *d).unwrap_or("unknown");
        vec![
            TaskSnapshot::new(
                COORD_EXPIRY,
                self.coord_expiry.load(Ordering::Relaxed),
                now,
                None,
            ),
            TaskSnapshot::new(
                USAGE_FLUSH,
                self.usage_flush.load(Ordering::Relaxed),
                now,
                None,
            ),
            TaskSnapshot::new(
                SUMMARIZER,
                self.summarizer.load(Ordering::Relaxed),
                now,
                Some(sum_detail),
            ),
            TaskSnapshot::new(
                RATE_EVICTION,
                self.rate_eviction.load(Ordering::Relaxed),
                now,
                None,
            ),
            TaskSnapshot::new(
                HEALTH_LOOP,
                self.health_loop.load(Ordering::Relaxed),
                now,
                None,
            ),
            TaskSnapshot::new(
                REMEDIATION,
                self.remediation.load(Ordering::Relaxed),
                now,
                None,
            ),
            TaskSnapshot::new(
                INTROSPECTION,
                self.introspection.load(Ordering::Relaxed),
                now,
                None,
            ),
            {
                // backfill is a one-shot: terminal detail ⇒ healthy forever.
                // Transient details ("scheduled", "consolidating", "running") still
                // age into "stale" so a silently stuck backfill is caught.
                // Public contract stays ok|stale|never — terminality lives in `detail`.
                let last = self.backfill.load(Ordering::Relaxed);
                let (status, last_success_ago) = if last == 0 {
                    ("never", 0)
                } else {
                    let ago = now.saturating_sub(last);
                    let s = if is_oneshot_terminal_detail(bf_detail) || ago <= 1200 {
                        "ok"
                    } else {
                        "stale"
                    };
                    (s, ago)
                };
                TaskSnapshot {
                    name: BACKFILL.name,
                    last_success_ago,
                    status,
                    criticality: BACKFILL.criticality,
                    consumer: BACKFILL.consumer,
                    failure_effect: BACKFILL.failure_effect,
                    detail: Some(bf_detail),
                }
            },
            TaskSnapshot::new(
                EVENT_CONSUMER,
                self.event_consumer.load(Ordering::Relaxed),
                now,
                None,
            ),
            TaskSnapshot::new(
                PROBE_SCHEDULER,
                self.probe_scheduler.load(Ordering::Relaxed),
                now,
                None,
            ),
            TaskSnapshot::new(DOG_TTL, self.dog_ttl.load(Ordering::Relaxed), now, None),
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
    pub criticality: TaskCriticality,
    pub consumer: &'static str,
    pub failure_effect: &'static str,
    /// Honest detail: what actually happened. None = simple task, no detail needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<&'static str>,
}

impl TaskSnapshot {
    fn new(contract: TaskContract, last: u64, now: u64, detail: Option<&'static str>) -> Self {
        if last == 0 {
            Self {
                name: contract.name,
                last_success_ago: 0,
                status: "never",
                criticality: contract.criticality,
                consumer: contract.consumer,
                failure_effect: contract.failure_effect,
                detail,
            }
        } else {
            let ago = now.saturating_sub(last);
            let status = if ago <= contract.expected_interval * 2 {
                "ok"
            } else {
                "stale"
            };
            Self {
                name: contract.name,
                last_success_ago: ago,
                status,
                criticality: contract.criticality,
                consumer: contract.consumer,
                failure_effect: contract.failure_effect,
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
    fn backfill_visible_in_snapshot() {
        let th = TaskHealth::new();
        assert!(th.snapshot().iter().any(|s| s.name == "backfill"));
        th.touch_backfill("done");
        let snap = th.snapshot();
        let bf = snap
            .iter()
            .find(|s| s.name == "backfill")
            .expect("backfill");
        assert_eq!(bf.status, "ok");
        assert_eq!(bf.detail, Some("done"));
    }

    #[test]
    fn oneshot_terminal_helper_tracks_contract() {
        assert!(is_oneshot_terminal_detail("done"));
        assert!(is_oneshot_terminal_detail("clean:0_orphans"));
        // timeout is intentionally NOT terminal — an incomplete run should still
        // surface as stale so /health is honest about stuck backfill.
        assert!(!is_oneshot_terminal_detail("timeout"));
        assert!(!is_oneshot_terminal_detail("consolidating"));
        assert!(!is_oneshot_terminal_detail("running"));
        assert!(!is_oneshot_terminal_detail("scheduled"));
        assert!(!is_oneshot_terminal_detail(""));
    }

    /// Backdate the backfill atomic to simulate age without a Clock abstraction.
    /// Tests live in this module so direct access to the private field is legal.
    fn backdate_backfill(th: &TaskHealth, seconds_ago: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        th.backfill
            .store(now.saturating_sub(seconds_ago), Ordering::Relaxed);
    }

    /// Locate a task entry in a snapshot by name. Panics with the task name if
    /// the snapshot doesn't contain it — every task in `snapshot()` is a
    /// static literal, so a missing entry means the snapshot shape changed and
    /// the calling test needs to know about it loudly.
    fn find_task<'a>(snap: &'a [TaskSnapshot], name: &str) -> &'a TaskSnapshot {
        snap.iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("task '{name}' missing from snapshot"))
    }

    #[test]
    fn backfill_terminal_done_stays_ok_after_long_age() {
        let th = TaskHealth::new();
        th.touch_backfill("done");
        // Simulate 1 hour elapsed — well beyond 2× expected_interval (1200s).
        backdate_backfill(&th, 3600);
        let snap = th.snapshot();
        let bf = find_task(&snap, "backfill");
        assert_eq!(
            bf.status, "ok",
            "terminal backfill must not degrade /health after aging"
        );
        assert_eq!(bf.detail, Some("done"));
        assert!(
            !th.has_stale(),
            "has_stale must not trip on terminal one-shot"
        );
    }

    #[test]
    fn backfill_terminal_clean_stays_ok_after_long_age() {
        let th = TaskHealth::new();
        th.touch_backfill("clean:0_orphans");
        backdate_backfill(&th, 3600);
        let snap = th.snapshot();
        let bf = find_task(&snap, "backfill");
        assert_eq!(bf.status, "ok");
        assert_eq!(bf.detail, Some("clean:0_orphans"));
        assert!(!th.has_stale());
    }

    #[test]
    fn backfill_transient_running_ages_to_stale() {
        let th = TaskHealth::new();
        th.touch_backfill("running");
        backdate_backfill(&th, 3600);
        let snap = th.snapshot();
        let bf = find_task(&snap, "backfill");
        assert_eq!(
            bf.status, "stale",
            "a stuck in-progress backfill must still age into stale"
        );
        assert_eq!(bf.detail, Some("running"));
        assert!(
            th.readiness_stale_tasks().is_empty(),
            "startup-one-shot backfill should stay visible without failing /ready"
        );
    }

    #[test]
    fn backfill_timeout_is_not_safe_terminal_and_ages() {
        let th = TaskHealth::new();
        th.touch_backfill("timeout");
        backdate_backfill(&th, 3600);
        let snap = th.snapshot();
        let bf = find_task(&snap, "backfill");
        assert_eq!(
            bf.status, "stale",
            "timeout is incomplete termination — must surface as stale"
        );
        assert!(
            th.readiness_stale_tasks().is_empty(),
            "timed out backfill should remain visible in /health without failing /ready"
        );
    }

    #[test]
    fn probe_scheduler_visible_in_snapshot() {
        let th = TaskHealth::new();
        assert!(th.snapshot().iter().any(|s| s.name == "probe_scheduler"));
        th.touch_probe_scheduler();
        let snap = th.snapshot();
        let ps = snap
            .iter()
            .find(|s| s.name == "probe_scheduler")
            .expect("probe_scheduler");
        assert_eq!(ps.status, "ok");
    }

    #[test]
    fn snapshot_exposes_contract_metadata() {
        let th = TaskHealth::new();
        let snap = th.snapshot();
        let health_loop = find_task(&snap, "health_loop");
        assert_eq!(health_loop.criticality, TaskCriticality::ReadinessCritical);
        assert_eq!(health_loop.consumer, "dog circuit state");
    }

    #[test]
    fn readiness_stale_tasks_list_only_readiness_critical_loops() {
        let th = TaskHealth::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        th.health_loop
            .store(now.saturating_sub(121), Ordering::Relaxed);
        th.event_consumer
            .store(now.saturating_sub(601), Ordering::Relaxed);

        assert_eq!(th.readiness_stale_tasks(), vec!["health_loop"]);
    }

    #[test]
    fn event_consumer_stale_stays_visible_without_failing_readiness() {
        let th = TaskHealth::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        th.event_consumer
            .store(now.saturating_sub(601), Ordering::Relaxed);

        let snap = th.snapshot();
        let event_consumer = find_task(&snap, "event_consumer");
        assert_eq!(event_consumer.status, "stale");
        assert!(
            !th.has_stale(),
            "event_consumer must stay visible in /health without failing /ready"
        );
    }
}
