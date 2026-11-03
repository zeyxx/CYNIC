use std::path::Path;

use crate::mesh_plan::{MeshRegistry, generate_mesh_plan};

pub use crate::adapters::local_network_mesh_registry::LocalNetworkMeshRegistryReader;
pub use crate::ports::network_mesh_registry_port::NetworkMeshRegistryPort;

/// Load a mesh plan from a registry file on disk.
///
/// Canonical registry→mesh-plan loading entry point. Reads the registry JSON,
/// parses it into `MeshRegistry`, then delegates the pure plan calculation to
/// `generate_mesh_plan`. The returned plan keeps the `registry_path` the
/// generator supplies (the canonical default `"infra/registry.json"` unless the
/// generator overrides it), and honours `provider_override`.
///
/// Side effects are confined to std `fs` + serde; the calculation is pure.
pub fn load_mesh_plan(
    registry_path: &str,
    provider_override: Option<&str>,
) -> Result<crate::types::NetworkMeshPlan, String> {
    let registry_path_ref = Path::new(registry_path);
    let registry = crate::adapters::local_network_mesh_registry::LocalNetworkMeshRegistryReader
        .load_mesh_registry(registry_path_ref)?;
    generate_mesh_plan(&registry, provider_override)
}

/// Test/kani helper: load a plan from a substituted registry reader.
///
/// Mirrors the node-owned `build_network_mesh_plan_with_reader` which was moved
/// to `cynic-network` as part of making the network crate canonical for
/// registry→mesh-plan loading. The loading policy (read + parse, then
/// `generate_mesh_plan`) is unchanged from the node-owned implementation.
#[cfg(any(test, kani))]
pub fn load_mesh_plan_with_reader(
    registry_path: &str,
    provider_override: Option<&str>,
    registry_reader: &impl crate::ports::network_mesh_registry_port::NetworkMeshRegistryPort,
) -> Result<crate::types::NetworkMeshPlan, String> {
    let registry_path_ref = Path::new(registry_path);
    let registry = registry_reader.load_mesh_registry(registry_path_ref)?;
    let mut plan = generate_mesh_plan(&registry, provider_override)?;
    plan.registry_path = registry_path_ref.to_string_lossy().to_string();
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::network_mesh_registry_port::NetworkMeshRegistryPort;
    use std::collections::BTreeMap;
    use std::path::Path;

    struct FakeRegistryReader;

    impl NetworkMeshRegistryPort for FakeRegistryReader {
        fn load_mesh_registry(
            &self,
            _registry_path: &Path,
        ) -> Result<crate::mesh_plan::MeshRegistry, String> {
            Ok(crate::mesh_plan::MeshRegistry {
                network_policy: Some(crate::mesh_plan::NetworkPolicy {
                    canonical_service_plane: Some("labmesh".to_string()),
                    planes: BTreeMap::from([(
                        "labmesh".to_string(),
                        crate::mesh_plan::NetworkPlane {
                            cidr: Some("10.30.0.0/24".to_string()),
                            dns_zone: Some("labmesh.example".to_string()),
                            canonical: Some(true),
                            interface: None,
                        },
                    )]),
                    reservations: Vec::new(),
                    mesh: Some(crate::mesh_plan::MeshPolicy {
                        provider: Some("wireguard".to_string()),
                    }),
                }),
                nodes: vec![
                    crate::mesh_plan::RegistryNode {
                        id: None,
                        name: Some("node-a".to_string()),
                        role: Some("kernel".to_string()),
                        status: Some("active".to_string()),
                        network: BTreeMap::from([
                            ("labmesh".to_string(), "10.30.0.10".to_string()),
                            ("local".to_string(), "192.168.1.10".to_string()),
                        ]),
                        services: vec![crate::mesh_plan::RegistryService {
                            name: Some("ssh".to_string()),
                            port: Some(22),
                            network_plane: None,
                            protocol: None,
                        }],
                    },
                    crate::mesh_plan::RegistryNode {
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
        let plan = load_mesh_plan(&registry_path, None).expect("plan");
        assert_eq!(plan.schema, crate::types::NETWORK_MESH_PLAN_SCHEMA);
        assert_eq!(plan.status, "planned");
        assert_eq!(plan.desired_peer_count, 3);
        assert_eq!(plan.service_plane, "labmesh");
        assert_eq!(plan.dns_zone.as_deref(), Some("labmesh.example"));
        assert!(plan.routes.iter().any(|route| route.via_peer == "lab-kernel"));
        assert!(plan.dns.iter().any(|record| record.name == "kernel.labmesh.example"));
        assert!(!plan.can_apply);
    }

    #[test]
    fn plan_can_be_built_from_substituted_registry_port() {
        let plan = load_mesh_plan_with_reader(
            "/virtual/registry.json",
            None,
            &FakeRegistryReader,
        )
        .expect("plan");
        assert_eq!(plan.registry_path, "/virtual/registry.json");
        assert_eq!(plan.provider, "wireguard");
        assert_eq!(plan.desired_peer_count, 2);
        assert_eq!(plan.status, "planned");
        assert!(plan.routes.iter().all(|route| route.provider == "wireguard"));
    }
}
