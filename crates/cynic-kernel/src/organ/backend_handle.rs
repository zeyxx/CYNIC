//! BackendEntry and BackendHandle for thread-safe concurrent stat recording.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::domain::dog_health::{DogStats, ScoreFailureKind, ScoreOutcome};
use crate::organ::health::ParseFailureGate;
use crate::organ::inference_organ::InferenceOrgan;
use crate::organ::registry::{Backend, BackendHealth};

/// Per-backend health tracking entry (private — exposed only via BackendHandle).
#[derive(Debug)]
pub struct BackendEntry {
    pub backend: Backend,
    pub stats: DogStats,
    pub gate: ParseFailureGate,
    /// Timestamp of last quality probe allowed for a degraded Dog.
    /// Mirrors CircuitBreaker HalfOpen pattern: at most one probe per TTL period.
    pub last_quality_probe: Option<Instant>,
    /// Shared handle to the Dog's dynamic completion budget.
    /// Updated here after each successful eval; read by InferenceDog at eval time.
    pub budget_handle: Option<Arc<AtomicU32>>,
    /// Shared handle to the Dog's max observed thinking tokens.
    /// Written by InferenceDog after every chat() response (including parse failures).
    /// Read by the organ during update_stats_entry to feed thinking-aware calibration.
    pub thinking_handle: Option<Arc<AtomicU32>>,
}

/// Opaque handle to a backend entry, obtained from `InferenceOrgan::register_backend()`.
/// Callers pass this to `InferenceOrgan::update_stats_entry()` to avoid a HashMap lookup.
#[derive(Debug, Clone)]
pub struct BackendHandle(pub(crate) Arc<Mutex<BackendEntry>>);

impl BackendHandle {
    pub fn new(backend: Backend) -> Self {
        Self(Arc::new(Mutex::new(BackendEntry {
            backend,
            stats: DogStats::new(),
            gate: ParseFailureGate::new(),
            last_quality_probe: None,
            budget_handle: None,
            thinking_handle: None,
        })))
    }

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

    /// Total calls this boot cycle (K22: 0 means no baseline to judge quality).
    pub fn total_calls(&self) -> u64 {
        self.0.lock().ok().map(|g| g.stats.total_calls).unwrap_or(0)
    }

    /// Allow one quality probe per TTL period for degraded Dogs.
    /// Mirrors CircuitBreaker HalfOpen: after TTL expires since degradation,
    /// allow one evaluation so `update_stats_entry → sync_parse_gate_health`
    /// can promote back to Healthy if the gate has cleared.
    pub fn should_allow_quality_probe(&self) -> bool {
        let Ok(mut guard) = self.0.lock() else {
            return false;
        };
        let BackendHealth::Degraded { since, .. } = &guard.backend.health else {
            return false;
        };
        if since.elapsed() <= ParseFailureGate::TTL_DURATION {
            return false;
        }
        if let Some(last_probe) = guard.last_quality_probe
            && last_probe.elapsed() <= ParseFailureGate::TTL_DURATION
        {
            return false;
        }
        guard.last_quality_probe = Some(Instant::now());
        true
    }

    /// Snapshot DogStats from this handle. Returns None if Mutex is poisoned.
    pub fn stats_snapshot(&self) -> Option<DogStats> {
        self.0.lock().ok().map(|guard| guard.stats.clone())
    }

    /// Record an evaluation outcome. Self-contained — caller doesn't need InferenceOrgan.
    pub fn record_outcome(&self, kind: ScoreOutcome) {
        let Ok(mut guard) = self.0.lock() else {
            return;
        };
        match kind {
            ScoreOutcome::Success {
                elapsed_ms,
                completion_tokens,
                thinking_tokens,
            } => {
                guard.stats.record_success_with_latency(elapsed_ms);
                guard
                    .stats
                    .record_completion_tokens(completion_tokens, thinking_tokens);
                guard.gate.record_success();
                if let (Some(budget_val), Some(bh)) =
                    (guard.stats.completion_budget(), &guard.budget_handle)
                {
                    bh.store(budget_val, Ordering::Relaxed);
                }
            }
            ScoreOutcome::Failure(failure_kind) => {
                guard.stats.record_failure(failure_kind);
                match failure_kind {
                    ScoreFailureKind::Timeout | ScoreFailureKind::ApiError => {}
                    _ => guard.gate.record_failure(),
                }
            }
        }

        // Read the Dog's observed thinking_max
        if let Some(th) = &guard.thinking_handle {
            let observed_thinking = th.load(Ordering::Relaxed);
            if observed_thinking > guard.stats.max_thinking_tokens {
                guard.stats.max_thinking_tokens = observed_thinking;
            }
        }
        // Re-push calibrated budget
        if let (Some(budget_val), Some(bh)) =
            (guard.stats.completion_budget(), &guard.budget_handle)
        {
            bh.store(budget_val, Ordering::Relaxed);
        }

        InferenceOrgan::sync_parse_gate_health(&mut guard);
    }
}
