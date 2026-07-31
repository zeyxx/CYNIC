use cynic_protocol::{SourceMaterializationObservation, SourceMaterializationRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMaterializationPortError {
    pub code: String,
    pub message: String,
}

impl SourceMaterializationPortError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait SourceSnapshotAcquisitionPort: Send + Sync {
    fn acquire(
        &self,
        snapshot_digest: &str,
        snapshot_format: &str,
    ) -> Result<(), SourceMaterializationPortError>;
}

pub trait SourceMaterializationPort: Send + Sync {
    fn observe(
        &self,
        request: &SourceMaterializationRequest,
    ) -> Result<SourceMaterializationObservation, SourceMaterializationPortError>;

    fn materialize(
        &self,
        request: &SourceMaterializationRequest,
    ) -> Result<(), SourceMaterializationPortError>;
}
