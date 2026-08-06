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
    /// Explicit human-in-the-loop operator confirmation for heavy model switches (>= 4GB VRAM or 9B+ param class).
    pub confirmed: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct OperationalBenchmarkParams {
    /// Model alias to benchmark (e.g. 'lfm25-2b-q8', 'minicpm-1b-q8').
    pub alias: String,
    /// Target endpoint base URL (default http://127.0.0.1:8080).
    pub endpoint: Option<String>,
    /// Optional specific task_id from infra/operational-benchmarks.json.
    pub task_id: Option<String>,
}

