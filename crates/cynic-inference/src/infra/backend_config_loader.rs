use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml;

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
        #[source]
        source: toml::de::Error,
    },

    #[error(
        "Backend '{name}' has sovereign=true but no base_url pointing to localhost — sovereign backends must be local"
    )]
    SovereignNotLocal { name: String },
}

/// Public path-based loader for chat endpoint resolution (multi-candidate).
pub fn load_inference_config_from_path_public(
    path: &Path,
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

/// Resolve the runtime registry path.
/// Lookup order:
/// 1. `CYNIC_REGISTRY_PATH` env var (explicit override)
/// 2. `/etc/cynic/registry.json` (system-wide)
/// 3. `$XDG_CONFIG_HOME/cynic/registry.json` (user)
pub(crate) fn resolve_runtime_registry_path(_override: Option<&str>) -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("CYNIC_REGISTRY_PATH") {
        return Ok(PathBuf::from(path));
    }

    let system_path = PathBuf::from("/etc/cynic/registry.json");
    if system_path.exists() {
        return Ok(system_path);
    }

    let user_path = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|base| base.join("cynic").join("registry.json"));

    if let Ok(ref p) = user_path {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    Ok(system_path)
}

pub(crate) fn load_file(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub(crate) fn resolve_registry_placeholders(mut content: String) -> String {
    let Ok(registry_path) = resolve_runtime_registry_path(None) else {
        return content;
    };

    let Ok(registry_content) = std::fs::read_to_string(&registry_path) else {
        return content;
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&registry_content) else {
        return content;
    };

    let Some(nodes) = json.get("nodes").and_then(|n| n.as_array()) else {
        return content;
    };

    for node in nodes {
        let Some(name) = node.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        let name_upper = if let Some(stripped) = name.strip_prefix("cynic-") {
            stripped.to_uppercase().replace('-', "_")
        } else {
            name.to_uppercase().replace('-', "_")
        };

        if let Some(local) = node
            .get("network")
            .and_then(|nw| nw.get("local"))
            .and_then(|t| t.as_str())
        {
            let key = format!("<LOCAL_{name_upper}>");
            content = content.replace(&key, local);
        }
    }

    content
}
