//! SlotTracker — Soma L2: shared slot utilization state for sovereign inference backends.
//!
//! The health loop writes slot data every 30s by probing llama-server `/slots`.
//! Consumers (Judge, REST /inference/slots, /health) read without blocking.
//! K14: unknown/stale = pessimistic (assume busy → skip, not assume free → timeout).

use std::collections::BTreeMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Snapshot of a single llama-server slot (from GET /slots).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SlotInfo {
    pub id: u32,
    pub is_processing: bool,
    /// Context size for this slot (total / parallel).
    pub n_ctx: u32,
}

/// Aggregated slot state for one backend, updated by health loop.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackendSlots {
    pub total: u32,
    pub busy: u32,
    /// Per-slot context (total_ctx / parallel). 0 if unknown.
    pub per_slot_ctx: u32,
    /// When this data was last refreshed.
    #[serde(skip)]
    pub updated_at: Instant,
    /// Individual slot details.
    pub slots: Vec<SlotInfo>,
}

impl BackendSlots {
    /// Fraction of slots in use (0.0 – 1.0).
    pub fn utilization(&self) -> f64 {
        if self.total == 0 {
            return 1.0; // K14: unknown = pessimistic
        }
        self.busy as f64 / self.total as f64
    }

    pub fn all_busy(&self) -> bool {
        self.total > 0 && self.busy >= self.total
    }

    pub fn has_free_slot(&self) -> bool {
        self.total > 0 && self.busy < self.total
    }
}

/// Staleness threshold — slot data older than this is treated as unknown.
const STALE_THRESHOLD: Duration = Duration::from_secs(90);

/// Thread-safe slot state shared between health loop (writer) and consumers (readers).
/// Uses std::sync::RwLock: readers never block each other, writer blocks briefly.
#[derive(Debug)]
pub struct SlotTracker {
    /// dog_id → slot snapshot. BTreeMap for deterministic serialization (K19).
    state: RwLock<BTreeMap<String, BackendSlots>>,
}

impl SlotTracker {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(BTreeMap::new()),
        }
    }

    /// Update slot data for a Dog's backend. Called by health loop.
    pub fn update(&self, dog_id: &str, slots: BackendSlots) {
        if let Ok(mut guard) = self.state.write() {
            guard.insert(dog_id.to_string(), slots);
        }
    }

    /// Are all slots busy for this Dog? Returns true if all slots are processing
    /// AND data is fresh (< STALE_THRESHOLD).
    ///
    /// K14 note: stale or poisoned data returns false (allow the request, don't
    /// block on stale info). This is the OPPOSITE of the general K14 rule because
    /// blocking on stale data is worse than allowing one request that might queue
    /// in llama-server.
    pub fn all_slots_busy(&self, dog_id: &str) -> bool {
        let Ok(guard) = self.state.read() else {
            return false; // K14: poisoned lock → allow request
        };
        match guard.get(dog_id) {
            Some(slots) if slots.updated_at.elapsed() < STALE_THRESHOLD => slots.all_busy(),
            _ => false, // No data or stale → don't block
        }
    }

    /// Per-slot context size for a Dog's backend. Returns 0 if unknown/stale.
    /// Used by Judge for context routing: if per_slot_ctx < estimated tokens,
    /// the Dog will crash with context overflow even though total context is large.
    pub fn per_slot_ctx(&self, dog_id: &str) -> u32 {
        let Ok(guard) = self.state.read() else {
            return 0;
        };
        match guard.get(dog_id) {
            Some(slots) if slots.updated_at.elapsed() < STALE_THRESHOLD => slots.per_slot_ctx,
            _ => 0, // Unknown → 0 (don't restrict)
        }
    }

    /// Full snapshot for REST exposure. Filters out stale entries.
    pub fn snapshot(&self) -> BTreeMap<String, BackendSlots> {
        let guard = match self.state.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        guard
            .iter()
            .filter(|(_, slots)| slots.updated_at.elapsed() < STALE_THRESHOLD)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Summary for /health endpoint: (dog_id, total, busy, utilization, per_slot_ctx, fresh).
    pub fn health_summary(&self) -> Vec<SlotHealthEntry> {
        let guard = match self.state.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        guard
            .iter()
            .map(|(dog_id, slots)| {
                let fresh = slots.updated_at.elapsed() < STALE_THRESHOLD;
                SlotHealthEntry {
                    dog_id: dog_id.clone(),
                    total: slots.total,
                    busy: slots.busy,
                    utilization: slots.utilization(),
                    per_slot_ctx: slots.per_slot_ctx,
                    fresh,
                }
            })
            .collect()
    }
}

impl Default for SlotTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// One entry in the /health slot utilization report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SlotHealthEntry {
    pub dog_id: String,
    pub total: u32,
    pub busy: u32,
    pub utilization: f64,
    pub per_slot_ctx: u32,
    pub fresh: bool,
}

/// Build `BackendSlots` from a list of slot descriptors (parsed by infra layer).
/// Domain stays clean of `serde_json::Value` (K5).
pub fn build_backend_slots(slot_descriptors: Vec<(u32, bool, u32)>) -> BackendSlots {
    let mut busy = 0u32;
    let mut per_slot_ctx = 0u32;
    let mut slots = Vec::with_capacity(slot_descriptors.len());

    for (id, is_processing, n_ctx) in slot_descriptors {
        if is_processing {
            busy += 1;
        }
        if per_slot_ctx == 0 && n_ctx > 0 {
            per_slot_ctx = n_ctx;
        }
        slots.push(SlotInfo {
            id,
            is_processing,
            n_ctx,
        });
    }

    BackendSlots {
        total: slots.len() as u32,
        busy,
        per_slot_ctx,
        updated_at: Instant::now(),
        slots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_slots_two_one_busy() {
        let result = build_backend_slots(vec![(0, false, 32768), (1, true, 32768)]);
        assert_eq!(result.total, 2);
        assert_eq!(result.busy, 1);
        assert_eq!(result.per_slot_ctx, 32768);
        assert!(!result.all_busy());
        assert!(result.has_free_slot());
        assert!((result.utilization() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn build_slots_all_busy() {
        let result = build_backend_slots(vec![(0, true, 16384), (1, true, 16384)]);
        assert!(result.all_busy());
        assert!(!result.has_free_slot());
        assert!((result.utilization() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn build_slots_empty() {
        let result = build_backend_slots(vec![]);
        assert_eq!(result.total, 0);
        assert!(!result.all_busy());
        assert!((result.utilization() - 1.0).abs() < 1e-10); // K14: unknown = pessimistic
    }

    #[test]
    fn tracker_update_and_read() {
        let tracker = SlotTracker::new();
        tracker.update(
            "test-dog",
            BackendSlots {
                total: 2,
                busy: 2,
                per_slot_ctx: 16384,
                updated_at: Instant::now(),
                slots: vec![],
            },
        );
        assert!(tracker.all_slots_busy("test-dog"));
        assert!(!tracker.all_slots_busy("unknown-dog"));
    }

    #[test]
    fn tracker_stale_data_not_blocking() {
        let tracker = SlotTracker::new();
        tracker.update(
            "stale-dog",
            BackendSlots {
                total: 2,
                busy: 2,
                per_slot_ctx: 16384,
                updated_at: Instant::now() - Duration::from_secs(120), // 2 min old
                slots: vec![],
            },
        );
        // Stale data → don't block
        assert!(!tracker.all_slots_busy("stale-dog"));
    }

    #[test]
    fn tracker_snapshot_excludes_stale() {
        let tracker = SlotTracker::new();
        tracker.update(
            "fresh",
            BackendSlots {
                total: 2,
                busy: 1,
                per_slot_ctx: 32768,
                updated_at: Instant::now(),
                slots: vec![],
            },
        );
        tracker.update(
            "stale",
            BackendSlots {
                total: 2,
                busy: 2,
                per_slot_ctx: 16384,
                updated_at: Instant::now() - Duration::from_secs(120),
                slots: vec![],
            },
        );
        let snap = tracker.snapshot();
        assert_eq!(snap.len(), 1);
        assert!(snap.contains_key("fresh"));
    }

    #[test]
    fn health_summary_includes_freshness() {
        let tracker = SlotTracker::new();
        tracker.update(
            "dog-a",
            BackendSlots {
                total: 4,
                busy: 3,
                per_slot_ctx: 8192,
                updated_at: Instant::now(),
                slots: vec![],
            },
        );
        let summary = tracker.health_summary();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].dog_id, "dog-a");
        assert_eq!(summary[0].total, 4);
        assert_eq!(summary[0].busy, 3);
        assert!(summary[0].fresh);
    }
}
