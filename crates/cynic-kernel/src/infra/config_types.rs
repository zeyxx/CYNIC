//! Pure kernel configuration domain objects used by infra adapters and services.

use cynic_inference::domain::backend_config as inference_backend_config;

/// Controls how much domain context a Dog receives in its prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptTier {
    /// Generic 6-line axiom descriptions. Best for small models.
    Lite,
    /// Full domain-specific rubrics. Best for capable models.
    #[default]
    Full,
}

impl PromptTier {
    pub(crate) fn from_str_opt(s: Option<&str>) -> Self {
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

/// SurrealDB connection config loaded from `[storage]` in `backends.toml`.
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

/// One inference backend definition in `backends.toml`.
#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub name: String,
    /// Adapter used to run the backend.
    pub backend_type: BackendType,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub auth_style: AuthStyle,
    /// Max context tokens this backend supports. `0` = unknown/unlimited.
    pub context_size: u32,
    /// Timeout in seconds.
    pub timeout_secs: u64,
    /// Max completion tokens per request.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
    /// `true` disables internal "think" mode for Qwen3-like models.
    pub disable_thinking: bool,
    /// Enforce `response_format: {"type":"json_object"}` where supported.
    pub json_mode: bool,
    /// Function calling / tool-use support.
    pub supports_tool_use: bool,
    /// Prompt tier.
    pub prompt_tier: PromptTier,
    /// Cost in USD / 1M input tokens.
    pub cost_input_per_mtok: f64,
    /// Cost in USD / 1M output tokens.
    pub cost_output_per_mtok: f64,
    /// Health endpoint if available.
    pub health_url: Option<String>,
    /// Remediation plan for restart/recovery.
    pub remediation: Option<BackendRemediation>,
    /// Fleet-aware hostname for remote remediation.
    pub fleet_node: Option<String>,
    /// Extra CLI args for `backend_type = "cli"`.
    pub cli_extra_args: Vec<String>,
    /// Observed latency in milliseconds.
    pub latency_ms: u32,
    /// Domain coverage filters.
    pub suitable_for_domains: Vec<String>,
    /// Whether this is sovereign/local infrastructure.
    pub sovereign: bool,
    /// Daily request budget (0 = unlimited).
    pub daily_budget: u32,
}

impl BackendConfig {
    pub fn from_inference_backend(
        backend: inference_backend_config::InferenceBackendConfig,
    ) -> Self {
        Self {
            name: backend.name,
            backend_type: backend.backend_type,
            base_url: backend.base_url,
            api_key: backend.api_key,
            model: backend.model,
            auth_style: backend.auth_style,
            context_size: backend.context_size,
            timeout_secs: backend.timeout_secs,
            max_tokens: backend.max_tokens,
            temperature: backend.temperature,
            disable_thinking: backend.disable_thinking,
            json_mode: backend.json_mode,
            supports_tool_use: backend.supports_tool_use,
            prompt_tier: PromptTier::default(),
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: backend.health_url,
            remediation: None,
            fleet_node: backend.fleet_node,
            cli_extra_args: backend.cli_extra_args,
            latency_ms: 0,
            suitable_for_domains: Vec::new(),
            sovereign: backend.sovereign,
            daily_budget: 0,
        }
    }

    pub fn to_inference_backend_config(&self) -> inference_backend_config::InferenceBackendConfig {
        inference_backend_config::InferenceBackendConfig {
            name: self.name.clone(),
            backend_type: self.backend_type,
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            auth_style: self.auth_style.clone(),
            context_size: self.context_size,
            timeout_secs: self.timeout_secs,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            disable_thinking: self.disable_thinking,
            json_mode: self.json_mode,
            supports_tool_use: self.supports_tool_use,
            health_url: self.health_url.clone(),
            cli_extra_args: self.cli_extra_args.clone(),
            sovereign: self.sovereign,
            fleet_node: self.fleet_node.clone(),
        }
    }
}

/// Defaults shared by all Dogs.
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

/// Per-dog threshold override.
#[derive(Debug, Clone)]
pub struct DogThreshold {
    pub dog_name: String,
    pub kind: String,
    pub enabled: bool,
    pub failure_rate_threshold: Option<f64>,
    pub collapse_threshold: Option<u32>,
    pub api_error_threshold: Option<u32>,
    pub evaluation_timeout_secs: Option<u64>,
    pub priority: Option<String>,
    pub quota_status: Option<String>,
    pub quota_pattern: Option<String>,
    pub skip_reason: Option<String>,
    pub skip_until: Option<String>,
}

/// Error pattern classification.
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

/// Judge quorum configuration.
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

/// Circuit breaker configuration.
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

/// Monitoring thresholds.
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

/// K-score weighting used by `cynic-intel` style heuristics.
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

/// Auto-convergence gate configuration.
#[derive(Debug, Clone)]
pub struct ConvergenceConfig {
    pub threshold: u32,
    pub window_hours: u64,
    pub cooldown_hours: u64,
}

impl Default for ConvergenceConfig {
    fn default() -> Self {
        Self {
            threshold: 3,
            window_hours: 1,
            cooldown_hours: 6,
        }
    }
}

/// Complete threshold package loaded from `backends.toml`.
#[derive(Debug, Clone, Default)]
pub struct DogThresholds {
    pub defaults: DogDefaults,
    pub dogs: std::collections::HashMap<String, DogThreshold>,
    pub error_detection: ErrorDetection,
    pub verdict: VerdictSettings,
    pub circuit: CircuitBreakerConfig,
    pub monitoring: MonitoringConfig,
    pub kscore: KScoreConfig,
    pub convergence: ConvergenceConfig,
}

#[derive(Debug, Clone)]
pub struct BackendRemediation {
    pub node: String,
    pub restart_command: String,
    pub max_retries: u32,
    pub cooldown_secs: u64,
}

fn default_silence_threshold() -> u64 {
    3600
}
fn default_organ_max_retries() -> u32 {
    3
}
fn default_organ_cooldown() -> u64 {
    300
}

/// Remediation config for non-Dog organs discovered by `organs` table.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OrganRemediation {
    /// observation source name matching `last_observation_per_source()` rows
    pub source: String,
    /// command to restart the organ (local or remote)
    pub restart_command: String,
    /// SSH target or `localhost`.
    pub node: String,
    /// seconds of silence before triggering restart
    #[serde(default = "default_silence_threshold")]
    pub silence_threshold_secs: u64,
    /// maximum restart attempts before giving up
    #[serde(default = "default_organ_max_retries")]
    pub max_retries: u32,
    /// minimum seconds between restart attempts
    #[serde(default = "default_organ_cooldown")]
    pub cooldown_secs: u64,
}

/// Supported backend adapter families.
pub use inference_backend_config::{AuthStyle, BackendType};
