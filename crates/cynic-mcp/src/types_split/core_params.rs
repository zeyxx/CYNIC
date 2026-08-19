use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SolanaPlanParams {
    pub cluster: String,
    pub target: String,
    pub helius_plan: Option<String>,
    pub signature_page_limit: Option<u16>,
    pub max_signature_pages: Option<u32>,
    pub max_transactions: Option<u64>,
    pub expected_transactions: Option<u64>,
    pub commitment: Option<String>,
}

// ── API discovery ─────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ApiContractsParams {
    /// Which API contract to fetch: manifest, openapi, topology, or layer_alignment.
    pub contract: Option<String>,
}

// ── Docker ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DockerContainerListParams {
    /// Include stopped containers. Defaults to true.
    pub all: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DockerContainerLogsParams {
    /// Container name or id.
    pub container: String,
    /// Return the last N log lines. Defaults to 200.
    pub tail: Option<u32>,
    /// Include timestamps in the output. Defaults to true.
    pub timestamps: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DockerContainerInspectParams {
    /// Container name or id.
    pub container: String,
}

// ── Build runner (distributed build trigger + workload observation) ─────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BuildTriggerParams {
    /// Exact source commit SHA (40 hex chars) to materialize on the runner.
    pub commit: String,
    /// Target build phase: check | build | activate.
    pub phase: Option<String>,
    /// Network mode for the build: offline | online.
    pub network_mode: Option<String>,
    /// Build runner SSH host (defaults to the canonical network runner).
    pub host: Option<String>,
    /// Build runner SSH user.
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ObserveWorkloadParams {
    /// Workload id to observe.
    pub workload_id: String,
    /// Node state root on remote nodes.
    pub state_root: Option<String>,
    /// Restrict to the local store (no SSH federation).
    pub local: Option<bool>,
    /// Path to the registry JSON for node discovery.
    pub registry: Option<String>,
    /// SSH user for remote nodes.
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListWorkloadsParams {
    /// Node state root on remote nodes.
    pub state_root: Option<String>,
    /// Restrict to the local store (no SSH federation).
    pub local: Option<bool>,
    /// Path to the registry JSON for node discovery.
    pub registry: Option<String>,
    /// SSH user for remote nodes.
    pub user: Option<String>,
}
