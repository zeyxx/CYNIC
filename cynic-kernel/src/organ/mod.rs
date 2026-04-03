// organ/ — InferenceOrgan: owns backend registry, health tracking, and routing data.
//
// Phase 1: registry + health counters. main.rs still assembles Dogs; organ provides data.
// Phase 2: routing (profile → cluster → backend selection), Welford drift detection.
//
// Layer boundary (I1): organ imports from domain/ only. Never imports from dogs/ or infra/.
// main.rs (composition root) assembles InferenceDog from organ registry data.

pub mod health;
pub mod registry;
pub mod router;

use crate::organ::health::{DogStats, ParseFailureGate, ScoreFailureKind};
use crate::organ::registry::{Backend, BackendHealth, BackendId, MeasuredCapabilities};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Per-backend health tracking entry (private — exposed only via BackendHandle).
#[derive(Debug)]
struct BackendEntry {
    backend: Backend,
    stats: DogStats,
    gate: ParseFailureGate,
}

/// Opaque handle to a backend entry, obtained from `InferenceOrgan::register_backend()`.
/// Callers pass this to `InferenceOrgan::update_stats_entry()` to avoid a HashMap lookup.
#[derive(Debug, Clone)]
pub struct BackendHandle(Arc<Mutex<BackendEntry>>);

/// InferenceOrgan — owns the registry of Dog backends and tracks their health.
///
/// # Thread safety
/// Each `BackendHandle` wraps `Arc<Mutex<BackendEntry>>` so the Judge can update stats
/// concurrently without holding the organ write lock.
///
/// # Layer boundary (I1)
/// The organ does NOT construct InferenceDog instances. It exposes registry data
/// via `backends()`. main.rs reads that data and builds the Dogs.
#[derive(Debug)]
pub struct InferenceOrgan {
    entries: HashMap<BackendId, BackendHandle>,
}

impl InferenceOrgan {
    /// Empty organ — register backends via `register_backend()`.
    pub fn boot_empty() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register a backend. Returns an opaque handle for subsequent `update_stats_entry` calls.
    pub fn register_backend(&mut self, backend: Backend) -> BackendHandle {
        let id = backend.id.clone();
        let handle = BackendHandle(Arc::new(Mutex::new(BackendEntry {
            backend,
            stats: DogStats::new(),
            gate: ParseFailureGate::new(),
        })));
        self.entries.insert(id, handle.clone());
        handle
    }

    /// Number of registered backends.
    pub fn backend_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over backend ids.
    pub fn backend_ids(&self) -> impl Iterator<Item = &BackendId> {
        self.entries.keys()
    }

    /// Current stats snapshot for a backend. Returns None if backend not registered.
    pub fn stats_snapshot(&self, id: &BackendId) -> Option<DogStats> {
        self.entries
            .get(id)
            .and_then(|h| h.0.lock().ok().map(|guard| guard.stats.clone()))
    }

    /// Update stats for a backend after a Dog evaluation.
    /// Uses the pre-obtained handle to avoid a HashMap lookup on the hot path.
    pub fn update_stats_entry(handle: &BackendHandle, kind: ScoreOutcome) {
        let Ok(mut guard) = handle.0.lock() else {
            return;
        };
        match kind {
            ScoreOutcome::Success => {
                guard.stats.record_success();
                guard.gate.record_success();
                let rate = guard.stats.json_valid_rate();
                guard.backend.measured.json_valid_rate = rate;
                guard.backend.measured.scoring_in_range_rate = rate;
            }
            ScoreOutcome::Failure(failure_kind) => {
                guard.stats.record_failure(failure_kind);
                guard.gate.record_failure();
                // K14: gate trip → degrade backend (safe default, not optimistic)
                if guard.gate.is_tripped() {
                    let rate = guard.stats.json_valid_rate();
                    guard.backend.health = BackendHealth::Degraded {
                        reason: format!(
                            "parse failure rate {:.0}% > 50% in last 10 calls",
                            guard.gate.failure_rate() * 100.0
                        ),
                        since: Instant::now(),
                    };
                    guard.backend.measured.json_valid_rate = rate;
                    guard.backend.measured.scoring_in_range_rate = rate;
                }
            }
        }
    }

    /// Aggregate json_valid_rate across all registered backends (simple average).
    pub fn overall_valid_rate(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let sum: f64 = self
            .entries
            .values()
            .filter_map(|h| h.0.lock().ok().map(|g| g.stats.json_valid_rate()))
            .sum();
        sum / self.entries.len() as f64
    }

    /// Snapshot of MeasuredCapabilities for Prometheus/API exposure.
    pub fn measured_snapshot(&self) -> Vec<(BackendId, MeasuredCapabilities)> {
        self.entries
            .iter()
            .filter_map(|(id, h)| {
                h.0.lock()
                    .ok()
                    .map(|g| (id.clone(), g.backend.measured.clone()))
            })
            .collect()
    }
}

/// Outcome of a single Dog evaluation, used to update organ health tracking.
#[derive(Debug, Clone, Copy)]
pub enum ScoreOutcome {
    Success,
    Failure(ScoreFailureKind),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organ::registry::{BackendHealth, DeclaredCapabilities, NodeId};

    fn make_backend(id: &str) -> Backend {
        Backend {
            id: BackendId(id.to_string()),
            node_id: NodeId("test-node".to_string()),
            endpoint: "http://localhost:8080/v1".to_string(),
            model: "test-model".to_string(),
            declared: DeclaredCapabilities::default(),
            measured: MeasuredCapabilities::default(),
            health: BackendHealth::Healthy,
            timeout_secs: 30,
            remediation: None,
        }
    }

    #[test]
    fn organ_starts_empty() {
        let organ = InferenceOrgan::boot_empty();
        assert_eq!(organ.backend_count(), 0);
    }

    #[test]
    fn register_increments_count() {
        let mut organ = InferenceOrgan::boot_empty();
        organ.register_backend(make_backend("dog-a"));
        organ.register_backend(make_backend("dog-b"));
        assert_eq!(organ.backend_count(), 2);
    }

    #[test]
    fn stats_start_pessimistic() {
        let mut organ = InferenceOrgan::boot_empty();
        organ.register_backend(make_backend("dog-a"));
        let stats = organ
            .stats_snapshot(&BackendId("dog-a".to_string()))
            .unwrap();
        assert_eq!(stats.json_valid_rate(), 0.0); // K14
        assert_eq!(stats.total_calls, 0);
    }

    #[test]
    fn update_success_increments_rate() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("dog-a"));
        InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success);
        InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success);
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ZeroFlood),
        );
        let stats = organ
            .stats_snapshot(&BackendId("dog-a".to_string()))
            .unwrap();
        assert_eq!(stats.total_calls, 3);
        assert!((stats.json_valid_rate() - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn gate_trips_and_degrades_backend() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("dog-a"));
        // 5 successes + 6 failures → gate trips at 6/11 > 50%
        for _ in 0..5 {
            InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success);
        }
        for _ in 0..6 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Failure(ScoreFailureKind::ZeroFlood),
            );
        }
        let guard = handle.0.lock().unwrap();
        assert!(
            matches!(guard.backend.health, BackendHealth::Degraded { .. }),
            "backend should be degraded after gate trips"
        );
    }

    #[test]
    fn overall_valid_rate_averages_backends() {
        let mut organ = InferenceOrgan::boot_empty();
        let h1 = organ.register_backend(make_backend("dog-a"));
        let h2 = organ.register_backend(make_backend("dog-b"));
        InferenceOrgan::update_stats_entry(&h1, ScoreOutcome::Success);
        InferenceOrgan::update_stats_entry(&h2, ScoreOutcome::Failure(ScoreFailureKind::Timeout));
        // dog-a: 1.0, dog-b: 0.0 → average 0.5
        let rate = organ.overall_valid_rate();
        assert!((rate - 0.5).abs() < 1e-10);
    }
}
