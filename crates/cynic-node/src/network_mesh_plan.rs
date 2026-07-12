use std::collections::BTreeMap;

use cynic_protocol::{
    NETWORK_MESH_PLAN_SCHEMA, NetworkMeshAclRule, NetworkMeshDnsRecord, NetworkMeshPeer,
    NetworkMeshPlan, NetworkMeshRoute, NetworkMeshService,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Registry {
    #[serde(default)]
    network_policy: Option<NetworkPolicy>,
    #[serde(default)]
    nodes: Vec<RegistryNode>,
}

#[derive(Debug, Deserialize)]
struct NetworkPolicy {
    #[serde(default)]
    canonical_service_plane: Option<String>,
    #[serde(default)]
    planes: BTreeMap<String, NetworkPlane>,
    #[serde(default)]
    reservations: Vec<NetworkReservation>,
    #[serde(default)]
    mesh: Option<MeshPolicy>,
}

#[derive(Debug, Deserialize)]
struct MeshPolicy {
    #[serde(default)]
    provider: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NetworkPlane {
    #[serde(default)]
    cidr: Option<String>,
    #[serde(default)]
    dns_zone: Option<String>,
    #[serde(default)]
    canonical: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct NetworkReservation {
    name: Option<String>,
    #[serde(default)]
    dns: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryNode {
    id: Option<String>,
    name: Option<String>,
    role: Option<String>,
    status: Option<String>,
    #[serde(default)]
    network: BTreeMap<String, String>,
    #[serde(default)]
    services: Vec<RegistryService>,
}

#[derive(Debug, Deserialize)]
struct RegistryService {
    name: Option<String>,
    port: Option<u16>,
}

pub(crate) fn build_network_mesh_plan(
    registry_path: &str,
    provider_override: Option<&str>,
) -> Result<NetworkMeshPlan, String> {
    let raw = std::fs::read_to_string(registry_path)
        .map_err(|error| format!("failed to read registry {registry_path}: {error}"))?;
    let registry: Registry = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {registry_path}: {error}"))?;
    let policy = registry.network_policy.as_ref();
    let service_plane = policy
        .and_then(|policy| policy.canonical_service_plane.clone())
        .unwrap_or_else(|| "synapse".to_string());
    let address_plane = address_plane(policy, &service_plane);
    let plane = policy.and_then(|policy| policy.planes.get(&address_plane));
    let provider = provider_override
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            policy
                .and_then(|policy| policy.mesh.as_ref())
                .and_then(|mesh| mesh.provider.clone())
        })
        .unwrap_or_else(|| "wireguard".to_string());
    let reservation_dns = reservation_dns_by_name(policy);
    let desired_peers = registry
        .nodes
        .iter()
        .filter_map(|node| peer_from_node(node, &address_plane, &reservation_dns, plane))
        .collect::<Vec<_>>();
    let routes = desired_peers
        .iter()
        .map(|peer| NetworkMeshRoute {
            destination: format!("{}/32", peer.address),
            via_peer: peer.id.clone(),
            provider: provider.clone(),
        })
        .collect::<Vec<_>>();
    let dns = desired_peers
        .iter()
        .flat_map(|peer| {
            peer.dns_names
                .iter()
                .map(|name| NetworkMeshDnsRecord {
                    name: name.clone(),
                    address: peer.address.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let acl = desired_peers
        .iter()
        .flat_map(|peer| {
            peer.services.iter().map(|service| NetworkMeshAclRule {
                id: format!("allow:{}:{}", peer.id, service.name),
                source: "mesh".to_string(),
                destination: peer.id.clone(),
                port: service.port,
                protocol: service.protocol.clone(),
                action: "allow".to_string(),
            })
        })
        .collect::<Vec<_>>();

    Ok(NetworkMeshPlan {
        schema: NETWORK_MESH_PLAN_SCHEMA,
        status: if desired_peers.len() > 1 {
            "planned"
        } else {
            "insufficient_peers"
        },
        provider,
        registry_path: registry_path.to_string(),
        service_plane,
        cidr: plane.and_then(|plane| plane.cidr.clone()),
        dns_zone: plane.and_then(|plane| plane.dns_zone.clone()),
        desired_peer_count: desired_peers.len(),
        desired_peers,
        routes,
        dns,
        acl,
        actuator_required: true,
        can_apply: false,
        apply_requires: vec![
            "network_actuator_bundle",
            "bounded_root_helper",
            "provider_specific_secret_material",
            "runtime_observation_after_apply",
        ],
    })
}

fn address_plane(policy: Option<&NetworkPolicy>, service_plane: &str) -> String {
    let Some(policy) = policy else {
        return service_plane.to_string();
    };
    if policy.planes.contains_key(service_plane) {
        return service_plane.to_string();
    }
    policy
        .planes
        .iter()
        .find_map(|(name, plane)| (plane.canonical == Some(true)).then(|| name.clone()))
        .unwrap_or_else(|| "synapse".to_string())
}

fn peer_from_node(
    node: &RegistryNode,
    service_plane: &str,
    reservation_dns: &BTreeMap<String, Vec<String>>,
    plane: Option<&NetworkPlane>,
) -> Option<NetworkMeshPeer> {
    let id = node
        .name
        .clone()
        .or_else(|| node.id.clone())
        .filter(|value| !value.is_empty())?;
    let address = node.network.get(service_plane).cloned()?;
    let dns_names = reservation_dns
        .get(&id)
        .cloned()
        .unwrap_or_else(|| default_dns_names(&id, plane));
    let local_address = node
        .network
        .get("local")
        .cloned()
        .filter(|value| value != &address);
    let services = node
        .services
        .iter()
        .filter_map(|service| {
            service.name.as_ref().map(|name| NetworkMeshService {
                name: name.clone(),
                port: service.port,
                protocol: "tcp".to_string(),
            })
        })
        .collect::<Vec<_>>();

    Some(NetworkMeshPeer {
        id,
        role: node.role.clone(),
        status: node.status.clone(),
        address,
        local_address,
        dns_names,
        services,
    })
}

fn reservation_dns_by_name(policy: Option<&NetworkPolicy>) -> BTreeMap<String, Vec<String>> {
    policy
        .map(|policy| {
            policy
                .reservations
                .iter()
                .filter_map(|reservation| {
                    let name = reservation.name.clone()?;
                    Some((name, reservation.dns.clone()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn default_dns_names(id: &str, plane: Option<&NetworkPlane>) -> Vec<String> {
    plane
        .and_then(|plane| plane.dns_zone.as_ref())
        .map(|zone| vec![format!("{id}.{zone}")])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_distributed_mesh_plan_from_lab_registry() {
        let registry_path = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let plan = build_network_mesh_plan(&registry_path, None).expect("plan");
        assert_eq!(plan.schema, NETWORK_MESH_PLAN_SCHEMA);
        assert_eq!(plan.status, "planned");
        assert_eq!(plan.desired_peer_count, 3);
        assert_eq!(plan.service_plane, "labmesh");
        assert_eq!(plan.dns_zone.as_deref(), Some("labmesh.example"));
        assert!(
            plan.routes
                .iter()
                .any(|route| route.via_peer == "lab-kernel")
        );
        assert!(
            plan.dns
                .iter()
                .any(|record| record.name == "kernel.labmesh.example")
        );
        assert!(!plan.can_apply);
    }
}
