//! Sovereign inference configuration loader.
//!
//! SSOT: `/etc/cynic/inference.toml`
//!
//! ## Lookup order
//! 1. `CYNIC_INFERENCE_CONFIG` env var (absolute path override)
//! 2. `/etc/cynic/inference.toml`
//!
//! ## API-key injection
//! Secrets are never stored in the TOML file. The loader reads the
//! well-known env var pattern `CYNIC_BACKEND_<NAME>_API_KEY` (name
//! upper-cased, hyphens → underscores) and injects it into `api_key`.
//! The TOML field `api_key` is kept for local-only overrides and test
//! fixtures; it takes lower priority than the env var.

use std::path::PathBuf;

use serde::Deserialize;

use crate::domain::backend_config::InferenceBackendConfig;

/// Top-level TOML document shape: `[[backend]]` array of tables.
#[derive(Debug, Deserialize)]
struct InferenceConfigFile {
    #[serde(default)]
    backend: Vec<InferenceBackendConfig>,
}

/// Everything the loader can report about where it loaded from.
#[derive(Debug, Clone)]
pub struct LoadedInferenceConfig {
    /// Path that was actually read (None if no file was found).
    pub path: Option<PathBuf>,
    /// Hydrated backend list, with env-injected API keys.
    pub backends: Vec<InferenceBackendConfig>,
}

/// All failure modes that can occur during config loading.
#[derive(Debug, thiserror::Error)]
pub enum InferenceConfigError {
    #[error("Config file not found — expected at {path}")]
    NotFound { path: PathBuf },

    #[error("Failed to read config file at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse TOML in {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error(
        "Backend '{name}' has sovereign=true but no base_url pointing to localhost — sovereign backends must be local"
    )]
    SovereignNotLocal { name: String },
}

/// Load and validate the inference config from disk.
///
/// Returns `Err(InferenceConfigError::NotFound)` when no config file exists,
/// so callers can distinguish "not configured yet" from "misconfigured".
pub fn load_inference_config() -> Result<LoadedInferenceConfig, InferenceConfigError> {
    let path = resolve_config_path();
    load_inference_config_from_path(path, |env_key| std::env::var(env_key).ok())
}

/// Public path-based loader for chat endpoint resolution (multi-candidate).
pub fn load_inference_config_from_path_public(
    path: &std::path::Path,
) -> Result<LoadedInferenceConfig, InferenceConfigError> {
    load_inference_config_from_path(path.to_path_buf(), |env_key| std::env::var(env_key).ok())
}

fn load_inference_config_from_path(
    path: PathBuf,
    api_key_for_env: impl Fn(&str) -> Option<String>,
) -> Result<LoadedInferenceConfig, InferenceConfigError> {
    if !path.exists() {
        return Err(InferenceConfigError::NotFound { path });
    }

    let raw = std::fs::read_to_string(&path).map_err(|source| InferenceConfigError::Io {
        path: path.clone(),
        source,
    })?;

    let mut doc: InferenceConfigFile =
        toml::from_str(&raw).map_err(|source| InferenceConfigError::Parse {
            path: path.clone(),
            source,
        })?;

    // Inject API keys from env vars (higher priority than TOML field).
    for backend in &mut doc.backend {
        let env_key = format!(
            "CYNIC_BACKEND_{}_API_KEY",
            backend.name.to_uppercase().replace('-', "_")
        );
        if let Some(key) = api_key_for_env(&env_key) {
            backend.api_key = Some(key);
        }
    }

    // Validate sovereign flag: a sovereign backend must bind to localhost.
    for backend in &doc.backend {
        if backend.sovereign {
            let url = backend.base_url.to_lowercase();
            let is_local = url.contains("localhost")
                || url.contains("127.0.0.")
                || url.starts_with("http://[::1]");
            if !is_local {
                return Err(InferenceConfigError::SovereignNotLocal {
                    name: backend.name.clone(),
                });
            }
        }
    }

    Ok(LoadedInferenceConfig {
        path: Some(path),
        backends: doc.backend,
    })
}

/// SSOT: `/etc/cynic/inference.toml`
const DEFAULT_INFERENCE_CONFIG_PATH: &str = "/etc/cynic/inference.toml";

/// Determine the config path, honouring only an explicit env-var override.
pub(crate) fn resolve_config_path() -> PathBuf {
    resolve_config_path_inner(std::env::var("CYNIC_INFERENCE_CONFIG").ok())
}

fn resolve_config_path_inner(override_path: Option<String>) -> PathBuf {
    if let Some(override_path) = override_path {
        return PathBuf::from(override_path);
    }
    PathBuf::from(DEFAULT_INFERENCE_CONFIG_PATH)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write `content` to a uniquely-named file in `/tmp` and return its path.
    /// The file is NOT automatically cleaned up — tests run in CI ephemeral envs.
    fn write_tmp(tag: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cynic_inference_test_{tag}_{}.toml",
            std::process::id()
        ));
        std::fs::write(&path, content).expect("write tmp file");
        path
    }

    #[test]
    fn not_found_when_config_points_to_nonexistent_file() {
        let result = load_inference_config_from_path(
            PathBuf::from("/tmp/cynic_test_nonexistent_inference_888.toml"),
            |_| None,
        );

        assert!(
            matches!(result, Err(InferenceConfigError::NotFound { .. })),
            "expected NotFound, got {result:?}"
        );
    }

    #[test]
    fn default_config_path_uses_installed_runtime_ssot() {
        assert_eq!(
            resolve_config_path_inner(None),
            std::path::PathBuf::from(DEFAULT_INFERENCE_CONFIG_PATH)
        );
    }

    #[test]
    fn explicit_config_override_is_honored() {
        assert_eq!(
            resolve_config_path_inner(Some("/tmp/cynic-inference.toml".to_string())),
            std::path::PathBuf::from("/tmp/cynic-inference.toml")
        );
    }

    #[test]
    fn parses_minimal_sovereign_backend() {
        let toml = r#"
[[backend]]
name = "llama-local"
base_url = "http://localhost:8080/v1"
model = "qwen2.5-coder-7b"
sovereign = true
"#;
        let path = write_tmp("sovereign", toml);
        let loaded = load_inference_config_from_path(path.clone(), |_| None).expect("should parse");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.backends.len(), 1);
        let b = &loaded.backends[0];
        assert_eq!(b.name, "llama-local");
        assert!(b.sovereign);
        assert_eq!(b.timeout_secs, 120); // default
    }

    #[test]
    fn parses_fleet_node_override_if_present() {
        let toml = r#"
[[backend]]
name = "qwen-node"
base_url = "http://10.0.0.20:8080/v1"
model = "qwen3-0.6b"
fleet_node = "cynic-host-gpu"
"#;
        let path = write_tmp("fleet", toml);
        let loaded = load_inference_config_from_path(path.clone(), |_| None).expect("should parse");
        let _ = std::fs::remove_file(&path);

        let b = &loaded.backends[0];
        assert_eq!(b.fleet_node.as_deref(), Some("cynic-host-gpu"));
    }

    #[test]
    fn rejects_sovereign_with_non_local_url() {
        let toml = r#"
[[backend]]
name = "cloud-wrongly-flagged"
base_url = "https://api.example.com/v1"
model = "some-model"
sovereign = true
"#;
        let path = write_tmp("bad_sovereign", toml);
        let result = load_inference_config_from_path(path.clone(), |_| None);
        let _ = std::fs::remove_file(&path);

        assert!(
            matches!(result, Err(InferenceConfigError::SovereignNotLocal { .. })),
            "expected SovereignNotLocal, got {result:?}"
        );
    }

    #[test]
    fn env_var_api_key_overrides_toml() {
        let toml = r#"
[[backend]]
name = "cloud-backend"
base_url = "http://localhost:9999/v1"
model = "test-model"
api_key = "from-toml"
sovereign = true
"#;
        let path = write_tmp("apikey", toml);
        let loaded = load_inference_config_from_path(path.clone(), |env_key| {
            (env_key == "CYNIC_BACKEND_CLOUD_BACKEND_API_KEY").then(|| "from-env".to_string())
        })
        .expect("should parse");

        let _ = std::fs::remove_file(&path);

        assert_eq!(
            loaded.backends[0].api_key.as_deref(),
            Some("from-env"),
            "env var should override TOML api_key"
        );
    }
}
