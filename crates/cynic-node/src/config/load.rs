use super::{Config, FederationConfig, WatchdogConfig};
use crate::domain::runtime_federation::{
    DEFAULT_NODE_PUBLIC_KEY, RuntimeFederationConfig, default_queue_dir, kernel_websocket_url,
};

pub(crate) fn derive_health_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    let stripped = trimmed.trim_end_matches("/v1");
    format!("{stripped}/health")
}

pub(crate) fn derive_models_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    format!("{trimmed}/models")
}

pub(crate) fn load(path: &str) -> Result<Config, String> {
    let raw =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read config {path}: {e}"))?;

    let mut cfg: Config =
        toml::from_str(&raw).map_err(|e| format!("config parse error in {path}: {e}"))?;

    resolve_kernel_api_key(&mut cfg.kernel.api_key, &cfg.kernel.api_key_env)?;

    if let Some(env_name) = &cfg.dog.api_key_env {
        let key = std::env::var(env_name).map_err(|e| {
            format!(
                "environment variable `{env_name}` not set (referenced in dog.api_key_env): {e}"
            )
        })?;
        if !key.is_empty() {
            cfg.dog.api_key = Some(key);
        }
    }

    cfg.process.command = cfg
        .process
        .command
        .iter()
        .map(|arg| expand_env_vars(arg))
        .collect::<Result<Vec<_>, _>>()?;

    validate(&cfg)?;
    Ok(cfg)
}

pub(crate) fn load_watchdog(path: &str) -> Result<WatchdogConfig, String> {
    let raw =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read config {path}: {e}"))?;

    let mut cfg: WatchdogConfig =
        toml::from_str(&raw).map_err(|e| format!("config parse error in {path}: {e}"))?;

    resolve_kernel_api_key(&mut cfg.kernel.api_key, &cfg.kernel.api_key_env)?;
    validate_watchdog(&cfg)?;
    Ok(cfg)
}

fn resolve_kernel_api_key(api_key: &mut String, env_name: &str) -> Result<(), String> {
    *api_key = std::env::var(env_name).map_err(|e| {
        format!("environment variable `{env_name}` not set (required for kernel auth): {e}")
    })?;
    if api_key.is_empty() {
        return Err(format!(
            "environment variable `{env_name}` is empty (must be non-empty for kernel auth)"
        ));
    }
    Ok(())
}

pub(crate) fn expand_env_vars(input: &str) -> Result<String, String> {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next();
            let var_name: String = chars.by_ref().take_while(|&c| c != '}').collect();
            if var_name.is_empty() {
                return Err("empty variable name in ${} expansion".to_owned());
            }
            let value = std::env::var(&var_name).map_err(|e| {
                format!(
                    "environment variable `{var_name}` not set (referenced in process.command): {e}"
                )
            })?;
            result.push_str(&value);
        } else {
            result.push(ch);
        }
    }
    Ok(result)
}

pub(crate) fn validate(cfg: &Config) -> Result<(), String> {
    if cfg.process.command.is_empty() {
        return Err("process.command must be non-empty".to_owned());
    }

    let name_len = cfg.dog.name.len();
    if name_len == 0 || name_len > 64 {
        return Err(format!(
            "dog.name must be 1–64 characters, got {name_len} chars"
        ));
    }

    if cfg.process.stop_timeout_secs == 0 {
        return Err("process.stop_timeout_secs must be > 0".to_owned());
    }

    if cfg.restart.max_attempts == 0 {
        return Err("restart.max_attempts must be > 0".to_owned());
    }

    if cfg.health.startup_timeout_secs <= cfg.health.timeout_secs {
        return Err(format!(
            "health.startup_timeout_secs ({}) must be > health.timeout_secs ({})",
            cfg.health.startup_timeout_secs, cfg.health.timeout_secs
        ));
    }

    Ok(())
}

pub(crate) fn runtime_federation_config(cfg: &Config) -> RuntimeFederationConfig {
    let federation = cfg.federation.as_ref();
    RuntimeFederationConfig {
        kernel_ws_url: federation
            .and_then(|config| config.kernel_ws_url.clone())
            .unwrap_or_else(|| kernel_websocket_url(&cfg.kernel.url)),
        api_key: federation_api_key(federation).unwrap_or_else(|| {
            std::env::var("CYNIC_API_KEY").unwrap_or_else(|_| {
                tracing::warn!("CYNIC_API_KEY not set; WebSocket authentication will fail");
                String::new()
            })
        }),
        node_public_key: federation_node_public_key(federation).unwrap_or_else(|| {
            std::env::var("CYNIC_NODE_PUBLIC_KEY")
                .unwrap_or_else(|_| DEFAULT_NODE_PUBLIC_KEY.to_string())
        }),
        node_identity: cfg.dog.name.clone(),
        queue_dir: federation
            .and_then(|config| config.queue_dir.clone())
            .unwrap_or_else(|| default_queue_dir(std::env::var("HOME").ok().map(Into::into))),
    }
}

fn federation_api_key(config: Option<&FederationConfig>) -> Option<String> {
    let config = config?;
    if let Some(value) = config.api_key.clone() {
        return Some(value);
    }
    config
        .api_key_env
        .as_ref()
        .and_then(|env_name| std::env::var(env_name).ok())
}

fn federation_node_public_key(config: Option<&FederationConfig>) -> Option<String> {
    let config = config?;
    if let Some(value) = config.node_public_key.clone() {
        return Some(value);
    }
    config
        .node_public_key_env
        .as_ref()
        .and_then(|env_name| std::env::var(env_name).ok())
}

pub(crate) fn validate_watchdog(cfg: &WatchdogConfig) -> Result<(), String> {
    let name_len = cfg.node.name.len();
    if name_len == 0 || name_len > 64 {
        return Err(format!(
            "node.name must be 1-64 characters, got {name_len} chars"
        ));
    }
    if cfg.node.role.is_empty() || cfg.node.role.len() > 64 {
        return Err("node.role must be 1-64 characters".to_string());
    }
    if cfg.node.constellation.is_empty() || cfg.node.constellation.len() > 64 {
        return Err("node.constellation must be 1-64 characters".to_string());
    }
    if cfg.watchdog.interval_secs == 0 {
        return Err("watchdog.interval_secs must be > 0".to_string());
    }
    if cfg.watchdog.http_timeout_secs == 0 {
        return Err("watchdog.http_timeout_secs must be > 0".to_string());
    }
    if cfg.watchdog.registry_path.trim().is_empty() {
        return Err("watchdog.registry_path must be non-empty".to_string());
    }
    Ok(())
}
