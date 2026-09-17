//! Pure Network Mesh Plan Calculation Domain
//! Clean-room extracted from cynic-node to cynic-network per Foundation §481 (SoC).
//! Pure transformation: (MeshRegistry) -> NetworkMeshPlan. Zero I/O, zero network calls.

use cynic_protocol::{
    NETWORK_MESH_PLAN_SCHEMA, NetworkMeshAclRule, NetworkMeshDnsRecord, NetworkMeshPeer,
    NetworkMeshPlan, NetworkMeshRoute, NetworkMeshService,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct MeshRegistry {
    #[serde(default)]
    pub network_policy: Option<NetworkPolicy>,
    #[serde(default)]
    pub nodes: Vec<RegistryNode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkPolicy {
    #[serde(default)]
    pub canonical_service_plane: Option<String>,
    #[serde(default)]
    pub planes: BTreeMap<String, NetworkPlane>,
    #[serde(default)]
    pub reservations: Vec<NetworkReservation>,
    #[serde(default)]
    pub mesh: Option<MeshPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MeshPolicy {
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkPlane {
    #[serde(default)]
    pub cidr: Option<String>,
    #[serde(default)]
    pub dns_zone: Option<String>,
    #[serde(default)]
    pub canonical: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkReservation {
    pub name: Option<String>,
    #[serde(default)]
    pub dns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryNode {
    pub id: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub network: BTreeMap<String, String>,
    #[serde(default)]
    pub services: Vec<RegistryService>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryService {
    pub name: Option<String>,
    pub port: Option<u16>,
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

/// Pure calculation of a NetworkMeshPlan from a parsed MeshRegistry structure.
pub fn generate_mesh_plan(
    registry: &MeshRegistry,
    provider_override: Option<&str>,
) -> Result<NetworkMeshPlan, String> {
    let policy = registry.network_policy.as_ref();
    let service_plane = policy
        .and_then(|policy| policy.canonical_service_plane.clone())
        .unwrap_or_else(|| "synapse".to_string());
    let addr_plane = address_plane(policy, &service_plane);
    let plane = policy.and_then(|policy| policy.planes.get(&addr_plane));
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
        .filter_map(|node| peer_from_node(node, &addr_plane, &reservation_dns, plane))
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
        registry_path: "infra/registry.json".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_pure_mesh_plan_from_registry() {
        let registry = MeshRegistry {
            network_policy: Some(NetworkPolicy {
                canonical_service_plane: Some("synapse".to_string()),
                planes: BTreeMap::new(),
                reservations: vec![],
                mesh: Some(MeshPolicy {
                    provider: Some("wireguard".to_string()),
                }),
            }),
            nodes: vec![RegistryNode {
                id: Some("cortex".to_string()),
                name: Some("cortex".to_string()),
                role: None,
                status: Some("observed".to_string()),
                network: BTreeMap::from([("synapse".to_string(), "10.10.10.100".to_string())]),
                services: vec![RegistryService {
                    name: Some("cynic-kernel".to_string()),
                    port: Some(3030),
                }],
            }],
        };

        let plan = generate_mesh_plan(&registry, None).expect("mesh plan generation ok");
        assert_eq!(plan.service_plane, "synapse");
        assert_eq!(plan.provider, "wireguard");
        assert_eq!(plan.desired_peers.len(), 1);
        assert_eq!(plan.desired_peers[0].id, "cortex");
        assert_eq!(plan.desired_peers[0].services.len(), 1);
        assert_eq!(plan.desired_peers[0].services[0].name, "cynic-kernel");
    }
}
