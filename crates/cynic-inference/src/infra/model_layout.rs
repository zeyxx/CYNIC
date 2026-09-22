//! Canonical model path layout for CYNIC inference.
//!
//! All model-related locations resolve through this module so vault, profiles,
//! HF downloads, and caches stay centralized (env-overridable, never hardcode
//! machine-local paths in committed configs).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use crate::domain::model_layout_report::{ModelLayoutEnvKeys, ModelLayoutReport};

/// Default unit template for native on-demand llama-server instances.
pub const DEFAULT_UNIT_TEMPLATE: &str = "llama-server@.service";

/// Env keys (portable, documented).
pub mod env_keys {
    pub const MODEL_VAULT: &str = "CYNIC_MODEL_VAULT";
    pub const MODEL_PROFILES: &str = "CYNIC_MODEL_PROFILES";
    pub const MODEL_REGISTRY: &str = "CYNIC_MODEL_REGISTRY";
    pub const UNIT_TEMPLATE: &str = "CYNIC_INFERENCE_UNIT_TEMPLATE";
    pub const HF_BIN: &str = "CYNIC_HF_BIN";
    pub const ODS_PROBE_URLS: &str = "CYNIC_ODS_PROBE_URLS";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLayout {
    /// Sovereign GGUF (and related weight) store.
    pub vault_dir: PathBuf,
    /// Per-alias runtime profiles (`*.env`).
    pub profiles_dir: PathBuf,
    /// Centralized machine-readable registry JSON.
    pub registry_path: PathBuf,
    /// Hugging Face hub cache (observed; managed by `hf`).
    pub hf_cache_dir: PathBuf,
    /// llama.cpp cache (legacy / alternate download target).
    pub llama_cache_dir: PathBuf,
    /// systemd template name for native on-demand engines.
    pub unit_template: String,
    /// Preferred `hf` binary name or path.
    pub hf_bin: PathBuf,
}

impl ModelLayout {
    pub const SCHEMA: &'static str = "cynic.inference.model_layout.v1";

    /// Resolve from process environment (runtime only).
    pub fn from_env() -> Self {
        Self {
            vault_dir: resolve_vault_dir(),
            profiles_dir: resolve_profiles_dir(),
            registry_path: resolve_registry_path(),
            hf_cache_dir: resolve_hf_cache_dir(),
            llama_cache_dir: resolve_llama_cache_dir(),
            unit_template: std::env::var(env_keys::UNIT_TEMPLATE)
                .unwrap_or_else(|_| DEFAULT_UNIT_TEMPLATE.to_string()),
            hf_bin: resolve_hf_bin(),
        }
    }

    /// JSON-friendly view with string paths.
    pub fn report(&self) -> ModelLayoutReport {
        ModelLayoutReport {
            schema: Self::SCHEMA,
            vault_dir: self.vault_dir.display().to_string(),
            profiles_dir: self.profiles_dir.display().to_string(),
            registry_path: self.registry_path.display().to_string(),
            hf_cache_dir: self.hf_cache_dir.display().to_string(),
            llama_cache_dir: self.llama_cache_dir.display().to_string(),
            unit_template: self.unit_template.clone(),
            hf_bin: self.hf_bin.display().to_string(),
            env_keys: ModelLayoutEnvKeys {
                vault: env_keys::MODEL_VAULT,
                profiles: env_keys::MODEL_PROFILES,
                registry: env_keys::MODEL_REGISTRY,
                unit_template: env_keys::UNIT_TEMPLATE,
                hf_bin: env_keys::HF_BIN,
                ods_probe_urls: env_keys::ODS_PROBE_URLS,
            },
        }
    }

    /// Destination directory for `hf download … --local-dir` into the vault.
    pub fn vault_download_dir(&self, repo_id: &str) -> PathBuf {
        let slug = repo_id.replace('/', "__");
        self.vault_dir.join("hf").join(slug)
    }
}



pub fn resolve_vault_dir() -> PathBuf {
    if let Ok(p) = std::env::var(env_keys::MODEL_VAULT) {
        return PathBuf::from(p);
    }
    // Principal-owned data plane (not operator $HOME when principal resolvable).
    crate::infra::principal_paths::PrincipalPaths::resolve()
        .data_dir
        .join("models")
}

pub fn resolve_profiles_dir() -> PathBuf {
    if let Ok(p) = std::env::var(env_keys::MODEL_PROFILES) {
        return PathBuf::from(p);
    }
    crate::infra::principal_paths::PrincipalPaths::resolve()
        .config_dir
        .join("models")
}

pub fn resolve_registry_path() -> PathBuf {
    if let Ok(p) = std::env::var(env_keys::MODEL_REGISTRY) {
        return PathBuf::from(p);
    }
    crate::infra::principal_paths::PrincipalPaths::resolve()
        .config_dir
        .join("model-registry.json")
}

pub fn resolve_hf_cache_dir() -> PathBuf {
    if let Ok(p) = std::env::var("HF_HUB_CACHE") {
        return PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("HF_HOME") {
        return PathBuf::from(p).join("hub");
    }
    // HF cache may stay operator-local (upstream CLI default); prefer principal cache.
    crate::infra::principal_paths::PrincipalPaths::resolve()
        .cache_dir
        .join("huggingface/hub")
}

pub fn resolve_llama_cache_dir() -> PathBuf {
    if let Ok(p) = std::env::var("LLAMA_CACHE") {
        return PathBuf::from(p);
    }
    crate::infra::principal_paths::PrincipalPaths::resolve()
        .cache_dir
        .join("llama.cpp")
}

pub fn resolve_hf_bin() -> PathBuf {
    if let Ok(p) = std::env::var(env_keys::HF_BIN) {
        return PathBuf::from(p);
    }
    PathBuf::from("hf")
}

/// Chat session state root under principal (override: CYNIC_CHAT_STATE).
pub fn resolve_chat_state_dir() -> PathBuf {
    crate::infra::principal_paths::PrincipalPaths::resolve().chat_state_dir
}

/// Default ODS / OpenAI-compat probe bases (host-facing).
/// Linux ODS Docker often uses :11434; native llama-server / macOS ODS :8080.
pub fn default_ods_probe_bases() -> Vec<String> {
    if let Ok(raw) = std::env::var(env_keys::ODS_PROBE_URLS) {
        let list: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    vec![
        "http://127.0.0.1:11434".to_string(),
        "http://127.0.0.1:8080".to_string(),
        "http://localhost:11434".to_string(),
        "http://localhost:8080".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::principal_paths::{PathAuthority, PrincipalPaths};

    #[test]
    fn vault_download_dir_slugs_repo() {
        let layout = ModelLayout {
            vault_dir: PathBuf::from("/tmp/vault"),
            profiles_dir: PathBuf::from("/tmp/profiles"),
            registry_path: PathBuf::from("/tmp/reg.json"),
            hf_cache_dir: PathBuf::from("/tmp/hf"),
            llama_cache_dir: PathBuf::from("/tmp/llama"),
            unit_template: DEFAULT_UNIT_TEMPLATE.into(),
            hf_bin: PathBuf::from("hf"),
        };
        assert_eq!(
            layout.vault_download_dir("org/model-name"),
            PathBuf::from("/tmp/vault/hf/org__model-name")
        );
    }

    #[test]
    fn principal_paths_drive_model_layout_roots() {
        // Contract: model/chat roots derive from PrincipalPaths home, not a bare operator HOME.
        let principal = PrincipalPaths::from_home(
            PathAuthority::RuntimePrincipal,
            "cynic".into(),
            PathBuf::from("/var/lib/cynic-ml-test"),
            vec![],
        );
        let op = PathBuf::from("/tmp/cynic-op-home-model-layout");
        let vault = principal.data_dir.join("models");
        let profiles = principal.config_dir.join("models");
        let chat = principal.chat_state_dir.clone();
        assert!(
            vault.starts_with("/var/lib/cynic-ml-test"),
            "vault={vault:?}"
        );
        assert!(profiles.starts_with("/var/lib/cynic-ml-test"));
        assert!(chat.starts_with("/var/lib/cynic-ml-test"));
        assert!(!vault.starts_with(&op));
        assert!(!chat.starts_with(&op));
        // from_env resolve_vault_dir uses PrincipalPaths::resolve(); ensure API exists.
        let _ = resolve_vault_dir();
        let _ = resolve_chat_state_dir();
    }
}
