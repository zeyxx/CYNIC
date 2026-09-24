use cynic_storage::identity::StorageEvidenceState;
use cynic_storage::identity::{StorageIdentityPlan, StorageIdentityRole};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageAdminObservation {
    pub database_identity: StorageEvidenceState,
    pub role: Option<StorageIdentityRole>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageIdentityStage {
    pub id: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageIdentityEnrollmentReport {
    pub schema: &'static str,
    pub status: String,
    pub identity_id: String,
    pub plan: StorageIdentityPlan,
    pub mutated: bool,
    pub compensated: bool,
    pub compensation_failed: bool,
    pub secret_exposed: bool,
    pub stages: Vec<StorageIdentityStage>,
    pub proofs: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

impl StorageIdentityEnrollmentReport {
    pub(super) fn blocked(plan: StorageIdentityPlan, residual_unknowns: Vec<String>) -> Self {
        Self {
            schema: "cynic.storage_identity_enrollment_report.v1",
            status: "blocked".to_string(),
            identity_id: plan.identity_id.clone(),
            plan,
            mutated: false,
            compensated: false,
            compensation_failed: false,
            secret_exposed: false,
            stages: Vec::new(),
            proofs: Vec::new(),
            residual_unknowns,
        }
    }
}
