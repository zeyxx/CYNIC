//! DORMANT: ResourceGate — superseded by SlotSemaphore (Task 4, Soma L2).
//! All slot coordination now happens inside Judge::evaluate via SlotSemaphoreMap.
//! This module is kept for git history. Can be deleted after 2026-06-01.
// Original doc: ResourceGate — Soma L3: slot-aware GPU allocation with priority thresholds.
//!
//! Before dispatching high-contention tasks (Hermes agent, nightshift Dog evals),
//! consult the gate. Decision based on real slot utilization from SlotTracker
//! (populated by health loop every 30s) and caller priority.
//!
//! Priority thresholds (utilization above which the caller is queued):
//! - Hermes:     100% (only blocked when ALL slots busy)
//! - Nightshift:  50% (blocked when majority busy)
//! - Background:  25% (blocked early to preserve capacity)

use std::sync::Arc;

use super::slot_tracker::SlotTracker;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Background = 0,
    Nightshift = 1,
    Hermes = 2,
}

impl Priority {
    /// Utilization threshold above which this priority level is queued.
    /// Higher priority = higher threshold = harder to block.
    fn saturation_threshold(self) -> f64 {
        match self {
            Self::Background => 0.25,
            Self::Nightshift => 0.50,
            Self::Hermes => 1.0, // only blocked when 100% busy
        }
    }

    /// How long to wait before retrying when queued.
    fn backoff_secs(self) -> u64 {
        match self {
            Self::Hermes => 5,
            Self::Nightshift => 15,
            Self::Background => 30,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GateDecision {
    /// Allocate immediately. Task has GPU slot reserved.
    Allocate { slot_id: String },
    /// Utilization too high. Queue with estimated wait time (seconds).
    Queue { wait_secs: u64 },
}

/// Per-task request to the resource gate.
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    pub task_name: String,
    pub priority: Priority,
    pub estimated_duration_secs: u64,
    /// Dog ID to check slot utilization for. If None, uses aggregate.
    pub dog_id: Option<String>,
}

/// Soma L3: Gate that checks real slot utilization via SlotTracker.
/// No HTTP probing — reads cached data written by the health loop.
#[derive(Debug)]
pub struct ResourceGate {
    slot_tracker: Arc<SlotTracker>,
}

impl ResourceGate {
    pub fn new(slot_tracker: Arc<SlotTracker>) -> Self {
        Self { slot_tracker }
    }

    /// Request GPU allocation. Returns decision: allocate or queue.
    ///
    /// Reads utilization from SlotTracker (written by health loop every 30s).
    /// If no slot data exists for the dog, defaults to allocate (optimistic —
    /// the judge's own slot gate will catch saturation at evaluation time).
    pub fn request(&self, req: &ResourceRequest) -> GateDecision {
        let utilization = match &req.dog_id {
            Some(id) => self.dog_utilization(id),
            None => self.aggregate_utilization(),
        };

        let threshold = req.priority.saturation_threshold();

        if utilization >= threshold {
            GateDecision::Queue {
                wait_secs: req.priority.backoff_secs(),
            }
        } else {
            GateDecision::Allocate {
                slot_id: format!(
                    "{}-{}",
                    req.task_name,
                    chrono::Utc::now().timestamp_millis()
                ),
            }
        }
    }

    /// Utilization for a specific Dog's backend (0.0 – 1.0).
    /// Returns 0.0 if no data (optimistic — don't block on missing data).
    fn dog_utilization(&self, dog_id: &str) -> f64 {
        let snap = self.slot_tracker.snapshot();
        snap.get(dog_id).map(|s| s.utilization()).unwrap_or(0.0)
    }

    /// Max utilization across all tracked backends.
    /// Returns 0.0 if no data.
    fn aggregate_utilization(&self) -> f64 {
        let snap = self.slot_tracker.snapshot();
        snap.values()
            .map(|s| s.utilization())
            .fold(0.0_f64, f64::max)
    }

    /// Expose current utilization for /health.
    pub fn last_utilization(&self) -> Option<f64> {
        let util = self.aggregate_utilization();
        if util > 0.0 { Some(util) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::slot_tracker::BackendSlots;
    use std::time::Instant;

    fn tracker_with(dog_id: &str, total: u32, busy: u32) -> Arc<SlotTracker> {
        let tracker = Arc::new(SlotTracker::new());
        tracker.update(
            dog_id,
            BackendSlots {
                total,
                busy,
                per_slot_ctx: 32768,
                updated_at: Instant::now(),
                slots: vec![],
            },
        );
        tracker
    }

    #[test]
    fn hermes_allowed_at_50_percent() {
        let tracker = tracker_with("gpu-dog", 2, 1); // 50%
        let gate = ResourceGate::new(tracker);
        let req = ResourceRequest {
            task_name: "hermes-1".into(),
            priority: Priority::Hermes,
            estimated_duration_secs: 300,
            dog_id: Some("gpu-dog".into()),
        };
        assert!(matches!(gate.request(&req), GateDecision::Allocate { .. }));
    }

    #[test]
    fn hermes_blocked_at_100_percent() {
        let tracker = tracker_with("gpu-dog", 2, 2); // 100%
        let gate = ResourceGate::new(tracker);
        let req = ResourceRequest {
            task_name: "hermes-1".into(),
            priority: Priority::Hermes,
            estimated_duration_secs: 300,
            dog_id: Some("gpu-dog".into()),
        };
        assert!(matches!(
            gate.request(&req),
            GateDecision::Queue { wait_secs: 5 }
        ));
    }

    #[test]
    fn nightshift_blocked_at_50_percent() {
        let tracker = tracker_with("gpu-dog", 2, 1); // 50%
        let gate = ResourceGate::new(tracker);
        let req = ResourceRequest {
            task_name: "nightshift-1".into(),
            priority: Priority::Nightshift,
            estimated_duration_secs: 60,
            dog_id: Some("gpu-dog".into()),
        };
        assert!(matches!(
            gate.request(&req),
            GateDecision::Queue { wait_secs: 15 }
        ));
    }

    #[test]
    fn nightshift_allowed_at_0_percent() {
        let tracker = tracker_with("gpu-dog", 2, 0); // 0%
        let gate = ResourceGate::new(tracker);
        let req = ResourceRequest {
            task_name: "nightshift-1".into(),
            priority: Priority::Nightshift,
            estimated_duration_secs: 60,
            dog_id: Some("gpu-dog".into()),
        };
        assert!(matches!(gate.request(&req), GateDecision::Allocate { .. }));
    }

    #[test]
    fn background_blocked_at_25_percent() {
        let tracker = tracker_with("gpu-dog", 4, 1); // 25%
        let gate = ResourceGate::new(tracker);
        let req = ResourceRequest {
            task_name: "bg-1".into(),
            priority: Priority::Background,
            estimated_duration_secs: 600,
            dog_id: Some("gpu-dog".into()),
        };
        assert!(matches!(
            gate.request(&req),
            GateDecision::Queue { wait_secs: 30 }
        ));
    }

    #[test]
    fn unknown_dog_allows_optimistically() {
        let tracker = Arc::new(SlotTracker::new()); // empty
        let gate = ResourceGate::new(tracker);
        let req = ResourceRequest {
            task_name: "hermes-1".into(),
            priority: Priority::Background, // lowest priority
            estimated_duration_secs: 300,
            dog_id: Some("nonexistent".into()),
        };
        // No data → 0.0 utilization → allocate
        assert!(matches!(gate.request(&req), GateDecision::Allocate { .. }));
    }

    #[test]
    fn aggregate_uses_max_utilization() {
        let tracker = Arc::new(SlotTracker::new());
        tracker.update(
            "dog-a",
            BackendSlots {
                total: 4,
                busy: 1,
                per_slot_ctx: 32768,
                updated_at: Instant::now(),
                slots: vec![],
            },
        ); // 25%
        tracker.update(
            "dog-b",
            BackendSlots {
                total: 2,
                busy: 2,
                per_slot_ctx: 16384,
                updated_at: Instant::now(),
                slots: vec![],
            },
        ); // 100%
        let gate = ResourceGate::new(tracker);
        // Aggregate = max(25%, 100%) = 100%
        let req = ResourceRequest {
            task_name: "hermes-1".into(),
            priority: Priority::Hermes,
            estimated_duration_secs: 300,
            dog_id: None, // aggregate
        };
        assert!(matches!(gate.request(&req), GateDecision::Queue { .. }));
    }
}
