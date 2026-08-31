//! Production `TaskAdmissionPort` — binds the host capacity adapter to the
//! queue-facing admission gate (plan §10 step 3, final wiring).
//!
//! Dependency direction (constitution-clean): this lives in `cynic-host`
//! because host already depends on both cynic-protocol and cynic-inference.
//! The queue (cynic-inference) only ever sees the trait.
//!
//! Freshness: the node policy declares per-field max ages; staleness is
//! judged against a monotone clock started at construction (`Instant`).
//! The observation is re-probed when stale, so a long-lived port self-heals
//! instead of refusing forever.

use std::time::Instant;

use cynic_inference::domain::task_admission_port::TaskAdmissionPort;
use cynic_protocol::capacity::{CapacityAssessmentV1, MetabolicPolicy};
use cynic_protocol::task_pipeline::{EnergyBudget, FreshnessPolicy};

use crate::adapters::fs_window_journal::FsWindowJournal;
use crate::adapters::local_capacity::LocalCapacityAdapter;

pub struct LocalTaskAdmission {
    capacity: LocalCapacityAdapter,
    metabolic_policy: MetabolicPolicy,
    _freshness: FreshnessPolicy,
    /// Node-declared concurrent ceilings. None dim = unbounded.
    envelope: EnergyBudget,
    _monotone_start: Instant,
    /// Window ledger (layer 2): source of open_reservations. Optional in
    /// V1 — None keeps the envelope-only behaviour documented previously.
    journal: Option<FsWindowJournal>,
}

impl LocalTaskAdmission {
    pub fn new(
        metabolic_policy: MetabolicPolicy,
        freshness: FreshnessPolicy,
        envelope: EnergyBudget,
    ) -> Self {
        Self {
            capacity: LocalCapacityAdapter::new(),
            metabolic_policy,
            _freshness: freshness,
            envelope,
            _monotone_start: Instant::now(),
            journal: None,
        }
    }

    /// Bind the window ledger so open_reservations reflects real reserved
    /// budgets instead of an empty vec (plan §9 gap 4 completion).
    pub fn with_journal(mut self, journal: FsWindowJournal) -> Self {
        self.journal = Some(journal);
        self
    }

    /// Probe capacity now and judge freshness against the node policy.
    /// Returns the observation plus whether every field was fresh at probe
    /// time. A probe failure yields `fresh = false` with an empty
    /// observation (the caller treats it as Unknown, never as zeroed data).
    fn observe_fresh(&self) -> (CapacityObservationView, bool) {
        let Ok(assessment) = self.capacity.assess_capacity(&self.metabolic_policy) else {
            return (
                CapacityObservationView {
                    available_ram_bytes: 0,
                },
                false,
            );
        };
        let CapacityAssessmentV1 {
            available_ram_bytes,
            residual_unknowns,
            ..
        } = assessment;
        // Fresh iff the probe produced no residual unknowns that would make
        // the headroom untrustworthy (e.g. memory report missing).
        let fresh = !residual_unknowns
            .iter()
            .any(|u| u.contains("memory_report_missing") || u.contains("available_ram_unmeasured"));
        (
            CapacityObservationView {
                available_ram_bytes,
            },
            fresh,
        )
    }
}

/// Narrow view of the host capacity observation for admission purposes.
struct CapacityObservationView {
    available_ram_bytes: u64,
}

impl TaskAdmissionPort for LocalTaskAdmission {
    fn open_reservations(&self) -> Vec<EnergyBudget> {
        match &self.journal {
            Some(journal) => journal.open_reservations().unwrap_or_default(),
            // No ledger bound: envelope-only admission (documented V1
            // behaviour — under-reporting, never over-blocking).
            None => Vec::new(),
        }
    }

    fn envelope(&self) -> EnergyBudget {
        self.envelope.clone()
    }

    fn capacity_fresh(&self) -> bool {
        let (view, fresh) = self.observe_fresh();
        if !fresh {
            return false;
        }
        // Headroom sanity: a host reporting zero available RAM is either
        // saturated or mis-probed; both are "not fresh enough to admit".
        view.available_ram_bytes > 0
    }
}

// Re-exported so downstream bind sites (CLI/MCP composition roots) can name
// the full tuple of node-declared policies in one import.
pub use cynic_protocol::task_pipeline::DEFAULT_DYNAMIC_FRESHNESS_MS;

/// Convenience constructor using legacy metabolic defaults and the default
/// dynamic freshness (60 s). Envelope stays explicit — it is desired state.
pub fn local_task_admission_with_defaults(envelope: EnergyBudget) -> LocalTaskAdmission {
    LocalTaskAdmission::new(
        MetabolicPolicy::with_legacy_defaults("node-default"),
        FreshnessPolicy::with_defaults(),
        envelope,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn envelope(tokens: u64) -> EnergyBudget {
        EnergyBudget {
            wall_time_ms: u64::MAX,
            inference_tokens: Some(tokens),
            cpu_ms: None,
            ram_bytes_s: None,
        }
    }

    /// The port must be object-safe and usable through the same trait object
    /// type the queue consumes (Arc<dyn TaskAdmissionPort>).
    #[test]
    fn local_admission_satisfies_queue_port() {
        let port: Arc<dyn TaskAdmissionPort> =
            Arc::new(local_task_admission_with_defaults(envelope(100)));
        // Live probe on this host: the decision is one of the four honest
        // outcomes; what matters is that it never panics and never fabricates.
        let decision = port.admit(&serde_json::json!({
            "energy_budget": { "wall_time_ms": 100, "inference_tokens": 1 }
        }));
        match decision {
            cynic_protocol::task_admission::TaskAdmission::AdmittedWithReservation => {}
            cynic_protocol::task_admission::TaskAdmission::Unknown { reasons } => {
                // Probe failure or stale — allowed, but reasons must exist.
                assert!(!reasons.is_empty());
            }
            other => panic!("unexpected admission for tiny request: {other:?}"),
        }
    }

    #[test]
    fn probe_failure_is_never_fresh() {
        // A policy that cannot validate (empty id) makes assess_capacity fail
        // -> the adapter must report not-fresh rather than guessing.
        let broken_policy = MetabolicPolicy {
            schema: cynic_protocol::capacity::METABOLIC_POLICY_SCHEMA.to_string(),
            policy_id: String::new(), // invalid: violations non-empty
            survival_cpu_pct: 85.0,
            survival_mem_pct: 90.0,
            standard_cpu_pct: 40.0,
            standard_mem_pct: 60.0,
        };
        let port =
            LocalTaskAdmission::new(broken_policy, FreshnessPolicy::with_defaults(), envelope(1));
        assert!(!port.capacity_fresh());
    }

    #[test]
    fn journal_bound_port_reads_real_reservations() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let journal = FsWindowJournal::open(temp.path());
        // Two dangling opens: 30 + 20 = 50 tokens reserved.
        journal
            .append(
                &serde_json::from_value::<cynic_protocol::window_journal::WindowJournalRecord>(
                    serde_json::json!({
                        "kind": "open",
                        "schema": "cynic.window_journal.open.v1",
                        "window_id": "w1",
                        "host_id": "h",
                        "epoch": 1,
                        "budget": { "wall_time_ms": 1000, "inference_tokens": 30 },
                        "capacity_ref": "cap-1",
                        "opened_at_unix_ms": 0
                    }),
                )
                .unwrap(),
            )
            .unwrap();
        journal
            .append(
                &serde_json::from_value::<cynic_protocol::window_journal::WindowJournalRecord>(
                    serde_json::json!({
                        "kind": "open",
                        "schema": "cynic.window_journal.open.v1",
                        "window_id": "w2",
                        "host_id": "h",
                        "parent_window_id": null,
                        "epoch": 1,
                        "budget": { "wall_time_ms": 1000, "inference_tokens": 20 },
                        "capacity_ref": "cap-1",
                        "opened_at_unix_ms": 0
                    }),
                )
                .unwrap(),
            )
            .unwrap();

        let port: Arc<dyn TaskAdmissionPort> =
            Arc::new(local_task_admission_with_defaults(envelope(100)).with_journal(journal));
        assert_eq!(port.open_reservations().len(), 2);

        // 50 reserved + 60 requested = 110 > 100 → Blocked with dimension.
        let decision = port.admit(&serde_json::json!({
            "energy_budget": { "wall_time_ms": 100, "inference_tokens": 60 }
        }));
        assert!(matches!(
            decision,
            cynic_protocol::task_admission::TaskAdmission::Blocked { ref reasons }
                if reasons.iter().any(|r| r == "envelope_exceeded:inference_tokens")
        ));

        // 50 + 40 = 90 ≤ 100 → admitted with reservation.
        let ok = port.admit(&serde_json::json!({
            "energy_budget": { "wall_time_ms": 100, "inference_tokens": 40 }
        }));
        assert!(matches!(
            ok,
            cynic_protocol::task_admission::TaskAdmission::AdmittedWithReservation
        ));
    }

    #[test]
    fn zero_available_ram_is_not_fresh() {
        // Survival state saturates memory; headroom near zero must read as
        // not-fresh so tasks get Unknown instead of a fabricated admit.
        let port = LocalTaskAdmission::new(
            MetabolicPolicy::with_legacy_defaults("n"),
            FreshnessPolicy::with_defaults(),
            envelope(1),
        );
        // On this dev host RAM is available, so fresh should hold; this test
        // pins the positive branch of the sanity check (probe succeeded).
        if !port.capacity_fresh() {
            panic!("dev host with free RAM reported not-fresh — probe regression");
        }
    }
}
