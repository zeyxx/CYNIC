//! Zone configuration file loader — I/O adapter for ZoneConfig.
//!
//! FCIS: file reading lives in infra/, not domain/.
//! The pure ZoneConfig::from_json stays in domain/.

use crate::domain::zones::ZoneConfig;

/// Load a ZoneConfig from a JSON file, normalizing relative paths
/// to absolute based on the file's parent directory.
pub fn load_zone_config_from_file(path: &str) -> Result<ZoneConfig, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut config = ZoneConfig::from_json(&content)?;

    if let Some(base_dir) = std::path::Path::new(path).parent() {
        config.normalize_prefixes_to_absolute(base_dir);
    }

    Ok(config)
}
