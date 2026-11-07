//! Verticale 4/4 (rouge): le domaine pur agent_access vit en L0 (cynic-protocol)
//! et ne dépend que du port L0 `AgentAccessOperation` — aucune dépendance
//! vers cynic-host / HostOperation / cynic-registry.
use cynic_protocol::agent_access::{
    AGENT_ACCESS_OBSERVATION_SCHEMA, AGENT_ACCESS_POLICY_SCHEMA, AgentAccessObservation,
    AgentAccessPolicy, reconcile_agent_access,
};
use cynic_protocol::agent_access_port::AgentAccessOperation;

fn desired() -> AgentAccessPolicy {
    AgentAccessPolicy {
        schema: AGENT_ACCESS_POLICY_SCHEMA.to_string(),
        runtime_user: "cynic".to_string(),
        access_group: "cynic-agent".to_string(),
        authorized_principals: vec!["operator".to_string()],
        runtime_directory: "/run/cynic".to_string(),
        socket_path: "/run/cynic/kernel.sock".to_string(),
        socket_mode: 0o660,
    }
}

#[test]
fn missing_group_and_members_produce_bounded_l0_operations() {
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
    let plan = reconcile_agent_access(&desired(), &observed);
    assert_eq!(plan.status, "planned");
    assert!(plan.operations.iter().any(|op| matches!(
        op,
        AgentAccessOperation::EnsureSystemGroup { group } if group == "cynic-agent"
    )));
    assert_eq!(
        plan.operations
            .iter()
            .filter(|op| matches!(
                op,
                AgentAccessOperation::EnsureSupplementaryGroupMember { .. }
            ))
            .count(),
        2
    );
}

#[test]
fn wrong_socket_permissions_block_convergence() {
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
    let plan = reconcile_agent_access(&desired(), &observed);
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
