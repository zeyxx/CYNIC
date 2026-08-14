use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::domain::network_mesh_apply::{
    MeshApplyBundle, MeshBundleManifest, MeshPlanFile, validate_mesh_apply_bundle,
};
use crate::ports::network_mesh_bundle_port::NetworkMeshBundlePort;

const MANIFEST_FILENAME: &str = "manifest.json";
const PLAN_FILENAME: &str = "mesh-plan.json";

pub(crate) struct LocalNetworkMeshBundleReader;

impl NetworkMeshBundlePort for LocalNetworkMeshBundleReader {
    fn load_mesh_bundle(&self, bundle_dir: &Path) -> Result<MeshApplyBundle, String> {
        let manifest: MeshBundleManifest = read_json(bundle_dir.join(MANIFEST_FILENAME))?;
        let plan: MeshPlanFile = read_json(bundle_dir.join(PLAN_FILENAME))?;
        validate_mesh_apply_bundle(&manifest, &plan)?;
        for file in &manifest.bundle_files {
            validate_relative_bundle_path(file)?;
        }
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

fn validate_relative_bundle_path(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.trim().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "mesh bundle contains unsafe relative path: {value}"
        ));
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T, String> {
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::validate_relative_bundle_path;

    #[test]
    fn bundle_paths_stay_below_the_declared_directory() {
        assert!(validate_relative_bundle_path("wireguard/node-a.conf.tpl").is_ok());
        assert!(validate_relative_bundle_path("../outside").is_err());
        assert!(validate_relative_bundle_path("/absolute").is_err());
    }
}
