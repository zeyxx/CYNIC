//! Fresh-data probe types and predicates.
//!
//! Pure domain types for LLM provider/model discovery and probing.
//! No I/O — side effects live in adapters.

use serde::{Deserialize, Serialize};

pub const FRESH_DATA_SCHEMA: &str = "cynic.fresh_data.v1";
pub const MODEL_METADATA_SCHEMA: &str = "cynic.model_metadata.v1";

/// A single provider probe result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderProbe {
    pub provider: String,
    pub model: Option<String>,
    pub status: ProbeStatus,
    pub latency_ms: Option<u64>,
    pub response: Option<String>,
    pub error: Option<ProbeError>,
    pub credits_available: Option<String>,
    pub is_free: bool,
    pub cached: bool,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Ok,
    Degraded,
    Fail,
    Skipped,
    Unknown,
}

/// Structured probe error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

/// Model metadata (data-centric, loaded from JSON).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub provider: String,
    pub params: String,
    pub context: String,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub benchmarks: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub strengths: Vec<String>,
    #[serde(default)]
    pub weaknesses: Vec<String>,
    pub role: String,
    pub data_policy: String,
    #[serde(default)]
    pub plan_availability: Option<String>,
    #[serde(default)]
    pub api_pricing: Option<String>,
    #[serde(default)]
    pub cost_on_plan: Option<String>,
}

/// Top-level fresh-data snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshData {
    pub schema: String,
    pub timestamp: String,
    pub version: String,
    pub auth: std::collections::BTreeMap<String, AuthStatus>,
    pub probes: Vec<ProviderProbe>,
    pub quota: std::collections::BTreeMap<String, QuotaInfo>,
    pub models: std::collections::BTreeMap<String, ModelMetadata>,
    pub models_available: std::collections::BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub new_models_detected: Vec<NewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthStatus {
    pub credentials: u32,
    #[serde(default)]
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaInfo {
    #[serde(default)]
    pub raw: Option<String>,
    #[serde(default)]
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewModel {
    pub provider: String,
    pub model: String,
}

/// Freshness decision for a probe result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessDecision {
    Fresh,
    Stale,
    Unknown,
}

/// Pure predicate: decide if a probe result is fresh.
pub fn decide_freshness(
    observed_at_unix_ms: u128,
    assessed_at_unix_ms: u128,
    max_age_ms: u128,
) -> FreshnessDecision {
    if observed_at_unix_ms == 0 || assessed_at_unix_ms == 0 || max_age_ms == 0 {
        return FreshnessDecision::Unknown;
    }
    if observed_at_unix_ms > assessed_at_unix_ms {
        return FreshnessDecision::Unknown;
    }
    let age = assessed_at_unix_ms - observed_at_unix_ms;
    if age <= max_age_ms {
        FreshnessDecision::Fresh
    } else {
        FreshnessDecision::Stale
    }
}

/// Pure predicate: classify a raw probe error into a structured error.
pub fn classify_probe_error(
    provider: &str,
    model: Option<&str>,
    raw_error: &str,
) -> ProbeError {
    let err = raw_error.to_lowercase();
    let code = if err.contains("no model in cache") || err.contains("no model") {
        "no_model_in_cache"
    } else if err.contains("contributor") || err.contains("trains on your data") {
        "contributor_tier_requires_confirmation"
    } else if err.contains("no credits") || err.contains("credits exhausted") || err.contains("billing") {
        "no_credits"
    } else if err.contains("timeout") {
        "probe_timeout"
    } else if err.contains("404") || err.contains("not found") {
        "model_not_found"
    } else if err.contains("401") || err.contains("unauthorized") || err.contains("auth") {
        "auth_failed"
    } else if err.contains("model not available") {
        "model_unavailable"
    } else {
        "unknown_error"
    };
    let message = match code {
        "no_model_in_cache" => format!("Provider '{}' has no model in cache. Run 'hermes model --refresh' to populate.", provider),
        "contributor_tier_requires_confirmation" => format!("Model '{}' is Meta contributor tier (trains on data). Set security.allow_data_training_tiers_noninteractive=true in config.yaml to enable.", model.unwrap_or("unknown")),
        "no_credits" => format!("Provider '{}' has no credits. Add credits at portal or pick a free model.", provider),
        "probe_timeout" => format!("Probe for '{}' timed out. Model may be slow or unavailable.", provider),
        "model_not_found" => format!("Model '{}' not found on provider '{}'.", model.unwrap_or("unknown"), provider),
        "auth_failed" => format!("Authentication failed for '{}'. Re-run 'hermes auth add {}'.", provider, provider),
        "model_unavailable" => format!("Model '{}' is not available for probing on '{}'.", model.unwrap_or("unknown"), provider),
        _ => format!("Unknown error probing '{}': {}", provider, &raw_error[..raw_error.len().min(100)]),
    };
    ProbeError {
        code: code.to_string(),
        message,
        details: Some(raw_error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_freshness_fresh() {
        assert_eq!(decide_freshness(1000, 1100, 200), FreshnessDecision::Fresh);
    }

    #[test]
    fn decide_freshness_stale() {
        assert_eq!(decide_freshness(1000, 1300, 200), FreshnessDecision::Stale);
    }

    #[test]
    fn decide_freshness_unknown_zero() {
        assert_eq!(decide_freshness(0, 1000, 200), FreshnessDecision::Unknown);
    }

    #[test]
    fn classify_probe_error_no_model() {
        let err = classify_probe_error("agy", None, "no model in cache");
        assert_eq!(err.code, "no_model_in_cache");
    }

    #[test]
    fn classify_probe_error_contributor() {
        let err = classify_probe_error("opencode-go", Some("muse-spark-1.3-contributor"), "contributor tier requires confirmation");
        assert_eq!(err.code, "contributor_tier_requires_confirmation");
    }
}