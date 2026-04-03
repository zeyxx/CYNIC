// organ/ — InferenceOrgan: owns backend registry, health tracking, and routing data.
//
// Phase 1: registry + health counters. main.rs still assembles Dogs; organ provides data.
// Phase 2: routing (profile → cluster → backend selection), Welford drift detection.
//
// Layer boundary (I1): organ imports from domain/ only. Never imports from dogs/ or infra/.
// main.rs (composition root) assembles InferenceDog from organ registry data.

pub mod health;
pub mod registry;

use crate::organ::health::{DogStats, ParseFailureGate, ScoreFailureKind};
use crate::organ::registry::{Backend, BackendHealth, BackendId};
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

impl BackendHandle {
    /// Returns true if this backend's quality gate has tripped.
    /// Acquires Mutex briefly, reads health, releases. No async, no hold across .await.
    /// K14: Mutex poison = assume degraded (safe default).
    pub fn is_quality_degraded(&self) -> bool {
        self.0.lock().ok().is_none_or(|guard| {
            matches!(
                guard.backend.health,
                BackendHealth::Degraded { .. } | BackendHealth::Dead { .. }
            )
        })
    }

    /// Snapshot DogStats from this handle. Returns None if Mutex is poisoned.
    pub fn stats_snapshot(&self) -> Option<DogStats> {
        self.0.lock().ok().map(|guard| guard.stats.clone())
    }
}

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

    /// Update stats for a backend after a Dog evaluation.
    /// Uses the pre-obtained handle to avoid a HashMap lookup on the hot path.
    pub fn update_stats_entry(handle: &BackendHandle, kind: ScoreOutcome) {
        let Ok(mut guard) = handle.0.lock() else {
            return;
        };
        match kind {
            ScoreOutcome::Success { elapsed_ms } => {
                guard.stats.record_success_with_latency(elapsed_ms);
                guard.gate.record_success();
                let rate = guard.stats.json_valid_rate();
                guard.backend.measured.json_valid_rate = rate;
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
                }
            }
        }
        // Recovery: if gate is no longer tripped and backend was degraded, promote to Healthy
        if !guard.gate.is_tripped()
            && matches!(guard.backend.health, BackendHealth::Degraded { .. })
        {
            tracing::info!(
                backend = %guard.backend.id.0,
                "organ: quality recovered — promoting to Healthy"
            );
            guard.backend.health = BackendHealth::Healthy;
        }
    }
}

/// Outcome of a single Dog evaluation, used to update organ health tracking.
#[derive(Debug, Clone, Copy)]
pub enum ScoreOutcome {
    Success { elapsed_ms: u64 },
    Failure(ScoreFailureKind),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organ::registry::{BackendHealth, DeclaredCapabilities, MeasuredCapabilities};

    fn make_backend(id: &str) -> Backend {
        Backend {
            id: BackendId(id.to_string()),
            declared: DeclaredCapabilities::default(),
            measured: MeasuredCapabilities::default(),
            health: BackendHealth::Healthy,
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
        let handle = organ.register_backend(make_backend("dog-a"));
        let stats = handle.stats_snapshot().unwrap();
        assert_eq!(stats.json_valid_rate(), 0.0); // K14
        assert_eq!(stats.total_calls, 0);
    }

    #[test]
    fn update_success_increments_rate() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("dog-a"));
        InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success { elapsed_ms: 0 });
        InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success { elapsed_ms: 0 });
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ZeroFlood),
        );
        let stats = handle.stats_snapshot().unwrap();
        assert_eq!(stats.total_calls, 3);
        assert!((stats.json_valid_rate() - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn healthy_backend_is_not_quality_degraded() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("dog-a"));
        assert!(!handle.is_quality_degraded());
    }

    #[test]
    fn degraded_backend_is_quality_degraded() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("dog-a"));
        // Trip the gate: 6 failures out of 10 (>50%)
        for _ in 0..4 {
            InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success { elapsed_ms: 0 });
        }
        for _ in 0..6 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Failure(ScoreFailureKind::ParseError),
            );
        }
        assert!(handle.is_quality_degraded());
    }

    #[test]
    fn recovery_promotes_degraded_to_healthy() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("dog-a"));
        // Trip the gate: fill window with failures
        for _ in 0..10 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Failure(ScoreFailureKind::ParseError),
            );
        }
        assert!(handle.is_quality_degraded());

        // Recover: 10 successes evict all failures from window
        for _ in 0..10 {
            InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success { elapsed_ms: 0 });
        }
        // Gate is no longer tripped → should recover to Healthy
        assert!(!handle.is_quality_degraded());
    }

    #[test]
    fn gate_trips_and_degrades_backend() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("dog-a"));
        // 5 successes + 6 failures → gate trips at 6/11 > 50%
        for _ in 0..5 {
            InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success { elapsed_ms: 0 });
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
}
