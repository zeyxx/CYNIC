use serde::Serialize;

use super::adoption::AgentRuntimeAdoptionPlan;

pub const AGENT_RUNTIME_INVENTORY_SCHEMA: &str = "cynic.agent_runtime_inventory.v1";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeMode {
    Interactive,
    Autonomous,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeInstallOwnership {
    RuntimePrincipal,
    ExternalPrincipal,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeInstallChannel {
    Scripted,
    DistroManaged,
    Distributed,
    Unclassified,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRuntimeInstallProvenance {
    pub payload_owner: AgentRuntimeInstallOwnership,
    pub install_channel: AgentRuntimeInstallChannel,
    pub reason: String,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AgentRuntimeSpec {
    pub id: &'static str,
    pub mode: AgentRuntimeMode,
    pub version_argv: &'static [&'static str],
    pub status_argv: Option<&'static [&'static str]>,
    pub status_projection: StatusProjection,
}

impl AgentRuntimeSpec {
    pub const fn new(
        id: &'static str,
        mode: AgentRuntimeMode,
        version_argv: &'static [&'static str],
        status_argv: Option<&'static [&'static str]>,
        status_projection: StatusProjection,
    ) -> Self {
        Self {
            id,
            mode,
            version_argv,
            status_argv,
            status_projection,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum StatusProjection {
    None,
    Hermes,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Observed,
    Degraded,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeActivation {
    Blocked,
    Discovered,
    SupervisedReadOnly,
    InteractiveAdmitted,
    AutonomousCandidate,
    AutonomousActive,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRuntimeSupervision {
    pub activation: AgentRuntimeActivation,
    pub retained_operations: Vec<&'static str>,
    pub blocked_operations: Vec<&'static str>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityNode {
    pub id: &'static str,
    pub state: CapabilityState,
    pub depends_on: Vec<&'static str>,
    pub retained_capabilities: Vec<&'static str>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRuntimeObservation {
    pub id: &'static str,
    pub mode: AgentRuntimeMode,
    pub version: Option<String>,
    pub install_provenance: AgentRuntimeInstallProvenance,
    pub graph: Vec<CapabilityNode>,
    pub supervision: AgentRuntimeSupervision,
    pub adoption: Option<AgentRuntimeAdoptionPlan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRuntimeInventoryReport {
    pub schema: &'static str,
    pub authority: &'static str,
    pub runtimes: Vec<AgentRuntimeObservation>,
}
