use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Declared,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentLoopCapability {
    pub id: &'static str,
    pub phase: &'static str,
    pub critical: bool,
    pub status: CapabilityStatus,
    pub evidence: Vec<String>,
    pub expectation: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentLoopAuditReport {
    pub schema_version: &'static str,
    pub scope: Vec<&'static str>,
    pub status: &'static str,
    pub authority: &'static str,
    pub target_percent: f64,
    pub declared_percent: f64,
    pub observed_percent: Option<f64>,
    pub hermes_component_pinned: bool,
    pub capabilities: Vec<AgentLoopCapability>,
    pub residual_risks: Vec<&'static str>,
}
