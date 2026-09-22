//! Pure value objects for model layout reporting.
//!
//! These types are serializable snapshots with no I/O. The runtime resolution
//! lives in `crate::infra::model_layout`.

use serde::{Deserialize, Serialize};

/// JSON-friendly model layout snapshot (pure value object).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLayoutReport {
    pub schema: &'static str,
    pub vault_dir: String,
    pub profiles_dir: String,
    pub registry_path: String,
    pub hf_cache_dir: String,
    pub llama_cache_dir: String,
    pub unit_template: String,
    pub hf_bin: String,
    pub env_keys: ModelLayoutEnvKeys,
}

/// Documented environment keys (pure static strings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLayoutEnvKeys {
    pub vault: &'static str,
    pub profiles: &'static str,
    pub registry: &'static str,
    pub unit_template: &'static str,
    pub hf_bin: &'static str,
    pub ods_probe_urls: &'static str,
}
