use std::path::Path;

use cynic_protocol::RuntimeArtifactSource;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeArtifactTransferReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub source: RuntimeArtifactSource,
    pub destination: String,
    pub expected_digest: Option<String>,
    pub observed_digest: String,
    pub bytes_written: u64,
    pub mutates: bool,
    pub evidence_ref: String,
    pub blockers: Vec<String>,
}

pub trait RuntimeArtifactTransportPort {
    async fn stage(
        &self,
        source: &RuntimeArtifactSource,
        destination: &Path,
        expected_digest: Option<&str>,
    ) -> Result<RuntimeArtifactTransferReceipt, String>;
}
