use super::*;

fn desired() -> NodeSubstrateDesiredState {
    NodeSubstrateDesiredState {
        node_id: "builder".to_string(),
        provider: Some("provider-adapter".to_string()),
        resource_id: Some("machine/42".to_string()),
        desired_power: NodeSubstratePower::Running,
        provisioning: None,
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

fn provider_control_desired(endpoint: &str) -> ProviderControlDesiredState {
    ProviderControlDesiredState {
        schema: PROVIDER_CONTROL_DESIRED_STATE_SCHEMA.to_string(),
        provider: "proxmox".to_string(),
        endpoint: endpoint.to_string(),
        token_identity: "cynic@pve!runtime".to_string(),
        credential_ref: "/etc/cynic/secrets/proxmox-token".to_string(),
        ca_ref: None,
    }
}

#[test]
fn provider_control_endpoint_is_a_credential_free_https_origin() {
    assert!(
        validate_provider_control_desired_state(&provider_control_desired(
            "https://pve.internal:8006"
        ))
        .is_empty()
    );
    for endpoint in [
        "http://pve.internal:8006",
        "https://root:secret@pve.internal:8006",
        "https://pve.internal:8006/api2/json",
    ] {
        assert!(
            validate_provider_control_desired_state(&provider_control_desired(endpoint))
                .contains(&"provider_endpoint_not_credential_free_https_origin".to_string())
        );
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

#[test]
fn absent_substrate_without_reconstruction_identity_stays_blocked() {
    let mut observation = observed();
    observation.existence = NodeSubstrateExistence::Absent;
    observation.power = NodeSubstratePower::Unknown;

    let decision = reconcile_node_substrate(&desired(), &observation);

    assert_eq!(decision.status, "blocked");
    assert_eq!(decision.action, None);
    assert!(
        decision
            .blockers
            .contains(&"node_substrate_provisioning_spec_missing".to_string())
    );
}

#[test]
fn absent_substrate_with_content_addressed_reconstruction_plans_provisioning() {
    let mut desired = desired();
    desired.provisioning = Some(NodeSubstrateProvisioningSpec {
        schema: NODE_SUBSTRATE_PROVISIONING_SCHEMA.to_string(),
        image_ref: "artifact://images/builder".to_string(),
        image_digest: format!("sha256:{}", "a".repeat(64)),
        runtime_seed_ref: "artifact://runtime-seeds/builder".to_string(),
        runtime_seed_digest: format!("sha256:{}", "b".repeat(64)),
        network_profile_ref: "registry://network_profiles/build_runner".to_string(),
    });
    let mut observation = observed();
    observation.existence = NodeSubstrateExistence::Absent;
    observation.power = NodeSubstratePower::Unknown;

    let decision = reconcile_node_substrate(&desired, &observation);

    assert_eq!(decision.status, "planned");
    assert_eq!(decision.action, Some(NodeSubstrateAction::Provision));
    assert!(decision.mutates);
    assert!(
        decision
            .required_read_back
            .contains(&"provisioned_image_digest_observed".to_string())
    );
    assert!(
        decision
            .required_read_back
            .contains(&"runtime_seed_digest_observed".to_string())
    );
}
