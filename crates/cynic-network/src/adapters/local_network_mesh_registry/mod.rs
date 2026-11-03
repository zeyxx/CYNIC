use std::fs;
use std::path::Path;

use crate::mesh_plan::MeshRegistry;
use crate::ports::network_mesh_registry_port::NetworkMeshRegistryPort;

/// Local filesystem reader that parses a `MeshRegistry` JSON file.
///
/// Mirrors the node-owned `cynic_node::adapters::local_network_mesh_registry::LocalNetworkMeshRegistryReader`
/// which was moved to `cynic-network` as part of making the network crate canonical for
/// registry→mesh-plan loading. The parsing/loading policy (read-as-string, then
/// `serde_json::from_str`) is unchanged from the node-owned implementation.
pub struct LocalNetworkMeshRegistryReader;

impl NetworkMeshRegistryPort for LocalNetworkMeshRegistryReader {
    fn load_mesh_registry(&self, registry_path: &Path) -> Result<MeshRegistry, String> {
        let raw = fs::read_to_string(registry_path).map_err(|error| {
            format!(
                "failed to read registry {}: {error}",
                registry_path.display()
            )
        })?;
        serde_json::from_str(&raw).map_err(|error| {
            format!(
                "failed to parse registry {}: {error}",
                registry_path.display()
            )
        })
    }
}
