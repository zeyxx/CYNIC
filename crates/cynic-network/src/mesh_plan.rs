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

    let mut peers = Vec::new();
    let mut services = Vec::new();

    for node in &registry.nodes {
        if let Some(node_id) = &node.id {
            peers.push(NetworkMeshPeer {
                node_id: node_id.clone(),
                address: format!("peer:{}", node_id),
                allowed_ips: vec!["10.10.10.0/24".to_string()],
                endpoint: None,
            });

            for svc in &node.services {
                if let (Some(name), Some(port)) = (&svc.name, svc.port) {
                    services.push(NetworkMeshService {
                        name: name.clone(),
                        node_id: node_id.clone(),
                        port,
                        protocol: "tcp".to_string(),
                    });
                }
            }
        }
    }

    Ok(NetworkMeshPlan {
        schema: NETWORK_MESH_PLAN_SCHEMA.to_string(),
        service_plane,
        provider,
        peers,
        routes: vec![NetworkMeshRoute {
            destination: "10.10.10.0/24".to_string(),
            gateway: None,
            metric: Some(100),
        }],
        dns_records: vec![],
        acl_rules: vec![NetworkMeshAclRule {
            source: "10.10.10.0/24".to_string(),
            destination: "10.10.10.0/24".to_string(),
            action: "accept".to_string(),
        }],
        unknowns: vec![],
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
        assert_eq!(plan.peers.len(), 1);
        assert_eq!(plan.peers[0].node_id, "cortex");
        assert_eq!(plan.services.len(), 1);
        assert_eq!(plan.services[0].name, "cynic-kernel");
    }
}
