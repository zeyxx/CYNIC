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
