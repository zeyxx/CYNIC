//! Pure Network Mesh Plan Calculation Domain
//! Clean-room extracted from cynic-node to cynic-network per Foundation §481 (SoC).
//! Pure transformation: (MeshRegistry) -> NetworkMeshPlan. Zero I/O, zero network calls.

use crate::types::{
    NETWORK_MESH_PLAN_SCHEMA, NetworkMeshAclRule, NetworkMeshDnsRecord, NetworkMeshPeer,
    NetworkMeshPlan, NetworkMeshRoute, NetworkMeshService,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The canonical service plane name (RFC-NET-001 §2).
const CANONICAL_SERVICE_PLANE: &str = crate::plane_vocabulary::CANONICAL_PLANE_KEYS[0];

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshRegistry {
    #[serde(default)]
    pub network_policy: Option<NetworkPolicy>,
    #[serde(default)]
    pub nodes: Vec<RegistryNode>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshPolicy {
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkPlane {
    #[serde(default)]
    pub cidr: Option<String>,
    #[serde(default)]
    pub dns_zone: Option<String>,
    #[serde(default)]
    pub canonical: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkReservation {
    pub name: Option<String>,
    #[serde(default)]
    pub dns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryService {
    pub name: Option<String>,
    pub port: Option<u16>,
}

/// Resolve which registry key carries node addresses.
///
/// The declared `canonical_service_plane` wins. When it names no declared
/// plane the value is untrustworthy, so the canonical `service` plane is
/// resolved through the shared vocabulary instead of a hardcoded legacy
/// literal. A lab may still declare its own plane name (`labmesh`), which
/// passes through untouched.
pub fn address_plane(policy: Option<&NetworkPolicy>, service_plane: &str) -> String {
    let Some(policy) = policy else {
        return service_plane.to_string();
    };
    if policy.planes.contains_key(service_plane) {
        return service_plane.to_string();
    }
    if policy.planes.contains_key(CANONICAL_SERVICE_PLANE) {
        return CANONICAL_SERVICE_PLANE.to_string();
    }
    policy
        .planes
        .iter()
        .find_map(|(name, plane)| (plane.canonical == Some(true)).then(|| name.clone()))
        .unwrap_or_else(|| CANONICAL_SERVICE_PLANE.to_string())
}

/// Resolve a node's address on a plane: canonical trio key first, then the
/// pre-migration legacy aliases.
///
/// DECLARED DRIFT (removal condition): delete the alias arm once every
/// admitted registry carries trio keys on all node `network` maps. The
/// canonical `infra/registry.json` path is proven alias-free by
/// `tests/mesh_plan_registry_proof.rs`; lab fixtures keep exercising the
/// alias arm under their local dialect on purpose.
pub fn resolve_plane_address(
    network: &BTreeMap<String, String>,
    plane_name: &str,
) -> Option<String> {
    if let Some(address) = network.get(plane_name).cloned() {
        return Some(address);
    }
    let legacy_key = if plane_name == Plane::Service.name() {
        Some("synapse")
    } else if plane_name == Plane::Admin.name() {
        Some("management")
    } else {
        None
    };
    legacy_key.and_then(|key| network.get(key).cloned())
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

pub fn default_dns_names(id: &str, plane: Option<&NetworkPlane>) -> Vec<String> {
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
    // Resolve through the shared vocabulary: `addr_plane` is a canonical plane
    // name, but the registry may still carry its legacy key, so a plain
    // `network.get(service_plane)` would silently drop the node.
    let address =
        crate::plane_vocabulary::plane_address_in_map(&node.network, service_plane)?.to_string();
    let dns_names = reservation_dns
        .get(&id)
        .cloned()
        .unwrap_or_else(|| default_dns_names(&id, plane));
    // `local` is the node underlay / L2 adjacency address (spec §5.1), read under
    // its own key rather than as a plane. It is omitted when it duplicates the
    // plane address, which is what a single-plane node looks like.
    let local_address = crate::plane_vocabulary::underlay_address_in_map(&node.network)
        .filter(|value| *value != address.as_str())
        .map(ToString::to_string);
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
        .unwrap_or_else(|| CANONICAL_SERVICE_PLANE.to_string());
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
                canonical_service_plane: Some("service".to_string()),
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
                network: BTreeMap::from([("service".to_string(), "10.10.10.100".to_string())]),
                services: vec![RegistryService {
                    name: Some("cynic-kernel".to_string()),
                    port: Some(3030),
                }],
            }],
        };

        let plan = generate_mesh_plan(&registry, None).expect("mesh plan generation ok");
        assert_eq!(plan.service_plane, "service");
        assert_eq!(plan.provider, "wireguard");
        assert_eq!(plan.desired_peers.len(), 1);
        assert_eq!(plan.desired_peers[0].id, "cortex");
        assert_eq!(plan.desired_peers[0].services.len(), 1);
        assert_eq!(plan.desired_peers[0].services[0].name, "cynic-kernel");
    }

    fn registry_with(network: &[(&str, &str)], declared_plane: Option<&str>) -> MeshRegistry {
        MeshRegistry {
            network_policy: Some(NetworkPolicy {
                canonical_service_plane: declared_plane.map(ToString::to_string),
                planes: BTreeMap::from([(
                    "service".to_string(),
                    NetworkPlane {
                        cidr: Some("10.10.10.0/24".to_string()),
                        dns_zone: Some("service.cynic".to_string()),
                        canonical: Some(true),
                    },
                )]),
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
                network: network
                    .iter()
                    .map(|(plane, address)| (plane.to_string(), address.to_string()))
                    .collect(),
                services: vec![],
            }],
        }
    }

    /// The regression T5 exists for: a canonical registry with no declared
    /// plane used to fall back to the hardcoded legacy key `synapse`, so the
    /// node resolved to None and vanished from the plan with no error.
    #[test]
    fn canonical_registry_without_declared_plane_resolves_the_node() {
        let registry = registry_with(&[("service", "10.10.10.100")], None);
        let plan = generate_mesh_plan(&registry, None).expect("plan");
        assert_eq!(plan.service_plane, "service");
        assert_eq!(plan.desired_peers.len(), 1);
        assert_eq!(plan.desired_peers[0].address, "10.10.10.100");
    }

    #[test]
    fn legacy_registry_key_still_resolves_the_node() {
        let registry = registry_with(&[("synapse", "10.10.10.100")], None);
        let plan = generate_mesh_plan(&registry, None).expect("plan");
        assert_eq!(plan.desired_peers.len(), 1);
        assert_eq!(plan.desired_peers[0].address, "10.10.10.100");
    }

    #[test]
    fn node_without_any_plane_address_is_excluded_not_fabricated() {
        let registry = registry_with(&[("admin", "192.168.100.1")], None);
        let plan = generate_mesh_plan(&registry, None).expect("plan");
        assert!(
            plan.desired_peers.is_empty(),
            "a node with no address on the resolved plane must be absent, never defaulted"
        );
    }

    #[test]
    fn underlay_is_reported_separately_and_dropped_when_redundant() {
        let with_underlay = registry_with(
            &[("service", "10.10.10.100"), ("local", "192.168.0.37")],
            None,
        );
        let plan = generate_mesh_plan(&with_underlay, None).expect("plan");
        assert_eq!(
            plan.desired_peers[0].local_address.as_deref(),
            Some("192.168.0.37")
        );

        // A single-plane node whose underlay equals its plane address has no
        // distinct underlay to report.
        let redundant = registry_with(
            &[("service", "10.10.10.100"), ("local", "10.10.10.100")],
            None,
        );
        let plan = generate_mesh_plan(&redundant, None).expect("plan");
        assert_eq!(plan.desired_peers[0].local_address, None);
    }

    /// A lab may declare its own plane name. That must keep working: the
    /// canonical fallback only applies when the declared name is absent.
    #[test]
    fn declared_custom_plane_name_is_preserved() {
        let mut registry = registry_with(&[("labmesh", "172.30.40.10")], Some("labmesh"));
        if let Some(policy) = registry.network_policy.as_mut() {
            policy.planes = BTreeMap::from([(
                "labmesh".to_string(),
                NetworkPlane {
                    cidr: Some("172.30.40.0/24".to_string()),
                    dns_zone: Some("labmesh.example".to_string()),
                    canonical: Some(true),
                },
            )]);
        }
        let plan = generate_mesh_plan(&registry, None).expect("plan");
        assert_eq!(plan.service_plane, "labmesh");
        assert_eq!(plan.desired_peers.len(), 1);
        assert_eq!(plan.desired_peers[0].address, "172.30.40.10");
    }
}
