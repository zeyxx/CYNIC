use std::collections::BTreeMap;
use std::path::Path;

use crate::adapters::local_network_mesh_registry::LocalNetworkMeshRegistryReader;
use crate::ports::network_mesh_registry_port::NetworkMeshRegistryPort;
use cynic_protocol::{
    NETWORK_MESH_PLAN_SCHEMA, NetworkMeshAclRule, NetworkMeshDnsRecord, NetworkMeshPeer,
    NetworkMeshPlan, NetworkMeshRoute, NetworkMeshService,
};

pub(crate) mod model;
#[cfg(kani)]
mod verification;

use model::{NetworkPlane, NetworkPolicy, RegistryNode};

pub(crate) fn build_network_mesh_plan(
    registry_path: &str,
    provider_override: Option<&str>,
) -> Result<NetworkMeshPlan, String> {
    let registry_path_ref = Path::new(registry_path);
    let registry: model::MeshRegistry =
        LocalNetworkMeshRegistryReader.load_mesh_registry(registry_path_ref)?;

    // Delegate to pure cynic-network domain engine
    let net_registry: cynic_network::MeshRegistry =
        serde_json::from_value(serde_json::to_value(&registry).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

    cynic_network::generate_mesh_plan(&net_registry, provider_override)
}

/// Test/kani-only legacy engine: the live path delegates to cynic-network.
/// Kept for the unit tests below and the kani proofs in verification.rs.
#[cfg(any(test, kani))]
fn build_network_mesh_plan_with_reader(
    registry_path: &str,
    provider_override: Option<&str>,
    registry_reader: &impl NetworkMeshRegistryPort,
) -> Result<NetworkMeshPlan, String> {
    let registry_path_ref = Path::new(registry_path);
    let registry = registry_reader.load_mesh_registry(registry_path_ref)?;
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
        registry_path: registry_path_ref.to_string_lossy().to_string(),
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

#[cfg(any(test, kani))]
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

#[cfg(any(test, kani))]
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

#[cfg(any(test, kani))]
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

#[cfg(any(test, kani))]
fn default_dns_names(id: &str, plane: Option<&NetworkPlane>) -> Vec<String> {
    plane
        .and_then(|plane| plane.dns_zone.as_ref())
        .map(|zone| vec![format!("{id}.{zone}")])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::network_mesh_registry_port::NetworkMeshRegistryPort;
    use std::path::Path;

    struct FakeRegistryReader;

    impl NetworkMeshRegistryPort for FakeRegistryReader {
        fn load_mesh_registry(&self, _registry_path: &Path) -> Result<model::MeshRegistry, String> {
            Ok(model::MeshRegistry {
                network_policy: Some(NetworkPolicy {
                    canonical_service_plane: Some("labmesh".to_string()),
                    planes: BTreeMap::from([(
                        "labmesh".to_string(),
                        NetworkPlane {
                            cidr: Some("10.30.0.0/24".to_string()),
                            dns_zone: Some("labmesh.example".to_string()),
                            canonical: Some(true),
                        },
                    )]),
                    reservations: Vec::new(),
                    mesh: Some(model::MeshPolicy {
                        provider: Some("wireguard".to_string()),
                    }),
                }),
                nodes: vec![
                    RegistryNode {
                        id: None,
                        name: Some("node-a".to_string()),
                        role: Some("kernel".to_string()),
                        status: Some("active".to_string()),
                        network: BTreeMap::from([
                            ("labmesh".to_string(), "10.30.0.10".to_string()),
                            ("local".to_string(), "192.168.1.10".to_string()),
                        ]),
                        services: vec![model::RegistryService {
                            name: Some("ssh".to_string()),
                            port: Some(22),
                        }],
                    },
                    RegistryNode {
                        id: None,
                        name: Some("node-b".to_string()),
                        role: Some("worker".to_string()),
                        status: Some("active".to_string()),
                        network: BTreeMap::from([
                            ("labmesh".to_string(), "10.30.0.11".to_string()),
                            ("local".to_string(), "192.168.1.11".to_string()),
                        ]),
                        services: Vec::new(),
                    },
                ],
            })
        }
    }

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

    #[test]
    fn plan_can_be_built_from_substituted_registry_port() {
        let plan = build_network_mesh_plan_with_reader(
            "/virtual/registry.json",
            None,
            &FakeRegistryReader,
        )
        .expect("plan");
        assert_eq!(plan.registry_path, "/virtual/registry.json");
        assert_eq!(plan.provider, "wireguard");
        assert_eq!(plan.desired_peer_count, 2);
        assert_eq!(plan.status, "planned");
        assert!(
            plan.routes
                .iter()
                .all(|route| route.provider == "wireguard")
        );
    }
}
