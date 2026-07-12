use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::domain::network_mesh_apply::{MeshApplyBundle, MeshBundleManifest, MeshPlanFile};
use crate::ports::network_mesh_bundle_port::NetworkMeshBundlePort;

const MANIFEST_FILENAME: &str = "manifest.json";
const PLAN_FILENAME: &str = "mesh-plan.json";

pub(crate) struct LocalNetworkMeshBundleReader;

impl NetworkMeshBundlePort for LocalNetworkMeshBundleReader {
    fn load_mesh_bundle(&self, bundle_dir: &Path) -> Result<MeshApplyBundle, String> {
        let manifest: MeshBundleManifest = read_json(bundle_dir.join(MANIFEST_FILENAME))?;
        let plan: MeshPlanFile = read_json(bundle_dir.join(PLAN_FILENAME))?;
        let complete = manifest
            .bundle_files
            .iter()
            .all(|file| bundle_dir.join(file).is_file());

        Ok(MeshApplyBundle {
            manifest,
            plan,
            complete,
        })
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T, String> {
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}
