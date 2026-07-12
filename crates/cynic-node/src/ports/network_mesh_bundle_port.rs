use std::path::Path;

use crate::domain::network_mesh_apply::MeshApplyBundle;

pub(crate) trait NetworkMeshBundlePort {
    fn load_mesh_bundle(&self, bundle_dir: &Path) -> Result<MeshApplyBundle, String>;
}
