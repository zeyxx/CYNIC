//! Pure admission rules for a resident standby kernel.
//!
//! This contract does not elect a node or mutate a supervisor. It makes the
//! safety decision explicit so a future fleet adapter cannot promote a
//! standby without an expiring lease, a newer fencing token, and a proved
//! release on the candidate.

use serde::{Deserialize, Serialize};

pub const RUNTIME_TAKEOVER_SCHEMA: &str = "cynic.runtime_takeover.v1";
pub const RUNTIME_TAKEOVER_LEASE_SCHEMA: &str = "cynic.runtime_takeover.lease_observation.v1";

/// An unexpired lease owned by another node is a hard publication conflict.
/// This rule is deliberately pure so every storage adapter can apply the same
/// split-brain guard before allocating a newer fencing token.
pub fn takeover_publish_conflicts(
    current_primary_node_id: &str,
    requested_primary_node_id: &str,
    lease_valid: bool,
) -> bool {
    lease_valid
        && !current_primary_node_id.trim().is_empty()
        && current_primary_node_id != requested_primary_node_id
}

/// Read-back proof that the local active runtime still owns the durable lease.
/// A node that loses this proof must stop serving mutations instead of
/// continuing as a split-brain primary.
pub fn takeover_lease_owned_by(observation: &TakeoverLeaseObservation, node_id: &str) -> bool {
    !node_id.trim().is_empty()
        && observation.primary_node_id == node_id
        && observation.primary_live
        && observation.lease_valid
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TakeoverLeaseObservation {
    pub schema: String,
    pub primary_node_id: String,
    pub primary_live: bool,
    pub lease_valid: bool,
    pub release_proved: bool,
    pub fencing_token: u64,
    pub prior_fencing_token: u64,
}

impl TakeoverLeaseObservation {
    pub fn unknown() -> Self {
        Self {
            schema: RUNTIME_TAKEOVER_LEASE_SCHEMA.to_string(),
            primary_node_id: String::new(),
            primary_live: true,
            lease_valid: false,
            release_proved: false,
            fencing_token: 0,
            prior_fencing_token: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandbyMode {
    Disabled,
    Resident,
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverObservation {
    pub mode: StandbyMode,
    pub primary_live: bool,
    pub lease_valid: bool,
    pub candidate_ready: bool,
    pub release_proved: bool,
    pub storage_available: bool,
    pub fencing_token: u64,
    pub prior_fencing_token: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverDecision {
    pub schema: &'static str,
    pub status: &'static str,
    pub promote: bool,
    pub mode: StandbyMode,
    pub fencing_token: u64,
    pub retained_capabilities: Vec<&'static str>,
    pub blocked_mutations: Vec<&'static str>,
    pub blockers: Vec<&'static str>,
}

pub fn decide_runtime_takeover(observation: RuntimeTakeoverObservation) -> RuntimeTakeoverDecision {
    let mut blockers = Vec::new();
    if observation.mode == StandbyMode::Disabled {
        blockers.push("standby_mode_disabled");
    }
    if observation.primary_live {
        blockers.push("primary_still_live");
    }
    // `lease_valid` describes the primary lease. Once that lease has expired
    // or been released, promotion is possible only with an explicit release
    // proof below; requiring it here would make every legitimate takeover
    // impossible. A live primary is still never promotable.
    if !observation.candidate_ready {
        blockers.push("standby_candidate_not_ready");
    }
    if !observation.release_proved {
        blockers.push("release_read_back_unproved");
    }
    if !observation.storage_available {
        blockers.push("takeover_storage_unavailable");
    }
    if observation.fencing_token == 0
        || observation.fencing_token <= observation.prior_fencing_token
    {
        blockers.push("fencing_token_not_monotonic");
    }

    let promote = blockers.is_empty();
    RuntimeTakeoverDecision {
        schema: RUNTIME_TAKEOVER_SCHEMA,
        status: if promote { "promotable" } else { "blocked" },
        promote,
        mode: if promote {
            StandbyMode::Active
        } else {
            observation.mode
        },
        fencing_token: observation.fencing_token,
        retained_capabilities: if promote {
            vec!["identity", "health", "observe", "judge", "replay_spool"]
        } else {
            vec!["identity", "health"]
        },
        blocked_mutations: vec!["fleet_mutation", "release_promotion", "authority_transfer"],
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> RuntimeTakeoverObservation {
        RuntimeTakeoverObservation {
            mode: StandbyMode::Resident,
            primary_live: false,
            lease_valid: true,
            candidate_ready: true,
            release_proved: true,
            storage_available: true,
            fencing_token: 2,
            prior_fencing_token: 1,
        }
    }

    #[test]
    fn takeover_requires_fencing_and_release_proof() {
        let decision = decide_runtime_takeover(ready());
        assert!(decision.promote);
        assert_eq!(decision.mode, StandbyMode::Active);

        let mut stale = ready();
        stale.fencing_token = 1;
        let decision = decide_runtime_takeover(stale);
        assert!(!decision.promote);
        assert!(decision.blockers.contains(&"fencing_token_not_monotonic"));
    }

    #[test]
    fn live_primary_blocks_split_brain() {
        let mut observation = ready();
        observation.primary_live = true;
        let decision = decide_runtime_takeover(observation);
        assert!(!decision.promote);
        assert!(decision.blockers.contains(&"primary_still_live"));
    }

    #[test]
    fn expired_primary_lease_with_read_back_release_promotes() {
        let mut observation = ready();
        observation.lease_valid = false;
        observation.fencing_token = 3;
        observation.prior_fencing_token = 2;
        let decision = decide_runtime_takeover(observation);
        assert!(decision.promote);
    }

    #[test]
    fn unavailable_storage_blocks_promotion() {
        let mut observation = ready();
        observation.storage_available = false;
        let decision = decide_runtime_takeover(observation);
        assert!(!decision.promote);
        assert!(decision.blockers.contains(&"takeover_storage_unavailable"));
    }

    #[test]
    fn active_lease_of_another_node_blocks_publication() {
        assert!(takeover_publish_conflicts("node-a", "node-b", true));
        assert!(!takeover_publish_conflicts("node-a", "node-a", true));
        assert!(!takeover_publish_conflicts("node-a", "node-b", false));
        assert!(!takeover_publish_conflicts("", "node-b", true));
    }

    #[test]
    fn lease_read_back_loss_is_not_owned() {
        let mut observation = TakeoverLeaseObservation {
            schema: RUNTIME_TAKEOVER_LEASE_SCHEMA.to_string(),
            primary_node_id: "node-a".to_string(),
            primary_live: true,
            lease_valid: true,
            release_proved: false,
            fencing_token: 2,
            prior_fencing_token: 1,
        };
        assert!(takeover_lease_owned_by(&observation, "node-a"));
        observation.primary_node_id = "node-b".to_string();
        assert!(!takeover_lease_owned_by(&observation, "node-a"));
        observation.primary_node_id = "node-a".to_string();
        observation.lease_valid = false;
        assert!(!takeover_lease_owned_by(&observation, "node-a"));
    }
}
