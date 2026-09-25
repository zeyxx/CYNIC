use cynic_inference::config_loader::{
    InferenceConfigError, load_inference_config_from_path_public as load_inference_config,
};
use std::path::Path;

use crate::infra::config_schema::{BackendsFile, OrgansFile};
use crate::infra::config_types as cfg;
use crate::infra::config_types::{
    AuthStyle, BackendConfig, BackendRemediation, BackendType, OrganRemediation,
};

fn load_file(path: &Path) -> Option<String> {
    super::helpers::load_file(path)
}

fn resolve_registry_placeholders(content: String) -> String {
    super::helpers::resolve_registry_placeholders(content)
}

pub(crate) fn derive_health_url(base_url: &str) -> String {
    let base = if let Some(stripped) = base_url.strip_suffix("/v1") {
        stripped
    } else {
        base_url.trim_end_matches('/')
    };
    format!("{base}/health")
}

/// Same pattern as `derive_health_url` — only sovereign backends expose this.
pub fn derive_slots_url(base_url: &str) -> String {
    let base = if let Some(stripped) = base_url.strip_suffix("/v1") {
        stripped
    } else {
        base_url.trim_end_matches('/')
    };
    format!("{base}/slots")
}

/// Load organ remediation blocks from `[[organs]]` entries in `backends.toml`.
pub fn load_organ_remediations(path: &Path) -> Vec<OrganRemediation> {
    let Some(content) = load_file(path) else {
        return Vec::new();
    };

    match toml::from_str::<OrgansFile>(&content) {
        Ok(f) => f.organs,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "organ remediation: parse failed");
            Vec::new()
        }
    }
}

/// Load backend configs from TOML. Applies env substitution and filtering.
pub fn load_backends(path: &Path) -> Vec<BackendConfig> {
    if let Some(inference_backends) = load_backends_from_inference_config(path) {
        return inference_backends;
    }

    load_backends_legacy(path)
}

fn load_backends_from_inference_config(path: &Path) -> Option<Vec<BackendConfig>> {
    let loaded = match load_inference_config(path) {
        Ok(inference) => {
            return Some(
                inference
                    .backends
                    .into_iter()
                    .map(cfg::BackendConfig::from_inference_backend)
                    .collect(),
            );
        }
        Err(error) => error,
    };

    match loaded {
        InferenceConfigError::Parse { path, source: _ } => {
            tracing::debug!(path = %path.display(), "inference config parser rejected legacy schema — using legacy backend loader");
            None
        }
        InferenceConfigError::NotFound { path } => {
            tracing::debug!(path = %path.display(), "inference config missing — falling back to legacy loader");
            None
        }
        other => {
            tracing::warn!(error = %other, "inference config load failed — skipping backends");
            Some(Vec::new())
        }
    }
}

/// Legacy `backends.toml` parser retained for non-inference policy fields and migration.
fn load_backends_legacy(path: &Path) -> Vec<BackendConfig> {
    let Some(mut content) = load_file(path) else {
        tracing::warn!(path = %path.display(), "cannot read config file");
        return Vec::new();
    };

    content = resolve_registry_placeholders(content);

    let file: BackendsFile = match toml::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "invalid TOML in config");
            return Vec::new();
        }
    };

    file
        .backend
        .into_iter()
        .filter_map(|(name, entry)| {
            // K15 validation: skip unresolved placeholders.
            if entry.base_url.contains('<') && entry.base_url.contains('>') {
                if entry.base_url.contains("TAILSCALE_") {
                    tracing::error!(
                        backend = %name,
                        base_url = %entry.base_url,
                        "Unresolved placeholder in backend config. Run scripts/config-sync.sh deploy"
                    );
                } else {
                    tracing::warn!(
                        backend = %name,
                        base_url = %entry.base_url,
                        "Unknown placeholder in backend URL — review manually"
                    );
                }
                return None;
            }

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

            if entry.api_key_env.is_some() && api_key.is_none() {
                return None;
            }

            let backend_type = match entry.backend_type.as_deref() {
                Some("cli") => BackendType::Cli,
                Some("openai") | None => BackendType::OpenAi,
                Some(other) => {
                    tracing::warn!(backend_type = %other, backend = %name, "unknown backend_type, defaulting to openai");
                    BackendType::OpenAi
                }
            };

            let auth_style = match entry.auth_style.as_deref() {
                Some("bearer") | None => AuthStyle::Bearer,
                Some("none") => AuthStyle::None,
                Some(other) if other.starts_with("query:") => {
                    AuthStyle::QueryParam {
                        param: other.trim_start_matches("query:").to_string(),
                    }
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

            // health_url: explicit if provided, derived for sovereign (http://) backends
            // without cli adapter.
            let is_sovereign = entry.base_url.starts_with("http://");
            let health_url = if backend_type == BackendType::Cli {
                None
            } else {
                entry.health_url.or_else(|| {
                    if is_sovereign {
                        Some(derive_health_url(&entry.base_url))
                    } else {
                        None
                    }
                })
            };

            Some(BackendConfig {
                name,
                backend_type,
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
                supports_tool_use: entry.supports_tool_use.unwrap_or(false),
                prompt_tier: cfg::PromptTier::from_str_opt(entry.prompt_tier.as_deref()),
                cost_input_per_mtok: entry.cost_input_per_mtok.unwrap_or(0.0),
                cost_output_per_mtok: entry.cost_output_per_mtok.unwrap_or(0.0),
                health_url,
                remediation,
                fleet_node: entry.fleet_node,
                cli_extra_args: entry
                    .cli_extra_args
                    .map(|s| s.split_whitespace().map(String::from).collect())
                    .unwrap_or_default(),
                latency_ms: entry.latency_ms.unwrap_or(0),
                suitable_for_domains: entry.suitable_for_domains.unwrap_or_default(),
                sovereign: entry.sovereign.unwrap_or(is_sovereign),
                daily_budget: entry.daily_budget.unwrap_or(0),
            })
        })
        .collect()
}

/// Fallback config source for legacy `GEMINI_*` env vars.
pub fn load_backends_from_env() -> Vec<BackendConfig> {
    let mut configs = Vec::new();

<<<<<<< HEAD
    if let Some(api_key) = cynic_secrets::env_var("gemini-api-key", "GEMINI_API_KEY") {
=======
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
>>>>>>> github/main
        let model =
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_string());
        let base_url = "https://generativelanguage.googleapis.com/v1beta/openai".to_string();
        configs.push(BackendConfig {
            name: "gemini".to_string(),
            backend_type: BackendType::OpenAi,
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
            supports_tool_use: false,
            prompt_tier: cfg::PromptTier::Full,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: None,
            remediation: None,
            fleet_node: None,
            cli_extra_args: vec![],
            latency_ms: 0,
            suitable_for_domains: vec![],
            sovereign: false,
            daily_budget: 0,
        });
    }

    configs
}
