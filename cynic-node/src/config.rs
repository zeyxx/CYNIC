// Config module: TOML structs, validation, derived values.
// Env vars named in `api_key_env` fields are resolved at load time — the
// resolved secrets are stored in the corresponding `api_key` fields and the
// env var name is discarded (never stored after load).

// WHY: config structs consumed by supervise/announce/verify/main — modules are
// empty stubs until Tasks 2-5. Fields are alive once callers exist.
#![allow(dead_code)]

use serde::Deserialize;
use std::collections::HashMap;

// ── Default value functions (required by serde) ──────────────────────────────

fn default_heartbeat_interval_secs() -> u64 {
    40
}

fn default_context_size() -> u32 {
    4096
}

fn default_dog_timeout_secs() -> u64 {
    60
}

fn default_max_attempts() -> u32 {
    5
}

fn default_initial_delay_secs() -> u64 {
    2
}

fn default_max_delay_secs() -> u64 {
    120
}

fn default_min_uptime_secs() -> u64 {
    10
}

fn default_health_interval_secs() -> u64 {
    15
}

fn default_verify_interval_secs() -> u64 {
    60
}

fn default_health_timeout_secs() -> u64 {
    5
}

fn default_max_failures() -> u32 {
    3
}

fn default_startup_timeout_secs() -> u64 {
    120
}

fn default_stop_timeout_secs() -> u64 {
    10
}

// ── Config structs ────────────────────────────────────────────────────────────

/// Top-level configuration parsed from a node TOML file.
#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) kernel: KernelConfig,
    pub(crate) dog: DogConfig,
    pub(crate) process: ProcessConfig,
    #[serde(default)]
    pub(crate) restart: RestartConfig,
    #[serde(default)]
    pub(crate) health: HealthConfig,
}

/// Kernel connection settings.
#[derive(Debug, Deserialize)]
pub(crate) struct KernelConfig {
    /// Base URL of the CYNIC kernel REST API.
    pub(crate) url: String,
    /// Name of the environment variable holding the kernel API key.
    pub(crate) api_key_env: String,
    /// Resolved API key value — populated by `load()`, never from TOML.
    #[serde(skip)]
    pub(crate) api_key: String,
    /// How often to send heartbeats (seconds). Default: 40 (= kernel TTL / 3).
    #[serde(default = "default_heartbeat_interval_secs")]
    pub(crate) heartbeat_interval_secs: u64,
}

/// Dog (inference backend) identity and connection settings.
#[derive(Debug, Deserialize)]
pub(crate) struct DogConfig {
    /// Unique name for this dog as registered with the kernel (1–64 chars).
    pub(crate) name: String,
    /// Model identifier string (must match what the backend reports).
    pub(crate) model: String,
    /// Base URL of the inference backend's OpenAI-compatible API.
    /// Must be reachable FROM THE KERNEL, not just from this node.
    pub(crate) base_url: String,
    /// Context window size. Default: 4096.
    #[serde(default = "default_context_size")]
    pub(crate) context_size: u32,
    /// HTTP timeout for inference requests (seconds). Default: 60.
    #[serde(default = "default_dog_timeout_secs")]
    pub(crate) timeout_secs: u64,
    /// Optional env var name for backend authentication.
    pub(crate) api_key_env: Option<String>,
    /// Resolved backend API key — populated by `load()` if `api_key_env` is set.
    #[serde(skip)]
    pub(crate) api_key: Option<String>,
}

/// Process lifecycle settings.
#[derive(Debug, Deserialize)]
pub(crate) struct ProcessConfig {
    /// Command + arguments to launch the inference backend. Must be non-empty.
    pub(crate) command: Vec<String>,
    /// Optional working directory for the child process.
    pub(crate) working_dir: Option<String>,
    /// Seconds to wait for SIGTERM before sending SIGKILL. Must be > 0.
    #[serde(default = "default_stop_timeout_secs")]
    pub(crate) stop_timeout_secs: u64,
    /// Environment variables injected into the child process.
    #[serde(default)]
    pub(crate) env: HashMap<String, String>,
}

/// Restart / backoff settings.
#[derive(Debug, Deserialize)]
pub(crate) struct RestartConfig {
    /// Maximum consecutive restart attempts before exit(1). Must be > 0.
    #[serde(default = "default_max_attempts")]
    pub(crate) max_attempts: u32,
    /// Initial backoff delay in seconds.
    #[serde(default = "default_initial_delay_secs")]
    pub(crate) initial_delay_secs: u64,
    /// Maximum backoff delay cap in seconds.
    #[serde(default = "default_max_delay_secs")]
    pub(crate) max_delay_secs: u64,
    /// If the backend lived this long, the failure counter resets.
    #[serde(default = "default_min_uptime_secs")]
    pub(crate) min_uptime_secs: u64,
}

impl Default for RestartConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_delay_secs: default_initial_delay_secs(),
            max_delay_secs: default_max_delay_secs(),
            min_uptime_secs: default_min_uptime_secs(),
        }
    }
}

/// Health probe and identity verification settings.
#[derive(Debug, Deserialize)]
pub(crate) struct HealthConfig {
    /// How often to probe the backend's /health endpoint (seconds).
    #[serde(default = "default_health_interval_secs")]
    pub(crate) interval_secs: u64,
    /// How often to verify the backend's model identity (seconds).
    #[serde(default = "default_verify_interval_secs")]
    pub(crate) verify_interval_secs: u64,
    /// HTTP timeout per probe (seconds).
    #[serde(default = "default_health_timeout_secs")]
    pub(crate) timeout_secs: u64,
    /// Consecutive failures before killing and restarting the backend.
    #[serde(default = "default_max_failures")]
    pub(crate) max_failures: u32,
    /// Maximum seconds to wait for the first healthy response on startup.
    #[serde(default = "default_startup_timeout_secs")]
    pub(crate) startup_timeout_secs: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            interval_secs: default_health_interval_secs(),
            verify_interval_secs: default_verify_interval_secs(),
            timeout_secs: default_health_timeout_secs(),
            max_failures: default_max_failures(),
            startup_timeout_secs: default_startup_timeout_secs(),
        }
    }
}

// ── Derived values ────────────────────────────────────────────────────────────

/// Derive the health probe URL from the dog's base_url.
///
/// Strips any trailing slash and any `/v1` suffix, then appends `/health`.
/// Examples:
///   `http://host:8080/v1`   → `http://host:8080/health`
///   `http://host:8080/v1/`  → `http://host:8080/health`
///   `http://host:8080`      → `http://host:8080/health`
pub(crate) fn derive_health_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    let stripped = trimmed.trim_end_matches("/v1");
    format!("{stripped}/health")
}

/// Derive the models listing URL from the dog's base_url.
///
/// Ensures no double slashes by trimming a trailing slash first.
/// Example: `http://host:8080/v1` → `http://host:8080/v1/models`
pub(crate) fn derive_models_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    format!("{trimmed}/models")
}

// ── Load + Validate ───────────────────────────────────────────────────────────

/// Load and validate a node config from the given file path.
///
/// Returns `Ok(Config)` on success, or `Err(String)` describing the failure.
/// All `api_key_env` fields are resolved from the environment at this point.
/// The binary entrypoint is responsible for reporting errors and exiting.
///
/// # Errors
/// - File cannot be read
/// - TOML cannot be parsed
/// - Validation fails (see `validate`)
pub(crate) fn load(path: &str) -> Result<Config, String> {
    let raw =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read config {path}: {e}"))?;

    let mut cfg: Config =
        toml::from_str(&raw).map_err(|e| format!("config parse error in {path}: {e}"))?;

    // Resolve kernel API key from env var (required).
    cfg.kernel.api_key = std::env::var(&cfg.kernel.api_key_env).map_err(|e| {
        format!(
            "environment variable `{}` not set (required for kernel auth): {e}",
            cfg.kernel.api_key_env
        )
    })?;
    if cfg.kernel.api_key.is_empty() {
        return Err(format!(
            "environment variable `{}` is empty (must be non-empty for kernel auth)",
            cfg.kernel.api_key_env
        ));
    }

    // Resolve optional dog API key from env var.
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

    validate(&cfg)?;
    Ok(cfg)
}

/// Validate config constraints. Returns `Ok(())` on success or `Err(String)`
/// describing the first validation failure found.
///
/// This function MUST NOT panic — it returns `Result` so callers can report
/// errors gracefully.
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TOML: &str = r#"
[kernel]
url = "http://127.0.0.1:3030"
api_key_env = "TEST_CYNIC_API_KEY"

[dog]
name = "test-dog"
model = "test-model:7b"
base_url = "http://127.0.0.1:8080/v1"

[process]
command = ["llama-server", "-m", "/models/test.gguf"]
"#;

    fn make_config_with_env(toml: &str, kernel_key: &str) -> Result<Config, String> {
        // Set the env var that the TOML references, then parse manually
        // (we call toml::from_str + manual env resolution instead of load()
        //  to avoid needing a real file on disk)
        let mut cfg: Config = toml::from_str(toml).map_err(|e| e.to_string())?;
        cfg.kernel.api_key = kernel_key.to_owned();
        if let Some(env_name) = &cfg.dog.api_key_env {
            if let Ok(v) = std::env::var(env_name) {
                cfg.dog.api_key = Some(v);
            }
        }
        Ok(cfg)
    }

    // ── derive_health_url ─────────────────────────────────────────────────────

    #[test]
    fn derive_health_url_strips_v1_suffix() {
        assert_eq!(
            derive_health_url("http://host:8080/v1"),
            "http://host:8080/health"
        );
    }

    #[test]
    fn derive_health_url_strips_trailing_slash_and_v1() {
        assert_eq!(
            derive_health_url("http://host:8080/v1/"),
            "http://host:8080/health"
        );
    }

    #[test]
    fn derive_health_url_no_v1_suffix() {
        assert_eq!(
            derive_health_url("http://host:8080"),
            "http://host:8080/health"
        );
    }

    #[test]
    fn derive_health_url_trailing_slash_only() {
        assert_eq!(
            derive_health_url("http://host:8080/"),
            "http://host:8080/health"
        );
    }

    // ── derive_models_url ─────────────────────────────────────────────────────

    #[test]
    fn derive_models_url_appends_models() {
        assert_eq!(
            derive_models_url("http://host:8080/v1"),
            "http://host:8080/v1/models"
        );
    }

    #[test]
    fn derive_models_url_strips_trailing_slash() {
        assert_eq!(
            derive_models_url("http://host:8080/v1/"),
            "http://host:8080/v1/models"
        );
    }

    // ── parse valid config ────────────────────────────────────────────────────

    #[test]
    fn parse_valid_config_succeeds() {
        let cfg = make_config_with_env(VALID_TOML, "secret-key").unwrap();
        assert_eq!(cfg.dog.name, "test-dog");
        assert_eq!(cfg.dog.model, "test-model:7b");
        assert_eq!(cfg.dog.base_url, "http://127.0.0.1:8080/v1");
        assert_eq!(cfg.kernel.url, "http://127.0.0.1:3030");
        assert_eq!(cfg.kernel.api_key, "secret-key");
    }

    #[test]
    fn parse_config_serde_defaults() {
        let cfg = make_config_with_env(VALID_TOML, "key").unwrap();
        assert_eq!(cfg.dog.context_size, 4096);
        assert_eq!(cfg.dog.timeout_secs, 60);
        assert_eq!(cfg.kernel.heartbeat_interval_secs, 40);
        assert_eq!(cfg.restart.max_attempts, 5);
        assert_eq!(cfg.restart.initial_delay_secs, 2);
        assert_eq!(cfg.restart.max_delay_secs, 120);
        assert_eq!(cfg.restart.min_uptime_secs, 10);
        assert_eq!(cfg.health.interval_secs, 15);
        assert_eq!(cfg.health.verify_interval_secs, 60);
        assert_eq!(cfg.health.timeout_secs, 5);
        assert_eq!(cfg.health.max_failures, 3);
        assert_eq!(cfg.health.startup_timeout_secs, 120);
        assert_eq!(cfg.process.stop_timeout_secs, 10);
    }

    // ── validate: reject empty command ───────────────────────────────────────

    #[test]
    fn validate_rejects_empty_command() {
        let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
        cfg.process.command.clear();
        let result = validate(&cfg);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("process.command"));
    }

    // ── validate: reject empty name ──────────────────────────────────────────

    #[test]
    fn validate_rejects_empty_name() {
        let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
        cfg.dog.name = String::new();
        let result = validate(&cfg);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("dog.name"));
    }

    #[test]
    fn validate_rejects_name_too_long() {
        let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
        cfg.dog.name = "x".repeat(65);
        let result = validate(&cfg);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("dog.name"));
    }

    #[test]
    fn validate_accepts_name_max_length() {
        let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
        cfg.dog.name = "x".repeat(64);
        assert!(validate(&cfg).is_ok());
    }

    // ── validate: stop_timeout > 0 ───────────────────────────────────────────

    #[test]
    fn validate_rejects_zero_stop_timeout() {
        let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
        cfg.process.stop_timeout_secs = 0;
        let result = validate(&cfg);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("stop_timeout_secs"));
    }

    // ── validate: max_attempts > 0 ───────────────────────────────────────────

    #[test]
    fn validate_rejects_zero_max_attempts() {
        let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
        cfg.restart.max_attempts = 0;
        let result = validate(&cfg);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_attempts"));
    }

    // ── validate: startup_timeout > health.timeout ───────────────────────────

    #[test]
    fn validate_rejects_startup_timeout_not_greater_than_health_timeout() {
        let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
        // startup_timeout_secs (120) > timeout_secs (5) by default — this should pass
        assert!(validate(&cfg).is_ok());

        // Set startup equal to timeout — should fail
        cfg.health.startup_timeout_secs = cfg.health.timeout_secs;
        let result = validate(&cfg);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("startup_timeout_secs"));
    }

    // ── load: env var not set ────────────────────────────────────────────────

    #[test]
    fn load_fails_if_api_key_env_not_set() {
        use std::io::Write;
        // Use a TOML that references an env var guaranteed to not exist
        let toml_with_missing_env = VALID_TOML.replace(
            "TEST_CYNIC_API_KEY",
            "DEFINITELY_NONEXISTENT_ENV_VAR_XYZ_42",
        );
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(toml_with_missing_env.as_bytes()).unwrap();
        let path = f.path().to_str().unwrap().to_owned();

        let result = load(&path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("DEFINITELY_NONEXISTENT_ENV_VAR_XYZ_42")
        );
    }
}
