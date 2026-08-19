use super::*;

fn desired() -> NodeSubstrateDesiredState {
    NodeSubstrateDesiredState {
        node_id: "builder".to_string(),
        provider: Some("proxmox".to_string()),
        resource_id: Some("lxc/105".to_string()),
        desired_power: NodeSubstratePower::Running,
        provisioning: None,
    }
}

#[test]
fn parse_inline_network_ip_works() {
    assert_eq!(
        parse_inline_network_ip("name=eth0,ip=10.0.0.42/24,gw=10.0.0.1"),
        Some("10.0.0.42".to_string())
    );
    assert_eq!(
        parse_inline_network_ip("foo=bar ip=127.0.0.1/16"),
        Some("127.0.0.1".to_string())
    );
    assert_eq!(parse_inline_network_ip("noip"), None);
}

#[test]
fn provider_selection_is_registry_driven_without_an_explicit_override() {
    let file = tempfile::NamedTempFile::new().expect("registry temp file");
    std::fs::write(
        file.path(),
        serde_json::json!({
            "fleet_desired": {"targets": [{"provider": "test-provider"}]},
            "nodes": [{"provider_controls": ["secondary-provider"]}]
        })
        .to_string(),
    )
    .expect("registry write");
    let registry: Value =
        serde_json::from_str(&std::fs::read_to_string(file.path()).expect("registry read"))
            .expect("registry json");
    assert_eq!(
        declared_provider_names(&registry),
        vec!["secondary-provider", "test-provider"]
    );
}

#[test]
fn unavailable_provider_remains_an_unproven_unknown() {
    let observation = node_substrate_observation(
        &desired(),
        &FleetInventoryDetails {
            schema: cynic_protocol::FLEET_INVENTORY_REPORT_SCHEMA,
            status: "unavailable",
            registry_digest: None,
            observed_at_unix_ms: 0,
            nodes: Vec::new(),
            summary: FleetInventorySummary::default(),
        },
    );

    assert_eq!(observation.status, "unknown");
    assert_eq!(observation.existence, NodeSubstrateExistence::Unknown);
    assert!(observation.evidence_refs.is_empty());
}

#[test]
fn successful_provider_inventory_proves_exact_resource_power() {
    let inventory = FleetInventoryDetails {
        schema: cynic_protocol::FLEET_INVENTORY_REPORT_SCHEMA,
        status: "ok",
        registry_digest: Some("test-digest".to_string()),
        observed_at_unix_ms: 1,
        nodes: vec![FleetNodeObservation {
            provider: "proxmox".to_string(),
            provider_node: Some("pve".to_string()),
            node_id: "lxc/105".to_string(),
            name: "cynic-builder".to_string(),
            status: "stopped".to_string(),
            liveness: "down".to_string(),
            ip: None,
            desired: true,
            observed: true,
            meta: None,
        }],
        summary: FleetInventorySummary {
            providers: vec![FleetInventoryProviderSummary::ok("proxmox", 1)],
            ..FleetInventorySummary::default()
        },
    };

    let observation = node_substrate_observation(&desired(), &inventory);

    assert_eq!(observation.status, "observed");
    assert_eq!(observation.existence, NodeSubstrateExistence::Present);
    assert_eq!(observation.power, NodeSubstratePower::Stopped);
    assert!(!observation.evidence_refs.is_empty());
}

#[test]
fn complete_provider_inventory_proves_resource_absence_without_fabricating_power() {
    let inventory = FleetInventoryDetails {
        schema: cynic_protocol::FLEET_INVENTORY_REPORT_SCHEMA,
        status: "ok",
        registry_digest: Some("test-digest".to_string()),
        observed_at_unix_ms: 1,
        nodes: Vec::new(),
        summary: FleetInventorySummary {
            providers: vec![FleetInventoryProviderSummary::ok("proxmox", 0)],
            ..FleetInventorySummary::default()
        },
    };

    let observation = node_substrate_observation(&desired(), &inventory);

    assert_eq!(observation.status, "observed");
    assert_eq!(observation.existence, NodeSubstrateExistence::Absent);
    assert_eq!(observation.power, NodeSubstratePower::Unknown);
    assert!(
        observation
            .unknowns
            .contains(&"provider_resource_absent".to_string())
    );
}
