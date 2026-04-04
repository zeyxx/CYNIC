//! Backend + storage configuration — loaded from backends.toml or env vars.
//! Lives in infrastructure layer. NEVER imported by domain core.

use serde::Deserialize;
use std::path::Path;

// ── STORAGE CONFIG ────────────────────────────────────────

/// SurrealDB connection config. Read from [storage] in backends.toml,
/// with env var fallbacks for URL and credentials.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub url: String,
    pub namespace: String,
    pub database: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8000".to_string(),
            namespace: "cynic".to_string(),
            database: "main".to_string(),
        }
    }
}

// ── BACKEND CONFIG ────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub auth_style: AuthStyle,
    /// Max context tokens this backend supports. 0 = unknown/unlimited.
    pub context_size: u32,
    /// HTTP + evaluation timeout in seconds. Default: 30. Sovereign CPU models need 60+.
    pub timeout_secs: u64,
    /// Max completion tokens per request. Default: 4096.
    pub max_tokens: u32,
    /// Sampling temperature. Default: 0.3.
    pub temperature: f32,
    /// Prepend /no_think to user prompts (disables thinking mode in Qwen3 family).
    /// Default: false. Set true for sovereign backends running thinking-capable models.
    pub disable_thinking: bool,
    /// Send `response_format: {"type": "json_object"}` to force valid JSON output.
    /// Default: false. Set true for llama-server backends where JSON reliability matters.
    /// Not all backends support this (cloud APIs may use different mechanisms).
    pub json_mode: bool,
    /// Cost per 1M input tokens in USD. 0.0 = free (sovereign, free tier).
    pub cost_input_per_mtok: f64,
    /// Cost per 1M output tokens in USD. 0.0 = free.
    pub cost_output_per_mtok: f64,
    /// Health URL — derived from base_url for backends with remediation or explicit health_url.
    /// None for cloud APIs (no health endpoint) — health loop skips them.
    pub health_url: Option<String>,
    /// Remediation config — optional. Only for backends that can be restarted.
    pub remediation: Option<BackendRemediation>,
}

/// Remediation config for a backend — how to restart it when the circuit breaker opens.
#[derive(Debug, Clone)]
pub struct BackendRemediation {
    /// SSH target, e.g. `"user@<TAILSCALE_NODE>"` or `"localhost"`
    pub node: String,
    /// Command to execute on the node to restart the service
    pub restart_command: String,
    /// Maximum recovery attempts before giving up (default: 3)
    pub max_retries: u32,
    /// Minimum seconds between restart attempts (default: 60)
    pub cooldown_secs: u64,
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
    storage: Option<StorageEntry>,
}

#[derive(Deserialize)]
struct StorageEntry {
    url: Option<String>,
    namespace: Option<String>,
    database: Option<String>,
}

#[derive(Deserialize)]
struct BackendEntry {
    base_url: String,
    api_key_env: Option<String>,
    model: String,
    auth_style: Option<String>,
    context_size: Option<u32>,
    timeout_secs: Option<u64>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    disable_thinking: Option<bool>,
    json_mode: Option<bool>,
    cost_input_per_mtok: Option<f64>,
    cost_output_per_mtok: Option<f64>,
    /// Explicit health URL — if omitted, derived from base_url.
    health_url: Option<String>,
    /// Inline remediation config.
    remediation: Option<RemediationEntry>,
}

#[derive(Deserialize)]
struct RemediationEntry {
    node: String,
    restart_command: String,
    max_retries: Option<u32>,
    cooldown_secs: Option<u64>,
}

/// Derive health URL from base_url: strip "/v1" suffix, append "/health".
fn derive_health_url(base_url: &str) -> String {
    let base = if let Some(stripped) = base_url.strip_suffix("/v1") {
        stripped
    } else {
        base_url.trim_end_matches('/')
    };
    format!("{base}/health")
}

/// Load backend configs from TOML file. Resolves api_key_env to actual env var values.
pub fn load_backends(path: &Path) -> Vec<BackendConfig> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "cannot read config file");
            return Vec::new();
        }
    };

    let file: BackendsFile = match toml::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "invalid TOML in config");
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
                        tracing::warn!(env = %env_name, backend = %name, "env var empty, skipping backend");
                        None
                    }
                    Err(_) => {
                        tracing::warn!(env = %env_name, backend = %name, "env var not set, skipping backend");
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
                    tracing::warn!(auth_style = %other, backend = %name, "unknown auth_style, defaulting to bearer");
                    AuthStyle::Bearer
                }
            };

            let remediation = entry.remediation.map(|r| BackendRemediation {
                node: r.node,
                restart_command: r.restart_command,
                max_retries: r.max_retries.unwrap_or(3),
                cooldown_secs: r.cooldown_secs.unwrap_or(60),
            });

            // health_url: explicit if provided, derived if remediation exists, None for cloud APIs
            let health_url = entry.health_url
                .or_else(|| remediation.as_ref().map(|_| derive_health_url(&entry.base_url)));

            Some(BackendConfig {
                name,
                base_url: entry.base_url,
                api_key,
                model: entry.model,
                auth_style,
                context_size: entry.context_size.unwrap_or(0),
                timeout_secs: entry.timeout_secs.unwrap_or(30),
                max_tokens: entry.max_tokens.unwrap_or(4096),
                temperature: entry.temperature.unwrap_or(0.3),
                disable_thinking: entry.disable_thinking.unwrap_or(false),
                json_mode: entry.json_mode.unwrap_or(false),
                cost_input_per_mtok: entry.cost_input_per_mtok.unwrap_or(0.0),
                cost_output_per_mtok: entry.cost_output_per_mtok.unwrap_or(0.0),
                health_url,
                remediation,
            })
        })
        .collect()
}

/// Load storage config from [storage] section in backends.toml.
/// Falls back to env vars, then defaults.
/// Priority: TOML > env var > default.
pub fn load_storage_config(path: &Path) -> StorageConfig {
    let defaults = StorageConfig::default();

    let from_toml = std::fs::read_to_string(path).ok()
        .and_then(|content| toml::from_str::<BackendsFile>(&content)
            .inspect_err(|e| tracing::warn!(path = %path.display(), error = %e, "backends.toml parse failed — falling back to env vars"))
            .ok())
        .and_then(|f| f.storage);

    let url = from_toml
        .as_ref()
        .and_then(|s| s.url.clone())
        .or_else(|| std::env::var("SURREALDB_URL").ok())
        .unwrap_or(defaults.url);

    let namespace = from_toml
        .as_ref()
        .and_then(|s| s.namespace.clone())
        .or_else(|| std::env::var("SURREALDB_NS").ok())
        .unwrap_or(defaults.namespace);

    let database = from_toml
        .as_ref()
        .and_then(|s| s.database.clone())
        .or_else(|| std::env::var("SURREALDB_DB").ok())
        .unwrap_or(defaults.database);

    StorageConfig {
        url,
        namespace,
        database,
    }
}

/// Fallback: build configs from legacy env vars (backward compat).
pub fn load_backends_from_env() -> Vec<BackendConfig> {
    let mut configs = Vec::new();

    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        let model =
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_string());
        let base_url = "https://generativelanguage.googleapis.com/v1beta/openai".to_string();
        configs.push(BackendConfig {
            name: "gemini".to_string(),
            base_url,
            api_key: Some(api_key),
            model,
            auth_style: AuthStyle::Bearer,
            context_size: 1_000_000,
            timeout_secs: 30,
            max_tokens: 4096,
            temperature: 0.3,
            disable_thinking: false,
            json_mode: false,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: None, // Cloud API — no health endpoint
            remediation: None,
        });
    }

    configs
}

/// Validate config at boot — probe health URLs, log warnings for unreachable backends.
/// Does NOT block boot — sovereign degradation is preferred over refusing to start.
pub async fn validate_config(configs: &[BackendConfig]) {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "HTTP client build failed (TLS?) — skipping health validation");
            return;
        }
    };

    for cfg in configs {
        let Some(ref health_url) = cfg.health_url else {
            klog!(
                "[config] — {} no health probe (cloud API, error-driven circuit breaker)",
                cfg.name
            );
            continue;
        };
        match client.get(health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                klog!("[config] ✓ {} health OK ({})", cfg.name, health_url);
                // RC3: Verify configured model is actually loaded (sovereign backends only)
                verify_model_loaded(&client, cfg).await;
            }
            Ok(resp) => {
                klog!(
                    "[config] ⚠ {} health returned {} ({})",
                    cfg.name,
                    resp.status(),
                    health_url
                );
            }
            Err(_) => {
                klog!(
                    "[config] ✗ {} UNREACHABLE at {} — will load anyway, health loop will recover",
                    cfg.name,
                    health_url
                );
            }
        }
    }
}

/// RC3 gate: verify the configured model name is actually loaded on the backend.
/// Checks /v1/models (OpenAI-compatible) for backends with health URLs.
/// Non-fatal: logs warning if model is missing or endpoint unavailable.
async fn verify_model_loaded(client: &reqwest::Client, cfg: &BackendConfig) {
    let models_url = format!("{}/models", cfg.base_url.trim_end_matches('/'));
    let Ok(resp) = client.get(&models_url).send().await else {
        return; // Already logged as unreachable in health check
    };
    if !resp.status().is_success() {
        return; // /v1/models not supported — skip silently
    }
    let Ok(body) = resp.text().await else {
        return;
    };
    // Check if the configured model name appears in the response.
    // OpenAI-compatible: {"data": [{"id": "model-name", ...}]}
    // llama-server: {"data": [{"id": "model-name"}]} or flat list
    if body.contains(&cfg.model) {
        klog!(
            "[config] ✓ {} model '{}' verified loaded",
            cfg.name,
            cfg.model
        );
    } else {
        klog!(
            "[config] ⚠ {} model '{}' NOT FOUND in /models response — config drift?",
            cfg.name,
            cfg.model
        );
        tracing::warn!(
            backend = %cfg.name,
            configured_model = %cfg.model,
            "Model not found in backend /models — config may not match running server"
        );
    }
}

// ── DOMAIN PROMPTS ───────────────────────────────────────────

/// Strip the H1 title from domain prompt content, preserving all H2+ sections.
/// Pure function — no I/O. Extracted for testability (K3/K13).
fn strip_domain_heading(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let first_content = lines.iter().find(|l| !l.trim().is_empty());
    // Space after `#` required: `# Title` is H1, `## FIDELITY` is H2 (must not skip).
    // The `!starts_with("## ")` guard prevents reintroducing the original bug.
    let skip_heading = first_content.is_some_and(|l| l.starts_with("# ") && !l.starts_with("## "));
    content
        .lines()
        .skip_while(|l| l.trim().is_empty())
        .skip(if skip_heading { 1 } else { 0 })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Load domain-specific axiom evaluation prompts from `domains/*.md`.
/// Returns a map of domain name → axiom prompt text.
/// The markdown heading (first `# ...` line) and any content before the first
/// `## FIDELITY` section is used as a preamble. Each `## AXIOM` section
/// provides domain-specific evaluation criteria for that axiom.
///
/// If the directory doesn't exist or is empty, returns an empty map (graceful degradation).
pub fn load_domain_prompts(project_root: &Path) -> std::collections::HashMap<String, String> {
    let domains_dir = project_root.join("domains");
    let mut prompts = std::collections::HashMap::new();

    let Ok(entries) = std::fs::read_dir(&domains_dir) else {
        klog!("[config] No domains/ directory — using generic prompts for all domains");
        return prompts;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let domain = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if domain.is_empty() {
            continue;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                // Strip the H1 title (# Domain Name) but preserve H2+ (## FIDELITY etc.)
                let prompt = strip_domain_heading(&content);
                if !prompt.is_empty() {
                    klog!(
                        "[config] Domain prompt loaded: '{}' ({} chars)",
                        domain,
                        prompt.len()
                    );
                    prompts.insert(domain, prompt);
                }
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "failed to read domain prompt file");
            }
        }
    }

    if prompts.is_empty() {
        klog!("[config] No domain prompts found — using generic prompts");
    } else {
        klog!("[config] {} domain prompt(s) loaded", prompts.len());
    }

    prompts
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

    #[test]
    fn derive_health_url_strips_v1() {
        assert_eq!(
            derive_health_url("http://10.0.0.1:8080/v1"),
            "http://10.0.0.1:8080/health"
        );
        assert_eq!(
            derive_health_url("https://api.example.com/v1"),
            "https://api.example.com/health"
        );
    }

    #[test]
    fn derive_health_url_no_v1() {
        assert_eq!(
            derive_health_url("http://10.0.0.1:8080"),
            "http://10.0.0.1:8080/health"
        );
        assert_eq!(
            derive_health_url("http://10.0.0.1:8080/"),
            "http://10.0.0.1:8080/health"
        );
    }

    #[test]
    fn parse_toml_with_remediation() {
        let toml_content = r#"
[backend.sovereign]
base_url = "http://10.0.0.1:8080/v1"
model = "qwen3"
auth_style = "bearer"

[backend.sovereign.remediation]
node = "user@10.0.0.1"
restart_command = "systemctl --user restart llama-server"
max_retries = 5
cooldown_secs = 30
"#;
        let dir = std::env::temp_dir().join("cynic_config_remediation_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("backends.toml");
        std::fs::write(&path, toml_content).unwrap();

        let configs = load_backends(&path);
        assert_eq!(configs.len(), 1);
        assert_eq!(
            configs[0].health_url.as_deref(),
            Some("http://10.0.0.1:8080/health")
        );
        let rem = configs[0]
            .remediation
            .as_ref()
            .expect("remediation should be present");
        assert_eq!(rem.node, "user@10.0.0.1");
        assert_eq!(rem.max_retries, 5);
        assert_eq!(rem.cooldown_secs, 30);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn cloud_backend_without_remediation_gets_no_health_url() {
        let toml_content = r#"
[backend.gemini]
base_url = "https://api.google.com/v1"
model = "gemini-flash"
"#;
        let dir = std::env::temp_dir().join("cynic_config_no_rem_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("backends.toml");
        std::fs::write(&path, toml_content).unwrap();

        let configs = load_backends(&path);
        assert_eq!(configs.len(), 1);
        assert!(configs[0].remediation.is_none());
        // Cloud APIs without remediation or explicit health_url get None
        assert!(
            configs[0].health_url.is_none(),
            "cloud backend should have no health_url, got {:?}",
            configs[0].health_url
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn sovereign_backend_with_remediation_gets_derived_health_url() {
        let toml_content = r#"
[backend.sovereign]
base_url = "http://10.0.0.1:8080/v1"
model = "qwen3"

[backend.sovereign.remediation]
node = "user@10.0.0.1"
restart_command = "systemctl restart llama-server"
"#;
        let dir = std::env::temp_dir().join("cynic_config_sov_health_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("backends.toml");
        std::fs::write(&path, toml_content).unwrap();

        let configs = load_backends(&path);
        assert_eq!(configs.len(), 1);
        assert!(configs[0].remediation.is_some());
        assert_eq!(
            configs[0].health_url.as_deref(),
            Some("http://10.0.0.1:8080/health"),
            "sovereign with remediation should get derived health_url"
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn explicit_health_url_overrides_derivation() {
        let toml_content = r#"
[backend.custom]
base_url = "http://10.0.0.1:8080/v1"
model = "custom-model"
health_url = "http://custom-health:9090/ready"
"#;
        let dir = std::env::temp_dir().join("cynic_config_explicit_health_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("backends.toml");
        std::fs::write(&path, toml_content).unwrap();

        let configs = load_backends(&path);
        assert_eq!(configs.len(), 1);
        assert_eq!(
            configs[0].health_url.as_deref(),
            Some("http://custom-health:9090/ready"),
            "explicit health_url should be used even without remediation"
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn parse_toml_with_inference_params() {
        let toml_content = r#"
[backend.sovereign]
base_url = "http://10.0.0.1:8080/v1"
model = "qwen3-4b"
auth_style = "none"
timeout_secs = 90
max_tokens = 2048
temperature = 0.2
disable_thinking = true
"#;
        let dir = std::env::temp_dir().join("cynic_config_inference_params_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("backends.toml");
        std::fs::write(&path, toml_content).unwrap();

        let configs = load_backends(&path);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].timeout_secs, 90);
        assert_eq!(configs[0].max_tokens, 2048);
        assert!((configs[0].temperature - 0.2).abs() < 0.01);
        assert!(configs[0].disable_thinking);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn inference_params_default_when_absent() {
        let toml_content = r#"
[backend.cloud]
base_url = "https://api.example.com/v1"
model = "gpt-4"
"#;
        let dir = std::env::temp_dir().join("cynic_config_inference_defaults_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("backends.toml");
        std::fs::write(&path, toml_content).unwrap();

        let configs = load_backends(&path);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].timeout_secs, 30);
        assert_eq!(configs[0].max_tokens, 4096);
        assert!((configs[0].temperature - 0.3).abs() < 0.01);
        assert!(!configs[0].disable_thinking);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn skips_h1_title() {
        let input = "# Chess Domain\n## FIDELITY\nIs this faithful?";
        assert!(strip_domain_heading(input).starts_with("## FIDELITY"));
    }

    #[test]
    fn keeps_h2_when_no_title() {
        let input = "## FIDELITY\nIs this faithful?";
        assert!(strip_domain_heading(input).starts_with("## FIDELITY"));
    }

    #[test]
    fn skips_blank_lines_and_title() {
        let input = "\n\n# Title\n## FIDELITY\nContent";
        assert!(strip_domain_heading(input).starts_with("## FIDELITY"));
    }

    #[test]
    fn keeps_h2_after_blank_lines() {
        let input = "\n\n## FIDELITY\nContent";
        assert!(strip_domain_heading(input).starts_with("## FIDELITY"));
    }
}
