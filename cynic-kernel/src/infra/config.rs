//! Backend + storage configuration — loaded from backends.toml or env vars.
//! Lives in infrastructure layer. NEVER imported by domain core.

use serde::Deserialize;
use std::path::Path;

// ── PROMPT TIER ──────────────────────────────────────────

/// Controls how much domain context a Dog receives in its prompt.
/// Small models (<10B) work better with Lite (concise instructions).
/// Larger models benefit from Full (detailed rubrics per axiom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptTier {
    /// Generic 6-line axiom descriptions. Best for small models.
    Lite,
    /// Full domain-specific rubrics (HIGH/MEDIUM/LOW per axiom). Best for capable models.
    #[default]
    Full,
}

impl PromptTier {
    fn from_str_opt(s: Option<&str>) -> Self {
        match s {
            Some("lite") => Self::Lite,
            Some("full") | None => Self::Full,
            Some(other) => {
                tracing::warn!(tier = other, "unknown prompt_tier — defaulting to Full");
                Self::Full
            }
        }
    }
}

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
    /// Which adapter to use. Default: `OpenAi` (HTTP). Use `Cli` for subprocess-based tools.
    pub backend_type: BackendType,
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
    /// Prompt tier: controls how much domain context this Dog receives.
    /// Lite = generic 6-line axioms (better for small models <10B).
    /// Full = complete domain prompt with rubrics (better for capable models).
    /// Default: Full.
    pub prompt_tier: PromptTier,
    /// Cost per 1M input tokens in USD. 0.0 = free (sovereign, free tier).
    pub cost_input_per_mtok: f64,
    /// Cost per 1M output tokens in USD. 0.0 = free.
    pub cost_output_per_mtok: f64,
    /// Health URL — derived from base_url for backends with remediation or explicit health_url.
    /// None for cloud APIs (no health endpoint) — health loop skips them.
    pub health_url: Option<String>,
    /// Remediation config — optional. Only for backends that can be restarted.
    pub remediation: Option<BackendRemediation>,
    /// Tailscale hostname — if set, health_loop can preemptively open circuit
    /// when this node goes offline (before Dog times out).
    pub fleet_node: Option<String>,
    /// Extra CLI arguments passed before --prompt (CLI backends only).
    /// Parsed by splitting on whitespace. Example: "-o json --approval-mode plan"
    pub cli_extra_args: Vec<String>,
    /// Observed latency in milliseconds (informational, used for scheduling).
    /// 0 = unknown/unmeasured. Used by domain-aware Dog selection.
    pub latency_ms: u32,
    /// Domains this Dog is suitable for. Empty = suitable for all (default).
    /// If not empty, this Dog is only queried when one of these domains is active.
    pub suitable_for_domains: Vec<String>,
    /// Is this backend sovereign (local, no cloud APIs)? True for local Dogs.
    /// Used for Layer 4 of the sensitivity filter: sensitive content routes to sovereign Dogs only.
    /// Default: inferred from is_sovereign_url(base_url). Explicit config overrides inference.
    pub sovereign: bool,
    /// Daily call budget (0 = unlimited). When exhausted, Dog is skipped until UTC midnight.
    /// Designed for quota-constrained backends (e.g. Gemini free tier: ~1500 RPD).
    pub daily_budget: u32,
}

// ── DOG THRESHOLDS (from backends.toml [defaults], [dog.*], [error_detection], etc.) ──

/// Global default thresholds for all Dogs — can be overridden per-dog.
#[derive(Debug, Clone)]
pub struct DogDefaults {
    pub failure_rate_threshold: f64,
    pub api_error_threshold: u32,
    pub transient_error_max: u32,
    pub quota_grace_period_secs: u64,
    pub evaluation_timeout_secs: u64,
}

impl Default for DogDefaults {
    fn default() -> Self {
        Self {
            failure_rate_threshold: 0.15,
            api_error_threshold: 300,
            transient_error_max: 5,
            quota_grace_period_secs: 600,
            evaluation_timeout_secs: 30,
        }
    }
}

/// Per-Dog threshold overrides. Empty = use DogDefaults.
#[derive(Debug, Clone)]
pub struct DogThreshold {
    pub dog_name: String,
    pub kind: String, // "heuristic" or "inference"
    pub enabled: bool,
    pub failure_rate_threshold: Option<f64>,
    pub collapse_threshold: Option<u32>,
    pub api_error_threshold: Option<u32>,
    pub evaluation_timeout_secs: Option<u64>,
    pub priority: Option<String>, // "backup" or "primary" (default)
    pub quota_status: Option<String>,
    pub quota_pattern: Option<String>,
    pub skip_reason: Option<String>,
    pub skip_until: Option<String>,
}

/// Error pattern matching — classifies errors into quota/transient/critical.
#[derive(Debug, Clone)]
pub struct ErrorDetection {
    pub quota_patterns: Vec<String>,
    pub transient_patterns: Vec<String>,
    pub critical_patterns: Vec<String>,
}

impl Default for ErrorDetection {
    fn default() -> Self {
        Self {
            quota_patterns: vec![
                "TerminalQuotaError".to_string(),
                "You have exhausted your capacity".to_string(),
                "Rate limit exceeded".to_string(),
                "Quota".to_string(),
            ],
            transient_patterns: vec![
                "Timeout".to_string(),
                "temporarily unavailable".to_string(),
                "temporarily_unavailable".to_string(),
            ],
            critical_patterns: vec![
                "command not found".to_string(),
                "exit 1".to_string(),
                "permission denied".to_string(),
            ],
        }
    }
}

/// Verdict quorum settings — used by judge to determine confidence.
#[derive(Debug, Clone)]
pub struct VerdictSettings {
    pub min_voters: u32,
    pub min_confident_voters: u32,
    pub contested_threshold: f64,
}

impl Default for VerdictSettings {
    fn default() -> Self {
        Self {
            min_voters: 2,
            min_confident_voters: 2,
            contested_threshold: 0.618,
        }
    }
}

/// Circuit breaker behavior configuration.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub open_on_consecutive_failures: u32,
    pub open_on_failure_rate: bool,
    pub open_duration_secs: u64,
    pub half_open_eval_budget: u32,
    pub failure_rate_recovery_window_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            open_on_consecutive_failures: 10,
            open_on_failure_rate: true,
            open_duration_secs: 300,
            half_open_eval_budget: 2,
            failure_rate_recovery_window_secs: 3600,
        }
    }
}

/// Monitoring configuration.
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub failure_rate_check_interval_secs: u64,
    pub alert_threshold_critical: f64,
    pub alert_threshold_warning: f64,
    pub quota_exhaustion_alert: bool,
    pub circuit_breaker_alert: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            failure_rate_check_interval_secs: 300,
            alert_threshold_critical: 0.70,
            alert_threshold_warning: 0.40,
            quota_exhaustion_alert: true,
            circuit_breaker_alert: true,
        }
    }
}

/// K-Score behavioral composite weights — loaded from backends.toml [kscore].
/// No magic numbers: all weights configurable, tuned via measurement.
#[derive(Debug, Clone)]
pub struct KScoreConfig {
    pub weight_diamond_hands: f64,
    pub weight_organic_growth: f64,
    pub weight_longevity: f64,
    pub accumulator_threshold: f64,
    pub holder_threshold: f64,
    pub reducer_threshold: f64,
    pub top_n_wallets: usize,
    pub swap_history_limit: usize,
}

impl Default for KScoreConfig {
    fn default() -> Self {
        Self {
            weight_diamond_hands: 0.50,
            weight_organic_growth: 0.35,
            weight_longevity: 0.15,
            accumulator_threshold: 1.5,
            holder_threshold: 1.0,
            reducer_threshold: 0.5,
            top_n_wallets: 10,
            swap_history_limit: 100,
        }
    }
}

/// Complete Dog thresholds configuration — loaded from backends.toml.
#[derive(Debug, Clone, Default)]
pub struct DogThresholds {
    pub defaults: DogDefaults,
    pub dogs: std::collections::HashMap<String, DogThreshold>,
    pub error_detection: ErrorDetection,
    pub verdict: VerdictSettings,
    pub circuit: CircuitBreakerConfig,
    pub monitoring: MonitoringConfig,
    pub kscore: KScoreConfig,
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

/// Selects the adapter used to communicate with a backend.
/// - `OpenAi` (default): HTTP/OpenAI-compatible REST API.
/// - `Cli`: subprocess (`binary --prompt "..."`) — for CLI tools like `gemini`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendType {
    #[default]
    OpenAi,
    Cli,
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
    defaults: Option<DefaultsEntry>,
    /// Dog threshold configuration — loaded from [dog.*] sections in backends.toml
    #[serde(default)]
    dog: std::collections::HashMap<String, DogEntry>,
    error_detection: Option<ErrorDetectionEntry>,
    verdict: Option<VerdictEntry>,
    circuit: Option<CircuitEntry>,
    monitoring: Option<MonitoringEntry>,
    /// K-Score behavioral analysis weights — loaded from [kscore] section.
    kscore: Option<KScoreEntry>,
}

#[derive(Deserialize)]
struct KScoreEntry {
    weight_diamond_hands: Option<f64>,
    weight_organic_growth: Option<f64>,
    weight_longevity: Option<f64>,
    accumulator_threshold: Option<f64>,
    holder_threshold: Option<f64>,
    reducer_threshold: Option<f64>,
    top_n_wallets: Option<u64>,
    swap_history_limit: Option<u64>,
}

#[derive(Deserialize)]
struct DefaultsEntry {
    failure_rate_threshold: Option<f64>,
    api_error_threshold: Option<u32>,
    transient_error_max: Option<u32>,
    quota_grace_period_secs: Option<u64>,
    evaluation_timeout_secs: Option<u64>,
}

#[derive(Deserialize)]
struct DogEntry {
    kind: Option<String>,
    enabled: Option<bool>,
    failure_rate_threshold: Option<f64>,
    collapse_threshold: Option<u32>,
    api_error_threshold: Option<u32>,
    evaluation_timeout_secs: Option<u64>,
    priority: Option<String>,
    quota_status: Option<String>,
    quota_pattern: Option<String>,
    skip_reason: Option<String>,
    skip_until: Option<String>,
}

#[derive(Deserialize)]
struct ErrorDetectionEntry {
    quota_patterns: Option<Vec<String>>,
    transient_patterns: Option<Vec<String>>,
    critical_patterns: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct VerdictEntry {
    min_voters: Option<u32>,
    min_confident_voters: Option<u32>,
    contested_threshold: Option<f64>,
}

#[derive(Deserialize)]
struct CircuitEntry {
    open_on_consecutive_failures: Option<u32>,
    open_on_failure_rate: Option<bool>,
    open_duration_secs: Option<u64>,
    half_open_eval_budget: Option<u32>,
    failure_rate_recovery_window_secs: Option<u64>,
}

#[derive(Deserialize)]
struct MonitoringEntry {
    failure_rate_check_interval_secs: Option<u64>,
    alert_threshold_critical: Option<f64>,
    alert_threshold_warning: Option<f64>,
    quota_exhaustion_alert: Option<bool>,
    circuit_breaker_alert: Option<bool>,
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
    backend_type: Option<String>,
    api_key_env: Option<String>,
    model: String,
    auth_style: Option<String>,
    context_size: Option<u32>,
    timeout_secs: Option<u64>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    disable_thinking: Option<bool>,
    json_mode: Option<bool>,
    prompt_tier: Option<String>,
    cost_input_per_mtok: Option<f64>,
    cost_output_per_mtok: Option<f64>,
    /// Explicit health URL — if omitted, derived from base_url.
    health_url: Option<String>,
    /// Inline remediation config.
    remediation: Option<RemediationEntry>,
    /// Tailscale hostname for fleet awareness — maps this Dog to a fleet node.
    fleet_node: Option<String>,
    /// Extra CLI arguments (space-separated string, parsed at load time).
    cli_extra_args: Option<String>,
    /// Observed latency in milliseconds (informational, used for scheduling).
    latency_ms: Option<u32>,
    /// Domains this Dog is suitable for. Empty = suitable for all domains (default).
    /// Example: ["token", "general"] for fast Dogs, ["chess", "reasoning"] for powerful Dogs.
    suitable_for_domains: Option<Vec<String>>,
    /// Optional: explicitly mark this backend as sovereign (local).
    /// If omitted, inferred from is_sovereign_url(base_url). Used for sensitivity filter.
    sovereign: Option<bool>,
    /// Daily call budget (0 or absent = unlimited). Exhausted Dogs are skipped until UTC midnight.
    daily_budget: Option<u32>,
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

            // health_url: explicit if provided, derived for sovereign (http://) non-CLI backends,
            // None for cloud APIs (https://) and CLI backends (no HTTP endpoint).
            // Previously only derived when remediation existed, which left local Dogs unprobed.
            let is_sovereign = entry.base_url.starts_with("http://");
            let health_url = if backend_type == BackendType::Cli {
                None
            } else {
                entry
                    .health_url
                    .or_else(|| {
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
                prompt_tier: PromptTier::from_str_opt(entry.prompt_tier.as_deref()),
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
                // Layer 4: sovereign field defaults to inference from base_url (http://)
                // Can be overridden explicitly via TOML config
                sovereign: entry.sovereign.unwrap_or(is_sovereign),
                daily_budget: entry.daily_budget.unwrap_or(0),
            })
        })
        .collect()
}

/// Load storage config from [storage] section in backends.toml.
/// Falls back to env vars, then defaults.
/// Priority: TOML > env var > default.
pub fn load_storage_config(path: &Path) -> StorageConfig {
    let defaults = StorageConfig::default();

    let from_toml = std::fs::read_to_string(path)
        .inspect_err(|e| tracing::warn!(path = %path.display(), error = %e, "storage config: cannot read backends.toml — falling back to env vars"))
        .ok()
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

/// Build a SystemContract from backends.toml — extracts ALL declared backend names
/// (regardless of whether their env vars resolve), plus deterministic-dog.
/// This is the kernel's expected state: "what I should have at full capacity."
pub fn load_system_contract(path: &Path) -> crate::domain::contract::SystemContract {
    let mut expected_dogs = Vec::new();
    if let Ok(content) = std::fs::read_to_string(path)
        && let Ok(file) = toml::from_str::<BackendsFile>(&content)
    {
        for (name, entry) in file.backend {
            if let Some(env_name) = entry.api_key_env
                && std::env::var(env_name).is_err()
            {
                continue;
            }
            expected_dogs.push(name);
        }
    }

    let storage_required = true; // SurrealDB is required for CYNIC to function
    crate::domain::contract::SystemContract::new(expected_dogs, storage_required)
}

/// Load Dog thresholds and error detection config from backends.toml.
/// Returns DogThresholds with all configured settings — defaults can be overridden per-dog.
pub fn load_dog_thresholds(path: &Path) -> DogThresholds {
    let mut result = DogThresholds::default();

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "cannot read dog thresholds from backends.toml");
            return result;
        }
    };

    let file: BackendsFile = match toml::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "invalid TOML in dog thresholds");
            return result;
        }
    };

    // Load defaults
    if let Some(defaults_entry) = file.defaults {
        if let Some(v) = defaults_entry.failure_rate_threshold {
            result.defaults.failure_rate_threshold = v;
        }
        if let Some(v) = defaults_entry.api_error_threshold {
            result.defaults.api_error_threshold = v;
        }
        if let Some(v) = defaults_entry.transient_error_max {
            result.defaults.transient_error_max = v;
        }
        if let Some(v) = defaults_entry.quota_grace_period_secs {
            result.defaults.quota_grace_period_secs = v;
        }
        if let Some(v) = defaults_entry.evaluation_timeout_secs {
            result.defaults.evaluation_timeout_secs = v;
        }
    }

    // Load per-dog overrides from [dog.*] sections
    for (dog_name, entry) in file.dog {
        let threshold = DogThreshold {
            dog_name: dog_name.clone(),
            kind: entry.kind.unwrap_or_else(|| "inference".to_string()),
            enabled: entry.enabled.unwrap_or(true),
            failure_rate_threshold: entry.failure_rate_threshold,
            collapse_threshold: entry.collapse_threshold,
            api_error_threshold: entry.api_error_threshold,
            evaluation_timeout_secs: entry.evaluation_timeout_secs,
            priority: entry.priority,
            quota_status: entry.quota_status,
            quota_pattern: entry.quota_pattern,
            skip_reason: entry.skip_reason,
            skip_until: entry.skip_until,
        };
        result.dogs.insert(dog_name, threshold);
    }

    // Load error detection patterns
    if let Some(error_entry) = file.error_detection {
        if let Some(patterns) = error_entry.quota_patterns {
            result.error_detection.quota_patterns = patterns;
        }
        if let Some(patterns) = error_entry.transient_patterns {
            result.error_detection.transient_patterns = patterns;
        }
        if let Some(patterns) = error_entry.critical_patterns {
            result.error_detection.critical_patterns = patterns;
        }
    }

    // Load verdict settings
    if let Some(verdict_entry) = file.verdict {
        if let Some(v) = verdict_entry.min_voters {
            result.verdict.min_voters = v;
        }
        if let Some(v) = verdict_entry.min_confident_voters {
            result.verdict.min_confident_voters = v;
        }
        if let Some(v) = verdict_entry.contested_threshold {
            result.verdict.contested_threshold = v;
        }
    }

    // Load circuit breaker config
    if let Some(circuit_entry) = file.circuit {
        if let Some(v) = circuit_entry.open_on_consecutive_failures {
            result.circuit.open_on_consecutive_failures = v;
        }
        if let Some(v) = circuit_entry.open_on_failure_rate {
            result.circuit.open_on_failure_rate = v;
        }
        if let Some(v) = circuit_entry.open_duration_secs {
            result.circuit.open_duration_secs = v;
        }
        if let Some(v) = circuit_entry.half_open_eval_budget {
            result.circuit.half_open_eval_budget = v;
        }
        if let Some(v) = circuit_entry.failure_rate_recovery_window_secs {
            result.circuit.failure_rate_recovery_window_secs = v;
        }
    }

    // Load monitoring config
    if let Some(monitoring_entry) = file.monitoring {
        if let Some(v) = monitoring_entry.failure_rate_check_interval_secs {
            result.monitoring.failure_rate_check_interval_secs = v;
        }
        if let Some(v) = monitoring_entry.alert_threshold_critical {
            result.monitoring.alert_threshold_critical = v;
        }
        if let Some(v) = monitoring_entry.alert_threshold_warning {
            result.monitoring.alert_threshold_warning = v;
        }
        if let Some(v) = monitoring_entry.quota_exhaustion_alert {
            result.monitoring.quota_exhaustion_alert = v;
        }
        if let Some(v) = monitoring_entry.circuit_breaker_alert {
            result.monitoring.circuit_breaker_alert = v;
        }
    }

    // ── K-Score config ──
    if let Some(ks) = &file.kscore {
        if let Some(v) = ks.weight_diamond_hands {
            result.kscore.weight_diamond_hands = v;
        }
        if let Some(v) = ks.weight_organic_growth {
            result.kscore.weight_organic_growth = v;
        }
        if let Some(v) = ks.weight_longevity {
            result.kscore.weight_longevity = v;
        }
        if let Some(v) = ks.accumulator_threshold {
            result.kscore.accumulator_threshold = v;
        }
        if let Some(v) = ks.holder_threshold {
            result.kscore.holder_threshold = v;
        }
        if let Some(v) = ks.reducer_threshold {
            result.kscore.reducer_threshold = v;
        }
        if let Some(v) = ks.top_n_wallets {
            result.kscore.top_n_wallets = v as usize;
        }
        if let Some(v) = ks.swap_history_limit {
            result.kscore.swap_history_limit = v as usize;
        }
    }

    klog!(
        "[config] Dog thresholds loaded: {} dogs configured, kscore weights: DH={:.2}/OG={:.2}/L={:.2}, error patterns: quota({}) transient({}) critical({})",
        result.dogs.len(),
        result.kscore.weight_diamond_hands,
        result.kscore.weight_organic_growth,
        result.kscore.weight_longevity,
        result.error_detection.quota_patterns.len(),
        result.error_detection.transient_patterns.len(),
        result.error_detection.critical_patterns.len()
    );

    result
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
            prompt_tier: PromptTier::Full,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: None, // Cloud API — no health endpoint
            remediation: None,
            fleet_node: None,
            cli_extra_args: vec![],
            latency_ms: 0,
            suitable_for_domains: vec![],
            sovereign: false, // Gemini is cloud-based, not sovereign
            daily_budget: 0,
        });
    }

    configs
}

// Boot-time HTTP probing of configured backends lives in backends/health_probe.rs
// (K2: HTTP client construction is an adapter concern, not a config concern).

// ── DOMAIN PROMPTS ───────────────────────────────────────────

/// Load domain-specific axiom evaluation prompts.
/// Priority: filesystem (runtime, editable) → embedded (compile-time, fallback).
/// Filesystem prompts live in `{project_root}/domains/*.md`.
/// This enables prompt iteration without rebuild: edit file → restart kernel.
pub fn load_domain_prompts(project_root: &Path) -> std::collections::HashMap<String, String> {
    let domains_dir = project_root.join("domains");
    let mut prompts = std::collections::HashMap::new();

    // Try filesystem first — enables prompt iteration without rebuild
    if domains_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&domains_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let stripped = crate::infra::embedded_domains::strip_domain_heading(&content);
                    if !stripped.is_empty() {
                        klog!(
                            "[config] Domain prompt loaded (filesystem): '{}' ({} chars)",
                            name,
                            stripped.len()
                        );
                        prompts.insert(name.to_string(), stripped);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "failed to read domain prompt file"
                    );
                }
            }
        }
    }

    // Fallback: embedded prompts for any domain not found on filesystem
    let embedded = crate::infra::embedded_domains::load_embedded_domain_prompts();
    let mut fallback_count = 0;
    for (name, content) in embedded {
        prompts.entry(name).or_insert_with(|| {
            fallback_count += 1;
            content
        });
    }

    if prompts.is_empty() {
        klog!("[config] No domain prompts found — using generic prompts");
    } else {
        let fs_count = prompts.len() - fallback_count;
        klog!(
            "[config] {} domain prompt(s) loaded ({} filesystem, {} embedded fallback)",
            prompts.len(),
            fs_count,
            fallback_count
        );
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
    fn parse_cli_backend_type() {
        let toml_content = r#"
[backend.gemini-flash]
backend_type = "cli"
base_url = "gemini"
model = "gemini-2.5-flash"
auth_style = "none"
timeout_secs = 60
"#;
        let dir = std::env::temp_dir().join("cynic_config_cli_type_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("backends.toml");
        std::fs::write(&path, toml_content).unwrap();

        let configs = load_backends(&path);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].backend_type, BackendType::Cli);
        assert_eq!(configs[0].name, "gemini-flash");
        assert_eq!(configs[0].base_url, "gemini");
        // CLI backends must not get a derived health_url
        assert!(
            configs[0].health_url.is_none(),
            "CLI backend should have no health_url"
        );

        std::fs::remove_file(&path).ok();
    }
}
