#[derive(Debug, Deserialize, JsonSchema)]
pub struct AttentionParams {
    /// Agent id requesting the startup attention surface.
    pub agent_id: Option<String>,
    /// Optional task kind to include from the mempool. Defaults to cortex.
    pub task_kind: Option<String>,
    /// Optional task domain to include from the mempool.
    pub task_domain: Option<String>,
    /// Maximum pending tasks to include. Defaults to 10.
    pub task_limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ComplianceParams {
    /// Agent ID to score compliance for. If omitted, returns recent trend.
    pub agent_id: Option<String>,
    /// Number of recent compliance records to return (trend mode). Default: 20.
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuditQueryParams {
    pub tool: Option<String>,
    pub agent_id: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LibraryQueryParams {
    /// Observation domain, e.g. odysseum, solana, token-analysis.
    pub domain: Option<String>,
    /// Exact observation target, e.g. odysseum:pump_mechanisms.
    pub target: Option<String>,
    /// Producer agent id.
    pub agent_id: Option<String>,
    /// Project label, usually CYNIC.
    pub project: Option<String>,
    /// Coordination workstream when observations belong to a workstream.
    pub workstream: Option<String>,
    /// Coordination thread id when observations belong to a thread.
    pub thread_id: Option<String>,
    /// Maximum observations to return. Kernel caps this at 100.
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthParams {
    pub api_key: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitParams {
    pub op: crate::local_tools::GitOp,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterParams {
    pub agent_id: String,
    pub intent: String,
    pub agent_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimParams {
    pub agent_id: String,
    pub target: String,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchClaimParams {
    pub agent_id: String,
    pub targets: Vec<String>,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReleaseParams {
    pub agent_id: String,
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WhoParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ObserveParams {
    pub tool: String,
    pub target: Option<String>,
    pub domain: Option<String>,
    pub status: Option<String>,
    pub context: Option<String>,
    pub project: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub value: Option<serde_json::Value>,
    pub confidence: Option<String>,
    pub consumer: Option<String>,
    pub action: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub maturity: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationStatusPublishParams {
    pub project: String,
    pub workstream: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    pub branch: Option<String>,
    /// Canonical event types: workstream.started|workstream.blocked|workstream.updated|interface.changed|run.completed|decision.recorded.
    pub event: String,
    /// Short machine state label, e.g. "stable", "blocked", "requires-review".
    pub status: Option<String>,
    /// Human summary for humans and transport renderers.
    pub summary: String,
    /// Optional artifact references for source of truth.
    pub artifacts: Option<Vec<String>>,
    /// Optional open points.
    pub open_points: Option<Vec<String>>,
    /// Coordination thread identifier.
    pub thread_id: Option<String>,
    /// Optional payload extension (repo-specific fields).
    pub payload: Option<serde_json::Value>,
    /// Optional transport hint: "public", "team", "restricted".
    pub sensitivity: Option<String>,
    /// Contract version in payload metadata.
    #[serde(default = "default_coordination_contract_version")]
    pub contract_version: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationStatusReadParams {
    pub project: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    pub workstream: Option<String>,
    pub thread_id: Option<String>,
    pub limit: Option<u32>,
    pub domain: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationThreadReplyParams {
    pub project: String,
    pub workstream: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    pub thread_id: String,
    /// Message to post in the thread.
    pub message: String,
    pub branch: Option<String>,
    pub reply_to: Option<String>,
    /// If set, keeps a link to artifacts referenced in this reply.
    pub artifacts: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationArtifactShareParams {
    pub project: String,
    pub workstream: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    pub artifacts: Vec<String>,
    /// Optional thread context to continue discussion around shared artifacts.
    pub thread_id: Option<String>,
    pub message: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationDecisionRecordParams {
    pub project: String,
    pub workstream: String,
    /// Logical client identity invoking the coordination transport contract.
    pub client: Option<String>,
    /// Decision topic.
    pub topic: String,
    /// Decision outcome.
    pub decision: String,
    /// Decision rationale.
    pub rationale: String,
    /// Source artifact path or commit anchor.
    pub source_artifact: String,
    pub branch: Option<String>,
    pub thread_id: Option<String>,
    pub open_points: Option<Vec<String>>,
    pub artifacts: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinationContractManifestParams {
    pub client: Option<String>,
    pub project: Option<String>,
    pub operation: Option<String>,
    pub include_default_project_profile: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InfraRegistryValidateParams {
    /// Validation dimensions. Defaults to ["network_policy"].
    pub dimensions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimAgentTaskParams {
    pub task_id: String,
    pub agent_id: Option<String>,
}
