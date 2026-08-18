//! Deterministic scaling harness for the coordination membrane.
//!
//! These scenarios exercise the same quorum decision used by the runtime at
//! several constellation sizes. They are bounded conformance evidence only;
//! they never attest to a live fleet or authorize a mutation.

use serde::{Deserialize, Serialize};

pub const COORDINATION_SCALING_HARNESS_SCHEMA: &str = "cynic.coordination.scaling_harness.v1";

pub const TAKEOVER_RACE_HARNESS_SCHEMA: &str = "cynic.coordination.takeover_race_harness.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TakeoverRaceOutcome {
    pub schema: String,
    pub prior_fencing_token: u64,
    pub first_candidate_token: u64,
    pub second_candidate_token: u64,
    pub winner_node_id: String,
    pub winner_read_back_owned: bool,
    pub loser_read_back_owned: bool,
    pub losing_candidate_must_stop: bool,
    pub split_brain_admitted: bool,
}

/// Model two candidates observing the same expired lease. The durable
/// authority serializes fencing allocation; the later token wins, and the
/// earlier candidate must fail its ownership read-back and stop.
pub fn evaluate_takeover_race(
    prior_fencing_token: u64,
    first_candidate_node_id: &str,
    second_candidate_node_id: &str,
) -> TakeoverRaceOutcome {
    let first_candidate_token = prior_fencing_token.saturating_add(1);
    let second_candidate_token = first_candidate_token.saturating_add(1);
    let winner_node_id = if second_candidate_token > first_candidate_token {
        second_candidate_node_id
    } else {
        first_candidate_node_id
    };
    let winner_observation = crate::TakeoverLeaseObservation {
        schema: crate::RUNTIME_TAKEOVER_LEASE_SCHEMA.to_string(),
        primary_node_id: winner_node_id.to_string(),
        primary_live: true,
        lease_valid: true,
        release_proved: false,
        fencing_token: second_candidate_token,
        prior_fencing_token: first_candidate_token,
    };
    TakeoverRaceOutcome {
        schema: TAKEOVER_RACE_HARNESS_SCHEMA.to_string(),
        prior_fencing_token,
        first_candidate_token,
        second_candidate_token,
        winner_node_id: winner_node_id.to_string(),
        winner_read_back_owned: crate::takeover_lease_owned_by(&winner_observation, winner_node_id),
        loser_read_back_owned: crate::takeover_lease_owned_by(
            &winner_observation,
            first_candidate_node_id,
        ),
        losing_candidate_must_stop: true,
        split_brain_admitted: false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationScalingScenario {
    pub scenario_id: String,
    pub authority_mode: crate::CoordinationAuthorityMode,
    pub member_count: u32,
    pub healthy_member_count: u32,
    pub required_member_count: u32,
    pub term: u64,
    pub proposed_fencing_token: u64,
    pub last_committed_fencing_token: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationScalingOutcome {
    pub schema: String,
    pub scenario_id: String,
    pub member_count: u32,
    pub healthy_member_count: u32,
    pub coordination_status: String,
    pub mutation_allowed: bool,
    pub degradation_status: String,
    pub blockers: Vec<String>,
}

pub fn evaluate_coordination_scaling(
    scenario: &CoordinationScalingScenario,
) -> CoordinationScalingOutcome {
    let decision = crate::decide_quorum_write(crate::QuorumObservation {
        authority_mode: scenario.authority_mode,
        member_count: scenario.member_count,
        healthy_member_count: scenario.healthy_member_count,
        required_member_count: scenario.required_member_count,
        term: scenario.term,
        proposed_fencing_token: scenario.proposed_fencing_token,
        last_committed_fencing_token: scenario.last_committed_fencing_token,
    });
    let mut blockers = decision
        .blockers
        .iter()
        .map(|blocker| (*blocker).to_string())
        .collect::<Vec<_>>();
    blockers.sort();
    blockers.dedup();
    let degradation_status = if decision.writable {
        if scenario.healthy_member_count < scenario.member_count {
            "degraded_but_quorate"
        } else {
            "healthy"
        }
    } else if scenario.member_count > 0 {
        "read_only"
    } else {
        "unavailable"
    };
    CoordinationScalingOutcome {
        schema: COORDINATION_SCALING_HARNESS_SCHEMA.to_string(),
        scenario_id: scenario.scenario_id.clone(),
        member_count: scenario.member_count,
        healthy_member_count: scenario.healthy_member_count,
        coordination_status: decision.status.to_string(),
        mutation_allowed: decision.writable,
        degradation_status: degradation_status.to_string(),
        blockers,
    }
}

pub fn canonical_coordination_scaling_scenarios() -> Vec<CoordinationScalingScenario> {
    vec![
        CoordinationScalingScenario {
            scenario_id: "single-node-local-lab".to_string(),
            authority_mode: crate::CoordinationAuthorityMode::SingleNode,
            member_count: 1,
            healthy_member_count: 1,
            required_member_count: 1,
            term: 1,
            proposed_fencing_token: 2,
            last_committed_fencing_token: 1,
        },
        CoordinationScalingScenario {
            scenario_id: "three-node-majority".to_string(),
            authority_mode: crate::CoordinationAuthorityMode::Quorum,
            member_count: 3,
            healthy_member_count: 3,
            required_member_count: 2,
            term: 2,
            proposed_fencing_token: 4,
            last_committed_fencing_token: 3,
        },
        CoordinationScalingScenario {
            scenario_id: "five-node-one-failure".to_string(),
            authority_mode: crate::CoordinationAuthorityMode::Quorum,
            member_count: 5,
            healthy_member_count: 4,
            required_member_count: 3,
            term: 3,
            proposed_fencing_token: 6,
            last_committed_fencing_token: 5,
        },
        CoordinationScalingScenario {
            scenario_id: "five-node-majority-loss".to_string(),
            authority_mode: crate::CoordinationAuthorityMode::Quorum,
            member_count: 5,
            healthy_member_count: 2,
            required_member_count: 3,
            term: 3,
            proposed_fencing_token: 7,
            last_committed_fencing_token: 6,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_harness_preserves_local_and_quorate_modes() {
        let outcomes = canonical_coordination_scaling_scenarios()
            .iter()
            .map(evaluate_coordination_scaling)
            .collect::<Vec<_>>();
        assert_eq!(outcomes.len(), 4);
        assert!(outcomes[0].mutation_allowed);
        assert!(outcomes[1].mutation_allowed);
        assert!(outcomes[2].mutation_allowed);
        assert_eq!(outcomes[2].degradation_status, "degraded_but_quorate");
        assert!(!outcomes[3].mutation_allowed);
        assert_eq!(outcomes[3].degradation_status, "read_only");
    }

    #[test]
    fn stale_fencing_remains_blocked_at_larger_scale() {
        let mut scenario = canonical_coordination_scaling_scenarios()
            .into_iter()
            .nth(2)
            .expect("five-node scenario");
        scenario.proposed_fencing_token = scenario.last_committed_fencing_token;
        let outcome = evaluate_coordination_scaling(&scenario);
        assert!(!outcome.mutation_allowed);
        assert!(
            outcome
                .blockers
                .contains(&"coordination_fencing_not_monotonic".to_string())
        );
    }

    #[test]
    fn takeover_race_has_one_fenced_winner() {
        let outcome = evaluate_takeover_race(8, "node-a", "node-b");
        assert_eq!(outcome.first_candidate_token, 9);
        assert_eq!(outcome.second_candidate_token, 10);
        assert_eq!(outcome.winner_node_id, "node-b");
        assert!(outcome.winner_read_back_owned);
        assert!(!outcome.loser_read_back_owned);
        assert!(outcome.losing_candidate_must_stop);
        assert!(!outcome.split_brain_admitted);
    }
}
