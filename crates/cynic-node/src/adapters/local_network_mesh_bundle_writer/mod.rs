use std::fs;
use std::path::Path;

use crate::ports::network_mesh_bundle_writer_port::NetworkMeshBundleWriterPort;

pub(crate) struct LocalNetworkMeshBundleWriter;

impl NetworkMeshBundleWriterPort for LocalNetworkMeshBundleWriter {
    fn create_dir_all(&self, path: &Path) -> Result<(), String> {
        fs::create_dir_all(path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))
    }

    fn write_string(&self, path: &Path, contents: &str) -> Result<(), String> {
        fs::write(path, contents)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))
    }
}
