//! TOML schema models for kernel runtime configuration files.

use serde::Deserialize;

/// K-score weights and gates.
#[derive(Deserialize)]
pub(crate) struct KScoreEntry {
    pub weight_diamond_hands: Option<f64>,
    pub weight_organic_growth: Option<f64>,
    pub weight_longevity: Option<f64>,
    pub accumulator_threshold: Option<f64>,
    pub holder_threshold: Option<f64>,
    pub reducer_threshold: Option<f64>,
    pub top_n_wallets: Option<u64>,
    pub swap_history_limit: Option<u64>,
}

/// Default dog thresholds shared across all dogs.
#[derive(Deserialize)]
pub(crate) struct DefaultsEntry {
    pub failure_rate_threshold: Option<f64>,
    pub api_error_threshold: Option<u32>,
    pub transient_error_max: Option<u32>,
    pub quota_grace_period_secs: Option<u64>,
    pub evaluation_timeout_secs: Option<u64>,
}

/// Per-dog section.
#[derive(Deserialize)]
pub(crate) struct DogEntry {
    pub kind: Option<String>,
    pub enabled: Option<bool>,
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
#[derive(Deserialize)]
pub(crate) struct ErrorDetectionEntry {
    pub quota_patterns: Option<Vec<String>>,
    pub transient_patterns: Option<Vec<String>>,
    pub critical_patterns: Option<Vec<String>>,
}

/// Verdict/quorum settings.
#[derive(Deserialize)]
pub(crate) struct VerdictEntry {
    pub min_voters: Option<u32>,
    pub min_confident_voters: Option<u32>,
    pub contested_threshold: Option<f64>,
}

/// Circuit breaker settings.
#[derive(Deserialize)]
pub(crate) struct CircuitEntry {
    pub open_on_consecutive_failures: Option<u32>,
    pub open_on_failure_rate: Option<bool>,
    pub open_duration_secs: Option<u64>,
    pub half_open_eval_budget: Option<u32>,
    pub failure_rate_recovery_window_secs: Option<u64>,
}

/// Monitoring settings.
#[derive(Deserialize)]
pub(crate) struct MonitoringEntry {
    pub failure_rate_check_interval_secs: Option<u64>,
    pub alert_threshold_critical: Option<f64>,
    pub alert_threshold_warning: Option<f64>,
    pub quota_exhaustion_alert: Option<bool>,
    pub circuit_breaker_alert: Option<bool>,
}

/// Storage configuration section.
#[derive(Deserialize)]
pub(crate) struct StorageEntry {
    pub url: Option<String>,
    pub allow_loopback: Option<bool>,
    pub namespace: Option<String>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub secret_env: Option<String>,
    pub secret_file: Option<String>,
    pub os_credential: Option<String>,
}

/// Dedicated storage runtime document. Storage deliberately has its own
/// parser so inference schema changes cannot redirect the kernel to a fallback
/// database endpoint.
#[derive(Deserialize)]
pub(crate) struct StorageFile {
    pub storage: Option<StorageEntry>,
}

/// Backend remediation block.
#[derive(Deserialize)]
pub(crate) struct RemediationEntry {
    pub node: String,
    pub restart_command: String,
    pub max_retries: Option<u32>,
    pub cooldown_secs: Option<u64>,
}

/// Backend section.
#[derive(Deserialize)]
pub(crate) struct BackendEntry {
    pub base_url: String,
    pub backend_type: Option<String>,
    pub api_key_env: Option<String>,
    pub model: String,
    pub auth_style: Option<String>,
    pub context_size: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub disable_thinking: Option<bool>,
    pub json_mode: Option<bool>,
    pub supports_tool_use: Option<bool>,
    pub prompt_tier: Option<String>,
    pub cost_input_per_mtok: Option<f64>,
    pub cost_output_per_mtok: Option<f64>,
    /// Explicit health URL — if omitted, derived for sovereign HTTP backends.
    pub health_url: Option<String>,
    /// Inline remediation config.
    pub remediation: Option<RemediationEntry>,
    /// Tailscale hostname for fleet awareness.
    pub fleet_node: Option<String>,
    /// Extra CLI args as whitespace-separated string.
    pub cli_extra_args: Option<String>,
    /// Observed scheduling latency.
    pub latency_ms: Option<u32>,
    /// Domains this dog is suitable for.
    pub suitable_for_domains: Option<Vec<String>>,
    /// Explicit sovereign flag.
    pub sovereign: Option<bool>,
    /// Daily quota.
    pub daily_budget: Option<u32>,
}

/// Convergence settings.
#[derive(Deserialize)]
pub(crate) struct ConvergenceEntry {
    pub threshold: Option<u32>,
    pub window_hours: Option<u64>,
    pub cooldown_hours: Option<u64>,
}

/// Top-level kernel TOML document.
#[derive(Deserialize)]
pub(crate) struct BackendsFile {
    pub backend: std::collections::HashMap<String, BackendEntry>,
    pub storage: Option<StorageEntry>,
    pub defaults: Option<DefaultsEntry>,
    #[serde(default)]
    pub dog: std::collections::HashMap<String, DogEntry>,
    pub error_detection: Option<ErrorDetectionEntry>,
    pub verdict: Option<VerdictEntry>,
    pub circuit: Option<CircuitEntry>,
    pub monitoring: Option<MonitoringEntry>,
    pub kscore: Option<KScoreEntry>,
    pub convergence: Option<ConvergenceEntry>,
}

#[derive(Deserialize)]
pub(crate) struct OrgansFile {
    #[serde(default)]
    pub organs: Vec<crate::infra::config_types::OrganRemediation>,
}
