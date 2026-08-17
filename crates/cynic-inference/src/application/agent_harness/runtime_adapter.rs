//! Portable boundary for external agent runtimes.
//!
//! Codex, Antigravity, Hermes, and Mercury remain replaceable executors. They
//! may only receive a CYNIC workload after node-side admission and delivery;
//! their local memory, scheduler, and task board are observations, never
//! CYNIC coordination authority.

use async_trait::async_trait;
use cynic_protocol::{AgentWorkloadDelivery, AgentWorkloadDeliveryStatus};
use serde::{Deserialize, Serialize};

pub const AGENT_RUNTIME_RECEIPT_SCHEMA: &str = "cynic.agent_runtime_receipt.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeId {
    CodexCli,
    AntigravityCli,
    HermesAgent,
    MercuryAgent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeMode {
    Interactive,
    Autonomous,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeCapability {
    ReadOnlyAudit,
    HumanApproval,
    AdmittedWorkload,
    Cancellation,
    RunObservation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentRuntimeProfile {
    pub runtime: AgentRuntimeId,
    pub mode: AgentRuntimeMode,
    pub version: Option<String>,
    pub capabilities: Vec<AgentRuntimeCapability>,
    /// A pilot is deliberately prevented from accepting a mutating workload.
    pub read_only_pilot: bool,
}

/// Declared bootstrap posture, not an observation of an installed runtime.
///
/// Every known runtime begins as a read-only pilot. An adapter may advertise
/// `AdmittedWorkload` only after its own version, identity, and execution
/// boundary have been observed through CYNIC.
pub fn declared_runtime_profile(runtime: AgentRuntimeId) -> AgentRuntimeProfile {
    let mode = match runtime {
        AgentRuntimeId::CodexCli | AgentRuntimeId::AntigravityCli => AgentRuntimeMode::Interactive,
        AgentRuntimeId::HermesAgent | AgentRuntimeId::MercuryAgent => AgentRuntimeMode::Autonomous,
    };
    let mut capabilities = vec![
        AgentRuntimeCapability::ReadOnlyAudit,
        AgentRuntimeCapability::RunObservation,
    ];
    if mode == AgentRuntimeMode::Interactive {
        capabilities.push(AgentRuntimeCapability::HumanApproval);
    }
    AgentRuntimeProfile {
        runtime,
        mode,
        version: None,
        capabilities,
        read_only_pilot: true,
    }
}

impl AgentRuntimeProfile {
    pub fn permits(&self, delivery: &AgentWorkloadDelivery) -> Result<(), AgentRuntimeError> {
        if delivery.status != AgentWorkloadDeliveryStatus::Accepted {
            return Err(AgentRuntimeError::WorkloadNotAccepted);
        }
        if self.read_only_pilot && delivery.envelope.action.mutates {
            return Err(AgentRuntimeError::ReadOnlyPilotMutation);
        }
        if !self
            .capabilities
            .contains(&AgentRuntimeCapability::AdmittedWorkload)
        {
            return Err(AgentRuntimeError::CapabilityMissing("admitted_workload"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentRuntimeReceipt {
    pub schema: String,
    pub runtime: AgentRuntimeId,
    pub workload_id: String,
    pub runtime_run_id: Option<String>,
    pub status: String,
    /// References only; raw model output and external-memory state stay at the
    /// adapter boundary until CYNIC policy admits durable evidence.
    pub evidence_refs: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

impl AgentRuntimeReceipt {
    pub fn blocked(
        runtime: AgentRuntimeId,
        workload_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            schema: AGENT_RUNTIME_RECEIPT_SCHEMA.into(),
            runtime,
            workload_id: workload_id.into(),
            runtime_run_id: None,
            status: "blocked".into(),
            evidence_refs: Vec::new(),
            residual_unknowns: vec![reason.into()],
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AgentRuntimeError {
    #[error("agent workload delivery was not accepted by its target node")]
    WorkloadNotAccepted,
    #[error("read-only pilot cannot receive a mutating workload")]
    ReadOnlyPilotMutation,
    #[error("agent runtime capability missing: {0}")]
    CapabilityMissing(&'static str),
    #[error("agent runtime adapter error: {0}")]
    Adapter(String),
}

/// Adapter port for a replaceable external agent runtime.
///
/// The caller must provide a target-node accepted delivery. Implementations
/// must not synthesize admission, mutate CYNIC coordination state, or treat an
/// upstream success message as a CYNIC completion proof.
#[async_trait]
pub trait AgentRuntimePort: Send + Sync {
    fn profile(&self) -> AgentRuntimeProfile;

    async fn prepare(
        &self,
        delivery: &AgentWorkloadDelivery,
    ) -> Result<AgentRuntimeReceipt, AgentRuntimeError>;

    async fn execute(
        &self,
        delivery: &AgentWorkloadDelivery,
    ) -> Result<AgentRuntimeReceipt, AgentRuntimeError>;

    async fn cancel(&self, workload_id: &str) -> Result<AgentRuntimeReceipt, AgentRuntimeError>;

    async fn observe(&self, workload_id: &str) -> Result<AgentRuntimeReceipt, AgentRuntimeError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_preserves_a_blocker_as_an_explicit_unknown() {
        let receipt = AgentRuntimeReceipt::blocked(
            AgentRuntimeId::MercuryAgent,
            "workload-1",
            "mercury_runtime_unobserved",
        );
        assert_eq!(receipt.schema, AGENT_RUNTIME_RECEIPT_SCHEMA);
        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.residual_unknowns,
            vec!["mercury_runtime_unobserved".to_string()]
        );
    }

    #[test]
    fn all_known_runtimes_start_as_read_only_pilots() {
        for runtime in [
            AgentRuntimeId::CodexCli,
            AgentRuntimeId::AntigravityCli,
            AgentRuntimeId::HermesAgent,
            AgentRuntimeId::MercuryAgent,
        ] {
            let profile = declared_runtime_profile(runtime);
            assert!(profile.read_only_pilot);
            assert!(
                !profile
                    .capabilities
                    .contains(&AgentRuntimeCapability::AdmittedWorkload)
            );
        }
        assert_eq!(
            declared_runtime_profile(AgentRuntimeId::MercuryAgent).mode,
            AgentRuntimeMode::Autonomous
        );
        assert_eq!(
            declared_runtime_profile(AgentRuntimeId::CodexCli).mode,
            AgentRuntimeMode::Interactive
        );
    }
}
