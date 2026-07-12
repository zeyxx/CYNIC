use std::path::Path;

pub(crate) trait NetworkMeshBundleWriterPort {
    fn create_dir_all(&self, path: &Path) -> Result<(), String>;
    fn write_string(&self, path: &Path, contents: &str) -> Result<(), String>;
}
