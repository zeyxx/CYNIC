use super::*;

fn desired() -> NodeSubstrateDesiredState {
    NodeSubstrateDesiredState {
        node_id: "builder".to_string(),
        provider: Some("provider-adapter".to_string()),
        resource_id: Some("machine/42".to_string()),
        desired_power: NodeSubstratePower::Running,
    }
}

fn observed() -> NodeSubstrateObservation {
    NodeSubstrateObservation {
        schema: NODE_SUBSTRATE_OBSERVATION_SCHEMA.to_string(),
        status: "observed".to_string(),
        node_id: "builder".to_string(),
        provider: Some("provider-adapter".to_string()),
        resource_id: Some("machine/42".to_string()),
        existence: NodeSubstrateExistence::Present,
        power: NodeSubstratePower::Running,
        network_attachment: NodeNetworkAttachment::Attached,
        observed_addresses: vec!["10.0.0.42".to_string()],
        management_reachable: Some(true),
        observation_authority: Some("provider-read-api".to_string()),
        observed_at_unix_ms: Some(42),
        evidence_refs: vec!["provider-observation:42".to_string()],
        unknowns: Vec::new(),
    }
}

#[test]
fn missing_provider_resource_ref_stays_blocked() {
    let mut desired = desired();
    desired.resource_id = None;

    let decision = reconcile_node_substrate(&desired, &observed());

    assert_eq!(decision.status, "blocked");
    assert!(
        decision
            .blockers
            .contains(&"provider_resource_ref_missing".to_string())
    );
}

#[test]
fn stopped_node_plans_bounded_start_with_read_back() {
    let mut observed = observed();
    observed.power = NodeSubstratePower::Stopped;

    let decision = reconcile_node_substrate(&desired(), &observed);

    assert_eq!(decision.status, "planned");
    assert_eq!(decision.action, Some(NodeSubstrateAction::Start));
    assert!(decision.mutates);
    assert!(!decision.required_read_back.is_empty());
}

#[test]
fn present_running_attached_reachable_node_is_converged() {
    let decision = reconcile_node_substrate(&desired(), &observed());

    assert_eq!(decision.status, "converged");
    assert_eq!(decision.action, None);
    assert!(!decision.mutates);
}
