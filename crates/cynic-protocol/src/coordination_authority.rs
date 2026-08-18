//! Desired-state and reconciliation contract for the coordination authority.
//!
//! This module deliberately produces a plan only. A node-owned scheduler must
//! apply the plan through an adapter and read back the resulting membership;
//! no caller may treat the plan itself as quorum proof.

use serde::{Deserialize, Serialize};

pub const COORDINATION_AUTHORITY_DESIRED_SCHEMA: &str =
    "cynic.coordination_authority.desired_state.v1";
pub const COORDINATION_AUTHORITY_OBSERVATION_SCHEMA: &str =
    "cynic.coordination_authority.observation.v1";
pub const COORDINATION_AUTHORITY_PLAN_SCHEMA: &str = "cynic.coordination_authority.plan.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationAuthorityDesiredState {
    pub schema: String,
    pub authority_id: String,
    pub member_ids: Vec<String>,
    pub required_member_count: u32,
    pub term: u64,
    pub next_fencing_token: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationAuthorityMemberObservation {
    pub node_id: String,
    pub active: bool,
    pub healthy: bool,
    pub term: u64,
    pub fencing_token: u64,
    pub last_committed_fencing_token: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationAuthorityObservation {
    pub schema: String,
    pub authority_id: String,
    pub members: Vec<CoordinationAuthorityMemberObservation>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationAuthorityAction {
    RegisterMember,
    RenewMember,
    AdvanceTerm,
    AdvanceFencing,
    HoldReadOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationAuthorityPlan {
    pub schema: String,
    pub authority_id: String,
    pub member_ids: Vec<String>,
    pub action: CoordinationAuthorityAction,
    pub next_term: u64,
    pub next_fencing_token: u64,
    pub blockers: Vec<String>,
}

pub fn reconcile_coordination_authority(
    desired: &CoordinationAuthorityDesiredState,
    observed: &CoordinationAuthorityObservation,
) -> CoordinationAuthorityPlan {
    let mut blockers = Vec::new();
    if desired.schema != COORDINATION_AUTHORITY_DESIRED_SCHEMA {
        blockers.push("desired_schema_invalid".to_string());
    }
    if observed.schema != COORDINATION_AUTHORITY_OBSERVATION_SCHEMA {
        blockers.push("observation_schema_invalid".to_string());
    }
    if desired.authority_id.trim().is_empty() || desired.authority_id != observed.authority_id {
        blockers.push("authority_identity_mismatch".to_string());
    }
    if desired.member_ids.is_empty() || desired.required_member_count == 0 {
        blockers.push("member_quorum_desired_state_missing".to_string());
    }
    if desired.required_member_count as usize > desired.member_ids.len() {
        blockers.push("required_quorum_exceeds_members".to_string());
    }
    if desired.term == 0 || desired.next_fencing_token == 0 {
        blockers.push("term_or_fencing_missing".to_string());
    }
    let observed_last_fencing = observed
        .members
        .iter()
        .map(|member| member.last_committed_fencing_token)
        .max()
        .unwrap_or(0);
    if observed_last_fencing >= desired.next_fencing_token && desired.next_fencing_token > 0 {
        blockers.push("desired_fencing_not_ahead".to_string());
    }
    let active_members: Vec<_> = observed
        .members
        .iter()
        .filter(|member| member.active && member.healthy)
        .collect();
    if active_members.len() < desired.required_member_count as usize {
        blockers.push("healthy_quorum_not_observed".to_string());
    }
    if observed
        .members
        .iter()
        .any(|member| member.term > desired.term)
    {
        blockers.push("observed_term_ahead_of_desired".to_string());
    }
    let next_term = desired.term.max(
        observed
            .members
            .iter()
            .map(|member| member.term)
            .max()
            .unwrap_or(0),
    );
    let next_fencing_token = desired.next_fencing_token;
    let action = if !blockers.is_empty() {
        CoordinationAuthorityAction::HoldReadOnly
    } else if observed.members.is_empty() {
        CoordinationAuthorityAction::RegisterMember
    } else if active_members.len() < desired.member_ids.len() {
        CoordinationAuthorityAction::RenewMember
    } else if next_term > desired.term {
        CoordinationAuthorityAction::AdvanceTerm
    } else if next_fencing_token > desired.next_fencing_token {
        CoordinationAuthorityAction::AdvanceFencing
    } else {
        CoordinationAuthorityAction::RenewMember
    };
    CoordinationAuthorityPlan {
        schema: COORDINATION_AUTHORITY_PLAN_SCHEMA.to_string(),
        authority_id: desired.authority_id.clone(),
        member_ids: desired.member_ids.clone(),
        action,
        next_term,
        next_fencing_token,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desired() -> CoordinationAuthorityDesiredState {
        CoordinationAuthorityDesiredState {
            schema: COORDINATION_AUTHORITY_DESIRED_SCHEMA.to_string(),
            authority_id: "coord-main".to_string(),
            member_ids: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            required_member_count: 2,
            term: 3,
            next_fencing_token: 9,
        }
    }

    #[test]
    fn missing_observation_is_read_only() {
        let observed = CoordinationAuthorityObservation {
            schema: COORDINATION_AUTHORITY_OBSERVATION_SCHEMA.to_string(),
            authority_id: "coord-main".to_string(),
            members: Vec::new(),
        };
        let plan = reconcile_coordination_authority(&desired(), &observed);
        assert_eq!(plan.action, CoordinationAuthorityAction::HoldReadOnly);
        assert!(
            plan.blockers
                .contains(&"healthy_quorum_not_observed".to_string())
        );
    }

    #[test]
    fn valid_membership_never_claims_more_than_observation() {
        let observed = CoordinationAuthorityObservation {
            schema: COORDINATION_AUTHORITY_OBSERVATION_SCHEMA.to_string(),
            authority_id: "coord-main".to_string(),
            members: vec![member("a", 3, 8), member("b", 3, 8), member("c", 3, 8)],
        };
        let plan = reconcile_coordination_authority(&desired(), &observed);
        assert!(plan.blockers.is_empty());
        assert_ne!(plan.action, CoordinationAuthorityAction::HoldReadOnly);
    }

    fn member(
        node_id: &str,
        term: u64,
        fencing_token: u64,
    ) -> CoordinationAuthorityMemberObservation {
        CoordinationAuthorityMemberObservation {
            node_id: node_id.to_string(),
            active: true,
            healthy: true,
            term,
            fencing_token,
            last_committed_fencing_token: fencing_token,
        }
    }
}
