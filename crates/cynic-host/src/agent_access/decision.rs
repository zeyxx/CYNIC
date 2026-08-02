use std::collections::BTreeSet;

use crate::ports::HostOperation;

use super::types::{
    AGENT_ACCESS_OBSERVATION_SCHEMA, AGENT_ACCESS_PLAN_SCHEMA, AGENT_ACCESS_POLICY_SCHEMA,
    AgentAccessObservation, AgentAccessPlan, AgentAccessPolicy,
};

pub fn reconcile_agent_access(
    desired: &AgentAccessPolicy,
    observed: &AgentAccessObservation,
) -> AgentAccessPlan {
    let mut blockers = Vec::new();
    let mut residual_unknowns = observed.unknowns.clone();
    let mut operations = Vec::new();
    if desired.schema != AGENT_ACCESS_POLICY_SCHEMA
        || desired.runtime_user.trim().is_empty()
        || desired.access_group.trim().is_empty()
        || desired.socket_mode != 0o660
    {
        blockers.push("agent_access_policy_invalid".to_string());
    }
    if desired.authorized_principals.is_empty() {
        blockers.push("agent_access_has_no_authorized_principal".to_string());
    }
    if observed.schema != AGENT_ACCESS_OBSERVATION_SCHEMA {
        residual_unknowns.push("agent_access_observation_schema_mismatch".to_string());
    }

    match observed.access_group_present {
        Some(false) => operations.push(HostOperation::EnsureSystemGroup {
            group: desired.access_group.clone(),
        }),
        None => residual_unknowns.push("agent_access_group_presence_unknown".to_string()),
        Some(true) => {}
    }
    let observed_members: BTreeSet<&str> = observed
        .observed_members
        .iter()
        .map(String::as_str)
        .collect();
    for user in std::iter::once(&desired.runtime_user).chain(&desired.authorized_principals) {
        if !observed_members.contains(user.as_str()) {
            operations.push(HostOperation::EnsureSupplementaryGroupMember {
                user: user.clone(),
                group: desired.access_group.clone(),
            });
        }
    }
    if observed.runtime_directory_present == Some(false) {
        operations.push(HostOperation::EnsureOwnedDir {
            path: desired.runtime_directory.clone(),
            user: desired.runtime_user.clone(),
            group: desired.runtime_user.clone(),
            mode: 0o755,
        });
    } else if observed.runtime_directory_present.is_none() {
        residual_unknowns.push("agent_runtime_directory_presence_unknown".to_string());
    }
    if observed.socket_present == Some(true) {
        if observed.socket_group_id != observed.access_group_id {
            blockers.push("agent_socket_group_mismatch".to_string());
        }
        if observed.socket_mode != Some(desired.socket_mode) {
            blockers.push("agent_socket_mode_mismatch".to_string());
        }
    } else {
        residual_unknowns.push("agent_socket_post_start_read_back_pending".to_string());
    }

    blockers.sort();
    blockers.dedup();
    residual_unknowns.sort();
    residual_unknowns.dedup();
    AgentAccessPlan {
        schema: AGENT_ACCESS_PLAN_SCHEMA.to_string(),
        status: if !blockers.is_empty() {
            "blocked"
        } else if operations.is_empty() {
            "converged"
        } else {
            "planned"
        }
        .to_string(),
        operations,
        blockers,
        residual_unknowns,
        expected_proofs: vec![
            "agent_access_group_present".to_string(),
            "authorized_principals_are_members".to_string(),
            "kernel_socket_group_and_mode_match".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_group_and_members_produce_bounded_host_operations() {
        let desired = AgentAccessPolicy::desired(vec!["operator".to_string()]);
        let observed = AgentAccessObservation {
            schema: AGENT_ACCESS_OBSERVATION_SCHEMA.to_string(),
            access_group_present: Some(false),
            access_group_id: None,
            observed_members: Vec::new(),
            runtime_directory_present: Some(false),
            socket_present: Some(false),
            socket_group_id: None,
            socket_mode: None,
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        };
        let plan = reconcile_agent_access(&desired, &observed);
        assert_eq!(plan.status, "planned");
        assert!(plan.operations.iter().any(|operation| matches!(
            operation,
            HostOperation::EnsureSystemGroup { group } if group == "cynic-agent"
        )));
        assert_eq!(
            plan.operations
                .iter()
                .filter(|operation| matches!(
                    operation,
                    HostOperation::EnsureSupplementaryGroupMember { .. }
                ))
                .count(),
            2
        );
    }

    #[test]
    fn wrong_socket_permissions_block_convergence() {
        let desired = AgentAccessPolicy::desired(vec!["operator".to_string()]);
        let observed = AgentAccessObservation {
            schema: AGENT_ACCESS_OBSERVATION_SCHEMA.to_string(),
            access_group_present: Some(true),
            access_group_id: Some(1901),
            observed_members: vec!["cynic".to_string(), "operator".to_string()],
            runtime_directory_present: Some(true),
            socket_present: Some(true),
            socket_group_id: Some(1900),
            socket_mode: Some(0o600),
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        };
        let plan = reconcile_agent_access(&desired, &observed);
        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"agent_socket_group_mismatch".to_string())
        );
        assert!(
            plan.blockers
                .contains(&"agent_socket_mode_mismatch".to_string())
        );
    }

    #[test]
    fn matching_access_state_is_converged_even_before_kernel_start() {
        let desired = AgentAccessPolicy::desired(vec!["operator".to_string()]);
        let observed = AgentAccessObservation {
            schema: AGENT_ACCESS_OBSERVATION_SCHEMA.to_string(),
            access_group_present: Some(true),
            access_group_id: Some(1901),
            observed_members: vec!["cynic".to_string(), "operator".to_string()],
            runtime_directory_present: Some(true),
            socket_present: Some(false),
            socket_group_id: None,
            socket_mode: None,
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        };
        let plan = reconcile_agent_access(&desired, &observed);
        assert_eq!(plan.status, "converged");
        assert!(plan.operations.is_empty());
        assert!(
            plan.residual_unknowns
                .contains(&"agent_socket_post_start_read_back_pending".to_string())
        );
    }
}
