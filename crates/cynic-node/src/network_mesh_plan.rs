use std::path::Path;

use crate::adapters::local_network_mesh_registry::LocalNetworkMeshRegistryReader;
use crate::ports::network_mesh_registry_port::NetworkMeshRegistryPort;
#[cfg(test)]
use cynic_protocol::NETWORK_MESH_PLAN_SCHEMA;
use cynic_protocol::NetworkMeshPlan;

pub(crate) mod model;
#[cfg(kani)]
mod verification;

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

/// Test/kani engine: delegates to pure cynic-network domain engine.
#[cfg(any(test, kani))]
fn build_network_mesh_plan_with_reader(
    registry_path: &str,
    provider_override: Option<&str>,
    registry_reader: &impl NetworkMeshRegistryPort,
) -> Result<NetworkMeshPlan, String> {
    let registry_path_ref = Path::new(registry_path);
    let registry = registry_reader.load_mesh_registry(registry_path_ref)?;
    let net_registry: cynic_network::MeshRegistry =
        serde_json::from_value(serde_json::to_value(&registry).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

    let mut plan = cynic_network::generate_mesh_plan(&net_registry, provider_override)?;
    plan.registry_path = registry_path_ref.to_string_lossy().to_string();
    Ok(plan)
}

#[cfg(any(test, kani))]
fn address_plane(policy: Option<&model::NetworkPolicy>, service_plane: &str) -> String {
    let net_policy: Option<cynic_network::NetworkPolicy> =
        policy.and_then(|p| serde_json::from_value(serde_json::to_value(p).ok()?).ok());
    cynic_network::address_plane(net_policy.as_ref(), service_plane)
}

#[cfg(any(test, kani))]
fn default_dns_names(id: &str, plane: Option<&model::NetworkPlane>) -> Vec<String> {
    let net_plane: Option<cynic_network::NetworkPlane> =
        plane.and_then(|p| serde_json::from_value(serde_json::to_value(p).ok()?).ok());
    cynic_network::default_dns_names(id, net_plane.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::network_mesh_registry_port::NetworkMeshRegistryPort;
    use std::collections::BTreeMap;
    use std::path::Path;

    struct FakeRegistryReader;

    impl NetworkMeshRegistryPort for FakeRegistryReader {
        fn load_mesh_registry(&self, _registry_path: &Path) -> Result<model::MeshRegistry, String> {
            Ok(model::MeshRegistry {
                network_policy: Some(model::NetworkPolicy {
                    canonical_service_plane: Some("labmesh".to_string()),
                    planes: BTreeMap::from([(
                        "labmesh".to_string(),
                        model::NetworkPlane {
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
                    model::RegistryNode {
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
                    model::RegistryNode {
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
