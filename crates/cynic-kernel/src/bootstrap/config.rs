use std::sync::Arc;

use crate::infra::config::{self, BackendConfig, DogThresholds, StorageConfig};

#[derive(Debug, Clone)]
pub struct KernelConfigBundle {
    pub backends_path: std::path::PathBuf,
    pub storage_config: StorageConfig,
    pub backend_configs: Vec<BackendConfig>,
    pub system_contract: crate::domain::contract::SystemContract,
    pub dog_thresholds: Arc<DogThresholds>,
}

pub fn resolve_kernel_backends_path() -> std::path::PathBuf {
    resolve_kernel_backends_path_with_env(|name| std::env::var(name).ok())
}

fn resolve_kernel_backends_path_with_env(
    mut env_var: impl FnMut(&str) -> Option<String>,
) -> std::path::PathBuf {
    if let Some(path) = env_var("CYNIC_BACKENDS_TOML") {
        return std::path::PathBuf::from(path);
    }

    if let Some(path) = env_var("CYNIC_BACKENDS_CONFIG") {
        return std::path::PathBuf::from(path);
    }

    if let Some(path) = env_var("CYNIC_INFERENCE_CONFIG") {
        return std::path::PathBuf::from(path);
    }

    if let Some(config_root) = env_var("CYNIC_CONFIG_ROOT") {
        let config_root = std::path::PathBuf::from(config_root);
        let inference_candidate = config_root.join("inference.toml");
        if inference_candidate.exists() {
            return inference_candidate;
        }

        let backends_candidate = config_root.join("backends.toml");
        if backends_candidate.exists() {
            return backends_candidate;
        }
    }

    if let Some(runtime_home) = env_var("CYNIC_RUNTIME_HOME") {
        let runtime_home = std::path::PathBuf::from(runtime_home);
        let inference_candidate = runtime_home.join("inference.toml");
        if inference_candidate.exists() {
            return inference_candidate;
        }

        let backends_candidate = runtime_home.join("backends.toml");
        if backends_candidate.exists() {
            return backends_candidate;
        }
    }

    for fallback in [
        std::path::PathBuf::from("/etc/cynic/inference.toml"),
        std::path::PathBuf::from("/etc/cynic/backends.toml"),
        std::path::PathBuf::from("/var/lib/cynic/inference.toml"),
        std::path::PathBuf::from("/var/lib/cynic/backends.toml"),
    ] {
        if fallback.exists() {
            return fallback;
        }
    }

    std::path::PathBuf::from("/etc/cynic/inference.toml")
}

pub fn load_kernel_config_bundle() -> KernelConfigBundle {
    let backends_path = resolve_kernel_backends_path();
    let storage_config = config::load_storage_config(&backends_path);
    crate::klog!("[Ring 1] Config: {}", backends_path.display());
    crate::klog!(
        "[Ring 1] Storage: url={} ns={} db={}",
        storage_config.url,
        storage_config.namespace,
        storage_config.database
    );

    let backend_configs = if backends_path.exists() {
        crate::klog!("[Ring 2] Loading backends from {}", backends_path.display());
        config::load_backends(&backends_path)
    } else {
        crate::klog!("[Ring 2] No backends.toml found, using env var fallback");
        config::load_backends_from_env()
    };

    let system_contract = config::load_system_contract(&backends_path);
    let expected_count = system_contract.expected_count();
    let expected_dogs = system_contract.expected_dogs().to_vec();
    crate::klog!(
        "[Ring 2] SystemContract: {} expected Dogs {:?}",
        expected_count,
        expected_dogs
    );

    let dog_thresholds = Arc::new(config::load_dog_thresholds(&backends_path));
    crate::klog!(
        "[Ring 2] Dog thresholds: circuit open={} failure_threshold={} error patterns: quota({}) transient({}) critical({})",
        dog_thresholds.circuit.open_duration_secs,
        dog_thresholds.circuit.open_on_consecutive_failures,
        dog_thresholds.error_detection.quota_patterns.len(),
        dog_thresholds.error_detection.transient_patterns.len(),
        dog_thresholds.error_detection.critical_patterns.len()
    );

    let backend_configs = filter_disabled_backends(backend_configs, &dog_thresholds);

    KernelConfigBundle {
        backends_path,
        storage_config,
        backend_configs,
        system_contract,
        dog_thresholds,
    }
}

fn filter_disabled_backends(
    backend_configs: Vec<BackendConfig>,
    dog_thresholds: &DogThresholds,
) -> Vec<BackendConfig> {
    backend_configs
        .into_iter()
        .filter(|config| {
            if let Some(threshold) = dog_thresholds.dogs.get(&config.name).filter(|t| !t.enabled) {
                crate::klog!(
                    "[Ring 2] Backend '{}' DISABLED — skipping (reason: {:?})",
                    config.name,
                    threshold.skip_reason
                );
                return false;
            }
            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::resolve_kernel_backends_path_with_env;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn resolve_kernel_backends_path_follows_kernel_precedence_and_fallback() {
        let explicit = NamedTempFile::new().unwrap().into_temp_path();
        let fallback_toml = NamedTempFile::new().unwrap().into_temp_path();
        let config_root = tempfile::tempdir().unwrap();
        let runtime_home = tempfile::tempdir().unwrap();

        let config_root_path = config_root.path().join("backends.toml");
        let runtime_path = runtime_home.path().join("backends.toml");
        std::fs::write(&config_root_path, "").unwrap();
        std::fs::write(&runtime_path, "").unwrap();

        let explicit_path = explicit.to_path_buf();
        let config_override_path = fallback_toml.to_path_buf();

        let mut env_map = HashMap::new();
        env_map.insert(
            "CYNIC_BACKENDS_TOML",
            Some(explicit_path.to_string_lossy().to_string()),
        );
        env_map.insert(
            "CYNIC_BACKENDS_CONFIG",
            Some(config_override_path.to_string_lossy().to_string()),
        );
        env_map.insert(
            "CYNIC_CONFIG_ROOT",
            Some(config_root.path().to_string_lossy().to_string()),
        );
        env_map.insert(
            "CYNIC_RUNTIME_HOME",
            Some(runtime_home.path().to_string_lossy().to_string()),
        );

        let resolve = |values: &HashMap<&str, Option<String>>| {
            resolve_kernel_backends_path_with_env(|key| {
                values.get(key).and_then(|val| val.as_ref()).cloned()
            })
        };

        assert_eq!(resolve(&env_map), explicit_path);

        env_map.insert("CYNIC_BACKENDS_TOML", None);
        assert_eq!(resolve(&env_map), config_override_path);

        env_map.insert("CYNIC_BACKENDS_CONFIG", None);
        env_map.insert(
            "CYNIC_INFERENCE_CONFIG",
            Some("/tmp/cynic-inference-override.toml".to_string()),
        );
        assert_eq!(
            resolve(&env_map),
            PathBuf::from("/tmp/cynic-inference-override.toml")
        );

        env_map.insert("CYNIC_INFERENCE_CONFIG", None);
        assert_eq!(resolve(&env_map), config_root_path);

        std::fs::remove_file(&config_root_path).unwrap();
        assert_eq!(resolve(&env_map), runtime_path);

        std::fs::remove_file(&runtime_path).unwrap();
        let config_root_fallback = tempfile::tempdir().unwrap();
        let runtime_home_fallback = tempfile::tempdir().unwrap();
        env_map.insert(
            "CYNIC_CONFIG_ROOT",
            Some(config_root_fallback.path().to_string_lossy().to_string()),
        );
        env_map.insert("CYNIC_INFERENCE_CONFIG", None);
        env_map.insert(
            "CYNIC_RUNTIME_HOME",
            Some(runtime_home_fallback.path().to_string_lossy().to_string()),
        );
        assert_eq!(
            resolve(&env_map),
            PathBuf::from("/etc/cynic/inference.toml")
        );

        explicit.close().ok();
        fallback_toml.close().ok();
    }
}
