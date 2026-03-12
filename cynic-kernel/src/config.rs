//! Backend configuration — loaded from backends.toml or env vars.
//! Lives in infrastructure layer. NEVER imported by domain core.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub auth_style: AuthStyle,
}

#[derive(Debug, Clone)]
pub enum AuthStyle {
    Bearer,
    QueryParam(String),
    None,
}

#[derive(Deserialize)]
struct BackendsFile {
    backend: std::collections::HashMap<String, BackendEntry>,
}

#[derive(Deserialize)]
struct BackendEntry {
    base_url: String,
    api_key_env: Option<String>,
    model: String,
    auth_style: Option<String>,
}

/// Load backend configs from TOML file. Resolves api_key_env to actual env var values.
pub fn load_backends(path: &Path) -> Vec<BackendConfig> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[config] Cannot read {}: {}", path.display(), e);
            return Vec::new();
        }
    };

    let file: BackendsFile = match toml::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[config] Invalid TOML in {}: {}", path.display(), e);
            return Vec::new();
        }
    };

    file.backend
        .into_iter()
        .filter_map(|(name, entry)| {
            let api_key = entry.api_key_env.as_ref().and_then(|env_name| {
                match std::env::var(env_name) {
                    Ok(val) if !val.is_empty() => Some(val),
                    Ok(_) => {
                        eprintln!("[config] {} env var is empty, skipping backend '{}'", env_name, name);
                        None
                    }
                    Err(_) => {
                        eprintln!("[config] {} not set, skipping backend '{}'", env_name, name);
                        None
                    }
                }
            });

            // If api_key_env was specified but not resolved, skip this backend
            if entry.api_key_env.is_some() && api_key.is_none() {
                return None;
            }

            let auth_style = match entry.auth_style.as_deref() {
                Some("bearer") | None => AuthStyle::Bearer,
                Some("none") => AuthStyle::None,
                Some(other) if other.starts_with("query:") => {
                    AuthStyle::QueryParam(other.trim_start_matches("query:").to_string())
                }
                Some(other) => {
                    eprintln!("[config] Unknown auth_style '{}' for backend '{}', defaulting to bearer", other, name);
                    AuthStyle::Bearer
                }
            };

            Some(BackendConfig {
                name,
                base_url: entry.base_url,
                api_key,
                model: entry.model,
                auth_style,
            })
        })
        .collect()
}

/// Fallback: build configs from legacy env vars (backward compat).
pub fn load_backends_from_env() -> Vec<BackendConfig> {
    let mut configs = Vec::new();

    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        let model = std::env::var("GEMINI_MODEL")
            .unwrap_or_else(|_| "gemini-2.5-flash".to_string());
        configs.push(BackendConfig {
            name: "gemini".to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
            api_key: Some(api_key),
            model,
            auth_style: AuthStyle::Bearer,
        });
    }

    configs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_valid_toml() {
        let toml_content = r#"
[backend.local]
base_url = "http://localhost:8080/v1"
model = "phi-3-mini"
auth_style = "none"
"#;
        let dir = std::env::temp_dir().join("cynic_config_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("backends.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let configs = load_backends(&path);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "local");
        assert!(configs[0].api_key.is_none());
        assert!(matches!(configs[0].auth_style, AuthStyle::None));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn missing_file_returns_empty() {
        let configs = load_backends(Path::new("/nonexistent/backends.toml"));
        assert!(configs.is_empty());
    }
}
