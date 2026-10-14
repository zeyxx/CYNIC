//! Vocabulary proof: mesh-plan generation against the REAL repository
//! registry (`infra/registry.json`), not an inline fixture.
//!
//! Inline fixtures carry whatever vocabulary the author had in mind, so a
//! plane-vocabulary rename stays green while production reads a file whose
//! keys no longer resolve. This test points at the real file: if the
//! canonical service plane does not match any node `network` key, peers
//! resolve to nothing and the plan comes back empty.

use std::path::PathBuf;

fn repo_registry_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../infra/registry.json")
}

fn load_repo_registry() -> cynic_network::MeshRegistry {
    let raw = std::fs::read_to_string(repo_registry_path())
        .expect("read infra/registry.json from workspace");
    serde_json::from_str(&raw).expect("parse infra/registry.json as MeshRegistry")
}

#[test]
fn mesh_plan_from_repo_registry_is_planned_with_peers() {
    let registry = load_repo_registry();
    let plan = cynic_network::generate_mesh_plan(&registry, None).expect("generate mesh plan");
    assert_eq!(
        plan.status, "planned",
        "mesh plan against infra/registry.json is '{}' with {} peers: \
         canonical plane resolves against no node network key (vocabulary drift)",
        plan.status, plan.desired_peer_count,
    );
    assert!(
        !plan.desired_peers.is_empty(),
        "empty desired_peers against the real registry"
    );
}

#[test]
fn every_node_with_a_service_plane_address_appears_as_peer() {
    let registry = load_repo_registry();
    let canonical = registry
        .network_policy
        .as_ref()
        .and_then(|policy| policy.canonical_service_plane.clone())
        .expect("registry declares a canonical service plane");
    let plan = cynic_network::generate_mesh_plan(&registry, None).expect("generate mesh plan");
    let peer_ids: Vec<&str> = plan
        .desired_peers
        .iter()
        .map(|peer| peer.id.as_str())
        .collect();
    for node in &registry.nodes {
        if node.network.contains_key(&canonical) {
            let id = node
                .name
                .clone()
                .or_else(|| node.id.clone())
                .unwrap_or_default();
            assert!(
                peer_ids.contains(&id.as_str()),
                "node '{id}' declares a '{canonical}' address but is missing from the plan"
            );
        }
    }
}

#[test]
fn plane_address_resolution_prefers_canonical_key_over_legacy_alias() {
    use std::collections::BTreeMap;
    let mut both = BTreeMap::new();
    both.insert("service".to_string(), "10.10.10.1".to_string());
    both.insert("synapse".to_string(), "10.20.0.1".to_string());
    assert_eq!(
        cynic_network::plane_vocabulary::plane_address_in_map(&both, "service").as_deref(),
        Some("10.10.10.1")
    );
    let mut legacy_only = BTreeMap::new();
    legacy_only.insert("synapse".to_string(), "10.20.0.1".to_string());
    assert_eq!(
        cynic_network::plane_vocabulary::plane_address_in_map(&legacy_only, "service").as_deref(),
        Some("10.20.0.1")
    );
    let mut admin_pair = BTreeMap::new();
    admin_pair.insert("admin".to_string(), "192.168.100.1".to_string());
    admin_pair.insert("management".to_string(), "192.168.100.9".to_string());
    assert_eq!(
        cynic_network::plane_vocabulary::plane_address_in_map(&admin_pair, "admin").as_deref(),
        Some("192.168.100.1")
    );
    let empty: BTreeMap<String, String> = BTreeMap::new();
    assert_eq!(
        cynic_network::plane_vocabulary::plane_address_in_map(&empty, "service"),
        None
    );
    assert_eq!(cynic_network::plane_vocabulary::plane_address_in_map(&empty, "mesh"), None);
}

#[test]
fn repo_registry_node_keys_use_canonical_trio_vocabulary() {
    let raw = std::fs::read_to_string(repo_registry_path()).expect("read infra/registry.json");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("parse registry JSON");
    let nodes = value
        .get("nodes")
        .and_then(|nodes| nodes.as_array())
        .expect("registry has a nodes array");
    let empty: &[serde_json::Value] = &[];
    for node in nodes {
        let name = node
            .get("name")
            .and_then(|name| name.as_str())
            .unwrap_or("?");
        if let Some(network) = node.get("network").and_then(|net| net.as_object()) {
            for key in network.keys() {
                assert!(
                    ["service", "admin", "mesh", "local"].contains(&key.as_str()),
                    "node '{name}' network key '{key}' is outside the canonical trio vocabulary"
                );
            }
        }
        let services = node
            .get("services")
            .and_then(|services| services.as_array())
            .map(Vec::as_slice)
            .unwrap_or(empty);
        for service in services {
            if let Some(plane) = service.get("network_plane").and_then(|p| p.as_str()) {
                assert!(
                    ["service", "admin", "mesh"].contains(&plane),
                    "node '{name}' service network_plane '{plane}' is outside the trio"
                );
            }
        }
    }
}

#[test]
fn every_repo_registry_service_declares_its_plane() {
    // Pins the firewall derivation to declared data: an undeclared service
    // silently falls back to ServiceOnly in derive_firewall_policy_from_registry.
    // This test fails the moment a service is added without network_plane.
    let raw = std::fs::read_to_string(repo_registry_path())
        .expect("read infra/registry.json from workspace");
    let registry: serde_json::Value =
        serde_json::from_str(&raw).expect("parse infra/registry.json");
    let nodes = registry
        .get("nodes")
        .and_then(|nodes| nodes.as_array())
        .expect("registry has nodes");
    let mut undeclared = Vec::new();
    for node in nodes {
        let name = node.get("name").and_then(|n| n.as_str()).unwrap_or("?");
        let empty = Vec::new();
        let services = node
            .get("services")
            .and_then(|services| services.as_array())
            .unwrap_or(&empty);
        for service in services {
            let plane = service
                .get("network_plane")
                .and_then(|plane| plane.as_str())
                .map(str::trim)
                .filter(|plane| !plane.is_empty());
            if plane.is_none() {
                let service_name = service.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                undeclared.push(format!("{name}/{service_name}"));
            }
        }
    }
    assert!(
        undeclared.is_empty(),
        "services without declared network_plane: {}",
        undeclared.join(", ")
    );
}
