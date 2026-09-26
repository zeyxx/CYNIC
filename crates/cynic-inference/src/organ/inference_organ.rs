//! InferenceOrgan — owns the registry of Dog backends and tracks their health.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use crate::organ::backend_handle::{BackendEntry, BackendHandle};
use crate::organ::registry::{Backend, BackendHealth, BackendId};
use cynic_eval::eval::dog_health::{DogStats, ScoreOutcome};

/// InferenceOrgan — owns the registry of Dog backends and tracks their health.
#[derive(Debug)]
pub struct InferenceOrgan {
    pub(crate) entries: HashMap<BackendId, BackendHandle>,
}

impl InferenceOrgan {
    pub(crate) fn is_parse_gate_degraded(health: &BackendHealth) -> bool {
        matches!(
            health,
            BackendHealth::Degraded { reason, .. }
                if reason.starts_with("parse failure rate ")
        )
    }

    pub(crate) fn sync_parse_gate_health(guard: &mut BackendEntry) {
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

        // Promote to Healthy if both gates clear
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
        let handle = BackendHandle::new(backend);
        self.entries.insert(id, handle.clone());
        handle
    }

    /// Number of registered backends.
    pub fn backend_count(&self) -> usize {
        self.entries.len()
    }

    /// Attach a Dog's budget handle so the organ can push calibrated budgets.
    pub fn attach_budget_handle(handle: &BackendHandle, budget: Arc<AtomicU32>) {
        if let Ok(mut guard) = handle.0.lock() {
            if let Some(val) = guard.stats.completion_budget() {
                budget.store(val, Ordering::Relaxed);
            }
            guard.budget_handle = Some(budget);
        }
    }

    /// Attach the Dog's thinking_max handle so the organ can read observed thinking overhead.
    pub fn attach_thinking_handle(handle: &BackendHandle, thinking: Arc<AtomicU32>) {
        if let Ok(mut guard) = handle.0.lock() {
            guard.thinking_handle = Some(thinking);
        }
    }

    /// Update stats for a backend after a Dog evaluation.
    pub fn update_stats_entry(handle: &BackendHandle, kind: ScoreOutcome) {
        handle.record_outcome(kind);
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
    pub fn promote_if_gate_clear(&self, dog_id: &str) {
        let key = BackendId(dog_id.to_string());
        if let Some(handle) = self.entries.get(&key)
            && let Ok(mut guard) = handle.0.lock()
            && matches!(guard.backend.health, BackendHealth::Degraded { .. })
            && !guard.gate.is_tripped()
        {
            if guard.stats.is_baseline_established() && guard.stats.json_valid_rate() < 0.5 {
                tracing::debug!(
                    backend = %dog_id,
                    json_valid_rate = format!("{:.1}%", guard.stats.json_valid_rate() * 100.0),
                    "promote blocked — json_valid_rate below 50% threshold"
                );
                return;
            }
            tracing::info!(
                backend = %dog_id,
                "organ: fleet signal clear + gate clear — promoting to Healthy"
            );
            guard.backend.health = BackendHealth::Healthy;
        }
    }

    /// Restore persisted DogStats at boot. Merges into existing entries by dog_id.
    pub fn restore_stats(&self, loaded: &[(String, DogStats)]) {
        for (dog_id, stats) in loaded {
            let key = BackendId(dog_id.clone());
            if let Some(handle) = self.entries.get(&key)
                && let Ok(mut guard) = handle.0.lock()
            {
                let preserved_budget = stats.completion_budget();

                guard.stats.max_content_tokens = stats.max_content_tokens;
                guard.stats.max_thinking_tokens = stats.max_thinking_tokens;
                guard.stats.last_success = stats.last_success;

                if let (Some(budget_val), Some(bh)) = (preserved_budget, &guard.budget_handle) {
                    bh.store(budget_val, Ordering::Relaxed);
                }
                tracing::info!(
                    backend = %dog_id,
                    historical_calls = stats.total_calls,
                    historical_success_rate = format!("{:.1}%", (stats.success_count as f64 / (stats.total_calls.max(1) as f64)) * 100.0),
                    budget = preserved_budget.unwrap_or(0),
                    "organ: restored calibration (quality counters reset at boot)"
                );
            }
        }
    }
}
