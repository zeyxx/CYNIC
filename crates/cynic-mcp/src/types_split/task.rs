#[derive(Debug, Deserialize, JsonSchema)]
pub struct JudgeParams {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    pub dogs: Option<Vec<String>>,
    pub agent_id: Option<String>,
    pub crystals: Option<bool>,
    /// Optional: sensitivity level. "high" forces routing to sovereign (local) Dogs only.
    /// Use for private content: DMs, wallet seeds, API keys. Default: none (auto-detected).
    pub sensitivity: Option<String>,
    /// Optional: slot acquisition priority. Controls queuing on sovereign inference.
    /// Values: "user" (30s wait, default), "hermes" (15s), "nightshift" (skip if busy), "background" (skip).
    pub priority: Option<String>,
}

/// Parameters for the `cynic_judgment` tool — projects the runtime's current
/// state into a single CynicJudgment through the three orthogonal lenses
/// (DIKW, SAGE, Cynefin).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct JudgmentParams {
    /// Optional subject id (defaults to local hostname).
    pub subject: Option<String>,
    /// Optional: promote the judgment N DIKW layers up before returning.
    /// 0 = observe only, 1 = promote once, etc.
    pub promote_layers: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListParams {
    pub limit: Option<u32>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InferParams {
    pub system: Option<String>,
    pub prompt: String,
    pub agent_id: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MetabolismParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DispatchAgentTaskParams {
    pub kind: String,   // "hermes" | "nightshift" | future agent
    pub domain: String, // "twitter" | "token" | "on-chain"
    /// JSON task payload. Repo-affecting hermes tasks must declare coord_targets, targets, repo_targets, or files.
    pub content: String,
    pub agent_id: Option<String>,
    pub due_at: Option<String>,
    pub priority: Option<i64>,
    pub consumer: Option<String>,
    pub escalation_policy: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPendingAgentTasksParams {
    pub kind: String,
    pub domain: Option<String>,
    pub limit: Option<u32>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WatchPendingAgentTasksParams {
    pub kind: String,
    pub domain: Option<String>,
    pub limit: Option<u32>,
    pub timeout_secs: Option<u32>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateAgentTaskResultParams {
    pub task_id: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub agent_id: Option<String>,
    pub kind: Option<String>,
    pub domain: Option<String>,
    pub limit: Option<u32>,
}
