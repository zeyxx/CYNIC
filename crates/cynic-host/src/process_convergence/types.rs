use serde::{Deserialize, Serialize};

use cynic_eval::convergence::{
    CausalConvergenceEpoch, CausalEpochIndexEntry, ConvergenceStatus,
};
use cynic_protocol::EvidenceEnvelope;

use crate::domain::services::{ServicePolicy, ServiceUnitPolicy};

pub const PROCESS_CONVERGENCE_DECISION_SCHEMA: &str = "cynic.host.process_convergence_decision.v1";
pub const PROCESS_CONVERGENCE_RECORD_SCHEMA: &str = "cynic.host.process_convergence_record.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessConvergenceRequest {
    pub epoch_id: String,
    pub subject_id: String,
    pub service_id: String,
    pub policy: ServicePolicy,
}

impl ProcessConvergenceRequest {
    pub(crate) fn desired_unit(&self) -> Result<&ServiceUnitPolicy, String> {
        self.policy
            .units
            .iter()
            .find(|unit| {
                unit.service_id == self.service_id
                    || (unit.service_id.is_empty() && unit.unit_name == self.service_id)
            })
            .ok_or_else(|| "process_convergence_desired_service_missing".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessConvergenceDecision {
    pub schema: String,
    pub subject_id: String,
    pub vertical_id: String,
    pub service_id: String,
    pub desired_state: String,
    pub observed_status: Option<String>,
    pub read_back_status: Option<String>,
    pub requested_status: ConvergenceStatus,
    pub residual_unknowns: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessConvergenceRecord {
    pub schema: String,
    pub epoch: CausalConvergenceEpoch,
    pub epoch_evidence: EvidenceEnvelope,
    pub persisted_evidence_digests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessConvergenceHistoryItem {
    pub index: CausalEpochIndexEntry,
    pub epoch: Option<CausalConvergenceEpoch>,
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessConvergenceProjection {
    pub schema: String,
    pub subject_id: String,
    pub vertical_id: String,
    pub status: ConvergenceStatus,
    pub latest: Option<CausalConvergenceEpoch>,
    pub history: Vec<ProcessConvergenceHistoryItem>,
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
}
