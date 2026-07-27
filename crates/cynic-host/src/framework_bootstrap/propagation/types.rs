use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FrameworkPropagationOperation {
    pub phase: &'static str,
    pub argv: Vec<String>,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub proof: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkPropagationPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub node_id: String,
    pub role: String,
    pub ssh_target: String,
    pub trust: String,
    pub bundle_dir: String,
    pub seed_id: String,
    pub blockers: Vec<String>,
    pub operations: Vec<FrameworkPropagationOperation>,
    pub stop_condition: &'static str,
    pub rollback_intent: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkPropagationExecution {
    pub phase: String,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
    pub proof: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkPropagationReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub node_id: String,
    pub role: String,
    pub seed_id: String,
    pub executions: Vec<FrameworkPropagationExecution>,
    pub read_back_evidence: Option<serde_json::Value>,
    pub blockers: Vec<String>,
    pub rollback_intent: &'static str,
}
