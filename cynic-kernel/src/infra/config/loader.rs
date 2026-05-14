//! Unified kernel configuration loader — single source of truth (SSOT).
//!
//! Consolidates config from 5 sources with explicit priority:
//! 1. CLI arguments / systemd config (highest)
//! 2. Environment variables (CYNIC_*, SURREALDB_*, GEMINI_*, etc.)
//! 3. TOML file (backends.toml)
//! 4. Filesystem defaults (project_root discovery, domain prompts)
//! 5. Embedded/hardcoded defaults (lowest)
//!
//! Returns a single KernelConfig struct that represents complete system state at boot.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Complete kernel configuration — all config from all 5 sources unified.
/// Produced by `KernelConfigLoader::load()`. Immutable after boot.
#[derive(Debug, Clone)]
pub struct KernelConfig {
    // ── Paths & Discovery ──
    pub project_root: PathBuf,
    pub backends_toml_path: PathBuf,
    pub config_dir: PathBuf,

    // ── Storage (SurrealDB) ──
    pub storage_url: String,
    pub storage_namespace: String,
    pub storage_database: String,
    pub storage_user: Option<String>,
    pub storage_pass: Option<String>,

    // ── Backend configs (Dogs) ──
    pub backend_configs: Vec<super::BackendConfig>,
    pub dog_thresholds: super::DogThresholds,

    // ── REST API & Networking ──
    pub rest_addr: String,
    pub rest_port: u16,
    pub api_key: String,
    pub api_key_cortex: Option<String>,
    pub api_key_internal: Option<String>,
    pub api_key_organ: Option<String>,
    pub cors_origins: Vec<String>,
    pub allow_open_api: bool,

    // ── Domain Prompts ──
    pub domain_prompts: HashMap<String, String>,

    // ── External APIs ──
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
    pub helius_api_key: Option<String>,
    pub helius_rpc_url: String,
    pub sovereign_api_key: Option<String>,
    pub agentmail_api_key: Option<String>,

    // ── Inference & Embedding ──
    pub summarizer_backend: Option<String>,
    pub summarizer_url: String,
    pub summarizer_timeout_secs: u64,
    pub embed_model: String,
    pub embed_url: String,
    pub embed_port: u16,

    // ── Directories & Paths ──
    pub models_dir: PathBuf,
    pub scripts_dir: PathBuf,
    pub curation_dir: PathBuf,
    pub backup_dir: PathBuf,

    // ── Slack & Alerts ──
    pub slack_webhook: Option<String>,

    // ── Additional flags ──
    pub force_reprobe: bool,
}

impl KernelConfig {
    /// Load kernel configuration from all 5 sources (SSOT).
    /// Priority: CLI > env > TOML > filesystem > embedded defaults.
    ///
    /// # Arguments
    /// * `force_reprobe` - Override hardware detection (from --reset flag)
    ///
    /// # Returns
    /// A fully populated KernelConfig, or Err if critical config is missing/invalid.
    pub fn load(force_reprobe: bool) -> Result<Self, String> {
        let loader = KernelConfigLoader::new(force_reprobe);
        loader.load()
    }
}

/// Placeholder resolution from fleet.toml.
/// Maps <TAILSCALE_CORE>, <TAILSCALE_GPU>, etc. to actual Tailscale IPs.
#[derive(Debug, Clone)]
struct FleetResolver {
    /// Map: placeholder name (e.g., "TAILSCALE_GPU") → IP address
    placeholders: HashMap<String, String>,
}

impl FleetResolver {
    /// Load fleet.toml and build placeholder map.
    /// Returns empty resolver if fleet.toml missing (non-fatal).
    fn load(fleet_path: &Path) -> Self {
        let mut placeholders = HashMap::new();

        if !fleet_path.exists() {
            return Self { placeholders };
        }

        if let Ok(content) = std::fs::read_to_string(fleet_path) {
            // Simple TOML parsing for [machine.NAME] sections.
            // Regex-free: split by [machine.* and extract tailscale_ip
            for line in content.lines() {
                if let Some(machine_name) = line
                    .strip_prefix("[machine.")
                    .and_then(|s| s.strip_suffix("]"))
                {
                    // Next lines contain tailscale_ip = "100.x.x.x"
                    // Find the IP in the next few lines
                    let rest = content
                        .split_once(&format!("[machine.{machine_name}]"))
                        .and_then(|(_, after)| after.split_once("[machine.").or(Some((after, ""))))
                        .map(|(section, _)| section)
                        .unwrap_or("");

                    for ip_line in rest.lines() {
                        if let Some(ip_str) = ip_line
                            .trim()
                            .strip_prefix("tailscale_ip = \"")
                            .and_then(|s| s.strip_suffix("\""))
                        {
                            let placeholder_key =
                                format!("TAILSCALE_{}", machine_name.to_uppercase());
                            placeholders.insert(placeholder_key, ip_str.to_string());
                            break;
                        }
                    }
                }
            }
        }

        Self { placeholders }
    }

    /// Resolve a string containing <PLACEHOLDER> tokens.
    /// Returns modified string with all placeholders replaced.
    fn resolve(&self, input: &str) -> String {
        let mut result = input.to_string();
        for (key, value) in &self.placeholders {
            let placeholder = format!(
                "<TAILSCALE_{}>",
                key.strip_prefix("TAILSCALE_").unwrap_or(key)
            );
            result = result.replace(&placeholder, value);
        }
        result
    }
}

/// Internal loader — orchestrates reading from all 5 sources.
struct KernelConfigLoader {
    force_reprobe: bool,
}

impl KernelConfigLoader {
    fn new(force_reprobe: bool) -> Self {
        Self { force_reprobe }
    }

    fn load(self) -> Result<KernelConfig, String> {
        // 1. Discover paths (env var > git > cwd)
        let project_root = Self::discover_project_root();
        let config_dir = Self::discover_config_dir();
        let backends_toml_path = config_dir.join("backends.toml");

        // 2. Load fleet.toml for placeholder resolution (KERNEL Phase 2)
        let fleet_toml_path = config_dir.join("fleet.toml");
        let fleet_resolver = FleetResolver::load(&fleet_toml_path);

        // 3. Load TOML (source 3) and resolve placeholders
        let backends_path_ref = &backends_toml_path;
        let storage_config = super::load_storage_config(backends_path_ref);
        let mut backend_configs = if backends_toml_path.exists() {
            super::load_backends(backends_path_ref)
        } else {
            super::load_backends_from_env()
        };

        // Resolve <TAILSCALE_*> placeholders in backend URLs
        for cfg in &mut backend_configs {
            cfg.base_url = fleet_resolver.resolve(&cfg.base_url);
        }

        let dog_thresholds = super::load_dog_thresholds(backends_path_ref);
        let _organ_remediations = super::load_organ_remediations(backends_path_ref);

        // 3. Load environment variables (source 2) — override TOML values
        let rest_addr =
            std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| "127.0.0.1:3030".to_string());

        let (_rest_host, rest_port) = Self::parse_addr(&rest_addr)?;

        // 4. Load domain prompts (source 4: filesystem > embedded)
        let domain_prompts = super::load_domain_prompts(&project_root);

        // 5. Assemble final config
        Ok(KernelConfig {
            project_root,
            backends_toml_path,
            config_dir,

            storage_url: std::env::var("SURREALDB_URL").unwrap_or(storage_config.url),
            storage_namespace: std::env::var("SURREALDB_NS").unwrap_or(storage_config.namespace),
            storage_database: std::env::var("SURREALDB_DB").unwrap_or(storage_config.database),
            storage_user: std::env::var("SURREALDB_USER").ok(),
            storage_pass: std::env::var("SURREALDB_PASS").ok(),

            backend_configs,
            dog_thresholds,

            rest_addr,
            rest_port,
            api_key: std::env::var("CYNIC_API_KEY").unwrap_or_default(),
            api_key_cortex: std::env::var("CYNIC_API_KEY_CORTEX").ok(),
            api_key_internal: std::env::var("CYNIC_API_KEY_INTERNAL").ok(),
            api_key_organ: std::env::var("CYNIC_API_KEY_ORGAN").ok(),
            cors_origins: std::env::var("CYNIC_CORS_ORIGINS")
                .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
                .unwrap_or_default(),
            allow_open_api: std::env::var("CYNIC_ALLOW_OPEN_API")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),

            domain_prompts,

            gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
            gemini_model: std::env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-2.5-flash".to_string()),
            helius_api_key: std::env::var("HELIUS_API_KEY").ok(),
            helius_rpc_url: std::env::var("HELIUS_RPC_URL")
                .unwrap_or_else(|_| "https://api.helius.xyz".to_string()),
            sovereign_api_key: std::env::var("SOVEREIGN_API_KEY").ok(),
            agentmail_api_key: std::env::var("AGENTMAIL_API_KEY").ok(),

            summarizer_backend: std::env::var("CYNIC_SUMMARIZER_BACKEND").ok(),
            summarizer_url: std::env::var("CYNIC_SUMMARIZER_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8000".to_string()),
            summarizer_timeout_secs: std::env::var("CYNIC_SUMMARIZER_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            embed_model: std::env::var("CYNIC_EMBED_MODEL")
                .unwrap_or_else(|_| "qwen3-embed".to_string()),
            embed_url: std::env::var("CYNIC_EMBED_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8082".to_string()),
            embed_port: std::env::var("CYNIC_EMBED_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8082),

            models_dir: PathBuf::from(
                std::env::var("CYNIC_MODELS_DIR")
                    .unwrap_or_else(|_| "/home/user/.ollama/models".to_string()),
            ),
            scripts_dir: PathBuf::from(
                std::env::var("CYNIC_SCRIPTS").unwrap_or_else(|_| "/home/user/bin".to_string()),
            ),
            curation_dir: PathBuf::from(std::env::var("CYNIC_CURATION_DIR").unwrap_or_else(|_| {
                format!(
                    "{}/.cynic/curation",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                )
            })),
            backup_dir: PathBuf::from(std::env::var("CYNIC_BACKUP_DIR").unwrap_or_else(|_| {
                format!(
                    "{}/.cynic/backups",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                )
            })),

            slack_webhook: std::env::var("CYNIC_SLACK_WEBHOOK").ok(),

            force_reprobe: self.force_reprobe,
        })
    }

    fn discover_project_root() -> PathBuf {
        std::env::var("CYNIC_PROJECT_ROOT")
            .map(PathBuf::from)
            .ok()
            .or_else(|| {
                std::process::Command::new("git")
                    .args(["rev-parse", "--show-toplevel"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| PathBuf::from(s.trim()))
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    fn discover_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cynic")
    }

    fn parse_addr(addr: &str) -> Result<(String, u16), String> {
        if let Some((host, port_str)) = addr.rsplit_once(':') {
            let port = port_str
                .parse::<u16>()
                .map_err(|e| format!("invalid port in REST_ADDR: {addr}: {e}"))?;
            Ok((host.to_string(), port))
        } else {
            Ok((addr.to_string(), 3030))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_addr_with_port() {
        let (host, port) = KernelConfigLoader::parse_addr("127.0.0.1:3030").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 3030);
    }

    #[test]
    fn parse_addr_hostname_with_port() {
        let (host, port) = KernelConfigLoader::parse_addr("localhost:8080").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);
    }

    #[test]
    fn parse_addr_ipv6() {
        let (host, port) = KernelConfigLoader::parse_addr("[::1]:3030").unwrap();
        assert_eq!(host, "[::1]");
        assert_eq!(port, 3030);
    }

    #[test]
    fn parse_addr_no_port() {
        let (host, port) = KernelConfigLoader::parse_addr("127.0.0.1").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 3030);
    }

    #[test]
    fn parse_addr_invalid_port() {
        assert!(KernelConfigLoader::parse_addr("127.0.0.1:invalid").is_err());
    }
}
