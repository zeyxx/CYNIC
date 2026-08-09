use cynic_protocol::RuntimeCompatibilityPlan;
use serde::Serialize;

pub const MCP_RUNTIME_OBSERVATION_SCHEMA: &str = "cynic.mcp.runtime_observation.v1";

#[derive(Debug, Clone, Serialize)]
pub struct McpRuntimeObservation {
    pub schema: &'static str,
    pub status: String,
    pub mutates: bool,
    pub binary: String,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub compatibility: Option<RuntimeCompatibilityPlan>,
    pub required_actions: Vec<String>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
}
