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
        credential_ref: cynic_types::paths::CynicPaths::secret_path("proxmox-token")
            .display()
            .to_string(),
        ca_ref: None,
        credential_lease_metadata_ref: None,
        credential_authority_public_key_ref: None,
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
        lxc: None,
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

fn lxc_spec() -> LxcProvisioningSpec {
    LxcProvisioningSpec {
        schema: LXC_PROVISIONING_SCHEMA.to_string(),
        hostname: "cynic-lxc-embedding".to_string(),
        template_volid: "local:vztmpl/debian-12-standard_12.12-1_amd64.tar.zst".to_string(),
        storage_pool: "local-lvm".to_string(),
        disk_gib: 16,
        cores: 2,
        memory_mib: 2048,
        swap_mib: 1024,
        bridge: "vmbr0".to_string(),
        ip_cidr: Some("10.10.10.106/24".to_string()),
        gateway: Some("10.10.10.1".to_string()),
        nameserver: Some("10.10.10.1".to_string()),
        unprivileged: true,
        onboot: true,
        features: vec!["nesting=1".to_string()],
    }
}

fn desired_with_lxc(lxc: LxcProvisioningSpec) -> NodeSubstrateDesiredState {
    NodeSubstrateDesiredState {
        node_id: "cynic-lxc-embedding".to_string(),
        provider: Some("proxmox".to_string()),
        resource_id: Some("lxc/106".to_string()),
        desired_power: NodeSubstratePower::Running,
        provisioning: Some(NodeSubstrateProvisioningSpec {
            schema: NODE_SUBSTRATE_PROVISIONING_SCHEMA.to_string(),
            image_ref: String::new(),
            image_digest: String::new(),
            runtime_seed_ref: String::new(),
            runtime_seed_digest: String::new(),
            network_profile_ref: String::new(),
            lxc: Some(lxc),
        }),
    }
}

#[test]
fn valid_lxc_provisioning_spec_passes_validation() {
    assert!(validate_lxc_provisioning_spec(&lxc_spec()).is_empty());
}

#[test]
fn lxc_spec_rejects_invalid_hostname_template_and_network() {
    let mut spec = lxc_spec();
    spec.hostname = "-bad_hostname-".to_string();
    assert!(validate_lxc_provisioning_spec(&spec).contains(&"lxc_hostname_invalid".to_string()));

    let mut spec = lxc_spec();
    spec.template_volid = "debian.tar.zst".to_string();
    assert!(
        validate_lxc_provisioning_spec(&spec).contains(&"lxc_template_volid_invalid".to_string())
    );

    let mut spec = lxc_spec();
    spec.ip_cidr = Some("10.10.10.999/24".to_string());
    assert!(validate_lxc_provisioning_spec(&spec).contains(&"lxc_ip_cidr_invalid".to_string()));

    let mut spec = lxc_spec();
    spec.gateway = None;
    assert!(
        validate_lxc_provisioning_spec(&spec)
            .contains(&"lxc_gateway_missing_or_invalid_for_static_ip".to_string())
    );
}

#[test]
fn absent_lxc_substrate_plans_provisioning_without_content_addressed_digests() {
    let desired = desired_with_lxc(lxc_spec());
    let mut observation = observed();
    observation.node_id = "cynic-lxc-embedding".to_string();
    observation.provider = Some("proxmox".to_string());
    observation.resource_id = Some("lxc/106".to_string());
    observation.existence = NodeSubstrateExistence::Absent;
    observation.power = NodeSubstratePower::Unknown;
    observation.network_attachment = NodeNetworkAttachment::Unknown;

    let decision = reconcile_node_substrate(&desired, &observation);

    assert_eq!(decision.status, "planned");
    assert_eq!(decision.action, Some(NodeSubstrateAction::Provision));
    assert!(decision.mutates);
    assert!(decision.blockers.is_empty());
}
