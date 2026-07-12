use std::path::Path;

use crate::network_mesh_plan::model::MeshRegistry;

pub(crate) trait NetworkMeshRegistryPort {
    fn load_mesh_registry(&self, registry_path: &Path) -> Result<MeshRegistry, String>;
}
