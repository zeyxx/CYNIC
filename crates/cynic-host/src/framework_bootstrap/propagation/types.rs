use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FrameworkPropagationProbe {
    pub phase: &'static str,
    pub argv: Vec<String>,
    pub status: &'static str,
    pub exit_code: Option<i32>,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkPropagationObservation {
    pub schema: &'static str,
    pub status: &'static str,
    pub ssh_authenticated: bool,
    pub remote_principal_status: &'static str,
    pub remote_principal: Option<String>,
    pub runtime_marker_status: &'static str,
    pub runtime_marker: Option<serde_json::Value>,
    pub node_observation_status: &'static str,
    pub node_observation: Option<serde_json::Value>,
    pub probes: Vec<FrameworkPropagationProbe>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FrameworkPropagationDecision {
    pub schema: &'static str,
    pub status: &'static str,
    pub next_action: &'static str,
    pub authority_class: Option<&'static str>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FrameworkPropagationOperation {
    pub phase: &'static str,
    pub argv: Vec<String>,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub proof: Option<&'static str>,
    pub expected_stdout_sha256: Option<String>,
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
    pub enrollment_authority_class: String,
    pub enrollment_principal: String,
    pub enrollment_credential_ref: String,
    pub bundle_dir: String,
    pub seed_id: String,
    pub expected_runtime_basket_digest: String,
    pub observation: FrameworkPropagationObservation,
    pub decision: FrameworkPropagationDecision,
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
    pub decision: &'static str,
    pub executions: Vec<FrameworkPropagationExecution>,
    pub read_back_evidence: Option<serde_json::Value>,
    pub blockers: Vec<String>,
    pub rollback_intent: &'static str,
}
