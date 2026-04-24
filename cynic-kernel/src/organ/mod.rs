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
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Per-backend health tracking entry (private — exposed only via BackendHandle).
#[derive(Debug)]
struct BackendEntry {
    backend: Backend,
    stats: DogStats,
    gate: ParseFailureGate,
    /// Timestamp of last quality probe allowed for a degraded Dog.
    /// Mirrors CircuitBreaker HalfOpen pattern: at most one probe per TTL period.
    last_quality_probe: Option<Instant>,
    /// Shared handle to the Dog's dynamic completion budget.
    /// Updated here after each successful eval; read by InferenceDog at eval time.
    budget_handle: Option<Arc<AtomicU32>>,
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

    /// Allow one quality probe per TTL period for degraded Dogs.
    /// Mirrors CircuitBreaker HalfOpen: after TTL expires since degradation,
    /// allow one evaluation so `update_stats_entry → sync_parse_gate_health`
    /// can promote back to Healthy if the gate has cleared.
    ///
    /// Without this, degraded Dogs are excluded from /judge forever — a deadlock
    /// where recovery requires participation but participation requires recovery.
    pub fn should_allow_quality_probe(&self) -> bool {
        let Ok(mut guard) = self.0.lock() else {
            return false; // K14: poison = no probe
        };
        let BackendHealth::Degraded { since, .. } = &guard.backend.health else {
            return false; // not degraded — no probe needed
        };
        if since.elapsed() <= ParseFailureGate::TTL_DURATION {
            return false; // degradation too recent
        }
        if let Some(last_probe) = guard.last_quality_probe
            && last_probe.elapsed() <= ParseFailureGate::TTL_DURATION
        {
            return false; // already probed within TTL window
        }
        guard.last_quality_probe = Some(Instant::now());
        true
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
    fn is_parse_gate_degraded(health: &BackendHealth) -> bool {
        matches!(
            health,
            BackendHealth::Degraded { reason, .. }
                if reason.starts_with("parse failure rate ")
        )
    }

    fn sync_parse_gate_health(guard: &mut BackendEntry) {
        let rate = guard.stats.json_valid_rate();
        guard.backend.measured.json_valid_rate = rate;

        // K14 gate 1: ParseFailureGate (sliding 10-call window, >50% failures)
        if guard.gate.is_tripped() {
            if matches!(guard.backend.health, BackendHealth::Healthy) {
                guard.backend.health = BackendHealth::Degraded {
                    reason: format!(
                        "parse failure rate {:.0}% > 50% in last 10 calls",
                        guard.gate.failure_rate() * 100.0
                    ),
                    since: Instant::now(),
                };
            }
            return;
        }

        // K14 gate 2: json_valid_rate >= 0.5 (after baseline established, >=20 calls)
        // Prevents Dogs with global low JSON validity from poisoning jury.
        let is_json_rate_degraded = guard.stats.is_baseline_established() && rate < 0.5;
        if is_json_rate_degraded {
            if matches!(guard.backend.health, BackendHealth::Healthy) {
                guard.backend.health = BackendHealth::Degraded {
                    reason: format!("json valid rate {:.1}% < 50%", rate * 100.0),
                    since: Instant::now(),
                };
            }
            return;
        }

        // Promote to Healthy if both gates clear (parse + json_valid_rate)
        if Self::is_parse_gate_degraded(&guard.backend.health)
            || (matches!(
                &guard.backend.health,
                BackendHealth::Degraded { reason, .. }
                    if reason.contains("json valid rate")
            ) && !is_json_rate_degraded)
        {
            tracing::info!(
                backend = %guard.backend.id.0,
                "organ: quality recovered — promoting to Healthy"
            );
            guard.backend.health = BackendHealth::Healthy;
        }
    }

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
            last_quality_probe: None,
            budget_handle: None,
        })));
        self.entries.insert(id, handle.clone());
        handle
    }

    /// Number of registered backends.
    pub fn backend_count(&self) -> usize {
        self.entries.len()
    }

    /// Attach a Dog's budget handle so the organ can push calibrated budgets.
    /// Called once after Dog construction, before any evaluations.
    /// If stats were already restored (restore_stats ran first), push the
    /// calibrated budget immediately so the Dog doesn't fall back to bootstrap.
    pub fn attach_budget_handle(handle: &BackendHandle, budget: Arc<AtomicU32>) {
        if let Ok(mut guard) = handle.0.lock() {
            if let Some(val) = guard.stats.completion_budget() {
                budget.store(val, Ordering::Relaxed);
            }
            guard.budget_handle = Some(budget);
        }
    }

    /// Update stats for a backend after a Dog evaluation.
    /// Uses the pre-obtained handle to avoid a HashMap lookup on the hot path.
    pub fn update_stats_entry(handle: &BackendHandle, kind: ScoreOutcome) {
        let Ok(mut guard) = handle.0.lock() else {
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
                // Push calibrated budget to the Dog's AtomicU32 if available.
                if let (Some(budget_val), Some(bh)) =
                    (guard.stats.completion_budget(), &guard.budget_handle)
                {
                    bh.store(budget_val, Ordering::Relaxed);
                }
            }
            ScoreOutcome::Failure(failure_kind) => {
                guard.stats.record_failure(failure_kind);
                // Only model-quality failures (parse/zero/collapse) feed the ParseFailureGate.
                // Infra failures (timeout/api_error) are tracked in DogStats but don't degrade
                // model quality assessment — the model may be fine, just unreachable.
                match failure_kind {
                    ScoreFailureKind::Timeout | ScoreFailureKind::ApiError => {}
                    _ => guard.gate.record_failure(),
                }
            }
        }

        Self::sync_parse_gate_health(&mut guard);
    }

    /// Snapshot all DogStats for persistence. Returns (dog_id, stats) pairs.
    pub fn snapshot_stats(&self) -> Vec<(String, DogStats)> {
        self.entries
            .iter()
            .filter_map(|(id, handle)| {
                let guard = handle.0.lock().ok()?;
                Some((id.0.clone(), guard.stats.clone()))
            })
            .collect()
    }

    /// Degrade a backend due to external signal (e.g., FleetProbe model mismatch).
    /// Idempotent — no-op if already Degraded or Dead, or if dog_id not found.
    /// K14: unknown = degraded (safe default).
    pub fn degrade_backend(&self, dog_id: &str, reason: String) {
        let key = BackendId(dog_id.to_string());
        if let Some(handle) = self.entries.get(&key)
            && let Ok(mut guard) = handle.0.lock()
            && !matches!(
                guard.backend.health,
                BackendHealth::Degraded { .. } | BackendHealth::Dead { .. }
            )
        {
            tracing::warn!(
                backend = %dog_id,
                reason = %reason,
                "organ: degrading backend — fleet probe signal"
            );
            guard.backend.health = BackendHealth::Degraded {
                reason,
                since: Instant::now(),
            };
        }
    }

    /// Promote a Degraded backend to Healthy if the ParseFailureGate is also clear.
    /// Called when the external signal (e.g., model mismatch) is resolved.
    /// No-op if: not found, not Degraded, or gate still tripped.
    pub fn promote_if_gate_clear(&self, dog_id: &str) {
        let key = BackendId(dog_id.to_string());
        if let Some(handle) = self.entries.get(&key)
            && let Ok(mut guard) = handle.0.lock()
            && matches!(guard.backend.health, BackendHealth::Degraded { .. })
            && !guard.gate.is_tripped()
        {
            tracing::info!(
                backend = %dog_id,
                "organ: fleet signal clear + gate clear — promoting to Healthy"
            );
            guard.backend.health = BackendHealth::Healthy;
        }
    }

    /// Restore persisted DogStats at boot. Merges into existing entries by dog_id.
    /// Dogs not found in loaded data keep their K14-pessimistic defaults.
    pub fn restore_stats(&self, loaded: &[(String, DogStats)]) {
        for (dog_id, stats) in loaded {
            let key = BackendId(dog_id.clone());
            if let Some(handle) = self.entries.get(&key)
                && let Ok(mut guard) = handle.0.lock()
            {
                guard.stats = stats.clone();
                // Push calibrated budget to Dog's AtomicU32 so it doesn't fall back
                // to the bootstrap formula (which ignores MIN_COMPLETION_BUDGET).
                if let (Some(budget_val), Some(bh)) =
                    (guard.stats.completion_budget(), &guard.budget_handle)
                {
                    bh.store(budget_val, Ordering::Relaxed);
                }
                // NOTE: Do NOT replay gate state at boot. K14 penalty: we marked Dogs as
                // quality_degraded before they had a chance to recover, which blocked all
                // /judge requests on first boot after a failure. Instead, let Dogs rebuild
                // their gate reputation naturally from requests after boot. Stats are restored
                // for observability/history, but gates start fresh each boot (B5 amnesia fix).
                tracing::info!(
                    backend = %dog_id,
                    total_calls = stats.total_calls,
                    success_rate = format!("{:.1}%", (stats.success_count as f64 / (stats.total_calls.max(1) as f64)) * 100.0),
                    budget = guard.stats.completion_budget().unwrap_or(0),
                    "organ: restored DogStats (gates reset — no replay at boot)"
                );
            }
        }
    }
}

/// Outcome of a single Dog evaluation, used to update organ health tracking.
#[derive(Debug, Clone, Copy)]
pub enum ScoreOutcome {
    Success {
        elapsed_ms: u64,
        completion_tokens: u32,
        thinking_tokens: u32,
    },
    Failure(ScoreFailureKind),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organ::registry::{BackendHealth, DeclaredCapabilities, MeasuredCapabilities};
    use std::time::Duration;

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
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 0,
                completion_tokens: 0,
                thinking_tokens: 0,
            },
        );
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 0,
                completion_tokens: 0,
                thinking_tokens: 0,
            },
        );
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
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 0,
                    completion_tokens: 0,
                    thinking_tokens: 0,
                },
            );
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
    fn quality_probe_allowed_after_ttl_expired() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("latched-dog"));
        // Trip the gate: 10 failures → degraded
        for _ in 0..10 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Failure(ScoreFailureKind::ParseError),
            );
        }
        assert!(
            handle.is_quality_degraded(),
            "precondition: must be degraded"
        );

        // Before TTL: no probe allowed
        assert!(
            !handle.should_allow_quality_probe(),
            "probe must be denied before TTL expires"
        );

        // Simulate TTL expiry by backdating the degradation timestamp
        {
            let mut guard = handle.0.lock().unwrap();
            guard.backend.health = BackendHealth::Degraded {
                reason: "parse failure rate 80% > 50% in last 10 calls".into(),
                since: Instant::now() - Duration::from_secs(31),
            };
        }

        // After TTL: first probe allowed
        assert!(
            handle.should_allow_quality_probe(),
            "probe must be allowed after TTL expires"
        );
        // Second probe immediately: denied (rate-limited)
        assert!(
            !handle.should_allow_quality_probe(),
            "second probe within TTL must be denied"
        );
    }

    #[test]
    fn quality_probe_success_recovers_dog() {
        // Full latch-recovery cycle: degrade → TTL expires → probe succeeds → Healthy
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("recovering-latched"));
        // Trip: 10 failures
        for _ in 0..10 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Failure(ScoreFailureKind::ParseError),
            );
        }
        assert!(handle.is_quality_degraded());

        // Simulate TTL expiry + clear the gate window with successes
        {
            let mut guard = handle.0.lock().unwrap();
            guard.backend.health = BackendHealth::Degraded {
                reason: "parse failure rate 80% > 50% in last 10 calls".into(),
                since: Instant::now() - Duration::from_secs(31),
            };
        }
        // Probe allowed — simulate what the judge does: evaluate + update_stats_entry
        assert!(handle.should_allow_quality_probe());
        // 10 successes push failures out of the 10-call sliding window
        for _ in 0..10 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 100,
                    completion_tokens: 200,
                    thinking_tokens: 0,
                },
            );
        }
        // Dog should be healthy now — gate cleared, sync_parse_gate_health promoted
        assert!(
            !handle.is_quality_degraded(),
            "Dog must recover after successful probe clears the gate window"
        );
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
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 0,
                    completion_tokens: 0,
                    thinking_tokens: 0,
                },
            );
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
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 0,
                    completion_tokens: 0,
                    thinking_tokens: 0,
                },
            );
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
    fn restore_stats_excludes_api_errors_from_gate() {
        // F1 fix: a Dog with 8 api_errors (e.g. 401 from wrong auth) and 0 success
        // must NOT have its gate tripped after restore. ApiError is infra, not model quality.
        let mut organ = InferenceOrgan::boot_empty();
        let _handle = organ.register_backend(make_backend("gemma-4b-core"));

        let stats = DogStats {
            total_calls: 8,
            success_count: 0,
            zero_flood_count: 0,
            collapse_count: 0,
            parse_error_count: 0,
            timeout_count: 0,
            api_error_count: 8,
            last_success: None,
            total_latency_ms: 0,
            total_completion_tokens: 0,
            max_completion_tokens: 0,
            max_content_tokens: 0,
            max_thinking_tokens: 0,
        };
        organ.restore_stats(&[("gemma-4b-core".to_string(), stats)]);

        let handle = organ
            .entries
            .get(&BackendId("gemma-4b-core".to_string()))
            .unwrap();
        // Gate should be empty (0 gate-relevant calls), backend stays Healthy
        assert!(
            !handle.is_quality_degraded(),
            "api_errors must not trip the gate"
        );
        let guard = handle.0.lock().unwrap();
        assert!(
            !guard.gate.is_tripped(),
            "gate must not be tripped from api_errors alone"
        );
    }

    #[test]
    fn json_valid_rate_gate_excludes_low_quality_dogs() {
        // K14 gate 2: Dogs with json_valid_rate < 0.5 after baseline (20+ calls) are degraded.
        // Use ApiError (infrastructure) instead of ParseError (model quality) so the
        // ParseFailureGate doesn't trip. This isolates the json_valid_rate gate test.
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("low-quality-dog"));

        // Accumulate 25 calls: 5 successes (20% valid rate), 20 infra failures (ApiError).
        // ApiError doesn't feed ParseFailureGate, so it won't degrade via parse gate.
        for _ in 0..5 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 0,
                    completion_tokens: 0,
                    thinking_tokens: 0,
                },
            );
        }
        for _ in 0..20 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Failure(ScoreFailureKind::ApiError),
            );
        }

        // Verify stats
        let stats = handle.stats_snapshot().unwrap();
        assert_eq!(stats.total_calls, 25);
        assert_eq!(stats.success_count, 5);
        assert_eq!(stats.json_valid_rate(), 0.2);

        // Verify json_valid_rate gate degraded the Dog (not parse gate)
        let guard = handle.0.lock().unwrap();
        assert!(matches!(
            &guard.backend.health,
            BackendHealth::Degraded { reason, .. }
                if reason.contains("json valid rate")
        ));
    }

    #[test]
    fn json_valid_rate_gate_requires_baseline() {
        // K14 gate: json_valid_rate gate only trips after baseline (20 calls).
        // Before baseline, pessimistic K14 defaults are used (0% assumed healthy).
        // Use ApiError so ParseFailureGate doesn't trip independently.
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("new-dog"));

        // 10 calls: 2 successes (20% — below 50%), 8 infra failures (ApiError).
        // ApiError doesn't feed ParseFailureGate, so only json_valid_rate gate matters.
        for _ in 0..2 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 0,
                    completion_tokens: 0,
                    thinking_tokens: 0,
                },
            );
        }
        for _ in 0..8 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Failure(ScoreFailureKind::ApiError),
            );
        }

        // Below baseline (10 < 20 calls): should still be Healthy
        let guard = handle.0.lock().unwrap();
        assert!(matches!(guard.backend.health, BackendHealth::Healthy));
    }

    #[test]
    fn json_valid_rate_gate_recovery() {
        // When json_valid_rate improves to >= 0.5, Dog recovers to Healthy.
        // Use ApiError so only json_valid_rate gate is being tested, not ParseFailureGate.
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("recovering-dog"));

        // 20 calls: 5 successes (25% — below gate), 15 ApiErrors (infrastructure, no parse gate impact)
        for _ in 0..5 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 0,
                    completion_tokens: 0,
                    thinking_tokens: 0,
                },
            );
        }
        for _ in 0..15 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Failure(ScoreFailureKind::ApiError),
            );
        }

        let guard = handle.0.lock().unwrap();
        assert!(matches!(
            &guard.backend.health,
            BackendHealth::Degraded { reason, .. }
                if reason.contains("json valid rate")
        ));
        drop(guard);

        // Add 30 successes (now 35/50 = 70% — above gate)
        for _ in 0..30 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 0,
                    completion_tokens: 0,
                    thinking_tokens: 0,
                },
            );
        }

        // Should recover to Healthy (json_valid_rate improved from 25% to 70%)
        let guard = handle.0.lock().unwrap();
        assert!(matches!(guard.backend.health, BackendHealth::Healthy));
    }

    #[test]
    fn restore_stats_does_not_replay_gates_at_boot() {
        // B5 amnesia fix: gates don't replay at boot. A Dog with model-quality failures
        // in its persisted stats should have those stats loaded but gate stays fresh.
        // This prevents blocking /judge on first boot after any previous failure.
        let mut organ = InferenceOrgan::boot_empty();
        let _handle = organ.register_backend(make_backend("reviving-dog"));

        let stats = DogStats {
            total_calls: 10,
            success_count: 2,
            zero_flood_count: 3,
            collapse_count: 0,
            parse_error_count: 5,
            timeout_count: 0,
            api_error_count: 0,
            last_success: None,
            total_latency_ms: 0,
            total_completion_tokens: 0,
            max_completion_tokens: 0,
            max_content_tokens: 0,
            max_thinking_tokens: 0,
        };
        organ.restore_stats(&[("reviving-dog".to_string(), stats)]);

        let handle = organ
            .entries
            .get(&BackendId("reviving-dog".to_string()))
            .unwrap();

        // Stats are loaded (for observability)
        let guard = handle.0.lock().unwrap();
        assert_eq!(guard.stats.success_count, 2, "stats should be restored");

        // But gate stays empty — doesn't replay failures
        assert!(
            !guard.gate.is_tripped(),
            "gate must NOT replay at boot — Dogs rebuild reputation from requests"
        );
    }

    // ── Dynamic budget tests ──────────────────────────────────

    #[test]
    fn completion_budget_not_available_before_baseline() {
        let stats = DogStats::new();
        assert!(
            stats.completion_budget().is_none(),
            "no budget before 20 calls"
        );
    }

    #[test]
    fn completion_budget_derived_from_observed_max() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("budget-dog"));

        // Accumulate 25 calls with varying completion tokens
        for ct in [
            150, 200, 180, 220, 195, 210, 175, 190, 205, 185, 200, 210, 195, 180, 220, 200, 190,
            205, 185, 200, 210, 195, 180, 190, 250,
        ] {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 100,
                    completion_tokens: ct,
                    thinking_tokens: 0,
                },
            );
        }

        let stats = handle.stats_snapshot().unwrap();
        assert!(stats.is_baseline_established());
        assert_eq!(stats.max_completion_tokens, 250);

        let budget = stats.completion_budget().unwrap();
        // content_budget = 250 * 1.2 = 300, thinking_budget = 0
        // floor = MIN_COMPLETION_BUDGET = 512
        assert_eq!(
            budget, 512,
            "budget = max(content*1.2, MIN_COMPLETION_BUDGET)"
        );
    }

    #[test]
    fn tok_per_sec_computed_from_history() {
        let mut stats = DogStats::new();
        // Simulate 10 calls: 200 tokens each, 1000ms each → 2000 tok / 10s = 200 tok/s
        for _ in 0..10 {
            stats.record_success_with_latency(1000);
            stats.record_completion_tokens(200, 0);
        }

        let tps = stats.tok_per_sec().unwrap();
        assert!((tps - 200.0).abs() < 1.0, "expected ~200 tok/s, got {tps}");
    }

    #[test]
    fn scoring_with_budget_overrides_default() {
        use crate::domain::chat::InferenceProfile;
        // Default scoring
        assert_eq!(InferenceProfile::SCORING.max_tokens(), 1024);
        // Dynamic budget
        let profile = InferenceProfile::scoring_with_budget(6000);
        assert_eq!(profile.max_tokens(), 6000);
    }

    #[test]
    fn thinking_budget_accounts_for_thinking_overhead() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("gemma-thinking"));

        // Simulate Gemma: 400 combined, 370 thinking + 30 content
        for _ in 0..25 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 5000,
                    completion_tokens: 400,
                    thinking_tokens: 370,
                },
            );
        }

        let stats = handle.stats_snapshot().unwrap();
        assert!(stats.is_baseline_established());
        assert_eq!(stats.max_thinking_tokens, 370);
        assert_eq!(stats.max_content_tokens, 30);

        let budget = stats.completion_budget().unwrap();
        // content_budget = ceil(30 * 1.2) = 36
        // thinking_budget = ceil(370 * 1.5) = 555
        // content_budget = ceil(30*1.2) = 36, thinking_budget = ceil(370*1.5) = 555
        // sum = 591 > floor 512 → 591
        assert!(
            budget >= 512,
            "budget must be at least MIN_COMPLETION_BUDGET, got {budget}"
        );
        assert_eq!(budget, 591, "budget = content*1.2 + thinking*1.5");
        assert!(
            budget > 370,
            "budget must exceed thinking overhead to leave room for content"
        );
    }

    #[test]
    fn non_thinking_model_budget_covers_content() {
        let mut organ = InferenceOrgan::boot_empty();
        let handle = organ.register_backend(make_backend("qwen-no-think"));

        // Non-thinking model: 300 content, 0 thinking
        for _ in 0..25 {
            InferenceOrgan::update_stats_entry(
                &handle,
                ScoreOutcome::Success {
                    elapsed_ms: 200,
                    completion_tokens: 300,
                    thinking_tokens: 0,
                },
            );
        }

        let stats = handle.stats_snapshot().unwrap();
        assert_eq!(stats.max_content_tokens, 300);
        assert_eq!(stats.max_thinking_tokens, 0);

        let budget = stats.completion_budget().unwrap();
        // content_budget = ceil(300 * 1.2) = 360, thinking = 0
        // floor = 512 → max(360, 512) = 512
        assert!(budget >= 360, "budget must cover content headroom");
    }

    #[test]
    fn min_completion_budget_floor_prevents_collapse() {
        use crate::domain::constants::MIN_COMPLETION_BUDGET;
        let mut stats = DogStats::new();
        // Worst case: 5 content tokens, 395 thinking (Gemma truncation)
        for _ in 0..25 {
            stats.record_success_with_latency(1000);
            stats.record_completion_tokens(400, 395);
        }

        let budget = stats.completion_budget().unwrap();
        assert!(
            budget >= MIN_COMPLETION_BUDGET,
            "budget must never fall below floor, got {budget}"
        );
    }
}
