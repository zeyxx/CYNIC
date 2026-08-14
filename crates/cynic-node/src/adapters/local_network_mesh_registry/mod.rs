use std::fs;
use std::path::Path;

use crate::network_mesh_plan::model::MeshRegistry;
use crate::ports::network_mesh_registry_port::NetworkMeshRegistryPort;

pub(crate) struct LocalNetworkMeshRegistryReader;

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
