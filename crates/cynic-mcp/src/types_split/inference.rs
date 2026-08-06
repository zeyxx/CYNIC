#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InferenceDiscoverParams {
    /// Persist discovered peers into the inference SSOT only when explicitly true.
    #[serde(default)]
    pub apply: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InferenceStartParams {
    /// Model alias to start (e.g. 'qwythos-9b-q4', 'minicpm-1b-q8').
    pub alias: String,
    /// Wait for server readiness before returning (default true).
    #[serde(default = "default_true")]
    pub wait_for_ready: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InferenceSwitchParams {
    /// Target model alias to switch to.
    pub target_alias: String,
    /// Wait for server readiness before returning (default true).
    #[serde(default = "default_true")]
    pub wait_for_ready: bool,
}

fn default_true() -> bool {
    true
}
