//! Deterministic coordination fault harness.
//!
//! The harness never mutates a node or fabricates a runtime proof. It combines
//! explicit quorum and takeover observations to prove the expected degraded
//! state for lab/conformance scenarios.

use serde::{Deserialize, Serialize};

use crate::coordination_authority::{
    CoordinationAuthorityMode, QuorumObservation, decide_quorum_write,
};

pub const COORDINATION_FAULT_HARNESS_SCHEMA: &str = "cynic.coordination.fault_harness.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationFaultScenario {
    pub scenario_id: String,
    pub quorum: QuorumObservation,
    pub takeover: crate::RuntimeTakeoverObservation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationFaultOutcome {
    pub schema: String,
    pub scenario_id: String,
    pub coordination_status: String,
    pub mutation_allowed: bool,
    pub takeover_status: String,
    pub promotion_allowed: bool,
    pub blockers: Vec<String>,
}

pub fn evaluate_coordination_fault(
    scenario: &CoordinationFaultScenario,
) -> CoordinationFaultOutcome {
    let quorum = decide_quorum_write(scenario.quorum.clone());
    let takeover = crate::decide_runtime_takeover(scenario.takeover.clone());
    let mut blockers: Vec<String> = quorum
        .blockers
        .iter()
        .map(|blocker| (*blocker).to_string())
        .collect();
    blockers.extend(
        takeover
            .blockers
            .iter()
            .map(|blocker| (*blocker).to_string()),
    );
    blockers.sort();
    blockers.dedup();
    CoordinationFaultOutcome {
        schema: COORDINATION_FAULT_HARNESS_SCHEMA.to_string(),
        scenario_id: scenario.scenario_id.clone(),
        coordination_status: quorum.status.to_string(),
        mutation_allowed: quorum.writable,
        takeover_status: takeover.status.to_string(),
        promotion_allowed: takeover.promote,
        blockers,
    }
}

pub fn canonical_coordination_fault_scenarios() -> Vec<CoordinationFaultScenario> {
    vec![
        CoordinationFaultScenario {
            scenario_id: "quorum-loss".to_string(),
            quorum: QuorumObservation {
                authority_mode: CoordinationAuthorityMode::Quorum,
                member_count: 3,
                healthy_member_count: 1,
                required_member_count: 2,
                term: 4,
                proposed_fencing_token: 9,
                last_committed_fencing_token: 8,
            },
            takeover: crate::RuntimeTakeoverObservation {
                mode: crate::StandbyMode::Resident,
                primary_live: true,
                lease_valid: true,
                candidate_ready: true,
                release_proved: false,
                storage_available: true,
                fencing_token: 9,
                prior_fencing_token: 8,
            },
        },
        CoordinationFaultScenario {
            scenario_id: "primary-release".to_string(),
            quorum: QuorumObservation {
                authority_mode: CoordinationAuthorityMode::Quorum,
                member_count: 3,
                healthy_member_count: 3,
                term: 4,
                proposed_fencing_token: 9,
                last_committed_fencing_token: 8,
            },
            takeover: crate::RuntimeTakeoverObservation {
                mode: crate::StandbyMode::Resident,
                primary_live: false,
                lease_valid: false,
                candidate_ready: true,
                release_proved: true,
                storage_available: true,
                fencing_token: 10,
                prior_fencing_token: 9,
            },
        },
        CoordinationFaultScenario {
            scenario_id: "stale-fence".to_string(),
            quorum: QuorumObservation {
                authority_mode: CoordinationAuthorityMode::Quorum,
                member_count: 3,
                healthy_member_count: 3,
                proposed_fencing_token: 9,
                last_committed_fencing_token: 8,
            },
            takeover: crate::RuntimeTakeoverObservation {
                mode: crate::StandbyMode::Resident,
                primary_live: false,
                lease_valid: false,
                candidate_ready: true,
                release_proved: true,
                storage_available: true,
                fencing_token: 8,
                prior_fencing_token: 9,
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quorum(healthy: u32, proposed: u64, committed: u64) -> crate::QuorumObservation {
        crate::QuorumObservation {
            authority_mode: crate::CoordinationAuthorityMode::Quorum,
            member_count: 3,
            healthy_member_count: healthy,
            required_member_count: 2,
            term: 4,
            proposed_fencing_token: proposed,
            last_committed_fencing_token: committed,
        }
    }

    fn takeover(
        primary_live: bool,
        release_proved: bool,
        fencing: u64,
    ) -> crate::RuntimeTakeoverObservation {
        takeover_with_prior(
            primary_live,
            release_proved,
            fencing,
            fencing.saturating_sub(1),
        )
    }

    fn takeover_with_prior(
        primary_live: bool,
        release_proved: bool,
        fencing: u64,
        prior_fencing: u64,
    ) -> crate::RuntimeTakeoverObservation {
        crate::RuntimeTakeoverObservation {
            mode: crate::StandbyMode::Resident,
            primary_live,
            lease_valid: primary_live,
            candidate_ready: true,
            release_proved,
            storage_available: true,
            fencing_token: fencing,
            prior_fencing_token: prior_fencing,
        }
    }

    #[test]
    fn quorum_loss_is_read_only_and_not_takeover_proof() {
        let outcome = evaluate_coordination_fault(&CoordinationFaultScenario {
            scenario_id: "quorum-loss".into(),
            quorum: quorum(1, 9, 8),
            takeover: takeover(true, false, 9),
        });
        assert!(!outcome.mutation_allowed);
        assert!(!outcome.promotion_allowed);
        assert!(
            outcome
                .blockers
                .contains(&"coordination_quorum_unobserved".into())
        );
    }

    #[test]
    fn released_primary_with_new_fence_is_promotable() {
        let outcome = evaluate_coordination_fault(&CoordinationFaultScenario {
            scenario_id: "primary-release".into(),
            quorum: quorum(3, 9, 8),
            takeover: takeover(false, true, 10),
        });
        assert!(outcome.mutation_allowed);
        assert!(outcome.promotion_allowed);
    }

    #[test]
    fn stale_takeover_fence_is_blocked() {
        let outcome = evaluate_coordination_fault(&CoordinationFaultScenario {
            scenario_id: "stale-fence".into(),
            quorum: quorum(3, 9, 8),
            takeover: takeover_with_prior(false, true, 8, 9),
        });
        assert!(!outcome.promotion_allowed);
        assert!(
            outcome
                .blockers
                .contains(&"fencing_token_not_monotonic".into())
        );
    }

    #[test]
    fn canonical_scenarios_cover_loss_release_and_stale_fence() {
        let outcomes: Vec<_> = canonical_coordination_fault_scenarios()
            .iter()
            .map(evaluate_coordination_fault)
            .collect();
        assert_eq!(outcomes.len(), 3);
        assert!(!outcomes[0].mutation_allowed);
        assert!(!outcomes[0].promotion_allowed);
        assert!(outcomes[1].mutation_allowed);
        assert!(outcomes[1].promotion_allowed);
        assert!(!outcomes[2].promotion_allowed);
    }
}
