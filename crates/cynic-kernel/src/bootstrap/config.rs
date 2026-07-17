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
    dirs::config_dir()
        .unwrap_or_else(|| {
            crate::klog!(
                "[Ring 1] dirs::config_dir() unavailable — falling back to current directory"
            );
            std::path::PathBuf::from(".")
        })
        .join("cynic")
        .join("backends.toml")
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
