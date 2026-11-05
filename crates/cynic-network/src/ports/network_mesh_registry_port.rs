use std::path::Path;

use crate::mesh_plan::MeshRegistry;

/// Trait implemented by concrete readers that load a `MeshRegistry` from disk.
///
/// The node previously owned this boundary in `cynic_node::ports::network_mesh_registry_port`
/// and `cynic_node::adapters::local_network_mesh_registry`. Both moved to `cynic-network` so
/// the registry→mesh-plan loading contract lives entirely in the canonical network crate.
pub trait NetworkMeshRegistryPort {
    fn load_mesh_registry(&self, registry_path: &Path) -> Result<MeshRegistry, String>;
}
