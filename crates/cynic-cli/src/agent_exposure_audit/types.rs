use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct AgentExposureAuditReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) exposure_path: String,
    pub(crate) exposed_tool_count: usize,
    pub(crate) source_tool_count: usize,
    pub(crate) domain_count: usize,
    pub(crate) baseline_runtime_count: usize,
    pub(crate) product_domain_count: usize,
    pub(crate) operator_local_count: usize,
    pub(crate) legacy_breakglass_count: usize,
    pub(crate) ok_count: usize,
    pub(crate) blocked_count: usize,
    pub(crate) findings: Vec<AgentExposureFinding>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AgentExposureFinding {
    pub(crate) severity: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) path: String,
    pub(crate) detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool: Option<String>,
}

pub(crate) struct AgentExposureCounts {
    pub(crate) exposed_tool_count: usize,
    pub(crate) source_tool_count: usize,
    pub(crate) domain_count: usize,
    pub(crate) baseline_runtime_count: usize,
    pub(crate) product_domain_count: usize,
    pub(crate) operator_local_count: usize,
    pub(crate) legacy_breakglass_count: usize,
}
