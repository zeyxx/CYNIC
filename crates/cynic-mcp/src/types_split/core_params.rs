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
