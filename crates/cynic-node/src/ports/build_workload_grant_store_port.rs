use std::path::PathBuf;

use cynic_protocol::SignedBuildWorkloadGrant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildWorkloadGrantStoreError {
    pub code: String,
    pub detail: String,
}

impl BuildWorkloadGrantStoreError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedBuildWorkloadGrant {
    pub grant: SignedBuildWorkloadGrant,
    pub evidence_ref: String,
    pub path: PathBuf,
}

pub trait BuildWorkloadGrantStorePort: Send + Sync {
    fn latest(
        &self,
        workload_id: &str,
    ) -> Result<Option<PersistedBuildWorkloadGrant>, BuildWorkloadGrantStoreError>;

    fn persist(
        &self,
        grant: &SignedBuildWorkloadGrant,
    ) -> Result<PersistedBuildWorkloadGrant, BuildWorkloadGrantStoreError>;
}
