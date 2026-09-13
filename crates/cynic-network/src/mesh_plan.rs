//! Pure Network Mesh Plan Calculation Domain
//! Clean-room extracted from cynic-node to cynic-network per Foundation §481 (SoC).
//! Pure transformation: (MeshRegistry) -> NetworkMeshPlan. Zero I/O, zero network calls.

use cynic_protocol::{
    NETWORK_MESH_PLAN_SCHEMA, NetworkMeshAclRule, NetworkMeshPeer, NetworkMeshPlan,
    NetworkMeshRoute, NetworkMeshService,
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
    pub status: Option<String>,
    #[serde(default)]
    pub services: Vec<RegistryService>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryService {
    pub name: Option<String>,
    pub port: Option<u16>,
}

/// Pure calculation of a NetworkMeshPlan from a parsed MeshRegistry structure.
pub fn generate_mesh_plan(
    registry: &MeshRegistry,
    provider_override: Option<&str>,
) -> Result<NetworkMeshPlan, String> {
    let policy = registry.network_policy.as_ref();
    let service_plane = policy
        .and_then(|p| p.canonical_service_plane.clone())
        .unwrap_or_else(|| "synapse".to_string());

    let provider = provider_override
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            policy
                .and_then(|p| p.mesh.as_ref())
                .and_then(|m| m.provider.clone())
        })
        .unwrap_or_else(|| "wireguard".to_string());

    let mut desired_peers = Vec::new();

    for node in &registry.nodes {
        if let Some(node_id) = &node.id {
            let mut services = Vec::new();
            for svc in &node.services {
                if let Some(name) = &svc.name {
                    services.push(NetworkMeshService {
                        name: name.clone(),
                        port: svc.port,
                        protocol: "tcp".to_string(),
                    });
                }
            }

            desired_peers.push(NetworkMeshPeer {
                id: node_id.clone(),
                role: None,
                status: node.status.clone(),
                address: format!("peer:{}", node_id),
                local_address: None,
                dns_names: vec![format!("{}.synapse.cynic", node_id)],
                services,
            });
        }
    }

    let peer_count = desired_peers.len();

    Ok(NetworkMeshPlan {
        schema: NETWORK_MESH_PLAN_SCHEMA,
        status: "planned",
        provider,
        registry_path: "infra/registry.json".to_string(),
        service_plane,
        cidr: Some("10.10.10.0/24".to_string()),
        dns_zone: Some("synapse.cynic".to_string()),
        desired_peer_count: peer_count,
        desired_peers,
        routes: vec![NetworkMeshRoute {
            destination: "10.10.10.0/24".to_string(),
            via_peer: "cortex".to_string(),
            provider: "wireguard".to_string(),
        }],
        dns: vec![],
        acl: vec![NetworkMeshAclRule {
            id: "acl-synapse-mesh".to_string(),
            source: "10.10.10.0/24".to_string(),
            destination: "10.10.10.0/24".to_string(),
            port: None,
            protocol: "any".to_string(),
            action: "accept".to_string(),
        }],
        actuator_required: true,
        can_apply: true,
        apply_requires: vec![],
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
                status: Some("observed".to_string()),
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
