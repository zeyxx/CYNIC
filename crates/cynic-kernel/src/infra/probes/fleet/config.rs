//! Input DTOs for FleetProbe.

/// A Dog endpoint to health-check.
#[derive(Debug, Clone)]
pub struct FleetTarget {
    pub dog_name: String,
    pub health_url: String,
    /// Explicit properties URL to check actual context limits.
    pub props_url: Option<String>,
    /// URL to query loaded model identity (e.g. /v1/models).
    pub models_url: Option<String>,
    /// Expected context tokens from configuration.
    pub expected_n_ctx: u32,
    /// Expected model name from backends.toml. Used for identity verification.
    pub expected_model: String,
    /// API key for authenticated endpoints (/props, /v1/models on some backends).
    pub api_key: Option<String>,
}
