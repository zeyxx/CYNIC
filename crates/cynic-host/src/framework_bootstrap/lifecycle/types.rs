use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FrameworkLifecycleDimension {
    pub name: &'static str,
    pub desired_status: &'static str,
    pub observation_status: &'static str,
    pub convergence_status: &'static str,
    pub owner: &'static str,
    pub required_proofs: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FrameworkLifecycleAudit {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub role: String,
    pub node_id: Option<String>,
    pub dimensions: Vec<FrameworkLifecycleDimension>,
    pub blockers: Vec<String>,
    pub next_actions: Vec<String>,
    pub residual_unknowns: Vec<String>,
}
